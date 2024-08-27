/*
 * Crab, a UCI-compatible chess engine
 * Copyright (C) 2024 Jasper Shovelton
 *
 * Crab is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Crab is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Crab. If not, see <https://www.gnu.org/licenses/>.
 */

use std::marker::PhantomData;

use crate::{
    board::Board,
    evaluation::Evaluation,
    movegen::{
        generate_moves, AllMoves, CapturesOnly, KingMovesOnly, Move, Moves, MovesType, QuietsOnly,
        ScoredMove,
    },
};

/// A [`MovePicker`] for the main search that searches all moves.
pub type AllMovesPicker = MovePicker<AllMoves>;
/// A [`MovePicker`] for the quiescence search that searches only captures
/// and/or evasions.
///
/// Whether or not it generates king quiet moves is given by the type parameter
/// to `new`.
pub type QuiescenceMovePicker = MovePicker<CapturesOnly>;

/// The stage of move picking.
#[derive(PartialEq)]
enum Stage {
    /// Return the TT move.
    TtMove,
    /// Generate all captures.
    GenerateCaptures,
    /// Return all good captures.
    GoodCaptures,
    /// Return the first killer.
    FirstKiller,
    /// Return the second killer.
    SecondKiller,
    /// Generate all remaining moves (i.e. quiets).
    GenerateRemaining,
    /// Return all remaining moves (bad captures and quiets).
    Remaining,
}

/// A selector of the next best move in a position.
#[allow(clippy::missing_docs_in_private_items)]
pub struct MovePicker<Type: MovesType> {
    tt_move: Option<Move>,
    killers: [Option<Move>; 2],
    stage: Stage,
    moves: Moves,
    /// `Type::KING_QUIETS` will always be false for quiescence moves. To see
    /// if a quiescence move picker generates king quiet moves, this parameter
    /// is used instead. `!Type::NON_KING_QUIETS && self.do_quiets` means
    /// generate king quiets but not regular quiets.
    do_quiets: bool,
    _type: PhantomData<Type>,
}

impl<Type: MovesType> MovePicker<Type> {
    /// Skip any future quiets.
    pub fn skip_quiets(&mut self) {
        self.do_quiets = false;
    }

    /// Return the next best [`Move`] in the list of legal moves.
    pub fn next(&mut self, board: &Board) -> Option<Move> {
        if self.stage == Stage::TtMove {
            self.stage = Stage::GenerateCaptures;
            if self.tt_move.is_some() {
                return self.tt_move;
            }
        }

        if self.stage == Stage::GenerateCaptures {
            self.stage = Stage::GoodCaptures;
            generate_moves::<CapturesOnly>(board, &mut self.moves);
            // SAFETY: either `self.moves.len() - 1` is a valid index,
            // or it's 0, in which case `moves[0..0]` will return an
            // empty array
            unsafe { self.score::<CapturesOnly>(board, 0, self.moves.len()) };
        }

        if self.stage == Stage::GoodCaptures {
            if let Some(scored_move) = self.find_next_best(board) {
                return Some(scored_move.mv);
            }

            if Type::NON_KING_QUIETS {
                self.stage = Stage::FirstKiller;
            } else {
                // this also skips bad captures
                if !self.do_quiets {
                    return None;
                }

                self.stage = Stage::GenerateRemaining;
            }
        }

        if self.stage == Stage::FirstKiller {
            self.stage = Stage::SecondKiller;
            if self.do_quiets && self.killers[0] != self.tt_move {
                if let Some(mv) = self.killers[0] {
                    if board.is_pseudolegal_killer(mv) {
                        return Some(mv);
                    }
                }
            }
        }

        if self.stage == Stage::SecondKiller {
            self.stage = Stage::GenerateRemaining;
            if self.do_quiets && self.killers[1] != self.tt_move {
                if let Some(mv) = self.killers[1] {
                    if board.is_pseudolegal_killer(mv) {
                        return Some(mv);
                    }
                }
            }
        }

        if self.stage == Stage::GenerateRemaining {
            self.stage = Stage::Remaining;
            let total_non_quiets = self.moves.len();
            if Type::NON_KING_QUIETS {
                generate_moves::<QuietsOnly>(board, &mut self.moves);
                // SAFETY: `total_non_quiets..self.moves.len()` is
                // always valid
                unsafe {
                    self.score::<QuietsOnly>(board, total_non_quiets, self.moves.len());
                }
            } else if self.do_quiets {
                generate_moves::<KingMovesOnly>(board, &mut self.moves);
                // SAFETY: `total_non_quiets..self.moves.len()` is
                // always valid
                unsafe {
                    self.score::<KingMovesOnly>(board, total_non_quiets, self.moves.len());
                }
            }
        }

        debug_assert!(self.stage == Stage::Remaining, "unhandled stage");
        if self.do_quiets {
            self.find_next_best(board).map(|scored_move| scored_move.mv)
        } else {
            None
        }
    }

    /// Find the next best move in the current list of generated moves.
    fn find_next_best(&mut self, board: &Board) -> Option<ScoredMove> {
        loop {
            if self.moves.is_empty() {
                return None;
            }

            let mut best_score = -Evaluation::INFINITY;
            let mut best_index = 0;
            for (index, scored_move) in self.moves.iter().enumerate() {
                if scored_move.score > best_score {
                    best_score = scored_move.score;
                    best_index = index;
                }
            }

            // SAFETY: `best_index` was created from within `self.moves` so it
            // must be valid
            let scored_move = unsafe { self.moves.get_unchecked_mut(best_index) };

            if self.tt_move == Some(scored_move.mv)
                || self.killers[0] == Some(scored_move.mv)
                || self.killers[1] == Some(scored_move.mv)
            {
                self.moves.remove(best_index);
                continue;
            }

            if best_score >= ScoredMove::WINNING_CAPTURE_SCORE
                && !board.is_winning_exchange(scored_move.mv)
            {
                scored_move.score -= ScoredMove::WINNING_CAPTURE_SCORE;
                continue;
            }

            if self.stage == Stage::GoodCaptures
                && scored_move.score < ScoredMove::WINNING_CAPTURE_SCORE
            {
                return None;
            }

            return Some(self.moves.remove(best_index));
        }
    }

    /// Scores the moves in `moves[start..end]`, given the information in
    /// `search_info` and the current height.
    ///
    /// The slice does not bounds check: if `moves[start..end]` would have
    /// panicked, this function will have undefined behaviour.
    unsafe fn score<T: MovesType>(&mut self, board: &Board, start: usize, end: usize) {
        // SAFETY: it's up to the caller to make sure this index is safe
        let moves = unsafe { self.moves.get_unchecked_mut(start..end).iter_mut() };
        for mv in moves {
            mv.score::<T>(board);
        }
    }
}

impl AllMovesPicker {
    /// Creates a new [`MovePicker`] for all moves based on the information in
    /// `board` and `tt_move`.
    pub fn new(tt_move: Option<Move>, killers: [Option<Move>; 2]) -> Self {
        Self {
            tt_move,
            killers,
            stage: Stage::TtMove,
            moves: Moves::new(),
            do_quiets: true,
            _type: PhantomData,
        }
    }
}

impl QuiescenceMovePicker {
    /// Creates a new [`MovePicker`] for captures only (and optionally king
    /// quiet moves).
    pub fn new<Type: MovesType>() -> Self {
        assert!(
            !Type::NON_KING_QUIETS,
            "generating quiet moves for a quiescence move picker"
        );

        Self {
            tt_move: None,
            killers: [None; 2],
            stage: Stage::GenerateCaptures,
            moves: Moves::new(),
            do_quiets: Type::KING_QUIETS,
            _type: PhantomData,
        }
    }
}
