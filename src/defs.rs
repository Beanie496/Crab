pub type Bitboard = u64;

pub struct Files;
impl Files {
    pub const FILE1: u8 = 0;
    pub const FILE8: u8 = 7;
}

pub struct Ranks;
impl Ranks {
    pub const RANK1: u8 = 0;
    pub const RANK8: u8 = 7;
}
