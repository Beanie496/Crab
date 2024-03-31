use std::{
    cmp::min,
    fmt::{self, Display, Formatter},
    ops::{Add, AddAssign, Neg, SubAssign},
};

use crate::{
    board::Board,
    defs::{Piece, Side, Square},
};
use piece_square_tables::create_piece_square_tables;

/// Items related to piece-square tables.
mod piece_square_tables;

/// The result of an evaluation.
pub type Eval = i16;

#[allow(clippy::doc_markdown)]
/// Piece-square tables: a bonus/malus for each piece depending on its
/// position.
///
/// Order: pawn, knight, bishop, rook, queen, king. An extra table is included
/// so that [`Piece::NONE`] can index into this array for a value of `0`.
///
/// Copied verbatim from PeSTO: <https://www.chessprogramming.org/PeSTO>.
pub static PIECE_SQUARE_TABLES: [[Score; Square::TOTAL]; Piece::TOTAL + 1] =
    create_piece_square_tables();

/// The weight of each piece towards a middlegame.
///
/// A total weight of 0 is interpreted as an endgame. A total weight of 24 (the
/// starting weight) is represented as the middlegame.
///
/// Order: Black pawn, White pawn, Black knight, etc. Kings always exist so
/// they have weight 0. An extra 0 is added to allow [`Piece::NONE`] to index
/// into it.
pub static PHASE_WEIGHTS: [u8; Piece::TOTAL + 1] = [0, 0, 1, 1, 1, 1, 2, 2, 4, 4, 0, 0, 0];

/// The highest possible (positive) evaluation.
pub const INF_EVAL: Eval = Eval::MAX;
/// The evaluation of a mate.
pub const MATE: Eval = INF_EVAL - 300;
/// The evaluation of a draw.
pub const DRAW: Eval = 0;

/// A blend between middlegame value and endgame value.
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

impl Display for Score {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Score({}, {})", self.0, self.1)
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
    /// Lerps the score between the middlegame and endgame value depending on
    /// the given phase. `0` means use the endgame value and `>= 24` means use
    /// the middlegame value.
    fn lerp_to_eval(self, phase: u8) -> Eval {
        // `>= 24` can happen because of early promotion
        let phase: Eval = min(24, Eval::from(phase));
        let diff = self.1 - self.0;
        self.1 - diff * phase / 24
    }
}

/// Calculates a static evaluation of the current board depending on various
/// heuristics.
///
/// Currently just calculates material balance with piece-square tables.
pub fn evaluate(board: &Board) -> Eval {
    let score = board.psq();
    let phase = board.phase();

    let eval = score.lerp_to_eval(phase);
    if board.side_to_move() == Side::WHITE {
        eval
    } else {
        -eval
    }
}

/// Calculates the evaluation if we're mating in `depth` halfmoves.
pub fn mate_in(depth: u8) -> Eval {
    MATE - Eval::from(depth)
}

/// Calculates the evaluation if we're getting mated in `depth` halfmoves.
pub fn mated_in(depth: u8) -> Eval {
    -MATE + Eval::from(depth)
}
