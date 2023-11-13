use crate::{
    bits::{as_bitboard, bitboard_from_pos},
    defs::{Bitboard, File, Files, Move, Nums, Piece, Rank, Ranks, Side, Sides, PIECE_CHARS},
    movegen::util::decompose_move,
    movelist::Movelist,
};

/// Stores information about the current state of the board.
#[derive(Debug, PartialEq)]
pub struct Board {
    /// `sides[1]` is the intersection of all White piece bitboards; `sides[0]`
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
                0xffff000000000000, // Black
                0x000000000000ffff, // White
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

    /// Converts `rank` into its character representation.
    fn rank_to_char(rank: Rank) -> char {
        unsafe { char::from_u32_unchecked((b'A' + rank as u8) as u32) }
    }
}

impl Board {
    /// Makes the given move on the internal board and pushes the move onto
    /// `ml`. `mv` is assumed to be a valid move.
    pub fn make_move(&mut self, mv: Move, ml: &mut Movelist) {
        ml.push_move(mv);
        let (start, end, piece, side) = decompose_move(mv);
        self.pieces[piece] ^= as_bitboard(start) | as_bitboard(end);
        self.sides[side] ^= as_bitboard(start) | as_bitboard(end);
        self.side_to_move ^= 1;
    }

    /// Returns all the occupied squares on the board.
    pub fn occupancies(&self) -> Bitboard {
        self.sides::<true>() | self.sides::<false>()
    }

    /// Returns the piece bitboard given by `piece`.
    pub fn pieces<const PIECE: Piece>(&self) -> Bitboard {
        self.pieces[PIECE]
    }

    /// Pretty-prints the current state of the board.
    pub fn pretty_print(&self) {
        for r in (Ranks::RANK1..=Ranks::RANK8).rev() {
            print!("{} | ", Self::rank_to_char(r));
            for f in Files::FILE1..=Files::FILE8 {
                print!("{} ", self.char_piece_from_pos(r, f));
            }
            println!();
        }
        println!("    ---------------");
        println!("    1 2 3 4 5 6 7 8");
    }

    /// Returns the board of the side according to `IS_WHITE`.
    pub fn sides<const IS_WHITE: bool>(&self) -> Bitboard {
        if IS_WHITE {
            self.sides[Sides::WHITE]
        } else {
            self.sides[Sides::BLACK]
        }
    }

    /// Pops a move off `ml` and unplays it.
    pub fn unmake_move(&mut self, ml: &mut Movelist) {
        let mv = ml.pop_move().unwrap();
        let (start, end, piece, side) = decompose_move(mv);
        self.pieces[piece] ^= as_bitboard(start) | as_bitboard(end);
        self.sides[side] ^= as_bitboard(start) | as_bitboard(end);
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

#[cfg(test)]
mod tests {
    use super::Board;

    use crate::{
        defs::{Pieces, Sides, Squares},
        movegen::util::create_move,
        movelist::Movelist,
    };

    #[test]
    fn make_and_unmake() {
        let mut board = Board::new();
        let mut ml = Movelist::new();

        let mv = create_move::<true, { Pieces::ROOK }>(Squares::A1, Squares::A3);
        board.make_move(mv, &mut ml);
        assert_eq!(board.sides[Sides::WHITE], 0x000000000001fffe);
        assert_eq!(board.pieces[Pieces::ROOK], 0x8100000000010080);
        board.unmake_move(&mut ml);
        assert_eq!(board, Board::new());
    }
}
