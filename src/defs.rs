use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl};

// the idea for wrapping these types in structs and implementing a tonne of
// methids/associated functions is taken from viridithas, so thanks cosmo
/// A wrapper for a `u64`, since a bitboard is 64 bits.
#[derive(Clone, Copy, Debug, PartialEq)]
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
pub struct Nums;
/// A wrapper for a `u8`, since a piece can go from 0 to 12.
#[derive(Clone, Copy, PartialEq)]
pub struct Piece {
    p: u8,
}
/// A wrapper for a `u8`, since a rank can go from 0 to 7.
#[derive(Clone, Copy)]
pub struct Rank {
    r: u8,
}
/// A wrapper for a `u8`, since a side is just 0, 1 or 2.
#[derive(Clone, Copy, PartialEq)]
pub struct Side {
    s: u8,
}
/// A wrapper for a `u8`, since a square can go from 0 to 64.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Square {
    sq: u8,
}

/** An array of character constants associated with each piece on both sides.
 * ```
 * assert_eq!(PIECE_CHARS[Side::WHITE][Piece::PAWN.to_index()], 'P');
 * assert_eq!(PIECE_CHARS[Side::BLACK][Piece::PAWN.to_index()], 'p');
 * // etc.
 * ```
 */
const PIECE_CHARS: [[char; Nums::PIECES]; Nums::SIDES] = [
    ['p', 'n', 'b', 'r', 'q', 'k'],
    ['P', 'N', 'B', 'R', 'Q', 'K'],
];

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::from(self.inner() & rhs.inner())
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.bb &= rhs.inner();
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::from(self.inner() | rhs.inner())
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bb |= rhs.inner();
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self::from(self.inner() ^ rhs.inner())
    }
}

impl BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.bb ^= rhs.inner();
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::from(!self.inner())
    }
}

impl Shl<u8> for Bitboard {
    type Output = Self;

    fn shl(self, rhs: u8) -> Self::Output {
        Self::from(self.inner() << rhs)
    }
}

/// Pre-constructed bitboards.
impl Bitboard {
    pub const EMPTY: Bitboard = Self::from(0);
    pub const CASTLING_SPACE_WK: Bitboard = Self::from(0x0000000000000060);
    pub const CASTLING_SPACE_WQ: Bitboard = Self::from(0x00000000000000e0);
    pub const CASTLING_SPACE_BK: Bitboard = Self::from(0x6000000000000000);
    pub const CASTLING_SPACE_BQ: Bitboard = Self::from(0x0e00000000000000);
}

/// The square difference in each of the 8 directions.
impl Direction {
    pub const N: Direction = Self::from(8);
    pub const NE: Direction = Self::from(9);
    pub const E: Direction = Self::from(1);
    pub const SE: Direction = Self::from(-7);
    pub const S: Direction = Self::from(-8);
    pub const SW: Direction = Self::from(-9);
    pub const W: Direction = Self::from(-1);
    pub const NW: Direction = Self::from(7);
}

/// Enumerates files.
impl File {
    pub const FILE1: File = Self::from(0);
    pub const FILE8: File = Self::from(7);
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
    pub const PAWN: Piece = Self::from(0);
    pub const KNIGHT: Piece = Self::from(1);
    pub const BISHOP: Piece = Self::from(2);
    pub const ROOK: Piece = Self::from(3);
    pub const QUEEN: Piece = Self::from(4);
    pub const KING: Piece = Self::from(5);
    pub const NONE: Piece = Self::from(6);
}

/// Enumerates ranks.
impl Rank {
    pub const RANK1: Rank = Self::from(0);
    pub const RANK2: Rank = Self::from(1);
    pub const RANK4: Rank = Self::from(3);
    pub const RANK5: Rank = Self::from(4);
    pub const RANK7: Rank = Self::from(6);
    pub const RANK8: Rank = Self::from(7);
}

/// Enumerates sides.
impl Side {
    pub const BLACK: Side = Self::from(0);
    pub const WHITE: Side = Self::from(1);
    pub const NONE: Side = Self::from(2);
}

/// Enumerates squares. This engine uses little-endian rank-file mapping.
// Allowed dead code because unit tests use these.
#[allow(dead_code)]
impl Square {
    pub const A1: Square = Self::from(0);
    pub const C1: Square = Self::from(2);
    pub const E1: Square = Self::from(4);
    pub const G1: Square = Self::from(6);
    pub const H1: Square = Self::from(7);
    pub const A3: Square = Self::from(16);
    pub const E4: Square = Self::from(28);
    pub const D5: Square = Self::from(35);
    pub const C6: Square = Self::from(42);
    pub const E6: Square = Self::from(44);
    pub const A7: Square = Self::from(48);
    pub const A8: Square = Self::from(56);
    pub const E8: Square = Self::from(60);
    pub const H8: Square = Self::from(63);
    pub const NONE: Square = Self::from(64);
    /// Should only be used when the [`Square`] needs to be 0.
    pub const NULL: Square = Self::from(0);
}

impl Bitboard {
    /// ```
    /// assert_eq!(Bitboard::file_bb(File::FILE1), Self::from(0x0101010101010101));
    /// assert_eq!(Bitboard::file_bb(File::FILE2), Self::from(0x0202020202020202));
    /// // etc.
    /// ```
    pub fn file_bb(file: File) -> Self {
        Self::from(0x0101010101010101 << (file.inner() as u32))
    }

    /// Wraps a `u64` in a [`Bitboard`].
    pub const fn from(bb: u64) -> Self {
        Self { bb }
    }

    /// Converts `rank` and `file` into a [`Bitboard`] with the bit in the
    /// given position set.
    pub fn from_pos(rank: Rank, file: File) -> Self {
        Self::from(1 << (rank.inner() * 8 + file.inner()))
    }

    /// Converts a [`Square`] into a [`Bitboard`] with the bit in the given
    /// position set.
    pub fn from_square(square: Square) -> Self {
        Self::from(1 << square.inner())
    }

    /// ```
    /// assert_eq!(Bitboard::rank_bb(Rank::RANK1), Self::from(0x00000000000000ff));
    /// assert_eq!(Bitboard::rank_bb(Rank::RANK2), Self::from(0x000000000000ff00));
    /// // etc.
    /// ```
    pub fn rank_bb(rank: Rank) -> Self {
        Self::from(0xff << (rank.inner() as u32 * 8))
    }
}

impl Direction {
    /// Wraps an `i8` in a [`Direction`].
    pub const fn from(direction: i8) -> Self {
        Self { d: direction }
    }
}

impl File {
    /// Wraps a `u8` in a [`File`].
    pub const fn from(file: u8) -> Self {
        Self { f: file }
    }
}

impl Piece {
    /// Converts the char `piece` into a [`Piece`]. Will return `None` if the
    /// piece is not valid.
    pub fn from_char(piece: char) -> Option<Self> {
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
    pub const fn from(piece: u8) -> Self {
        Self { p: piece }
    }
}

impl Rank {
    /// Wraps a `u8` in a [`Rank`].
    pub const fn from(rank: u8) -> Self {
        Self { r: rank }
    }
}

impl Side {
    /// Wraps a `u8` in a [`Side`].
    pub const fn from(side: u8) -> Self {
        Self { s: side }
    }
}

impl Square {
    /// Wraps a `u8` in a [`Square`].
    pub const fn from(square: u8) -> Self {
        Self { sq: square }
    }

    /// Converts `rank` and `file` into a [`Square`].
    pub fn from_pos(rank: Rank, file: File) -> Self {
        Self::from(rank.inner() * 8 + file.inner())
    }

    /// Converts a string representation of a square (e.g. "e4") into a
    /// [`Square`]. Will return [`None`] if the square is not valid.
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
    pub fn east(self) -> Self {
        Self::from(self.inner() << 1) & !Bitboard::file_bb(File::FILE1)
    }

    /// Returns the contents of `self`.
    pub fn inner(self) -> u64 {
        self.bb
    }

    /// Shifts `self` one square north without wrapping.
    pub fn north(self) -> Bitboard {
        Self::from(self.inner() << 8)
    }

    /// Shifts `self` one square north if `IS_WHITE` is true, otherwise shifts
    /// `self` one square south.
    pub fn pawn_push<const IS_WHITE: bool>(self) -> Bitboard {
        if IS_WHITE {
            self.north()
        } else {
            self.south()
        }
    }

    /// Clears the least significant bit of `self` and returns it.
    pub fn pop_lsb(&mut self) -> Self {
        let popped_bit = self.inner() & self.inner().wrapping_neg();
        self.bb ^= popped_bit;
        Self::from(popped_bit)
    }

    /// Clears the least significant bit of `self` and converts the position of
    /// that bit to a [`Square`].
    pub fn pop_next_square(&mut self) -> Square {
        let shift = self.inner().trailing_zeros();
        self.bb ^= 1 << shift;
        Square::from(shift as u8)
    }

    /// Pretty prints `self`.
    // Allowed dead code because this is occasionally useful for debugging.
    #[allow(dead_code)]
    pub fn pretty_print(self) {
        for r in (Rank::RANK1.inner()..=Rank::RANK8.inner()).rev() {
            for f in File::FILE1.inner()..=File::FILE8.inner() {
                print!(
                    "{} ",
                    (self & Self::from_pos(Rank::from(r), File::from(f)) != Self::from(0)) as u32
                );
            }
            println!();
        }
        println!();
    }

    /// Shifts `self` one square south without wrapping.
    pub fn south(self) -> Bitboard {
        Self::from(self.inner() >> 8)
    }

    /// Converts the position of the least significant bit of `self` to a
    /// [`Square`].
    pub fn to_square(self) -> Square {
        Square::from(self.inner().trailing_zeros() as u8)
    }

    /// Shifts `self` one square west without wrapping.
    pub fn west(self) -> Bitboard {
        Self::from(self.inner() >> 1) & !Bitboard::file_bb(File::FILE8)
    }
}

impl Direction {
    /// Returns the contents of `self`.
    pub const fn inner(self) -> i8 {
        self.d
    }
}

impl File {
    /// Returns the contents of `self`.
    pub fn inner(self) -> u8 {
        self.f
    }
}

impl Piece {
    /// Returns the contents of `self`.
    pub const fn inner(self) -> u8 {
        self.p
    }

    /// Returns the contents of `self` as a `usize`.
    pub const fn to_index(self) -> usize {
        self.inner() as usize
    }
}

impl Rank {
    /// Returns the contents of `self`.
    pub fn inner(self) -> u8 {
        self.r
    }
}

impl Side {
    /// Flips the contents of `self`. The result is undefined if
    /// `self == Sides::NONE`.
    /// ```
    /// assert_eq(Sides::WHITE.flip(), Sides::BLACK);
    /// assert_eq(Sides::BLACK.flip(), Sides::WHITE);
    /// ```
    pub fn flip(self) -> Self {
        Self::from(self.inner() ^ 1)
    }

    /// Returns the contents of `self`.
    pub const fn inner(self) -> u8 {
        self.s
    }

    /// Returns the contents of `self` as a bool: White is `true`, Black is
    /// `false`.
    pub const fn to_bool(self) -> bool {
        self.inner() != 0
    }

    /// Returns the contents of `self` as a `usize`.
    pub fn to_index(self) -> usize {
        self.inner() as usize
    }
}

impl Square {
    /// Calculates the file that `self` is on.
    pub fn file_of(self) -> File {
        File::from(self.inner() & 7)
    }

    /// Finds the horizontal distance between `self` and `other_square`
    pub fn horizontal_distance(self, other_square: Square) -> u8 {
        (self.file_of().inner() as i8 - other_square.file_of().inner() as i8).unsigned_abs()
    }

    /// Returns the contents of `self`.
    pub fn inner(self) -> u8 {
        self.sq
    }

    /// Checks if `self` is within the board.
    pub fn is_valid(self) -> bool {
        // `sq` is unsigned so it can't be less than 0.
        self <= Square::H8
    }

    /// Calculates the rank that `self` is on.
    pub fn rank_of(self) -> Rank {
        Rank::from(self.inner() >> 3)
    }

    /// Converts `self` into its string representation.
    pub fn stringify(self) -> String {
        let mut ret = String::with_capacity(2);
        ret.push((b'a' + self.file_of().inner()) as char);
        ret.push((b'1' + self.rank_of().inner()) as char);
        ret
    }

    /// Returns the contents of `self` as a `usize`.
    pub fn to_index(self) -> usize {
        self.inner() as usize
    }
}

/// Converts the piece `piece` on side `side` to a character.
/// ```
/// assert_eq!(piece_to_char(Side::WHITE, Piece::KNIGHT), 'N');
/// assert_eq!(piece_to_char(Side::BLACK, Piece::QUEEN), 'q');
/// // etc.
/// ```
pub fn piece_to_char(side: Side, piece: Piece) -> char {
    PIECE_CHARS[side.to_index()][piece.to_index()]
}
