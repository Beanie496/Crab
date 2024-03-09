use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

/// Tells the compiler that `index` cannot match or exceed `bound`.
///
/// # Panics
///
/// Will panic _in debug mode_ if `index >= bound`.
///
/// If `index >= bound` in release mode, it is undefined behaviour. Be careful
/// with this macro!
#[macro_export]
macro_rules! out_of_bounds_is_unreachable {
    ($index: expr, $bound: expr) => {{
        if $index >= $bound {
            #[cfg(debug_assertions)]
            panic!(
                "Unreachable code reached: index {} out of bound {}",
                $index, $bound
            );
            #[allow(unreachable_code)]
            std::hint::unreachable_unchecked()
        }
    }};
}

/// A wrapper for an `i8`, since a direction can go from -9 to 9.
pub struct Direction(pub i8);

/// A wrapper for a `u8`, since a file can go from 0 to 7.
#[derive(Clone, Copy)]
pub struct File(pub u8);

/// A wrapper for certain types of move. See associated constants for the
/// current types.
pub struct MoveType;

/// The error that happens if a parsed string is invalid.
pub struct ParseError;

/// A piece, containing the type of piece and side.
///
/// The internal order of pieces is the same as [`PieceType`], but the exact
/// constants are not.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Piece(pub u8);

/// A type of piece.
///
/// The internal order of pieces is the same as [`Piece`], but the exact
/// constants are not.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PieceType(pub u8);

/// A wrapper for a `u8`, since a rank can go from 0 to 7.
#[derive(Clone, Copy)]
pub struct Rank(pub u8);

/// A wrapper for a `u8`, since a side is just 0, 1 or 2.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Side(pub u8);

/// A wrapper for a `u8`, since a square can go from 0 to 64.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd)]
pub struct Square(pub u8);

/// An array of character constants associated with each piece on both sides,
/// with the character '0' at the end to allow conversion from [`Piece::NONE`].
///
/// e.g. `PIECE_CHARS[Piece::WKNIGHT] == 'N'`; `PIECE_CHARS[Piece::BKING] ==
/// 'k'`; `PIECE_CHARS[Piece::NONE] == '0'`.
const PIECE_CHARS: [char; Piece::TOTAL + 1] = [
    'p', 'P', 'n', 'N', 'b', 'B', 'r', 'R', 'q', 'Q', 'k', 'K', '0',
];

impl From<Piece> for char {
    fn from(piece: Piece) -> Self {
        PIECE_CHARS[piece.to_index()]
    }
}

impl From<PieceType> for char {
    fn from(piece_type: PieceType) -> Self {
        // default to lowercase, or Black
        let piece = Piece::from_piecetype(piece_type, Side::BLACK);
        Self::from(piece)
    }
}

impl From<Square> for File {
    fn from(square: Square) -> Self {
        Self(square.0 & 7)
    }
}

impl From<char> for Piece {
    fn from(piece: char) -> Self {
        match piece {
            'P' => Self::WPAWN,
            'N' => Self::WKNIGHT,
            'B' => Self::WBISHOP,
            'R' => Self::WROOK,
            'Q' => Self::WQUEEN,
            'K' => Self::WKING,
            'p' => Self::BPAWN,
            'n' => Self::BKNIGHT,
            'b' => Self::BBISHOP,
            'r' => Self::BROOK,
            'q' => Self::BQUEEN,
            'k' => Self::BKING,
            _ => Self::NONE,
        }
    }
}

impl From<char> for PieceType {
    fn from(piece: char) -> Self {
        let piece = piece.to_ascii_lowercase();
        match piece {
            'p' => Self::PAWN,
            'n' => Self::KNIGHT,
            'b' => Self::BISHOP,
            'r' => Self::ROOK,
            'q' => Self::QUEEN,
            'k' => Self::KING,
            _ => Self::NONE,
        }
    }
}

impl From<Piece> for PieceType {
    fn from(piece: Piece) -> Self {
        Self(piece.0 >> 1)
    }
}

impl From<Square> for Rank {
    fn from(square: Square) -> Self {
        Self(square.0 >> 3)
    }
}

impl From<Piece> for Side {
    /// Returns the side of `self`.
    fn from(piece: Piece) -> Self {
        Self(piece.0 & 1)
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            ((b'a' + File::from(*self).0) as char),
            ((b'1' + Rank::from(*self).0) as char),
        )
    }
}

impl FromStr for Square {
    type Err = ParseError;

    /// Converts a string representation of a square (e.g. "e4") into a
    /// [`Square`]. Will return `Ok(Self)` if the square is valid,
    /// `Ok(Self::NONE)` if the square is "-" and `Err(ParseError)` otherwise.
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if string == "-" {
            return Ok(Self::NONE);
        }

        let mut square = 0;
        let mut iter = string.as_bytes().iter();

        let file = iter.next().ok_or(ParseError)?;
        if (b'a'..=b'h').contains(file) {
            square += file - b'a';
        } else {
            return Err(ParseError);
        }

        let rank = iter.next().ok_or(ParseError)?;
        if (b'1'..=b'8').contains(rank) {
            square += (rank - b'1') * 8;
        } else {
            return Err(ParseError);
        }

        Ok(Self(square))
    }
}

/// The square difference in each of the 8 directions.
#[allow(dead_code, clippy::missing_docs_in_private_items)]
impl Direction {
    /// North.
    pub const N: Self = Self(8);
    /// North-east.
    pub const NE: Self = Self(9);
    /// East.
    pub const E: Self = Self(1);
    /// South-east.
    pub const SE: Self = Self(-7);
    /// South.
    pub const S: Self = Self(-8);
    /// South-west.
    pub const SW: Self = Self(-9);
    /// West.
    pub const W: Self = Self(-1);
    /// North-west.
    pub const NW: Self = Self(7);
}

/// Enumerates files.
///
/// To avoid casting everywhere, this isn't an enum.
#[allow(dead_code, clippy::missing_docs_in_private_items)]
impl File {
    pub const FILE1: Self = Self(0);
    pub const FILE2: Self = Self(1);
    pub const FILE3: Self = Self(2);
    pub const FILE4: Self = Self(3);
    pub const FILE5: Self = Self(4);
    pub const FILE6: Self = Self(5);
    pub const FILE7: Self = Self(6);
    pub const FILE8: Self = Self(7);
    pub const TOTAL: usize = 8;
}

#[allow(dead_code, clippy::missing_docs_in_private_items)]
impl MoveType {
    /// All moves.
    pub const ALL: u8 = 0;
    /// Captures only.
    pub const CAPTURES: u8 = 1;
}

/// Enumerates pieces for White and Black.
#[allow(dead_code, clippy::missing_docs_in_private_items)]
impl Piece {
    pub const WPAWN: Self = Self::from_piecetype(PieceType::PAWN, Side::WHITE);
    pub const WKNIGHT: Self = Self::from_piecetype(PieceType::KNIGHT, Side::WHITE);
    pub const WBISHOP: Self = Self::from_piecetype(PieceType::BISHOP, Side::WHITE);
    pub const WROOK: Self = Self::from_piecetype(PieceType::ROOK, Side::WHITE);
    pub const WQUEEN: Self = Self::from_piecetype(PieceType::QUEEN, Side::WHITE);
    pub const WKING: Self = Self::from_piecetype(PieceType::KING, Side::WHITE);
    pub const BPAWN: Self = Self::from_piecetype(PieceType::PAWN, Side::BLACK);
    pub const BKNIGHT: Self = Self::from_piecetype(PieceType::KNIGHT, Side::BLACK);
    pub const BBISHOP: Self = Self::from_piecetype(PieceType::BISHOP, Side::BLACK);
    pub const BROOK: Self = Self::from_piecetype(PieceType::ROOK, Side::BLACK);
    pub const BQUEEN: Self = Self::from_piecetype(PieceType::QUEEN, Side::BLACK);
    pub const BKING: Self = Self::from_piecetype(PieceType::KING, Side::BLACK);
    pub const TOTAL: usize = 12;
    pub const NONE: Self = Self(12);
}

/// Enumerates pieces.
#[allow(dead_code, clippy::missing_docs_in_private_items)]
impl PieceType {
    pub const PAWN: Self = Self(0);
    pub const KNIGHT: Self = Self(1);
    pub const BISHOP: Self = Self(2);
    pub const ROOK: Self = Self(3);
    pub const QUEEN: Self = Self(4);
    pub const KING: Self = Self(5);
    pub const TOTAL: usize = 6;
    pub const NONE: Self = Self(6);
}

/// Enumerates ranks.
///
/// To avoid casting everywhere, this isn't an enum.
#[allow(dead_code, clippy::missing_docs_in_private_items)]
impl Rank {
    pub const RANK1: Self = Self(0);
    pub const RANK2: Self = Self(1);
    pub const RANK3: Self = Self(2);
    pub const RANK4: Self = Self(3);
    pub const RANK5: Self = Self(4);
    pub const RANK6: Self = Self(5);
    pub const RANK7: Self = Self(6);
    pub const RANK8: Self = Self(7);
    pub const TOTAL: usize = 8;
}

/// Enumerates sides.
///
/// To avoid casting everywhere, this isn't an enum.
#[allow(dead_code, clippy::missing_docs_in_private_items)]
impl Side {
    pub const BLACK: Self = Self(0);
    pub const WHITE: Self = Self(1);
    pub const TOTAL: usize = 2;
    pub const NONE: Self = Self(2);
}

/// Enumerates squares. This engine uses little-endian rank-file mapping.
///
/// To avoid casting everywhere, this isn't an enum.
#[allow(dead_code, clippy::missing_docs_in_private_items)]
impl Square {
    pub const A1: Self = Self(0);
    pub const B1: Self = Self(1);
    pub const C1: Self = Self(2);
    pub const D1: Self = Self(3);
    pub const E1: Self = Self(4);
    pub const F1: Self = Self(5);
    pub const G1: Self = Self(6);
    pub const H1: Self = Self(7);
    pub const A2: Self = Self(8);
    pub const B2: Self = Self(9);
    pub const C2: Self = Self(10);
    pub const D2: Self = Self(11);
    pub const E2: Self = Self(12);
    pub const F2: Self = Self(13);
    pub const G2: Self = Self(14);
    pub const H2: Self = Self(15);
    pub const A3: Self = Self(16);
    pub const B3: Self = Self(17);
    pub const C3: Self = Self(18);
    pub const D3: Self = Self(19);
    pub const E3: Self = Self(20);
    pub const F3: Self = Self(21);
    pub const G3: Self = Self(22);
    pub const H3: Self = Self(23);
    pub const A4: Self = Self(24);
    pub const B4: Self = Self(25);
    pub const C4: Self = Self(26);
    pub const D4: Self = Self(27);
    pub const E4: Self = Self(28);
    pub const F4: Self = Self(29);
    pub const G4: Self = Self(30);
    pub const H4: Self = Self(31);
    pub const A5: Self = Self(32);
    pub const B5: Self = Self(33);
    pub const C5: Self = Self(34);
    pub const D5: Self = Self(35);
    pub const E5: Self = Self(36);
    pub const F5: Self = Self(37);
    pub const G5: Self = Self(38);
    pub const H5: Self = Self(39);
    pub const A6: Self = Self(40);
    pub const B6: Self = Self(41);
    pub const C6: Self = Self(42);
    pub const D6: Self = Self(43);
    pub const E6: Self = Self(44);
    pub const F6: Self = Self(45);
    pub const G6: Self = Self(46);
    pub const H6: Self = Self(47);
    pub const A7: Self = Self(48);
    pub const B7: Self = Self(49);
    pub const C7: Self = Self(50);
    pub const D7: Self = Self(51);
    pub const E7: Self = Self(52);
    pub const F7: Self = Self(53);
    pub const G7: Self = Self(54);
    pub const H7: Self = Self(55);
    pub const A8: Self = Self(56);
    pub const B8: Self = Self(57);
    pub const C8: Self = Self(58);
    pub const D8: Self = Self(59);
    pub const E8: Self = Self(60);
    pub const F8: Self = Self(61);
    pub const G8: Self = Self(62);
    pub const H8: Self = Self(63);
    pub const TOTAL: usize = 64;
    pub const NONE: Self = Self(64);
}

impl Piece {
    /// Creates a [`Piece`] from a [`PieceType`] and a [`Side`].
    #[must_use]
    pub const fn from_piecetype(piece: PieceType, side: Side) -> Self {
        Self((piece.0 << 1) + side.0)
    }

    /// Returns the contents of `self` as a `usize`.
    #[must_use]
    pub const fn to_index(self) -> usize {
        self.0 as usize
    }
}

impl PieceType {
    /// Returns the contents of `self` as a `usize`.
    #[must_use]
    pub const fn to_index(self) -> usize {
        self.0 as usize
    }
}

impl Side {
    /// Flips the contents of `self`.
    ///
    /// e.g. `Side::WHITE.flip() == Side::BLACK`.
    ///
    /// The result is undefined if `self` isn't `Side::WHITE` or `Side::BLACK`.
    #[must_use]
    pub const fn flip(self) -> Self {
        Self(self.0 ^ 1)
    }

    /// Returns the contents of `self` as a bool: White is `true`, Black is
    /// `false`.
    #[must_use]
    pub const fn to_bool(self) -> bool {
        self.0 != 0
    }

    /// Returns the contents of `self` as a `usize`.
    #[must_use]
    pub const fn to_index(self) -> usize {
        self.0 as usize
    }
}

impl Square {
    /// Converts `rank` and `file` into a [`Square`].
    #[must_use]
    pub const fn from_pos(rank: Rank, file: File) -> Self {
        Self(rank.0 * 8 + file.0)
    }

    /// Finds the horizontal distance between `self` and `other_square`
    #[must_use]
    pub fn horizontal_distance(self, other_square: Self) -> u8 {
        File::from(self).0.abs_diff(File::from(other_square).0)
    }

    /// Checks if `self` is within the board.
    #[must_use]
    pub fn is_valid(self) -> bool {
        // `sq` is unsigned so it can't be less than 0.
        self <= Self::H8
    }

    /// Returns the contents of `self` as a `usize`.
    #[must_use]
    pub const fn to_index(self) -> usize {
        self.0 as usize
    }
}
