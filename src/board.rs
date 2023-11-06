use crate::{
    defs::{ Bitboard, File, Files, Move, Nums, Rank, Ranks, Side, Sides, PIECE_CHARS },
    movelist::Movelist,
    util::decompose_move,
};

/// Stores information about the current state of the board.
pub struct Board {
    pub sides: [Bitboard; Nums::SIDES],
    pub pieces: [Bitboard; Nums::PIECES],
    pub side_to_move: Side,
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
            side_to_move: Sides::WHITE,
        }
    }
}

impl Board {
    /// Given a Move and a Movelist, update the board state to play the move
    /// and push the Move onto the Movelist.
    pub fn make_move(&mut self, mv: Move, ml: &mut Movelist) {
        ml.push_move(mv);
        let (start, end, piece, side) = decompose_move(mv);
        self.pieces[piece] ^= (1u64 << start) | (1u64 << end);
        self.sides[side] ^= (1u64 << start) | (1u64 << end);
        self.side_to_move ^= 1;
    }

    /// Given a a Movelist, pop the move off the Movelist and play it.
    pub fn unmake_move(&mut self, ml: &mut Movelist) {
        let mv = ml.pop_move().unwrap();
        let (start, end, piece, side) = decompose_move(mv);
        self.pieces[piece] ^= (1u64 << start) | (1u64 << end);
        self.sides[side] ^= (1u64 << start) | (1u64 << end);
        self.side_to_move ^= 1;
    }
}

impl Board {
    /// Pretty-prints the current state of the board.
    pub fn pretty_print(&self) {
        for r in (Ranks::RANK1..=Ranks::RANK8).rev() {
            print!("{} | ", rank_to_char(r));
            for f in Files::FILE1..=Files::FILE8 {
                print!("{} ", self.char_piece_from_pos(r, f));
            }
            println!();
        }
        println!("    ---------------");
        println!("    1 2 3 4 5 6 7 8");
    }

    /// Returns the piece on the square given by the rank and file, otherwise
    /// returns '0'.
    fn char_piece_from_pos(&self, rank: Rank, file: File) -> char {
        let sq_bb = bitboard_from_pos(rank, file);
        for (i, side_pieces) in PIECE_CHARS.iter().enumerate() {
            for (j, piece) in side_pieces.iter().enumerate() {
                if sq_bb & self.sides[i] & self.pieces[j] != 0 {
                    return *piece;
                }
            }
        }
        '0'
    }
}

/// Returns the character representation of the given rank.
fn rank_to_char(rank: Rank) -> char {
    unsafe { char::from_u32_unchecked((b'A' + rank) as u32) }
}

/// Returns a 0-initialised bitboard with the bit in the given position set.
fn bitboard_from_pos(rank: Rank, file: File) -> Bitboard {
    1u64 << (rank * 8 + file)
}
