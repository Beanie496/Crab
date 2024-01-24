use crate::defs::{File, Rank, Square};

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl};

/// A wrapper for a `u64`, since a bitboard is 64 bits.
// the idea for wrapping these types in structs and implementing a tonne of
// methods/associated functions is taken from viridithas, so thanks cosmo
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// The inner value of a wrapper does not need to be documented.
#[allow(clippy::missing_docs_in_private_items)]
pub struct Bitboard {
    bb: u64,
}

/// An iterator over the bits of a [`Bitboard`].
// The inner value of a wrapper does not need to be documented.
#[allow(clippy::missing_docs_in_private_items)]
pub struct BitIter {
    bb: u64,
}

impl BitAnd for Bitboard {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self::from(self.inner() & rhs.inner())
    }
}

impl BitAndAssign for Bitboard {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.bb &= rhs.inner();
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self::from(self.inner() | rhs.inner())
    }
}

impl BitOrAssign for Bitboard {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.bb |= rhs.inner();
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self::from(self.inner() ^ rhs.inner())
    }
}

impl BitXorAssign for Bitboard {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.bb ^= rhs.inner();
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        BitIter::new(self.inner())
    }
}

impl Not for Bitboard {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self::from(!self.inner())
    }
}

impl Shl<u8> for Bitboard {
    type Output = Self;

    #[inline]
    fn shl(self, rhs: u8) -> Self::Output {
        Self::from(self.inner() << rhs)
    }
}

impl Iterator for BitIter {
    type Item = Square;

    /// Clears the LSB of the wrapped [`Bitboard`] and returns the position of
    /// that bit. Returns [`None`] if there are no set bits.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.bb == 0 {
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
    pub const CASTLING_SPACE_WK: Self = Self::from(0x0000_0000_0000_0060);
    /// The squares betwen the White king and queenside rook in the starting
    /// position.
    pub const CASTLING_SPACE_WQ: Self = Self::from(0x0000_0000_0000_000e);
    /// The squares betwen the Black king and kingside rook in the starting
    /// position.
    pub const CASTLING_SPACE_BK: Self = Self::from(0x6000_0000_0000_0000);
    /// The squares betwen the Black king and queenside rook in the starting
    /// position.
    pub const CASTLING_SPACE_BQ: Self = Self::from(0x0e00_0000_0000_0000);
    /// An empty bitboard: `0x0`.
    pub const EMPTY: Self = Self::from(0);
}

impl Bitboard {
    /// Wraps a `u64` in a [`Bitboard`].
    #[inline]
    #[must_use]
    pub const fn from(bb: u64) -> Self {
        Self { bb }
    }

    /// Converts a [`Square`] into a [`Bitboard`] with the bit in the given
    /// position set.
    #[inline]
    #[must_use]
    pub const fn from_square(square: Square) -> Self {
        Self::from(1 << square.inner())
    }

    /// Converts `rank` and `file` into a [`Bitboard`] with the bit in the
    /// given position set.
    #[inline]
    #[must_use]
    pub const fn from_pos(rank: Rank, file: File) -> Self {
        Self::from(1 << (rank.inner() * 8 + file.inner()))
    }

    /// Returns the given file represented on a bitboard.
    ///
    /// e.g. `file_bb(File::FILE2) == 0x0202020202020202`.
    #[inline]
    #[must_use]
    pub const fn file_bb(file: File) -> Self {
        Self::from(0x0101_0101_0101_0101 << file.inner())
    }

    /// Returns the given rank represented on a bitboard.
    ///
    /// e.g. `rank_bb(Rank::RANK2) == 0x000000000000ff00`.
    #[inline]
    #[must_use]
    pub const fn rank_bb(rank: Rank) -> Self {
        Self::from(0xff << (rank.inner() * 8))
    }

    /// Returns the contents of `self`.
    #[inline]
    #[must_use]
    pub const fn inner(self) -> u64 {
        self.bb
    }

    /// Tests if no bits in `self` are set.
    #[inline]
    #[must_use]
    pub fn is_empty(self) -> bool {
        self == Self::EMPTY
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
        Self::from(self.inner() << 8)
    }

    /// Shifts `self` one square east without wrapping.
    #[inline]
    #[must_use]
    pub fn east(self) -> Self {
        Self::from(self.inner() << 1) & !Self::file_bb(File::FILE1)
    }

    /// Shifts `self` one square south without wrapping.
    #[inline]
    #[must_use]
    pub const fn south(self) -> Self {
        Self::from(self.inner() >> 8)
    }

    /// Shifts `self` one square west without wrapping.
    #[inline]
    #[must_use]
    pub fn west(self) -> Self {
        Self::from(self.inner() >> 1) & !Self::file_bb(File::FILE8)
    }

    /// Clears the least significant bit of `self` and returns it.
    #[inline]
    #[must_use]
    pub fn pop_lsb(&mut self) -> Self {
        let popped_bit = self.inner() & self.inner().wrapping_neg();
        self.bb ^= popped_bit;
        Self::from(popped_bit)
    }

    /// Converts the position of the least significant bit of `self` to a
    /// [`Square`].
    #[inline]
    #[must_use]
    pub const fn to_square(self) -> Square {
        Square::from(self.inner().trailing_zeros() as u8)
    }

    /// Pretty prints `self`.
    // Allowed dead code because this is occasionally useful for debugging.
    #[inline]
    pub fn pretty_print(self) {
        for r in (Rank::RANK1.inner()..=Rank::RANK8.inner()).rev() {
            for f in File::FILE1.inner()..=File::FILE8.inner() {
                print!(
                    "{} ",
                    u32::from(!(self & Self::from_pos(Rank::from(r), File::from(f))).is_empty())
                );
            }
            println!();
        }
        println!();
    }
}

impl BitIter {
    /// Creates a new [`BitIter`] from a [`Bitboard`].
    #[inline]
    #[must_use]
    pub const fn new(bb: u64) -> Self {
        Self { bb }
    }

    /// Clears the least significant bit of `self` and converts the position of
    /// that bit to a [`Square`].
    #[inline]
    #[must_use]
    pub fn pop_next_square(&mut self) -> Square {
        let shift = self.bb.trailing_zeros();
        self.bb ^= 1 << shift;
        Square::from(shift as u8)
    }
}
