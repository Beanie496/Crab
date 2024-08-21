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

use crate::{
    board::Board,
    defs::{Piece, PieceType, Side, Square},
    evaluation::Eval,
    lookups::ray_between,
    movegen::{
        generate_moves, CapturesOnly, KingMovesOnly, Move, Moves, MovesType, QuietsOnly, ScoredMove,
    },
};

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
pub struct MovePicker {
    tt_move: Option<Move>,
    killers: [Option<Move>; 2],
    stage: Stage,
    moves: Moves,
    skip_non_king_quiets: bool,
    skip_king_quiets: bool,
}

impl MovePicker {
    /// Creates a new [`MovePicker`] based on the information in `board` and
    /// `tt_move`.
    pub fn new<Type: MovesType>(tt_move: Option<Move>, killers: [Option<Move>; 2]) -> Self {
        assert!(
            Type::CAPTURES,
            "the movepicker relies on always generating captures"
        );
        Self {
            tt_move,
            killers,
            stage: Stage::TtMove,
            moves: Moves::new(),
            skip_non_king_quiets: !Type::NON_KING_QUIETS,
            skip_king_quiets: !Type::KING_QUIETS,
        }
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
            // this also skips bad captures
            if self.skip_non_king_quiets && self.skip_king_quiets {
                return None;
            }
            self.stage = Stage::FirstKiller;
        }

        if self.stage == Stage::FirstKiller {
            self.stage = Stage::SecondKiller;
            if self.killers[0] != self.tt_move {
                if let Some(mv) = self.killers[0] {
                    if is_pseudolegal_killer(board, mv) {
                        return Some(mv);
                    }
                }
            }
        }

        if self.stage == Stage::SecondKiller {
            self.stage = Stage::GenerateRemaining;
            if self.killers[1] != self.tt_move {
                if let Some(mv) = self.killers[1] {
                    if is_pseudolegal_killer(board, mv) {
                        return Some(mv);
                    }
                }
            }
        }

        if self.stage == Stage::GenerateRemaining {
            self.stage = Stage::Remaining;
            let total_non_quiets = self.moves.len();
            if self.skip_non_king_quiets {
                generate_moves::<KingMovesOnly>(board, &mut self.moves);
                // SAFETY: `total_non_quiets..self.moves.len()` is
                // always valid
                unsafe {
                    self.score::<KingMovesOnly>(board, total_non_quiets, self.moves.len());
                }
            } else {
                generate_moves::<QuietsOnly>(board, &mut self.moves);
                // SAFETY: `total_non_quiets..self.moves.len()` is
                // always valid
                unsafe {
                    self.score::<QuietsOnly>(board, total_non_quiets, self.moves.len());
                }
            }
        }

        debug_assert!(self.stage == Stage::Remaining, "unhandled stage");
        self.find_next_best(board).map(|scored_move| scored_move.mv)
    }

    /// Find the next best move in the current list of generated moves.
    fn find_next_best(&mut self, board: &Board) -> Option<ScoredMove> {
        loop {
            if self.moves.is_empty() {
                return None;
            }

            let mut best_score = -Eval::MAX;
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
    unsafe fn score<Type: MovesType>(&mut self, board: &Board, start: usize, end: usize) {
        // SAFETY: it's up to the caller to make sure this index is safe
        let moves = unsafe { self.moves.get_unchecked_mut(start..end).iter_mut() };
        for mv in moves {
            mv.score::<Type>(board);
        }
    }
}

/// Checks if `mv` is a legal killer on `board`, assuming it was legal in
/// the previous same-depth search.
fn is_pseudolegal_killer(board: &Board, mv: Move) -> bool {
    let start = mv.start();
    let end = mv.end();

    let piece = board.piece_on(start);
    let piece_type = PieceType::from(piece);
    // this might be wrong so it needs to be checked before it's used
    let piece_side = Side::from(piece);
    let captured = board.piece_on(end);
    let captured_type = PieceType::from(captured);
    // this also might be wrong
    let captured_side = Side::from(captured);

    // check the piece still exists (en passant can delete it) and hasn't been
    // captured
    if piece == Piece::NONE || piece_side != board.side_to_move() {
        return false;
    }

    // check we aren't capturing a friendly piece
    if captured != Piece::NONE && captured_side == board.side_to_move() {
        return false;
    }

    // check we weren't blocked
    if !(ray_between(start, end) & board.occupancies()).is_empty() {
        return false;
    }

    // check we aren't capturing a king
    if captured_type == PieceType::KING {
        return false;
    }

    // if the piece is a pawn, do some additional checks
    if piece_type == PieceType::PAWN && !is_pseudolegal_pawn_killer(board, mv) {
        return false;
    }

    if mv.is_castling() {
        let rook_start = Square(end.0.wrapping_add_signed(mv.rook_offset()));
        if !(ray_between(start, rook_start) & board.side_any(captured_side)).is_empty() {
            return false;
        }
        if board.piece_on(rook_start) != Piece::from_piecetype(PieceType::ROOK, piece_side) {
            return false;
        }
    }

    true
}

/// Checks if `mv` is a legal pawn killer, given the same assumptions as
/// [`is_pseudolegal_killer()`] and assuming the move is a pawn move.
fn is_pseudolegal_pawn_killer(board: &Board, mv: Move) -> bool {
    // small optimisation: if the best response to the first move was en
    // passant, it is impossible for that same en passant move to be legal
    // after any other move
    if mv.is_en_passant() {
        return false;
    }

    let start = mv.start();
    let end = mv.end();
    let diff = start.0.abs_diff(end.0);
    // a piece getting between the start and end of a double push was already
    // checked
    let is_push = diff == 8 || diff == 16;
    let is_piece_on_end = board.piece_on(end) != Piece::NONE;

    // check that there isn't a piece blocking us if we're pushing or that
    // there is a piece if we're capturing
    is_push && !is_piece_on_end || !is_push && is_piece_on_end
}
