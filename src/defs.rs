use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

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
pub const PIECE_CHARS: [[char; Nums::PIECES]; Nums::SIDES] = [
    ['p', 'n', 'b', 'r', 'q', 'k'],
    ['P', 'N', 'B', 'R', 'Q', 'K'],
];

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::new(self.inner() & rhs.inner())
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
        Self::new(self.inner() | rhs.inner())
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
        Self::new(self.inner() ^ rhs.inner())
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
        Self::new(!self.inner())
    }
}

/// Pre-constructed bitboards.
impl Bitboard {
    pub const EMPTY: Bitboard = Self::new(0);
    pub const CASTLING_SPACE_WK: Bitboard = Self::new(0x0000000000000060);
    pub const CASTLING_SPACE_WQ: Bitboard = Self::new(0x00000000000000e0);
    pub const CASTLING_SPACE_BK: Bitboard = Self::new(0x6000000000000000);
    pub const CASTLING_SPACE_BQ: Bitboard = Self::new(0x0e00000000000000);
    /// ```
    /// assert_eq!(Bitboard::FILE_BB[File::FILE1.to_index()], Self::new(0x0101010101010101));
    /// assert_eq!(Bitboard::FILE_BB[File::FILE2.to_index()], Self::new(0x0202020202020202));
    /// // etc.
    /// ```
    pub const FILE_BB: [Bitboard; Nums::FILES] = [
        Self::new(0x0101010101010101),
        Self::new(0x0202020202020202),
        Self::new(0x0404040404040404),
        Self::new(0x0808080808080808),
        Self::new(0x1010101010101010),
        Self::new(0x2020202020202020),
        Self::new(0x4040404040404040),
        Self::new(0x8080808080808080),
    ];
    /// ```
    /// assert_eq!(Bitboard::RANK_BB[Rank::RANK1.to_index()], Self::new(0x00000000000000ff));
    /// assert_eq!(Bitboard::RANK_BB[Rank::RANK2.to_index()], Self::new(0x000000000000ff00));
    /// // etc.
    /// ```
    pub const RANK_BB: [Bitboard; Nums::RANKS] = [
        Self::new(0x00000000000000ff),
        Self::new(0x000000000000ff00),
        Self::new(0x0000000000ff0000),
        Self::new(0x00000000ff000000),
        Self::new(0x000000ff00000000),
        Self::new(0x0000ff0000000000),
        Self::new(0x00ff000000000000),
        Self::new(0xff00000000000000),
    ];
}

/// The square difference in each of the 8 directions.
impl Direction {
    pub const N: Direction = Self::new(8);
    pub const NE: Direction = Self::new(9);
    pub const E: Direction = Self::new(1);
    pub const SE: Direction = Self::new(-7);
    pub const S: Direction = Self::new(-8);
    pub const SW: Direction = Self::new(-9);
    pub const W: Direction = Self::new(-1);
    pub const NW: Direction = Self::new(7);
}

/// Enumerates files.
impl File {
    pub const FILE1: File = Self::new(0);
    pub const FILE8: File = Self::new(7);
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
    pub const PAWN: Piece = Self::new(0);
    pub const KNIGHT: Piece = Self::new(1);
    pub const BISHOP: Piece = Self::new(2);
    pub const ROOK: Piece = Self::new(3);
    pub const QUEEN: Piece = Self::new(4);
    pub const KING: Piece = Self::new(5);
    pub const NONE: Piece = Self::new(6);
}

/// Enumerates ranks.
impl Rank {
    pub const RANK1: Rank = Self::new(0);
    pub const RANK2: Rank = Self::new(1);
    pub const RANK4: Rank = Self::new(3);
    pub const RANK5: Rank = Self::new(4);
    pub const RANK7: Rank = Self::new(6);
    pub const RANK8: Rank = Self::new(7);
}

/// Enumerates sides.
impl Side {
    pub const BLACK: Side = Self::new(0);
    pub const WHITE: Side = Self::new(1);
    pub const NONE: Side = Self::new(2);
}

/// Enumerates squares. This engine uses little-endian rank-file mapping.
// Allowed dead code because unit tests use these.
#[allow(dead_code)]
impl Square {
    pub const A1: Square = Self::new(0);
    pub const C1: Square = Self::new(2);
    pub const E1: Square = Self::new(4);
    pub const G1: Square = Self::new(6);
    pub const H1: Square = Self::new(7);
    pub const A3: Square = Self::new(16);
    pub const E4: Square = Self::new(28);
    pub const D5: Square = Self::new(35);
    pub const C6: Square = Self::new(42);
    pub const E6: Square = Self::new(44);
    pub const A7: Square = Self::new(48);
    pub const A8: Square = Self::new(56);
    pub const E8: Square = Self::new(60);
    pub const H8: Square = Self::new(63);
    pub const NONE: Square = Self::new(64);
    /// Should only be used when the [`Square`] needs to be 0.
    pub const NULL: Square = Self::new(0);
}

impl Bitboard {
    /// Converts `rank` and `file` into a [`Bitboard`] with the bit in the
    /// given position set.
    pub fn from_pos(rank: Rank, file: File) -> Self {
        Self::new(1 << (rank.r * 8 + file.f))
    }

    /// Creates a new [`Bitboard`] with contents `bb`.
    pub const fn new(bb: u64) -> Self {
        Self { bb }
    }
}

impl Direction {
    /// Creates a new [`Direction`] with contents `direction`.
    pub const fn new(direction: i8) -> Self {
        Self { d: direction }
    }
}

impl File {
    /// Creates a new [`File`] with contents `file`.
    pub const fn new(file: u8) -> Self {
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

    /// Creates a new [`Piece`] with contents `piece`.
    pub const fn new(piece: u8) -> Self {
        Self { p: piece }
    }
}

impl Rank {
    /// Creates a new [`Rank`] with contents `rank`.
    pub const fn new(rank: u8) -> Self {
        Self { r: rank }
    }
}

impl Side {
    /// Creates a new [`Side`] with contents `side`.
    pub const fn new(side: u8) -> Self {
        Self { s: side }
    }
}

impl Square {
    /// Creates a new [`Square`] with contents `square`.
    pub const fn new(square: u8) -> Self {
        Self { sq: square }
    }

    /// Converts `rank` and `file` into a [`Square`].
    pub fn from_pos(rank: Rank, file: File) -> Self {
        Self::new(rank.r * 8 + file.f)
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

        Some(Self::new(square))
    }
}

impl Bitboard {
    /// Shifts `self` one square east without wrapping.
    pub fn east(self) -> Self {
        Self::new(self.inner() << 1) & !Bitboard::FILE_BB[File::FILE1.to_index()]
    }

    /// Returns the contents of `self`.
    pub fn inner(self) -> u64 {
        self.bb
    }

    /// Shifts `self` one square north without wrapping.
    pub fn north(self) -> Bitboard {
        Self::new(self.inner() << 8)
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
        Self::new(popped_bit)
    }

    /// Clears the least significant bit of `self` and converts the position of
    /// that bit to a [`Square`].
    pub fn pop_next_square(&mut self) -> Square {
        let shift = self.inner().trailing_zeros();
        self.bb ^= 1 << shift;
        Square::new(shift as u8)
    }

    /// Pretty prints `self`.
    // Allowed dead code because this is occasionally useful for debugging.
    #[allow(dead_code)]
    pub fn pretty_print(self) {
        for r in (Rank::RANK1.inner()..=Rank::RANK8.inner()).rev() {
            for f in File::FILE1.inner()..=File::FILE8.inner() {
                print!(
                    "{} ",
                    (self & Self::from_pos(Rank::new(r), File::new(f)) != Self::new(0)) as u32
                );
            }
            println!();
        }
        println!();
    }

    /// Shifts `self` one square south without wrapping.
    pub fn south(self) -> Bitboard {
        Self::new(self.inner() >> 8)
    }

    /// Converts the position of the least significant bit of `self` to a
    /// [`Square`].
    pub fn to_square(self) -> Square {
        Square::new(self.inner().trailing_zeros() as u8)
    }

    /// Shifts `self` one square west without wrapping.
    pub fn west(self) -> Bitboard {
        Self::new(self.inner() >> 1) & !Bitboard::FILE_BB[File::FILE8.to_index()]
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

    /// Returns the contents of `self` as a `usize`.
    pub fn to_index(self) -> usize {
        self.inner() as usize
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

    /// Returns the contents of `self` as a `usize`.
    pub fn to_index(self) -> usize {
        self.inner() as usize
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
        Self::new(self.inner() ^ 1)
    }

    /// Returns the contents of `self`.
    pub fn inner(self) -> u8 {
        self.s
    }

    /// Returns the contents of `self` as a `usize`.
    pub fn to_index(self) -> usize {
        self.inner() as usize
    }
}

impl Square {
    /// Converts the contents of `self` into a [`Bitboard`] with the bit in the
    /// given position set.
    pub fn to_bitboard(self) -> Bitboard {
        Bitboard::new(1 << self.inner())
    }

    /// Calculates the file that `self` is on.
    pub fn file_of(self) -> File {
        File::new(self.inner() & 7)
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
        Rank::new(self.inner() >> 3)
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
