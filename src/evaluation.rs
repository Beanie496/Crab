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
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    iter::Sum,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::{
    board::Board,
    defs::{Piece, Side, Square},
    search::{Depth, Height},
    util::get_unchecked,
};

use values::create_piece_square_tables;

/// Values related to evaluation.
pub mod values;

/// An [`Evaluation`] with half the size.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct CompressedEvaluation(pub i16);

/// An evaluation.
///
/// When converting to a [`CompressedEvaluation`] or compared against mate scores,
/// this should always be in the range
/// `-`[`Self::INFINITY`]`..=`[`Self::INFINITY`].
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Evaluation(pub i32);

/// The phase of the game, represented by the sum of the weights of the pieces
/// of the board.
///
/// `0` is the endgame (because fewer pieces is closer to an endgame) and `24`
/// is the middlegame (because the sum of the weights of the pieces of the
/// starting position is 24).
#[derive(Clone, Copy, Default)]
pub struct Phase(u8);

/// A blend between a middlegame and endgame value.
#[derive(Clone, Copy, Default)]
pub struct Score(pub Evaluation, pub Evaluation);

/// The piece-square tables for White and black, with an extra table of 0's to
/// allow [`Piece::NONE`] to index into it. The piece values are baked in.
///
/// `PIECE_SQUARE_TABLES[Piece::PIECE_TYPE][Square::SQUARE] == value for that
/// piece type and side on that square`.
static PIECE_SQUARE_TABLES: [[Score; Square::TOTAL]; Piece::TOTAL + 1] =
    create_piece_square_tables();
/// The weight of each piece towards the phase.
///
/// Order: pawn, knight, bishop, rook, queen, king. Each piece has two values:
/// one per side. The order of those two values depends on the order of
/// [`Side::WHITE`] and [`Side::BLACK`]. An extra `0` is added at the end to
/// allow [`Piece::NONE`] to index into it.
static PHASE_WEIGHTS: [u8; Piece::TOTAL + 1] = [0, 0, 1, 1, 1, 1, 2, 2, 4, 4, 0, 0, 0];

impl Evaluation {
    /// An invalid evaluation.
    pub const NONE: Self = Self(Self::INFINITY.0 + 1);
    /// The highest possible (positive) evaluation.
    pub const INFINITY: Self = Self(i16::MAX as i32);
    /// The evaluation of a mate.
    pub const MATE: Self = Self::INFINITY;
    /// The lowest score a mate can have.
    pub const MATE_BOUND: Self = Self(Self::MATE.0 - Depth::MAX.0);
    /// The evaluation of a draw.
    pub const DRAW: Self = Self(0);
}

impl Display for Evaluation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[allow(clippy::unwrap_used)]
        if self.is_mate() {
            write!(f, "mate {}", self.moves_to_mate())
        } else {
            write!(f, "cp {}", self.0)
        }
    }
}

impl Add for CompressedEvaluation {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl AddAssign for CompressedEvaluation {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl DivAssign for CompressedEvaluation {
    fn div_assign(&mut self, other: Self) {
        self.0 /= other.0;
    }
}

impl DivAssign<i16> for CompressedEvaluation {
    fn div_assign(&mut self, other: i16) {
        *self /= Self(other);
    }
}

impl From<Evaluation> for CompressedEvaluation {
    fn from(eval: Evaluation) -> Self {
        debug_assert!(
            eval >= -Evaluation::INFINITY && eval <= Evaluation::INFINITY,
            "converting an Evaluation ({}) outside the permissible range for a CompressedEvaluation",
            eval.0
        );
        Self(eval.0 as i16)
    }
}

impl Neg for CompressedEvaluation {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl PartialEq<i16> for CompressedEvaluation {
    fn eq(&self, other: &i16) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<i16> for CompressedEvaluation {
    fn partial_cmp(&self, other: &i16) -> Option<Ordering> {
        Some(self.0.cmp(other))
    }
}

impl Sub for CompressedEvaluation {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl Sub<i16> for CompressedEvaluation {
    type Output = Self;

    fn sub(self, other: i16) -> Self::Output {
        self - Self(other)
    }
}

impl SubAssign for CompressedEvaluation {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl Sum for CompressedEvaluation {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|eval| eval.0).sum())
    }
}

impl Add for Evaluation {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Add<i16> for Evaluation {
    type Output = Self;

    fn add(self, other: i16) -> Self::Output {
        self + Self(other.into())
    }
}

impl AddAssign for Evaluation {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Div for Evaluation {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Self(self.0 / other.0)
    }
}

impl Div<i16> for Evaluation {
    type Output = Self;

    fn div(self, other: i16) -> Self::Output {
        self / Self(other.into())
    }
}

impl From<CompressedEvaluation> for Evaluation {
    fn from(eval: CompressedEvaluation) -> Self {
        Self(eval.0.into())
    }
}

impl From<Depth> for Evaluation {
    fn from(eval: Depth) -> Self {
        Self(eval.0)
    }
}

impl From<Height> for Evaluation {
    fn from(eval: Height) -> Self {
        Self(eval.0.into())
    }
}

impl Mul for Evaluation {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self(self.0 * other.0)
    }
}

impl Mul<i32> for Evaluation {
    type Output = Self;

    fn mul(self, other: i32) -> Self::Output {
        self * Self(other)
    }
}

impl Mul<Phase> for Evaluation {
    type Output = Self;

    fn mul(self, other: Phase) -> Self::Output {
        self * Self(other.inner().into())
    }
}

impl MulAssign for Evaluation {
    fn mul_assign(&mut self, other: Self) {
        self.0 *= other.0;
    }
}

impl MulAssign<i16> for Evaluation {
    fn mul_assign(&mut self, other: i16) {
        *self *= Self(other.into());
    }
}

impl Neg for Evaluation {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl PartialEq<i16> for Evaluation {
    fn eq(&self, other: &i16) -> bool {
        self.0 == (*other).into()
    }
}

impl PartialOrd<i16> for Evaluation {
    fn partial_cmp(&self, other: &i16) -> Option<Ordering> {
        Some(self.0.cmp(&(*other).into()))
    }
}

impl Sub for Evaluation {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl Sub<i16> for Evaluation {
    type Output = Self;

    fn sub(self, other: i16) -> Self::Output {
        self - Self(other.into())
    }
}

impl SubAssign for Evaluation {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl AddAssign for Phase {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Phase {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl SubAssign for Score {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl Evaluation {
    /// Checks if the score is low or high enough to be a mate score.
    pub fn is_mate(self) -> bool {
        self >= Self::MATE_BOUND || self <= -Self::MATE_BOUND
    }

    /// Calculates the number of fullmoves to a mate.
    ///
    /// A positive number represents fullmoves to performing a mate and a negative
    /// number means fullmoves to being mated.
    pub fn moves_to_mate(self) -> i32 {
        if self > 0 {
            (Self::MATE - self + 1) / 2
        } else {
            (-Self::MATE - self) / 2
        }
        .0
    }

    /// Calculates the evaluation if we're mating after `height` halfmoves.
    pub fn mate_after(height: Height) -> Self {
        Self::MATE - Self::from(height)
    }

    /// Calculates the evaluation if we're getting mated after `height` halfmoves.
    pub fn mated_after(height: Height) -> Self {
        -Self::MATE + Self::from(height)
    }
}

impl Phase {
    /// Returns the phase.
    ///
    /// The maximum is 24, even with early promotion.
    pub fn inner(self) -> u8 {
        self.0.min(24)
    }
}

impl Score {
    /// Lerps the score between its middlegame and endgame value depending on
    /// the phase.
    pub fn lerp_to(self, phase: Phase) -> Evaluation {
        let diff = self.1 - self.0;
        self.1 - (diff * phase) / 24
    }
}

/// Calculates a static evaluation of the current board.
pub fn evaluate(board: &Board) -> Evaluation {
    let phase = board.phase();
    let score = board.score();

    let eval = score.lerp_to(phase);

    if board.side_to_move() == Side::WHITE {
        eval
    } else {
        -eval
    }
}

/// Returns the value of the given piece on the given square.
///
/// The piece can be any type (even [`Piece::NONE`]) but the square must be
/// valid.
pub fn piece_score(square: Square, piece: Piece) -> Score {
    let piece_table = get_unchecked(&PIECE_SQUARE_TABLES, piece.to_index());
    *get_unchecked(piece_table, square.to_index())
}

/// Returns the phase of the given piece.
///
/// The piece can be any type (even [`Piece::NONE`]).
pub fn piece_phase(piece: Piece) -> Phase {
    Phase(*get_unchecked(&PHASE_WEIGHTS, piece.to_index()))
}
