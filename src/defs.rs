pub type Bitboard = u64;
/**
 * Start pos == 6 bits, 0-63
 *
 * End pos == 6 bits, 0-63
 *
 * Flags == 4 bits
 * ```
 * Move ==
 *     ((start & 0x3f) << 0)
 *     | ((end & 0x3f) << 6)
 *     | ((flags & 0xf) << 12);
 *
 * ```
 */
pub type Move = u16;
pub type Piece = u8;

/// Enumerates files, from file 1 = 0 to file 8 = 7.
pub struct Files;
/// Constants associated with Chess (sides = 2, etc.).
pub struct Nums;
/// Enumerates pieces, from pawn = 0 to king = 5.
pub struct Pieces;
/// Enumerates ranks, from rank 1 = 0 to rank 8 = 7.
pub struct Ranks;
/// Enumerates sides. White = 0, Black = 1.
pub struct Sides;
/// Enumerates squares, from a1 = 0 to a8 = 7 to h8 = 63.
pub struct Squares;

/** An array of character constants associated with each piece on both sides.
 * ```
 * assert_eq!(PIECE_CHARS[Sides::WHITE][Pieces::PAWN], 'P');
 * assert_eq!(PIECE_CHARS[Sides::BLACK][Pieces::PAWN], 'p');
 * // etc.
 * ```
 */
pub const PIECE_CHARS: [[char; Nums::PIECES as usize]; Nums::SIDES as usize] = [
    ['P', 'N', 'B', 'R', 'Q', 'K'],
    ['p', 'n', 'b', 'r', 'q', 'k'],
];

impl Files {
    pub const FILE1: u8 = 0;
    pub const FILE8: u8 = 7;
}

impl Nums {
    pub const SIDES:  u8 = 2;
    pub const PIECES: u8 = 6;
}

impl Pieces {
    pub const PAWN:   Piece = 0;
    pub const KNIGHT: Piece = 1;
    pub const BISHOP: Piece = 2;
    pub const ROOK:   Piece = 3;
    pub const QUEEN:  Piece = 4;
    pub const KING:   Piece = 5;
}

impl Ranks {
    pub const RANK1: u8 = 0;
    pub const RANK8: u8 = 7;
}

impl Sides {
    pub const WHITE: u8 = 0;
    pub const BLACK: u8 = 1;
}

impl Squares {
    pub const A1: u8 = 0;
    pub const A8: u8 = 7;
    pub const H1: u8 = 56;
    pub const H8: u8 = 63;
}
