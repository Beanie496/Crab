pub type Bitboard = u64;
pub type File = usize;
pub type Direction = isize;
/**
 * A container for a move.
 *
 * From LSB onwards, a [`Move`] is as follows:
 * * Start pos == 6 bits, 0-63
 * * End pos == 6 bits, 0-63
 * * Piece == 3 bits, 0-5
 * * Side == 1 bit.
 * Storing the side to move is >50% faster than reading `side_to_move` off
 * [`Board`](crate::board::Board).
 *
 * The start and end are obvious. The piece and side are for fast access into
 * the board's arrays instead of requiring a match statement.
 * ```
 * Move ==
 *     ((start & 0x3f) << 0)
 *     | ((end & 0x3f) << 6)
 *     | ((piece & 0x7) << 12)
 *     | ((side & 0x1) << 15);
 *
 * ```
 */
pub type Move = u16;
pub type Piece = usize;
pub type Rank = usize;
pub type Side = usize;
pub type Square = usize;

/// Pre-constructed bitboards.
pub struct Bitboards;
/// The square difference in each of the 8 directions.
pub struct Directions;
/// Enumerates files.
pub struct Files;
/// Miscellaneous constants associated with chess (`SIDES == 2`, etc.)
pub struct Nums;
/// Enumerates pieces.
pub struct Pieces;
/// Enumerates ranks.
pub struct Ranks;
/// Enumerates sides.
pub struct Sides;
/// Enumerates squares. This engine uses little-endian rank-file mapping.
pub struct Squares;

/** An array of character constants associated with each piece on both sides.
 * ```
 * assert_eq!(PIECE_CHARS[Sides::WHITE][Pieces::PAWN], 'P');
 * assert_eq!(PIECE_CHARS[Sides::BLACK][Pieces::PAWN], 'p');
 * // etc.
 * ```
 */
pub const PIECE_CHARS: [[char; Nums::PIECES]; Nums::SIDES] = [
    ['p', 'n', 'b', 'r', 'q', 'k'],
    ['P', 'N', 'B', 'R', 'Q', 'K'],
];

impl Bitboards {
    pub const EMPTY: Bitboard = 0;
    /// ```
    /// assert_eq!(Bitboards::FILE_BB[Files::FILE1], 0x0101010101010101);
    /// assert_eq!(Bitboards::FILE_BB[Files::FILE2], 0x0202020202020202);
    /// // etc.
    /// ```
    pub const FILE_BB: [Bitboard; Nums::FILES] = [
        0x0101010101010101,
        0x0202020202020202,
        0x0404040404040404,
        0x0808080808080808,
        0x1010101010101010,
        0x2020202020202020,
        0x4040404040404040,
        0x8080808080808080,
    ];
    /// ```
    /// assert_eq!(Bitboards::RANK_BB[Ranks::RANK1], 0x00000000000000ff);
    /// assert_eq!(Bitboards::RANK_BB[Ranks::RANK2], 0x000000000000ff00);
    /// // etc.
    /// ```
    pub const RANK_BB: [Bitboard; Nums::RANKS] = [
        0x00000000000000ff,
        0x000000000000ff00,
        0x0000000000ff0000,
        0x00000000ff000000,
        0x000000ff00000000,
        0x0000ff0000000000,
        0x00ff000000000000,
        0xff00000000000000,
    ];
}

impl Directions {
    pub const N: Direction = 8;
    pub const NE: Direction = 9;
    pub const E: Direction = 1;
    pub const SE: Direction = -7;
    pub const S: Direction = -8;
    pub const SW: Direction = -9;
    pub const W: Direction = -1;
    pub const NW: Direction = 7;
}

impl Files {
    pub const FILE1: File = 0;
    pub const FILE8: File = 7;
}

impl Nums {
    pub const FILES: usize = 8;
    pub const SIDES: usize = 2;
    pub const SQUARES: usize = 64;
    pub const PIECES: usize = 6;
    pub const RANKS: usize = 8;
}

impl Pieces {
    pub const PAWN: Piece = 0;
    pub const KNIGHT: Piece = 1;
    pub const BISHOP: Piece = 2;
    pub const ROOK: Piece = 3;
    pub const QUEEN: Piece = 4;
    pub const KING: Piece = 5;
}

impl Ranks {
    pub const RANK1: Rank = 0;
    pub const RANK4: Rank = 3;
    pub const RANK5: Rank = 4;
    pub const RANK8: Rank = 7;
}

// Allowed dead code because a unit test uses BLACK.
#[allow(dead_code)]
impl Sides {
    pub const BLACK: Side = 0;
    pub const WHITE: Side = 1;
}

// Allowed dead code because unit tests use these.
#[allow(dead_code)]
impl Squares {
    pub const A1: Square = 0;
    pub const H1: Square = 7;
    pub const A3: Square = 16;
    pub const E4: Square = 28;
    pub const D5: Square = 35;
    pub const A8: Square = 56;
    pub const H8: Square = 63;
}
