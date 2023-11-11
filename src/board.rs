use crate::{
    defs::{ Bitboard, File, Files, Move, Nums, Rank, Ranks, Side, Sides, PIECE_CHARS },
    movegen::util::decompose_move,
    movelist::Movelist,
};

/// Stores information about the current state of the board.
pub struct Board {
    /// `sides[0]` is the intersection of all White piece bitboards; `sides[1]`
    /// is is the intersection of all Black piece bitboards.
    pub sides: [Bitboard; Nums::SIDES],
    /// `pieces[0]` is the intersection of all pawns on the board, `pieces[1]`
    /// is the knights, and so on, as according to the order set by
    /// [`Pieces`](crate::defs::Pieces).
    pub pieces: [Bitboard; Nums::PIECES],
    /// The side to move - 0 or 1 for White or Black respectively.
    pub side_to_move: Side,
}

impl Board {
    /// Creates a new [`Board`] initialised with the state of the starting
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
    /// Makes the given move on the internal board and pushes the move onto
    /// `ml`.
    pub fn make_move(&mut self, mv: Move, ml: &mut Movelist) {
        ml.push_move(mv);
        let (start, end, piece, side) = decompose_move(mv);
        self.pieces[piece] ^= (1u64 << start) | (1u64 << end);
        self.sides[side] ^= (1u64 << start) | (1u64 << end);
        self.side_to_move ^= 1;
    }

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

    /// Pops a move off `ml` and unplays it.
    pub fn unmake_move(&mut self, ml: &mut Movelist) {
        let mv = ml.pop_move().unwrap();
        let (start, end, piece, side) = decompose_move(mv);
        self.pieces[piece] ^= (1u64 << start) | (1u64 << end);
        self.sides[side] ^= (1u64 << start) | (1u64 << end);
        self.side_to_move ^= 1;
    }
}

impl Board {
    /// Finds the piece on the given rank and file and converts it to its
    /// character representation. If no piece is on the square, returns '0'
    /// instead.
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

/// Converts `rank` and `file` into a bitboard with the bit in the given
/// position set.
fn bitboard_from_pos(rank: Rank, file: File) -> Bitboard {
    1u64 << (rank * 8 + file)
}

/// Converts `rank` into its character representation.
fn rank_to_char(rank: Rank) -> char {
    unsafe { char::from_u32_unchecked((b'A' + rank) as u32) }
}
