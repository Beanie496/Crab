use std::arch::x86_64::_rdrand16_step;
use crate::{
    defs::*,
    movelist::*,
    util::pop_next_square,
};

pub struct Board {
    sides:  [Bitboard; Nums::SIDES],
    pieces: [Bitboard; Nums::PIECES],
    ml:      Movelist,
}

impl Board {
    pub fn new() -> Board {
        Board {
            sides: [
                0x000000000000ffff, // White
                0xffff000000000000, // Black
            ],
            pieces: [
                0x00ff00000000ff00, // Pawns
                0x4200000000000042, // Knights
                0x2400000000000024, // Bishops
                0x8100000000000081, // Rooks
                0x0800000000000008, // Queens
                0x1000000000000010, // Kings
            ],
            ml: Movelist::new(),
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
        for i in 0..Nums::SIDES {
            for j in 0..Nums::PIECES {
                if sq_bb & self.sides[i] & self.pieces[j] != 0 {
                    return PIECE_CHARS[i][j];
                }
            }
        }
        '0'
    }
}

impl Board {
    pub fn perft(&mut self, depth: u8) -> u8 {
        if depth == 0 {
            return 1;
        }

        self.generate_moves();

        let mut total = 0;
        while let Some(result) = self.ml.next() {
            //make_move();
            total += self.perft(depth - 1);
            //unmake_move();
        }
        total
    }

    pub fn next_move(&mut self) -> Option<Move> {
        self.ml.next()
    }
}

impl Board {
    pub fn generate_moves(&mut self) {
        // pawn moves
        {
            let mut pawns = self.pieces[Pieces::PAWN];
            while pawns != 0 {
                let src = pop_next_square(&mut pawns);
                let mut dest: u16 = 0;
                unsafe { _rdrand16_step(&mut dest); }
                self.ml.push_move(src, dest as u8);
            }
        }
    }
}

fn rank_to_char(rank: u8) -> char {
    char::from_u32('A' as u32 + rank as u32).unwrap()
}

fn bitboard_from_pos(rank: u8, file: u8) -> Bitboard {
    1u64 << (rank * 8 + file)
}
