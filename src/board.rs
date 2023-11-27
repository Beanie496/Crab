use crate::{
    defs::{
        Bitboard, File, Files, Nums, Piece, Pieces, Rank, Ranks, Side, Sides, Square, PIECE_CHARS,
    },
    util::bitboard_from_pos,
};
use movegen::{Lookup, Move};

pub use movegen::magic::find_magics;

/// Items related to move generation.
pub mod movegen;

/// Stores information about the current state of the board.
pub struct Board {
    played_moves: Movelist,
    // `pieces[0]` is the intersection of all pawns on the board, `pieces[1]`
    // is the knights, and so on, as according to the order set by
    // [`Pieces`].
    pieces: [Bitboard; Nums::PIECES],
    // `sides[1]` is the intersection of all White piece bitboards; `sides[0]`
    // is is the intersection of all Black piece bitboards.
    sides: [Bitboard; Nums::SIDES],
    // An array of piece values, used for seeing which pieces are on the start
    // and end square.
    // `piece_board[SQUARE] == piece on that square`.
    piece_board: [Piece; Nums::SQUARES],
    side_to_move: Side,
}

/// A [`Move`] with metadata used for quickly unmaking moves.
#[derive(Clone, Copy)]
struct ChessMove {
    mv: Move,
    piece: Piece,
    captured: Piece,
}

/// The history of the board.
struct Movelist {
    moves: [ChessMove; MAX_GAME_MOVES],
    first_empty: usize,
}

/// There is no basis to this number other than 'yeah that seems good enough`.
const MAX_GAME_MOVES: usize = 250;

impl Board {
    /// Creates a new [`Board`] initialised with the state of the starting
    /// position and initialises the static lookup tables.
    pub fn new() -> Self {
        Lookup::init();
        Self {
            played_moves: Movelist::new(),
            piece_board: Self::default_piece_board(),
            pieces: Self::default_pieces(),
            sides: Self::default_sides(),
            side_to_move: Self::default_side(),
        }
    }
}

impl ChessMove {
    /// Creates a [`ChessMove`] with the data set to the parameters given.
    pub fn new(mv: Move, piece: Piece, captured: Piece) -> Self {
        Self {
            captured,
            mv,
            piece,
        }
    }
}

impl Movelist {
    /// Creates an empty [`Movelist`].
    pub fn new() -> Self {
        Self {
            moves: [ChessMove::new(Move::null(), Pieces::NONE, Pieces::NONE); MAX_GAME_MOVES],
            first_empty: 0,
        }
    }
}

impl Board {
    /// Returns the piece board of the starting position.
    /// ```
    /// assert_eq!(default_piece_board()[Squares::A1], Pieces::ROOK);
    /// assert_eq!(default_piece_board()[Squares::B1], Pieces::KNIGHT);
    /// assert_eq!(default_piece_board()[Squares::A8], Pieces::ROOK);
    /// // etc.
    /// ```
    #[rustfmt::skip]
    fn default_piece_board() -> [Piece; Nums::SQUARES] {
        let p = Pieces::PAWN;
        let n = Pieces::KNIGHT;
        let b = Pieces::BISHOP;
        let r = Pieces::ROOK;
        let q = Pieces::QUEEN;
        let k = Pieces::KING;
        let e = Pieces::NONE;
        [
            r, n, b, q, k, b, n, r,
            p, p, p, p, p, p, p, p,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            p, p, p, p, p, p, p, p,
            r, n, b, q, k, b, n, r,
        ]
    }
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

impl ChessMove {
    /// Seperates a [`ChessMove`] into a [`Move`] with its metadata:
    /// [all the fields of [`Move::decompose`]], piece being moved, piece
    /// being captured, in that order.
    pub fn decompose(&self) -> (Square, Square, bool, bool, bool, Piece, Piece, Piece) {
        let (start, end, is_castling, is_en_passant, is_promotion, promotion_piece) =
            self.mv.decompose();
        (
            start,
            end,
            is_castling,
            is_en_passant,
            is_promotion,
            promotion_piece,
            self.piece,
            self.captured,
        )
    }
}

impl Movelist {
    /// Pops a [`Move`] with its metadata from the list. Assumes that `self`
    /// contains at least one element.
    pub fn pop_move(&mut self) -> ChessMove {
        self.first_empty -= 1;
        self.moves[self.first_empty]
    }

    /// Pushes a move with metadata onto the list. Assumes that `self` will not
    /// overflow.
    pub fn push_move(&mut self, mv: Move, piece: Piece, captured: Piece) {
        self.moves[self.first_empty] = ChessMove::new(mv, piece, captured);
        self.first_empty += 1;
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

    /// Flip the side to move.
    fn flip_side(&mut self) {
        self.side_to_move ^= 1;
    }

    /// Returns all the occupied squares on the board.
    fn occupancies(&self) -> Bitboard {
        self.sides::<true>() | self.sides::<false>()
    }

    /// Returns the piece on `square`.
    fn piece_on(&self, square: Square) -> Piece {
        self.piece_board[square]
    }

    /// Returns the piece bitboard given by `piece`.
    fn pieces<const PIECE: Piece>(&self) -> Bitboard {
        self.pieces[PIECE]
    }

    /// Returns the side to move.
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
