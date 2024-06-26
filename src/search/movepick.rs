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

use std::cmp::Ordering;

use crate::{
    board::Board,
    defs::{MoveType, PieceType},
    evaluation::{Eval, INF_EVAL},
    movegen::{generate_moves, Move, Moves, MAX_LEGAL_MOVES},
    util::Stack,
};

/// A selector of the next best move in a position.
#[allow(clippy::missing_docs_in_private_items)]
pub struct MovePicker {
    moves: ScoredMoves,
}

/// A [`Move`] that has been given a certain score.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy)]
pub struct ScoredMove {
    mv: Move,
    score: Eval,
}

/// A scored stack of [`ScoredMove`]s.
#[allow(clippy::missing_docs_in_private_items)]
pub struct ScoredMoves {
    moves: Stack<ScoredMove, MAX_LEGAL_MOVES>,
}

/// The score of a quiet move.
const QUIET_SCORE: Eval = 5_000;
/// The score of a move found in the transposition table.
const TT_SCORE: Eval = INF_EVAL;
/// The score of a capture with a winning static exchange evaluation.
const WINNING_CAPTURE_SCORE: Eval = 10_000;

impl Iterator for MovePicker {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.moves.next()
    }
}

impl Eq for ScoredMove {}

impl Ord for ScoredMove {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialEq for ScoredMove {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl PartialOrd for ScoredMove {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.score.cmp(&other.score))
    }
}

impl FromIterator<ScoredMove> for ScoredMoves {
    fn from_iter<Moves: IntoIterator<Item = ScoredMove>>(other_stack: Moves) -> Self {
        let mut stack = Stack::new();

        for item in other_stack {
            stack.push(item);
        }

        Self { moves: stack }
    }
}

impl Iterator for ScoredMoves {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop().map(|scored_move| scored_move.mv)
    }
}

impl MovePicker {
    /// Creates a new [`MovePicker`] based on the information in `board` and
    /// `tt_move`.
    ///
    /// If `tt_move == Move::null()`, it will be ignored.
    pub fn new<const MOVE_TYPE: u8>(board: &Board, tt_move: Move) -> Self {
        let mut moves = generate_moves::<MOVE_TYPE>(board).score::<MOVE_TYPE>(board, tt_move);
        moves.sort();
        Self { moves }
    }
}

impl Moves {
    /// Scores the moves in `moves`, given the information in `search_info` and
    /// the current height.
    pub fn score<const MOVE_TYPE: u8>(self, board: &Board, tt_move: Move) -> ScoredMoves {
        self.map(|mv| ScoredMove::new::<MOVE_TYPE>(board, mv, tt_move))
            .collect()
    }
}

impl ScoredMove {
    /// Scores a [`Move`].
    pub fn new<const MOVE_TYPE: u8>(board: &Board, mv: Move, tt_move: Move) -> Self {
        if MOVE_TYPE != MoveType::CAPTURES && mv == tt_move {
            return Self {
                mv,
                score: TT_SCORE,
            };
        }

        let captured_piece = if mv.is_en_passant() {
            PieceType::PAWN
        } else if mv.is_promotion() {
            PieceType(mv.promotion_piece().0 - PieceType::PAWN.0)
        } else {
            PieceType::from(board.piece_on(mv.end()))
        };

        if captured_piece == PieceType::NONE {
            return Self {
                mv,
                score: QUIET_SCORE,
            };
        }

        debug_assert!(
            captured_piece != PieceType::KING,
            "How are you capturing a king?"
        );

        let mut score = captured_piece.mvv_bonus();
        if board.is_winning_exchange(mv) {
            score += WINNING_CAPTURE_SCORE;
        }
        Self { mv, score }
    }
}

impl ScoredMoves {
    /// Sorts the scored moves.
    pub fn sort(&mut self) {
        self.moves.sort_by(Ord::cmp);
    }

    /// Returns the last move.
    ///
    /// Assumes the moves have already been sorted.
    fn pop(&mut self) -> Option<ScoredMove> {
        self.moves.pop()
    }
}
