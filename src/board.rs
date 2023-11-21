use crate::{
    bits::{as_bitboard, bitboard_from_pos},
    defs::{Bitboard, File, Files, Move, Nums, Piece, Rank, Ranks, Side, Sides, PIECE_CHARS},
    movelist::Movelist,
};
use movegen::util::decompose_move;

/// Items related to move generation.
pub mod movegen;

/// Stores information about the current state of the board.
pub struct Board {
    /// The moves played since the initial position.
    played_moves: Movelist,
    /// `pieces[0]` is the intersection of all pawns on the board, `pieces[1]`
    /// is the knights, and so on, as according to the order set by
    /// [`Pieces`](crate::defs::Pieces).
    pieces: [Bitboard; Nums::PIECES],
    /// `sides[1]` is the intersection of all White piece bitboards; `sides[0]`
    /// is is the intersection of all Black piece bitboards.
    sides: [Bitboard; Nums::SIDES],
    /// The side to move - 0 or 1 for White or Black respectively.
    side_to_move: Side,
}

impl Board {
    /// Creates a new [`Board`] initialised with the state of the starting
    /// position.
    pub fn new() -> Self {
        Self {
            played_moves: Movelist::new(),
            pieces: [
                0x00ff00000000ff00, // Pawns
                0x4200000000000042, // Knights
                0x2400000000000024, // Bishops
                0x8100000000000081, // Rooks
                0x0800000000000008, // Queens
                0x1000000000000010, // Kings
            ],
            sides: [
                0xffff000000000000, // Black
                0x000000000000ffff, // White
            ],
            side_to_move: Sides::WHITE,
        }
    }
}

impl Board {
    /// Makes the given move on the internal board. `mv` is assumed to be a
    /// valid move.
    pub fn make_move(&mut self, mv: Move) {
        self.played_moves.push_move(mv);
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
            print!("{} | ", r + 1);
            for f in Files::FILE1..=Files::FILE8 {
                print!("{} ", self.char_piece_from_pos(r, f));
            }
            println!();
        }
        println!("    ---------------");
        println!("    a b c d e f g h");
    }

    /// Returns side to move
    pub fn side_to_move(&self) -> Side {
        self.side_to_move
    }

    /// Returns the board of the side according to `IS_WHITE`.
    pub fn sides<const IS_WHITE: bool>(&self) -> Bitboard {
        if IS_WHITE {
            self.sides[Sides::WHITE]
        } else {
            self.sides[Sides::BLACK]
        }
    }

    /// Unplays the most recent move. Assumes that a move has been played.
    pub fn unmake_move(&mut self) {
        let mv = self.played_moves.pop_move().unwrap();
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
    };

    #[test]
    fn make_and_unmake() {
        let mut board = Board::new();

        let mv = create_move::<true, { Pieces::ROOK }>(Squares::A1, Squares::A3);
        board.make_move(mv);
        assert_eq!(board.sides[Sides::WHITE], 0x000000000001fffe);
        assert_eq!(board.pieces[Pieces::ROOK], 0x8100000000010080);
        board.unmake_move();
        assert_eq!(board.sides[Sides::WHITE], 0x000000000000ffff);
        assert_eq!(board.pieces[Pieces::ROOK], 0x8100000000000081);
    }
}
