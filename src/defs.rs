pub type Bitboard = u64;
// Move = 0b0000000000000000
//          |--||----||----|
// First 6 bits for start pos (0-63)
// Next 6 bits for end pos (0-63)
// Last 4 bits for flags (unused)
pub type Move = u16;

pub struct Files;
pub struct Pieces;
pub struct Ranks;
pub struct Nums;

pub const PIECE_CHARS: [[char; Nums::PIECES]; Nums::SIDES] = [
    ['P', 'N', 'B', 'R', 'Q', 'K'],
    ['p', 'n', 'b', 'r', 'q', 'k'],
];

impl Files {
    pub const FILE1: u8 = 0;
    pub const FILE8: u8 = 7;
}

impl Pieces {
    pub const PAWN:   usize = 0;
    pub const KNIGHT: usize = 1;
    pub const BISHOP: usize = 2;
    pub const ROOK:   usize = 3;
    pub const QUEEN:  usize = 4;
    pub const KING:   usize = 5;
}

impl Ranks {
    pub const RANK1: u8 = 0;
    pub const RANK8: u8 = 7;
}

impl Nums {
    pub const SIDES:  usize = 2;
    pub const PIECES: usize = 6;
}
