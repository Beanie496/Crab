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

use std::{
    fmt::{self, Display, Formatter},
    ops::{Add, AddAssign, Neg, SubAssign},
};

use crate::{
    board::Board,
    defs::{Piece, Side},
};

use values::create_piece_values;

/// Values related to evaluation.
pub mod values;

/// The result of an evaluation.
pub type Eval = i16;

/// The highest possible (positive) evaluation.
pub const INF_EVAL: Eval = Eval::MAX;
/// The evaluation of a draw.
pub const DRAW: Eval = 0;
/// The piece values for White and black, with an extra value of 0 at the end
/// to allow [`Piece::NONE`] to index into it.
///
/// `PIECE_VALUES[Piece::PIECE_TYPE] == value for that piece type and side`.
pub static PIECE_VALUES: [Score; Piece::TOTAL + 1] = create_piece_values();

/// A blend between middlegame value and endgame value.
#[derive(Clone, Copy)]
pub struct Score(pub Eval);

impl Add for Score {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Display for Score {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Score({})", self.0)
    }
}

impl Neg for Score {
    type Output = Self;

    fn neg(mut self) -> Self {
        self.0 = -self.0;
        self
    }
}

impl SubAssign for Score {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

/// Calculates a static evaluation of the current board.
pub fn evaluate(board: &Board) -> Eval {
    let eval = board.eval().0;
    if board.side_to_move() == Side::WHITE {
        eval
    } else {
        -eval
    }
}
