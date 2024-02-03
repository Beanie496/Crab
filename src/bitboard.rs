use std::{
    fmt::{self, Display, Formatter},
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl},
};

use crate::defs::{File, Rank, Square};

/// A wrapper for a `u64`, since a bitboard is 64 bits.
// the idea for wrapping these types in structs and implementing a tonne of
// methods/associated functions is taken from viridithas, so thanks cosmo
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::exhaustive_structs)]
pub struct Bitboard(pub u64);

/// An iterator over the bits of a [`Bitboard`].
pub struct BitIter(u64);

impl BitAnd for Bitboard {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for Bitboard {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Display for Bitboard {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut ret = String::new();

        for r in (0..Rank::TOTAL as u8).rev() {
            for f in 0..File::TOTAL as u8 {
                let is_bit_set = !(*self & Self::from_pos(Rank(r), File(f))).is_empty();
                ret.push(char::from(b'0' + u8::from(is_bit_set)));
                ret.push(' ');
            }
            ret.pop();
            ret.push('\n');
        }
        ret.pop();

        f.write_str(&ret)
    }
}

impl From<Square> for Bitboard {
    #[inline]
    fn from(square: Square) -> Self {
        Self(1 << square.0)
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        BitIter(self.0)
    }
}

impl Not for Bitboard {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl Shl<u8> for Bitboard {
    type Output = Self;

    #[inline]
    fn shl(self, rhs: u8) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl Iterator for BitIter {
    type Item = Square;

    /// Clears the LSB of the wrapped [`Bitboard`] and returns the position of
    /// that bit. Returns [`None`] if there are no set bits.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            Some(self.pop_next_square())
        }
    }
}

/// Pre-constructed bitboards.
impl Bitboard {
    /// The squares betwen the White king and kingside rook in the starting
    /// position.
    pub const CASTLING_SPACE_WK: Self = Self(0x0000_0000_0000_0060);
    /// The squares betwen the White king and queenside rook in the starting
    /// position.
    pub const CASTLING_SPACE_WQ: Self = Self(0x0000_0000_0000_000e);
    /// The squares betwen the Black king and kingside rook in the starting
    /// position.
    pub const CASTLING_SPACE_BK: Self = Self(0x6000_0000_0000_0000);
    /// The squares betwen the Black king and queenside rook in the starting
    /// position.
    pub const CASTLING_SPACE_BQ: Self = Self(0x0e00_0000_0000_0000);
    /// An empty bitboard: `0x0`.
    pub const EMPTY: Self = Self(0);
}

impl Bitboard {
    /// Converts `rank` and `file` into a [`Bitboard`] with the bit in the
    /// given position set.
    #[inline]
    #[must_use]
    pub fn from_pos(rank: Rank, file: File) -> Self {
        Self::from(Square::from_pos(rank, file))
    }

    /// Returns the given file represented on a bitboard.
    ///
    /// e.g. `file_bb(File::FILE2) == 0x0202020202020202`.
    #[inline]
    #[must_use]
    pub const fn file_bb(file: File) -> Self {
        Self(0x0101_0101_0101_0101 << file.0)
    }

    /// Returns the given rank represented on a bitboard.
    ///
    /// e.g. `rank_bb(Rank::RANK2) == 0x000000000000ff00`.
    #[inline]
    #[must_use]
    pub const fn rank_bb(rank: Rank) -> Self {
        Self(0xff << (rank.0 * 8))
    }

    /// Calculates the union of the bitboard files and ranks that do not
    /// contain `square`.
    ///
    /// # Examples
    ///
    /// `bb_edges_without(Square::H4)` ->
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
    /// `bb_edges_without(Square::A1)` ->
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
    #[inline]
    #[must_use]
    pub fn edges_without(square: Square) -> Self {
        let excluded_ranks_bb = (Self::file_bb(File::FILE1) | Self::file_bb(File::FILE8))
            & !Self::file_bb(File::from(square));
        let excluded_files_bb = (Self::rank_bb(Rank::RANK1) | Self::rank_bb(Rank::RANK8))
            & !Self::rank_bb(Rank::from(square));
        excluded_ranks_bb | excluded_files_bb
    }

    /// Returns the contents of `self`.
    #[inline]
    #[must_use]
    pub const fn inner(self) -> u64 {
        self.0
    }

    /// Tests if no bits in `self` are set.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == Self::EMPTY.0
    }

    /// Shifts `self` one square north if `IS_WHITE` is true, otherwise shifts
    /// `self` one square south.
    #[inline]
    #[must_use]
    pub const fn pawn_push<const IS_WHITE: bool>(self) -> Self {
        if IS_WHITE {
            self.north()
        } else {
            self.south()
        }
    }

    /// Shifts `self` one square north without wrapping.
    #[inline]
    #[must_use]
    pub const fn north(self) -> Self {
        Self(self.0 << 8)
    }

    /// Shifts `self` one square east without wrapping.
    #[inline]
    #[must_use]
    pub const fn east(self) -> Self {
        Self(self.0 << 1 & !Self::file_bb(File::FILE1).0)
    }

    /// Shifts `self` one square south without wrapping.
    #[inline]
    #[must_use]
    pub const fn south(self) -> Self {
        Self(self.0 >> 8)
    }

    /// Shifts `self` one square west without wrapping.
    #[inline]
    #[must_use]
    pub const fn west(self) -> Self {
        Self(self.0 >> 1 & !Self::file_bb(File::FILE8).0)
    }

    /// Clears the least significant bit of `self` and returns it.
    #[inline]
    #[must_use]
    pub fn pop_lsb(&mut self) -> Self {
        let popped_bit = self.0 & self.0.wrapping_neg();
        self.0 ^= popped_bit;
        Self(popped_bit)
    }

    /// Converts the position of the least significant bit of `self` to a
    /// [`Square`].
    #[inline]
    #[must_use]
    pub const fn to_square(self) -> Square {
        Square(self.0.trailing_zeros() as u8)
    }
}

impl BitIter {
    /// Clears the least significant bit of `self` and converts the position of
    /// that bit to a [`Square`].
    #[inline]
    #[must_use]
    pub fn pop_next_square(&mut self) -> Square {
        let shift = self.0.trailing_zeros();
        self.0 ^= 1 << shift;
        Square(shift as u8)
    }
}
