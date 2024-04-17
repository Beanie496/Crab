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

use std::ops::{Add, AddAssign, Neg, SubAssign};

use crate::{
    board::Board,
    defs::{Piece, Side, Square},
    search::Depth,
};

use values::create_piece_square_tables;

/// Values related to evaluation.
pub mod values;

/// The result of an evaluation.
pub type Eval = i16;
/// The phase of the game, represented by the sum of the weights of the pieces
/// of the board.
///
/// `0` is the endgame (because fewer pieces is closer to an endgame) and `24`
/// is the middlegame (because the sum of the weights of the pieces of the
/// starting position is 24). `> 24` is allowed: it happens with early
/// promotion. It should be treated as 24.
pub type Phase = u8;

/// The highest possible (positive) evaluation.
pub const INF_EVAL: Eval = Eval::MAX;
/// The evaluation of a mate.
pub const MATE: Eval = INF_EVAL;
/// The lowest score a mate can have.
pub const MATE_BOUND: Eval = MATE - Depth::MAX as Eval;
/// The evaluation of a draw.
pub const DRAW: Eval = 0;

/// The piece-square tables for White and black, with an extra table of 0's to
/// allow [`Piece::NONE`] to index into it. The piece values are baked in.
///
/// `PIECE_SQUARE_TABLES[Piece::PIECE_TYPE][Square::SQUARE] == value for that
/// piece type and side on that square`.
pub static PIECE_SQUARE_TABLES: [[Score; Square::TOTAL]; Piece::TOTAL + 1] =
    create_piece_square_tables();
/// The weight of each piece towards the phase.
///
/// Order: pawn, knight, bishop, rook, queen, king. Each piece has two values:
/// one per side. The order of those two values depends on the order of
/// [`Side::WHITE`] and [`Side::BLACK`]. An extra `0` is added at the end to
/// allow [`Piece::NONE`] to index into it.
pub static PHASE_WEIGHTS: [Phase; Piece::TOTAL + 1] = [0, 0, 1, 1, 1, 1, 2, 2, 4, 4, 0, 0, 0];

/// A blend between a middlegame and endgame value.
#[derive(Clone, Copy)]
pub struct Score(pub Eval, pub Eval);

impl Add for Score {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl Neg for Score {
    type Output = Self;

    fn neg(mut self) -> Self {
        self.0 = -self.0;
        self.1 = -self.1;
        self
    }
}

impl SubAssign for Score {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl Score {
    /// Lerps the score between its middlegame and endgame value depending on
    /// the phase.
    fn lerp_to(self, phase: Phase) -> Eval {
        let phase = Eval::from(phase.min(24));
        let diff = self.1 - self.0;
        self.1 - (diff * phase) / 24
    }
}

/// Calculates a static evaluation of the current board.
pub fn evaluate(board: &Board) -> Eval {
    let phase = board.phase();
    let score = board.score();

    let eval = score.lerp_to(phase);

    if board.side_to_move() == Side::WHITE {
        eval
    } else {
        -eval
    }
}

/// Calculates the evaluation if we're mating in `depth` halfmoves.
pub fn mate_in(depth: Depth) -> Eval {
    MATE - Eval::from(depth)
}

/// Calculates the evaluation if we're getting mated in `depth` halfmoves.
pub fn mated_in(depth: Depth) -> Eval {
    -MATE + Eval::from(depth)
}

/// Checks if the score is low or high enough to be a mate score.
pub const fn is_mate(score: Eval) -> bool {
    score >= MATE_BOUND || score <= -MATE_BOUND
}

/// Calculates the number of fullmoves to a mate.
///
/// A positive number represents fullmoves to performing a mate and a negative
/// number means fullmoves to being mated.
pub const fn moves_to_mate(score: Eval) -> i16 {
    if score > 0 {
        (MATE - score + 1) / 2
    } else {
        (-MATE - score) / 2
    }
}
