use crate::defs::*;

pub struct Board {
    white:   Bitboard,
    black:   Bitboard,
    pawns:   Bitboard,
    knights: Bitboard,
    bishops: Bitboard,
    rooks:   Bitboard,
    queens:  Bitboard,
    kings:   Bitboard,
}

impl Board {
    pub fn new() -> Board {
        Board {
            white:   0x000000000000ffff,
            black:   0xffff000000000000,
            pawns:   0x00ff00000000ff00,
            knights: 0x4200000000000042,
            bishops: 0x2400000000000024,
            rooks:   0x8100000000000081,
            queens:  0x0800000000000008,
            kings:   0x1000000000000010,
        }
    }
}
