use std::{
    cmp::min,
    fmt::{self, Display, Formatter},
    ops::{Add, AddAssign, Neg, SubAssign},
};

use crate::{
    board::Board,
    defs::{Nums, Side},
};
use piece_square_tables::create_piece_square_tables;

/// Items related to piece-square tables.
mod piece_square_tables;

/// The result of an evaluation.
pub type Eval = i16;

/// Piece-square tables. A bonus/malus for each piece depending on its
/// position. Copied verbatim from
/// [`PeSTO`]<https://www.chessprogramming.org/PeSTO>.
///
/// Order: pawn, knight, bishop, rook, queen, king. An extra table is included
/// so that [`Piece::NONE`] can index into this array for a value of `0`.
pub const PIECE_SQUARE_TABLES: [[Score; Nums::SQUARES]; Nums::TOTAL_PIECE_VARIANTS] =
    create_piece_square_tables();

/// The weight of each piece towards a middlegame. A total weight of 0 means
/// it's an endgame. The starting weight (24) is the middlegame. The order is
/// Black pawn, White pawn, Black knight, etc. Kings always exist so they have
/// weight 0; an extra 0 is added to allow [`Piece::NONE`] to index into it.
pub const PHASE_WEIGHTS: [u8; Nums::TOTAL_PIECE_VARIANTS] = [0, 0, 1, 1, 1, 1, 2, 2, 4, 4, 0, 0, 0];

/// A blend between middlegame value and endgame value.
#[derive(Clone, Copy)]
pub struct Score(pub Eval, pub Eval);

impl Add for Score {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl AddAssign for Score {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl Display for Score {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Score({}, {})", self.0, self.1)
    }
}

impl Neg for Score {
    type Output = Self;

    #[inline]
    fn neg(mut self) -> Self {
        self.0 = -self.0;
        self.1 = -self.1;
        self
    }
}

impl SubAssign for Score {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl Score {
    /// Lerp the current score to an eval given a game phase, where `0` means
    /// use the middlegame score and `>= 24` means use the endgame score.
    fn lerp_to_eval(self, phase: u8) -> Eval {
        // `>= 24` can happen because of early promotion
        let phase: Eval = min(24, Eval::from(phase));
        let diff = self.1 - self.0;
        self.1 - diff * phase / 24
    }
}

/// Calculates a static evaluation of the current board depending on
/// various heuristics.
///
/// Currently just calculates material balance with piece-square tables.
#[inline]
#[must_use]
pub fn evaluate_board(board: &Board) -> Eval {
    let score = board.psq();
    let phase = board.phase();

    let eval = score.lerp_to_eval(phase);
    if board.side_to_move() == Side::WHITE {
        eval
    } else {
        -eval
    }
}
