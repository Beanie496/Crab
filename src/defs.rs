use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use crate::{bitboard::Bitboard, error::ParseError};

/// A file: file A = 0 to file F = 7.
#[derive(Clone, Copy)]
pub struct File(pub u8);

/// A wrapper for certain types of move. See associated constants for the
/// current types.
pub struct MoveType;

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

/// A rank: rank 1 = 0 to rank 8 = 7.
#[derive(Clone, Copy)]
pub struct Rank(pub u8);

/// A side: 0, 1 for a regular side or 2 for no side.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Side(pub u8);

/// A square: with little-endian rank-file mapping: a1 = 0, b1 = 1, etc.
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

/// Enumerates files.
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
#[allow(dead_code, clippy::missing_docs_in_private_items)]
impl Side {
    pub const BLACK: Self = Self(0);
    pub const WHITE: Self = Self(1);
    pub const TOTAL: usize = 2;
    pub const NONE: Self = Self(2);
}

/// Enumerates squares.
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

impl From<File> for char {
    /// Converts a rank into a character: 'a' to 'h'.
    fn from(file: File) -> Self {
        (b'a' + file.0) as Self
    }
}

impl From<Piece> for char {
    /// Converts a piece into a character: 'P' for White pawn, 'k' for Black
    /// king, etc.
    fn from(piece: Piece) -> Self {
        PIECE_CHARS[piece.to_index()]
    }
}

impl From<PieceType> for char {
    /// Converts a piece type into a character: 'p' for pawn to 'k' for king.
    fn from(piece_type: PieceType) -> Self {
        let piece = Piece::from_piecetype(piece_type, Side::BLACK);
        Self::from(piece)
    }
}

impl From<Rank> for char {
    /// Converts a rank into a character: '1' to '8'.
    fn from(rank: Rank) -> Self {
        (b'1' + rank.0) as Self
    }
}

impl From<Side> for char {
    /// Converts a side into a char, assuming the side is White or Black.
    ///
    /// 'w' if White and 'b' if Black; undefined otherwise.
    fn from(side: Side) -> Self {
        (b'b' + side.0 * 21) as Self
    }
}

impl From<Square> for File {
    /// Calculates the file of a square.
    fn from(square: Square) -> Self {
        Self(square.0 & 7)
    }
}

impl TryFrom<char> for Piece {
    type Error = ParseError;

    /// Converts a piece character specified by FEN into an actual piece.
    fn try_from(piece: char) -> Result<Self, Self::Error> {
        Ok(match piece {
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
            _ => return Err(ParseError::InvalidToken),
        })
    }
}

impl TryFrom<char> for PieceType {
    type Error = ParseError;

    /// Converts a piece character specified by FEN into an actual type of
    /// piece.
    fn try_from(piece: char) -> Result<Self, Self::Error> {
        let piece = piece.to_ascii_lowercase();
        Ok(match piece {
            'p' => Self::PAWN,
            'n' => Self::KNIGHT,
            'b' => Self::BISHOP,
            'r' => Self::ROOK,
            'q' => Self::QUEEN,
            'k' => Self::KING,
            _ => return Err(ParseError::InvalidToken),
        })
    }
}

impl From<Piece> for PieceType {
    /// Calculates the type of a piece.
    fn from(piece: Piece) -> Self {
        Self(piece.0 >> 1)
    }
}

impl From<Square> for Rank {
    /// Calculates the rank of a square.
    fn from(square: Square) -> Self {
        Self(square.0 >> 3)
    }
}

impl From<Piece> for Side {
    /// Calculates the side of a piece.
    fn from(piece: Piece) -> Self {
        Self(piece.0 & 1)
    }
}

impl Display for Square {
    /// Converts a square into its string representation: the square if `self`
    /// isn't [`NONE`](Self::NONE) (e.g. "b3") or "-" otherwise.
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let mut ret_str = String::new();
        if *self == Self::NONE {
            fmt.write_str("-")
        } else {
            ret_str.push(char::from(File::from(*self)));
            ret_str.push(char::from(Rank::from(*self)));
            fmt.write_str(&ret_str)
        }
    }
}

impl From<Bitboard> for Square {
    /// Converts the position of the LSB of `bb` to a [`Square`].
    fn from(bb: Bitboard) -> Self {
        Self(bb.0.trailing_zeros() as u8)
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

        let file = iter.next().ok_or(ParseError::ExpectedToken)?;
        if (b'a'..=b'h').contains(file) {
            square += file - b'a';
        } else {
            return Err(ParseError::ErroneousToken);
        }

        let rank = iter.next().ok_or(ParseError::ExpectedToken)?;
        if (b'1'..=b'8').contains(rank) {
            square += (rank - b'1') * 8;
        } else {
            return Err(ParseError::ErroneousToken);
        }

        Ok(Self(square))
    }
}

impl Piece {
    /// Creates a [`Piece`] from a [`PieceType`] and a [`Side`].
    pub const fn from_piecetype(piece: PieceType, side: Side) -> Self {
        Self((piece.0 << 1) + side.0)
    }

    /// Converts the piece to a usize.
    pub const fn to_index(self) -> usize {
        self.0 as usize
    }
}

impl PieceType {
    /// Converts the piece type to a usize.
    pub const fn to_index(self) -> usize {
        self.0 as usize
    }
}

impl Side {
    /// Flips the side.
    ///
    /// e.g. `Side::WHITE.flip() == Side::BLACK`.
    ///
    /// The result is undefined if the square isn't [`Side::WHITE`] or
    /// [`Side::BLACK`].
    pub const fn flip(self) -> Self {
        Self(self.0 ^ 1)
    }

    /// Converts the side to a usize.
    pub const fn to_index(self) -> usize {
        self.0 as usize
    }
}

impl Square {
    /// Converts `rank` and `file` into a [`Square`].
    pub const fn from_pos(rank: Rank, file: File) -> Self {
        Self(rank.0 * 8 + file.0)
    }

    /// Converts the square to a usize.
    pub const fn to_index(self) -> usize {
        self.0 as usize
    }
}
