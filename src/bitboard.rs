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
    ops::{
        BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
        ShrAssign,
    },
};

use crate::defs::{File, Rank, Square};

/// A bitboard: a set of bits representing a certain state of the board.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bitboard(pub u64);

/// An iterator over the bits of a [`Bitboard`].
pub struct BitIter(Bitboard);

impl Bitboard {
    /// The squares betwen the White king and kingside rook in the starting
    /// position.
    const CASTLING_SPACE_WK: Self = Self(0x0000_0000_0000_0060);
    /// The squares betwen the White king and queenside rook in the starting
    /// position.
    const CASTLING_SPACE_WQ: Self = Self(0x0000_0000_0000_000e);
    /// The squares betwen the Black king and kingside rook in the starting
    /// position.
    const CASTLING_SPACE_BK: Self = Self(0x6000_0000_0000_0000);
    /// The squares betwen the Black king and queenside rook in the starting
    /// position.
    const CASTLING_SPACE_BQ: Self = Self(0x0e00_0000_0000_0000);
}

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Display for Bitboard {
    /// Displays the bits of a bitboard in little-endian rank-file mapping.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut ret = String::with_capacity(121);
        let mut bb = Self::from(Square::A8);

        for _ in 0..Rank::TOTAL {
            for _ in 0..File::TOTAL {
                if (*self & bb).is_empty() {
                    ret.push_str("0 ");
                } else {
                    ret.push_str("1 ");
                }
                bb.0 = bb.0.rotate_left(1);
            }
            bb.0 = bb.0.rotate_right(16);
            ret.pop();
            ret.push('\n');
        }
        ret.pop();

        f.write_str(&ret)
    }
}

impl From<Square> for Bitboard {
    /// Converts a square into a bit on a bitboard.
    fn from(square: Square) -> Self {
        Self(1 << square.0)
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitIter;

    fn into_iter(self) -> Self::IntoIter {
        BitIter(self)
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl Shl<u8> for Bitboard {
    type Output = Self;

    fn shl(self, rhs: u8) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl Shr<u8> for Bitboard {
    type Output = Self;

    fn shr(self, rhs: u8) -> Self::Output {
        Self(self.0 >> rhs)
    }
}

impl ShlAssign<u8> for Bitboard {
    fn shl_assign(&mut self, rhs: u8) {
        self.0 <<= rhs;
    }
}

impl ShrAssign<u8> for Bitboard {
    fn shr_assign(&mut self, rhs: u8) {
        self.0 >>= rhs;
    }
}

impl Iterator for BitIter {
    type Item = Square;

    /// Clears the LSB of the wrapped [`Bitboard`] and returns the position of
    /// that bit.
    ///
    /// Returns [`None`] if there are no set bits.
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.pop_next_square())
        }
    }
}

impl Bitboard {
    /// Returns the given file represented on a bitboard.
    ///
    /// e.g. `file_bb(File::FILE2) == 0x0202020202020202`.
    pub const fn file_bb(file: File) -> Self {
        Self(0x0101_0101_0101_0101 << file.0)
    }

    /// Returns the given rank represented on a bitboard.
    ///
    /// e.g. `rank_bb(Rank::RANK2) == 0x000000000000ff00`.
    pub const fn rank_bb(rank: Rank) -> Self {
        Self(0xff << (rank.0 * 8))
    }

    /// Calculates the union of the bitboard files and ranks that do not
    /// contain `square`.
    ///
    /// # Examples
    ///
    /// `edges_without(Square::H4)` ->
    /// ```text
    /// 1 1 1 1 1 1 1 1
    /// 1 0 0 0 0 0 0 0
    /// 1 0 0 0 0 0 0 0
    /// 1 0 0 0 0 0 0 0
    /// 1 0 0 0 0 0 0 0
    /// 1 0 0 0 0 0 0 X
    /// 1 0 0 0 0 0 0 0
    /// 1 0 0 0 0 0 0 0
    /// 1 1 1 1 1 1 1 1
    ///
    /// ```
    ///
    /// `edges_without(Square::A1)` ->
    /// ```text
    /// 1 1 1 1 1 1 1 1
    /// 0 0 0 0 0 0 0 1
    /// 0 0 0 0 0 0 0 1
    /// 0 0 0 0 0 0 0 1
    /// 0 0 0 0 0 0 0 1
    /// 0 0 0 0 0 0 0 1
    /// 0 0 0 0 0 0 0 1
    /// 0 0 0 0 0 0 0 1
    /// X 0 0 0 0 0 0 1
    /// ```
    pub const fn edges_without(square: Square) -> Self {
        let rank_1 = 0x0000_0000_0000_00ff;
        let rank_1_and_8 = 0xff00_0000_0000_00ff;
        let file_a = 0x0101_0101_0101_0101;
        let file_a_and_h = 0x8181_8181_8181_8181;

        // the square on the first file of its rank
        let square_file_a = square.0 & 56;
        let square_file = square.0 & 7;
        let excluded_ranks_bb = rank_1_and_8 & !(rank_1 << square_file_a);
        let excluded_files_bb = file_a_and_h & !(file_a << square_file);

        Self(excluded_ranks_bb | excluded_files_bb)
    }

    /// Same as [`Self::is_clear_to_castle()`] but with the location as generic
    /// parameters.
    pub fn is_clear_to_castle_const<const IS_WHITE: bool, const IS_KINGSIDE: bool>(
        occupancies: Self,
    ) -> bool {
        #[allow(clippy::collapsible_else_if)]
        let castling_space = if IS_WHITE {
            if IS_KINGSIDE {
                Self::CASTLING_SPACE_WK
            } else {
                Self::CASTLING_SPACE_WQ
            }
        } else {
            if IS_KINGSIDE {
                Self::CASTLING_SPACE_BK
            } else {
                Self::CASTLING_SPACE_BQ
            }
        };
        (occupancies & castling_space).is_empty()
    }

    /// Calculates if there are no blocking pieces between the king and rook,
    /// given the side to move and castling direction.
    pub fn is_clear_to_castle(occupancies: Self, is_white: bool, is_kingside: bool) -> bool {
        #[allow(clippy::collapsible_else_if)]
        let castling_space = if is_white {
            if is_kingside {
                Self::CASTLING_SPACE_WK
            } else {
                Self::CASTLING_SPACE_WQ
            }
        } else {
            if is_kingside {
                Self::CASTLING_SPACE_BK
            } else {
                Self::CASTLING_SPACE_BQ
            }
        };
        (occupancies & castling_space).is_empty()
    }

    /// Returns an empty bitboard.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Tests if the bitboard is empty.
    pub fn is_empty(self) -> bool {
        self == Self::empty()
    }

    /// Shifts the bitboard one square north if `IS_WHITE` is true, otherwise
    /// shifts it one square south.
    pub fn pawn_push<const IS_WHITE: bool>(self) -> Self {
        if IS_WHITE {
            self.north()
        } else {
            self.south()
        }
    }

    /// Shifts the bitboard one square north without wrapping.
    pub fn north(self) -> Self {
        self << 8
    }

    /// Shifts the bitboard one square east without wrapping.
    pub fn east(self) -> Self {
        (self << 1) & !Self::file_bb(File::FILE1)
    }

    /// Shifts the bitboard one square south without wrapping.
    pub fn south(self) -> Self {
        self >> 8
    }

    /// Shifts the bitboard one square west without wrapping.
    pub fn west(self) -> Self {
        (self >> 1) & !Self::file_bb(File::FILE8)
    }

    /// Clears the least significant bit of the bitboard and returns it.
    pub fn pop_lsb(&mut self) -> Self {
        let popped_bit = self.0 & self.0.wrapping_neg();
        self.0 ^= popped_bit;
        Self(popped_bit)
    }

    /// Clears the least significant bit of the bitboard and converts the
    /// position of that bit to a [`Square`].
    pub fn pop_next_square(&mut self) -> Square {
        let square = Square(self.0.trailing_zeros() as u8);
        *self ^= Self::from(square);
        square
    }
}
