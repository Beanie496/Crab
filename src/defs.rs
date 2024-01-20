// all of the structs here contain only one field (their inner item), so
// documentation isn't necessary
#![allow(clippy::missing_docs_in_private_items)]

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl};

// the idea for wrapping these types in structs and implementing a tonne of
// methods/associated functions is taken from viridithas, so thanks cosmo
/// A wrapper for a `u64`, since a bitboard is 64 bits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bitboard {
    bb: u64,
}

/// A wrapper for an `i8`, since a direction can go from -9 to 9.
pub struct Direction {
    d: i8,
}

/// A wrapper for a `u8`, since a file can go from 0 to 7.
#[derive(Clone, Copy)]
pub struct File {
    f: u8,
}

/// Miscellaneous constants associated with chess (`SIDES == 2`, etc.)
#[allow(clippy::exhaustive_structs)]
pub struct Nums;

/// A wrapper for a `u8`, since a piece can go from 0 to 12.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Piece {
    p: u8,
}

/// A wrapper for a `u8`, since a rank can go from 0 to 7.
#[derive(Clone, Copy)]
pub struct Rank {
    r: u8,
}

/// A wrapper for a `u8`, since a side is just 0, 1 or 2.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Side {
    s: u8,
}

/// A wrapper for a `u8`, since a square can go from 0 to 64.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd)]
pub struct Square {
    sq: u8,
}

/// An array of character constants associated with each piece on both sides.
///
/// e.g. `PIECE_CHARS[Side::WHITE][Piece::KNIGHT] == 'N'`;
/// `PIECE_CHARS[Side::BLACK][Piece::KING] == 'k'`.
const PIECE_CHARS: [[char; Nums::PIECES]; Nums::SIDES] = [
    ['p', 'n', 'b', 'r', 'q', 'k'],
    ['P', 'N', 'B', 'R', 'Q', 'K'],
];

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

// TODO: `impl IntoIterator for Bitboard`
#[allow(clippy::copy_iterator)]
impl Iterator for Bitboard {
    type Item = Square;

    /// Clears the LSB of the wrapped [`Bitboard`] and returns the position of
    /// that bit. Returns [`None`] if there are no set bits.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.inner() == 0 {
            None
        } else {
            Some(self.pop_next_square())
        }
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

/// Pre-constructed bitboards.
impl Bitboard {
    pub const EMPTY: Self = Self::from(0);
    pub const CASTLING_SPACE_WK: Self = Self::from(0x0000_0000_0000_0060);
    pub const CASTLING_SPACE_WQ: Self = Self::from(0x0000_0000_0000_000e);
    pub const CASTLING_SPACE_BK: Self = Self::from(0x6000_0000_0000_0000);
    pub const CASTLING_SPACE_BQ: Self = Self::from(0x0e00_0000_0000_0000);
}

/// The square difference in each of the 8 directions.
impl Direction {
    pub const N: Self = Self::from(8);
    pub const NE: Self = Self::from(9);
    pub const E: Self = Self::from(1);
    pub const SE: Self = Self::from(-7);
    pub const S: Self = Self::from(-8);
    pub const SW: Self = Self::from(-9);
    pub const W: Self = Self::from(-1);
    pub const NW: Self = Self::from(7);
}

/// Enumerates files.
impl File {
    pub const FILE1: Self = Self::from(0);
    pub const FILE8: Self = Self::from(7);
}

impl Nums {
    pub const FILES: usize = 8;
    pub const SIDES: usize = 2;
    pub const SQUARES: usize = 64;
    pub const PIECES: usize = 6;
    pub const RANKS: usize = 8;
}

/// Enumerates pieces.
impl Piece {
    pub const PAWN: Self = Self::from(0);
    pub const KNIGHT: Self = Self::from(1);
    pub const BISHOP: Self = Self::from(2);
    pub const ROOK: Self = Self::from(3);
    pub const QUEEN: Self = Self::from(4);
    pub const KING: Self = Self::from(5);
    pub const NONE: Self = Self::from(6);
}

/// Enumerates ranks.
impl Rank {
    pub const RANK1: Self = Self::from(0);
    pub const RANK2: Self = Self::from(1);
    pub const RANK4: Self = Self::from(3);
    pub const RANK5: Self = Self::from(4);
    pub const RANK7: Self = Self::from(6);
    pub const RANK8: Self = Self::from(7);
}

/// Enumerates sides.
impl Side {
    pub const BLACK: Self = Self::from(0);
    pub const WHITE: Self = Self::from(1);
    pub const NONE: Self = Self::from(2);
}

/// Enumerates squares. This engine uses little-endian rank-file mapping.
impl Square {
    pub const A1: Self = Self::from(0);
    pub const E1: Self = Self::from(4);
    pub const H1: Self = Self::from(7);
    pub const A8: Self = Self::from(56);
    pub const E8: Self = Self::from(60);
    pub const H8: Self = Self::from(63);
    pub const NONE: Self = Self::from(64);
}

impl Bitboard {
    /// Returns the given file represented on a bitboard.
    ///
    /// e.g. `file_bb(File::FILE2) == 0x0202020202020202`.
    #[inline]
    #[must_use]
    pub const fn file_bb(file: File) -> Self {
        Self::from(0x0101_0101_0101_0101 << file.inner())
    }

    /// Wraps a `u64` in a [`Bitboard`].
    #[inline]
    #[must_use]
    pub const fn from(bb: u64) -> Self {
        Self { bb }
    }

    /// Converts `rank` and `file` into a [`Bitboard`] with the bit in the
    /// given position set.
    #[inline]
    #[must_use]
    pub const fn from_pos(rank: Rank, file: File) -> Self {
        Self::from(1 << (rank.inner() * 8 + file.inner()))
    }

    /// Converts a [`Square`] into a [`Bitboard`] with the bit in the given
    /// position set.
    #[inline]
    #[must_use]
    pub const fn from_square(square: Square) -> Self {
        Self::from(1 << square.inner())
    }

    /// Returns the given rank represented on a bitboard.
    ///
    /// e.g. `rank_bb(Rank::RANK2) == 0x000000000000ff00`.
    #[inline]
    #[must_use]
    pub const fn rank_bb(rank: Rank) -> Self {
        Self::from(0xff << (rank.inner() * 8))
    }
}

impl Direction {
    /// Wraps an `i8` in a [`Direction`].
    #[inline]
    #[must_use]
    pub const fn from(direction: i8) -> Self {
        Self { d: direction }
    }
}

impl File {
    /// Wraps a `u8` in a [`File`].
    #[inline]
    #[must_use]
    pub const fn from(file: u8) -> Self {
        Self { f: file }
    }
}

impl Piece {
    /// Converts the char `piece` into a [`Piece`]. Will return `None` if the
    /// piece is not valid.
    #[inline]
    #[must_use]
    pub const fn from_char(piece: char) -> Option<Self> {
        match piece {
            'p' => Some(Self::PAWN),
            'n' => Some(Self::KNIGHT),
            'b' => Some(Self::BISHOP),
            'r' => Some(Self::ROOK),
            'q' => Some(Self::QUEEN),
            'k' => Some(Self::KING),
            _ => None,
        }
    }

    /// Wraps a `u8` in a [`Piece`].
    #[inline]
    #[must_use]
    pub const fn from(piece: u8) -> Self {
        Self { p: piece }
    }
}

impl Rank {
    /// Wraps a `u8` in a [`Rank`].
    #[inline]
    #[must_use]
    pub const fn from(rank: u8) -> Self {
        Self { r: rank }
    }
}

impl Side {
    /// Wraps a `u8` in a [`Side`].
    #[inline]
    #[must_use]
    pub const fn from(side: u8) -> Self {
        Self { s: side }
    }
}

impl Square {
    /// Wraps a `u8` in a [`Square`].
    #[inline]
    #[must_use]
    pub const fn from(square: u8) -> Self {
        Self { sq: square }
    }

    /// Converts `rank` and `file` into a [`Square`].
    #[inline]
    #[must_use]
    pub const fn from_pos(rank: Rank, file: File) -> Self {
        Self::from(rank.inner() * 8 + file.inner())
    }

    /// Converts a string representation of a square (e.g. "e4") into a
    /// [`Square`]. Will return [`None`] if the square is not valid.
    #[inline]
    #[must_use]
    pub fn from_string(string: &str) -> Option<Self> {
        let mut square = 0;
        let mut iter = string.as_bytes().iter();

        let file = iter.next()?;
        if (b'a'..=b'h').contains(file) {
            square += file - b'a';
        } else {
            return None;
        }
        let rank = iter.next()?;
        if (b'1'..=b'8').contains(rank) {
            square += (rank - b'1') * 8;
        } else {
            return None;
        }

        Some(Self::from(square))
    }
}

impl Bitboard {
    /// Shifts `self` one square east without wrapping.
    #[inline]
    #[must_use]
    pub fn east(self) -> Self {
        Self::from(self.inner() << 1) & !Self::file_bb(File::FILE1)
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

    /// Shifts `self` one square north without wrapping.
    #[inline]
    #[must_use]
    pub const fn north(self) -> Self {
        Self::from(self.inner() << 8)
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

    /// Clears the least significant bit of `self` and returns it.
    #[inline]
    #[must_use]
    pub fn pop_lsb(&mut self) -> Self {
        let popped_bit = self.inner() & self.inner().wrapping_neg();
        self.bb ^= popped_bit;
        Self::from(popped_bit)
    }

    /// Clears the least significant bit of `self` and converts the position of
    /// that bit to a [`Square`].
    #[inline]
    #[must_use]
    pub fn pop_next_square(&mut self) -> Square {
        let shift = self.inner().trailing_zeros();
        self.bb ^= 1 << shift;
        Square::from(shift as u8)
    }

    /// Pretty prints `self`.
    // Allowed dead code because this is occasionally useful for debugging.
    #[allow(dead_code)]
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

    /// Shifts `self` one square south without wrapping.
    #[inline]
    #[must_use]
    pub const fn south(self) -> Self {
        Self::from(self.inner() >> 8)
    }

    /// Converts the position of the least significant bit of `self` to a
    /// [`Square`].
    #[inline]
    #[must_use]
    pub const fn to_square(self) -> Square {
        Square::from(self.inner().trailing_zeros() as u8)
    }

    /// Shifts `self` one square west without wrapping.
    #[inline]
    #[must_use]
    pub fn west(self) -> Self {
        Self::from(self.inner() >> 1) & !Self::file_bb(File::FILE8)
    }
}

impl Direction {
    /// Returns the contents of `self`.
    #[inline]
    #[must_use]
    pub const fn inner(self) -> i8 {
        self.d
    }
}

impl File {
    /// Returns the contents of `self`.
    #[inline]
    #[must_use]
    pub const fn inner(self) -> u8 {
        self.f
    }
}

impl Piece {
    /// Returns the contents of `self`.
    #[inline]
    #[must_use]
    pub const fn inner(self) -> u8 {
        self.p
    }

    /// Returns the contents of `self` as a `usize`.
    #[inline]
    #[must_use]
    pub const fn to_index(self) -> usize {
        self.inner() as usize
    }
}

impl Rank {
    /// Returns the contents of `self`.
    #[inline]
    #[must_use]
    pub const fn inner(self) -> u8 {
        self.r
    }
}

impl Side {
    /// Flips the contents of `self`.
    ///
    /// e.g. `Side::WHITE.flip() == Side::BLACK`.
    ///
    /// The result is undefined if `self` isn't `Side::WHITE` or `Side::BLACK`.
    #[inline]
    #[must_use]
    pub const fn flip(self) -> Self {
        Self::from(self.inner() ^ 1)
    }

    /// Returns the contents of `self`.
    #[inline]
    #[must_use]
    pub const fn inner(self) -> u8 {
        self.s
    }

    /// Returns the contents of `self` as a bool: White is `true`, Black is
    /// `false`.
    #[inline]
    #[must_use]
    pub const fn to_bool(self) -> bool {
        self.inner() != 0
    }

    /// Returns the contents of `self` as a `usize`.
    #[inline]
    #[must_use]
    pub const fn to_index(self) -> usize {
        self.inner() as usize
    }
}

impl Square {
    /// Calculates the file that `self` is on.
    #[inline]
    #[must_use]
    pub const fn file_of(self) -> File {
        File::from(self.inner() & 7)
    }

    /// Finds the horizontal distance between `self` and `other_square`
    #[inline]
    #[must_use]
    pub const fn horizontal_distance(self, other_square: Self) -> u8 {
        #[allow(clippy::cast_possible_wrap)]
        (self.file_of().inner() as i8 - other_square.file_of().inner() as i8).unsigned_abs()
    }

    /// Returns the contents of `self`.
    #[inline]
    #[must_use]
    pub const fn inner(self) -> u8 {
        self.sq
    }

    /// Checks if `self` is within the board.
    #[inline]
    #[must_use]
    pub fn is_valid(self) -> bool {
        // `sq` is unsigned so it can't be less than 0.
        self <= Self::H8
    }

    /// Calculates the rank that `self` is on.
    #[inline]
    #[must_use]
    pub const fn rank_of(self) -> Rank {
        Rank::from(self.inner() >> 3)
    }

    /// Converts `self` into its string representation.
    #[inline]
    #[must_use]
    pub fn stringify(self) -> String {
        let mut ret = String::with_capacity(2);
        ret.push((b'a' + self.file_of().inner()) as char);
        ret.push((b'1' + self.rank_of().inner()) as char);
        ret
    }

    /// Returns the contents of `self` as a `usize`.
    #[inline]
    #[must_use]
    pub const fn to_index(self) -> usize {
        self.inner() as usize
    }
}

/// Converts the piece `piece` on side `side` to a character.
///
/// e.g. `Side::WHITE, Piece::KNIGHT -> 'N'`;
/// `Side::BLACK, Piece::KING -> 'k'`.
#[inline]
#[must_use]
pub const fn piece_to_char(side: Side, piece: Piece) -> char {
    PIECE_CHARS[side.to_index()][piece.to_index()]
}
