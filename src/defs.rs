pub type Bitboard = u64;
// Move = 0b0000000000000000
//          |--||----||----|
// First 6 bits for start pos (0-63)
// Next 6 bits for end pos (0-63)
// Last 4 bits for flags (unused)
pub type Move = u16;

pub struct Files;
pub struct Ranks;

impl Files {
    pub const FILE1: u8 = 0;
    pub const FILE8: u8 = 7;
}

impl Ranks {
    pub const RANK1: u8 = 0;
    pub const RANK8: u8 = 7;
}
