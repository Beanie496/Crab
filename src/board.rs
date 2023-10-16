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

    pub fn pretty_print(&self) {
        for r in (Ranks::RANK1..=Ranks::RANK8).rev() {
            print!("{} | ", rank_to_char(r));
            for f in Files::FILE1..=Files::FILE8 {
                print!("{} ", self.char_piece_from_pos(r, f));
            }
            println!("")
        }
        println!("    ---------------");
        println!("    1 2 3 4 5 6 7 8");
    }

    fn char_piece_from_pos(&self, rank: u8, file: u8) -> char {
        let sq_bb = bitboard_from_pos(rank, file);
        if sq_bb & self.white != 0 {
            if sq_bb & self.pawns != 0 {
                'P'
            } else if sq_bb & self.knights != 0 {
                'N'
            } else if sq_bb & self.bishops != 0 {
                'B'
            } else if sq_bb & self.rooks != 0 {
                'R'
            } else if sq_bb & self.queens != 0 {
                'Q'
            } else if sq_bb & self.kings != 0 {
                'K'
            } else {
                panic!("White bb does not match up with the piece bbs");
            }
        } else if sq_bb & self.black != 0 {
            if sq_bb & self.pawns != 0 {
                'p'
            } else if sq_bb & self.knights != 0 {
                'n'
            } else if sq_bb & self.bishops != 0 {
                'b'
            } else if sq_bb & self.rooks != 0 {
                'r'
            } else if sq_bb & self.queens != 0 {
                'q'
            } else if sq_bb & self.kings != 0 {
                'k'
            } else {
                panic!("Black bb does not match up with the piece bbs");
            }
        } else {
            '0'
        }
    }
}

fn rank_to_char(rank: u8) -> char {
    char::from_u32('A' as u32 + rank as u32).unwrap()
}

fn bitboard_from_pos(rank: u8, file: u8) -> Bitboard {
    1u64 << (rank * 8 + file)
}
