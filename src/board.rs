use crate::{
    defs::*,
    movelist::*,
};

pub struct Board {
    sides:  [Bitboard; Nums::SIDES as usize],
    pieces: [Bitboard; Nums::PIECES as usize],
    ml:      Movelist,
}

impl Board {
    /// Returns a new Board object initialised with the state of the starting
    /// position.
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

    /// Pretty-prints the current state of the board.
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

    /// Returns the piece on the square given by the rank and file, otherwise
    /// returns '0'.
    fn char_piece_from_pos(&self, rank: u8, file: u8) -> char {
        let sq_bb = bitboard_from_pos(rank, file);
        for i in 0..Nums::SIDES as usize {
            for j in 0..Nums::PIECES as usize {
                if sq_bb & self.sides[i] & self.pieces[j] != 0 {
                    return PIECE_CHARS[i][j];
                }
            }
        }
        '0'
    }
}

impl Board {
    /// Runs perft on the given depth.
    pub fn perft(&mut self, depth: u8) -> u8 {
        depth
    }
}

/// Returns the character representation of the given rank.
fn rank_to_char(rank: u8) -> char {
    char::from_u32('A' as u32 + rank as u32).unwrap()
}

/// Returns a 0-initialised bitboard with the bit in the given position set.
fn bitboard_from_pos(rank: u8, file: u8) -> Bitboard {
    1u64 << (rank * 8 + file)
}
