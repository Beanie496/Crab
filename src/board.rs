use crate::{
    defs::{Bitboard, File, Files, Nums, Piece, Rank, Ranks, Side, Sides, PIECE_CHARS},
    movelist::Movelist,
    util::bitboard_from_pos,
};
use movegen::Lookup;

pub use movegen::magic::find_magics;

/// Bit-related functions relating to piece movement.
mod bits;
/// Items related to move generation.
mod movegen;

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
    /// position and initialises the static lookup tables.
    pub fn new() -> Self {
        Lookup::init();
        Self {
            played_moves: Movelist::new(),
            pieces: Self::default_pieces(),
            sides: Self::default_sides(),
            side_to_move: Self::default_side(),
        }
    }
}

impl Board {
    /// Returns the pieces of the starting position.
    /// ```
    /// assert_eq!(default_pieces()[Pieces::PAWN], 0x00ff00000000ff00);
    /// // etc.
    /// ```
    fn default_pieces() -> [Bitboard; Nums::PIECES] {
        [
            0x00ff00000000ff00, // Pawns
            0x4200000000000042, // Knights
            0x2400000000000024, // Bishops
            0x8100000000000081, // Rooks
            0x0800000000000008, // Queens
            0x1000000000000010, // Kings
        ]
    }

    /// Returns the sides of the starting position.
    /// ```
    /// assert_eq!(default_pieces()[Sides::WHITE], 0x000000000000ffff);
    /// assert_eq!(default_pieces()[Sides::BLACK], 0xffff000000000000);
    /// ```
    fn default_sides() -> [Bitboard; Nums::SIDES] {
        [
            0xffff000000000000, // Black
            0x000000000000ffff, // White
        ]
    }

    /// Returns the side to move from the starting position. Unless chess 1.1
    /// has been released, this will be [`Sides::WHITE`].
    fn default_side() -> Side {
        Sides::WHITE
    }
}

impl Board {
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

    /// Resets the board.
    pub fn set_startpos(&mut self) {
        self.pieces = Self::default_pieces();
        self.sides = Self::default_sides();
        self.side_to_move = Self::default_side();
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

    /// Returns all the occupied squares on the board.
    fn occupancies(&self) -> Bitboard {
        self.sides::<true>() | self.sides::<false>()
    }

    /// Returns the piece bitboard given by `piece`.
    fn pieces<const PIECE: Piece>(&self) -> Bitboard {
        self.pieces[PIECE]
    }

    /// Returns side to move
    fn side_to_move(&self) -> Side {
        self.side_to_move
    }

    /// Returns the board of the side according to `IS_WHITE`.
    fn sides<const IS_WHITE: bool>(&self) -> Bitboard {
        if IS_WHITE {
            self.sides[Sides::WHITE]
        } else {
            self.sides[Sides::BLACK]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Board;

    use crate::{
        board::movegen::util::create_move,
        defs::{Pieces, Sides, Squares},
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
