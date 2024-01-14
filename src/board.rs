use std::ops::{BitAnd, BitAndAssign, BitOrAssign, Not, Shl};

use crate::defs::{piece_to_char, Bitboard, File, Nums, Piece, Rank, Side, Square};
use movegen::Lookup;

pub use movegen::{magic::find_magics, Moves};

/// Stores castling rights.
#[derive(Clone, Copy, PartialEq)]
pub struct CastlingRights {
    // 4 bits: KQkq.
    cr: u8,
}

/// Items related to move generation.
mod movegen;

/// Stores information about the current state of the board.
#[derive(Clone)]
pub struct Board {
    // `pieces[0]` is the intersection of all pawns on the board, `pieces[1]`
    // is the knights, and so on, as according to the order set by
    // [`Piece`].
    pieces: [Bitboard; Nums::PIECES],
    // `sides[1]` is the intersection of all White piece bitboards; `sides[0]`
    // is is the intersection of all Black piece bitboards.
    sides: [Bitboard; Nums::SIDES],
    // An array of piece values, used for seeing which pieces are on the start
    // and end square.
    // `piece_board[SQUARE] == piece on that square`.
    piece_board: [Piece; Nums::SQUARES],
    // castling rights. Encoded as KQkq. E.g. 0b1101 would be castling rights
    // KQq.
    castling_rights: CastlingRights,
    ep_square: Square,
    side_to_move: Side,
}

impl BitAnd for CastlingRights {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::from(self.inner() & rhs.inner())
    }
}

impl BitAndAssign for CastlingRights {
    fn bitand_assign(&mut self, rhs: Self) {
        self.cr &= rhs.inner()
    }
}

impl BitOrAssign for CastlingRights {
    fn bitor_assign(&mut self, rhs: Self) {
        self.cr |= rhs.inner()
    }
}

impl Not for CastlingRights {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::from(!self.inner())
    }
}

impl Shl<u8> for CastlingRights {
    type Output = Self;

    fn shl(self, rhs: u8) -> Self::Output {
        Self::from(self.inner() << rhs)
    }
}

#[allow(non_upper_case_globals)]
impl CastlingRights {
    pub const CASTLE_FLAGS_K: CastlingRights = Self::from(0b1000);
    pub const CASTLE_FLAGS_Q: CastlingRights = Self::from(0b0100);
    pub const CASTLE_FLAGS_k: CastlingRights = Self::from(0b0010);
    pub const CASTLE_FLAGS_q: CastlingRights = Self::from(0b0001);
    pub const CASTLE_FLAGS_KQkq: CastlingRights = Self::from(0b1111);
    pub const CASTLE_FLAGS_NONE: CastlingRights = Self::from(0b0000);
}

impl Board {
    /// Creates a new [`Board`] initialised with the state of the starting
    /// position and initialises the static lookup tables.
    pub fn new() -> Self {
        Lookup::init();
        Self {
            piece_board: Self::default_piece_board(),
            pieces: Self::default_pieces(),
            sides: Self::default_sides(),
            castling_rights: CastlingRights::new(),
            ep_square: Self::default_ep_square(),
            side_to_move: Self::default_side(),
        }
    }
}

impl Board {
    /// Adds the given right to the castling rights.
    pub fn add_castling_right(&mut self, right: CastlingRights) {
        self.castling_rights.add_right(right);
    }

    /// Adds a piece to square `square` for side `side`. Assumes there is no
    /// piece on the square to be written to.
    pub fn add_piece(&mut self, side: Side, square: Square, piece: Piece) {
        let square_bb = Bitboard::from_square(square);
        self.set_piece(square, piece);
        self.toggle_piece_bb(piece, square_bb);
        self.toggle_side_bb(side, square_bb);
    }

    /// Clears `self`.
    pub fn clear_board(&mut self) {
        self.piece_board = Self::empty_piece_board();
        self.pieces = Self::no_pieces();
        self.sides = Self::no_sides();
        self.castling_rights = CastlingRights::none();
        self.ep_square = Square::NONE;
        self.side_to_move = Side::NONE;
    }

    /// Pretty-prints the current state of the board.
    pub fn pretty_print(&self) {
        for r in (0..Nums::RANKS as u8).rev() {
            print!("{} | ", r + 1);
            for f in 0..Nums::FILES as u8 {
                print!(
                    "{} ",
                    self.char_piece_from_pos(Rank::from(r), File::from(f))
                );
            }
            println!();
        }
        println!("    ---------------");
        println!("    a b c d e f g h");
    }

    /// Sets the default castling rights.
    pub fn set_default_castling_rights(&mut self) {
        self.castling_rights.set_to_default();
    }

    /// Sets the en passant square to `square`.
    pub fn set_ep_square(&mut self, square: Square) {
        self.ep_square = square;
    }

    /// Sets side to move to the default side.
    pub fn set_default_side_to_move(&mut self) {
        self.side_to_move = Self::default_side();
    }

    /// Sets fullmoves. Currently does nothing.
    pub fn set_fullmoves(&mut self, _count: u32) {
        /* unused */
    }

    /// Sets halfmoves. Currently does nothing.
    pub fn set_halfmoves(&mut self, _count: u32) {
        /* unused */
    }

    /// Sets side to move to `side`.
    pub fn set_side_to_move(&mut self, side: Side) {
        self.side_to_move = side;
    }

    /// Resets the board.
    pub fn set_startpos(&mut self) {
        self.piece_board = Self::default_piece_board();
        self.pieces = Self::default_pieces();
        self.sides = Self::default_sides();
        self.castling_rights = CastlingRights::new();
        self.ep_square = Self::default_ep_square();
        self.side_to_move = Self::default_side();
    }
}

impl Board {
    /// Returns the default en passant square.
    fn default_ep_square() -> Square {
        Square::NONE
    }

    /// Returns the [`Piece`] board of the starting position.
    #[rustfmt::skip]
    fn default_piece_board() -> [Piece; Nums::SQUARES] {
        let p = Piece::PAWN;
        let n = Piece::KNIGHT;
        let b = Piece::BISHOP;
        let r = Piece::ROOK;
        let q = Piece::QUEEN;
        let k = Piece::KING;
        let e = Piece::NONE;
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

    /// Returns the piece [`Bitboard`]s of the starting position.
    fn default_pieces() -> [Bitboard; Nums::PIECES] {
        [
            Bitboard::from(0x00ff00000000ff00), // Pawns
            Bitboard::from(0x4200000000000042), // Knights
            Bitboard::from(0x2400000000000024), // Bishops
            Bitboard::from(0x8100000000000081), // Rooks
            Bitboard::from(0x0800000000000008), // Queens
            Bitboard::from(0x1000000000000010), // Kings
        ]
    }

    /// Returns the side to move from the starting position. Unless chess 1.1
    /// has been released, this will be [`Side::WHITE`].
    fn default_side() -> Side {
        Side::WHITE
    }

    /// Returns the side [`Bitboard`]s of the starting position.
    fn default_sides() -> [Bitboard; Nums::SIDES] {
        [
            Bitboard::from(0xffff000000000000), // Black
            Bitboard::from(0x000000000000ffff), // White
        ]
    }

    /// Returns an empty [`Piece`] board.
    #[rustfmt::skip]
    fn empty_piece_board() -> [Piece; Nums::SQUARES] {
        // technically [Piece::NONE; 64] would be the same but this is more
        // clear
        let e = Piece::NONE;
        [
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
        ]
    }

    /// Returns the piece [`Bitboard`]s of an empty board.
    fn no_pieces() -> [Bitboard; Nums::PIECES] {
        [
            Bitboard::from(0x0000000000000000),
            Bitboard::from(0x0000000000000000),
            Bitboard::from(0x0000000000000000),
            Bitboard::from(0x0000000000000000),
            Bitboard::from(0x0000000000000000),
            Bitboard::from(0x0000000000000000),
        ]
    }

    /// Returns the side [`Bitboard`]s of an empty board.
    #[rustfmt::skip]
    fn no_sides() -> [Bitboard; Nums::SIDES] {
        [
            Bitboard::from(0x0000000000000000),
            Bitboard::from(0x0000000000000000),
        ]
    }
}

impl CastlingRights {
    /// Returns new [`CastlingRights`] with contents `cr`.
    const fn from(cr: u8) -> Self {
        Self { cr }
    }

    /// Returns new [`CastlingRights`] with the default castling rights.
    fn new() -> Self {
        Self::CASTLE_FLAGS_KQkq
    }

    /// Returns empty [`CastlingRights`].
    fn none() -> CastlingRights {
        Self::CASTLE_FLAGS_NONE
    }
}

impl Board {
    /// Finds the piece on the given rank and file and converts it to its
    /// character representation. If no piece is on the square, returns '0'
    /// instead.
    fn char_piece_from_pos(&self, rank: Rank, file: File) -> char {
        let sq_bb = Bitboard::from_pos(rank, file);
        let piece = self.piece_on(Square::from_pos(rank, file));
        if piece == Piece::NONE {
            return '0';
        }
        if self.side::<{ Side::WHITE.to_bool() }>() & sq_bb != Bitboard::from(0) {
            piece_to_char(Side::WHITE, piece)
        } else {
            piece_to_char(Side::BLACK, piece)
        }
    }

    /// Returns the piece on `square`.
    fn piece_on(&self, square: Square) -> Piece {
        self.piece_board[square.to_index()]
    }

    /// Sets the piece on `square` in the piece array to `piece`.
    fn set_piece(&mut self, square: Square, piece: Piece) {
        self.piece_board[square.to_index()] = piece;
    }

    /// Returns the board of the side according to `IS_WHITE`.
    fn side<const IS_WHITE: bool>(&self) -> Bitboard {
        if IS_WHITE {
            self.sides[Side::WHITE.to_index()]
        } else {
            self.sides[Side::BLACK.to_index()]
        }
    }

    /// Toggles the bits set in `bb` of the bitboard of `piece`.
    fn toggle_piece_bb(&mut self, piece: Piece, bb: Bitboard) {
        self.pieces[piece.to_index()] ^= bb;
    }

    /// Toggles the bits set in `bb` of the bitboard of `side`.
    fn toggle_side_bb(&mut self, side: Side, bb: Bitboard) {
        self.sides[side.to_index()] ^= bb;
    }
}

impl CastlingRights {
    /// Adds the given right to `self`.
    fn add_right(&mut self, right: Self) {
        *self |= right;
    }

    /// Calculates if the given side can castle kingside.
    fn can_castle_kingside<const IS_WHITE: bool>(&self) -> bool {
        if IS_WHITE {
            *self & Self::CASTLE_FLAGS_K == Self::CASTLE_FLAGS_K
        } else {
            *self & Self::CASTLE_FLAGS_k == Self::CASTLE_FLAGS_k
        }
    }

    /// Calculates if the given side can castle queenside.
    fn can_castle_queenside<const IS_WHITE: bool>(&self) -> bool {
        if IS_WHITE {
            *self & Self::CASTLE_FLAGS_Q == Self::CASTLE_FLAGS_Q
        } else {
            *self & Self::CASTLE_FLAGS_q == Self::CASTLE_FLAGS_q
        }
    }

    /// Clears the rights for `side`.
    fn clear_side(&mut self, side: Side) {
        debug_assert_eq!(Side::WHITE.inner(), 1);
        // `side * 2` is 2 for White and 0 for Black. `0b11 << (side * 2)` is
        // a mask for the bits for White or Black. `&`ing the rights with
        // `!(0b11 << (side * 2))` will clear the bits on the given side.
        *self &= Self::from(!(0b11 << (side.inner() * 2)));
    }

    /// Returns the contents of `self`.
    fn inner(self) -> u8 {
        self.cr
    }

    /// Removes the given right from `self`. `right` does not already have to
    /// be set to be removed.
    fn remove_right(&mut self, side: Side, right: Self) {
        // `side.inner() * 2` is 0 for Black and 2 for White. Thus, if `right`
        // is `0brr`, `right << side.inner()` is `0brr00` for White and `0brr`
        // for Black.
        *self &= !(right << (side.inner() * 2));
    }

    /// Sets the rights of `self` to default.
    fn set_to_default(&mut self) {
        *self = Self::new();
    }
}
