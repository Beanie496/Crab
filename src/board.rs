use std::ops::{BitAnd, BitAndAssign, BitOrAssign, Not, Shl};

use crate::defs::{Bitboard, File, Nums, Piece, Rank, Side, Square, PIECE_CHARS};
use movegen::{Lookup, Move, LOOKUPS};

pub use movegen::magic::find_magics;

/// Stores castling rights.
#[derive(Clone, Copy, PartialEq)]
pub struct CastlingRights {
    // 4 bits: KQkq.
    cr: u8,
}

/// Items related to move generation.
pub mod movegen;

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
            ep_square: Self::no_ep_square(),
            side_to_move: Self::default_side(),
        }
    }
}

impl Board {
    /// Returns the piece board of the starting position.
    /// ```
    /// assert_eq!(default_piece_board()[Square::A1.to_index()], Piece::ROOK);
    /// assert_eq!(default_piece_board()[Square::B1.to_index()], Piece::KNIGHT);
    /// assert_eq!(default_piece_board()[Square::A8.to_index()], Piece::ROOK);
    /// // etc.
    /// ```
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

    /// Returns the pieces of the starting position.
    /// ```
    /// assert_eq!(default_pieces()[Piece::PAWN.to_index()], Bitboard::new(0x00ff00000000ff00));
    /// // etc.
    /// ```
    fn default_pieces() -> [Bitboard; Nums::PIECES] {
        [
            Bitboard::new(0x00ff00000000ff00), // Pawns
            Bitboard::new(0x4200000000000042), // Knights
            Bitboard::new(0x2400000000000024), // Bishops
            Bitboard::new(0x8100000000000081), // Rooks
            Bitboard::new(0x0800000000000008), // Queens
            Bitboard::new(0x1000000000000010), // Kings
        ]
    }

    /// Returns the sides of the starting position.
    /// ```
    /// assert_eq!(default_pieces()[Side::WHITE.to_index()], Bitboard::new(0x000000000000ffff));
    /// assert_eq!(default_pieces()[Side::BLACK.to_index()], Bitboard::new(0xffff000000000000));
    /// ```
    fn default_sides() -> [Bitboard; Nums::SIDES] {
        [
            Bitboard::new(0xffff000000000000), // Black
            Bitboard::new(0x000000000000ffff), // White
        ]
    }

    /// Returns the side to move from the starting position. Unless chess 1.1
    /// has been released, this will be [`Side::WHITE`].
    fn default_side() -> Side {
        Side::WHITE
    }

    /// Returns an empty piece board.
    #[rustfmt::skip]
    fn empty_piece_board() -> [Piece; Nums::SQUARES] {
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

    /// Checks if the move is a double pawn push.
    fn is_double_pawn_push(start: Square, end: Square, piece: Piece) -> bool {
        if piece != Piece::PAWN {
            return false;
        }
        let start_bb = start.to_bitboard();
        let end_bb = end.to_bitboard();
        if start_bb & (Bitboard::rank_bb(Rank::RANK2) | Bitboard::rank_bb(Rank::RANK7))
            == Bitboard::new(0)
        {
            return false;
        }
        if end_bb & (Bitboard::rank_bb(Rank::RANK4) | Bitboard::rank_bb(Rank::RANK5))
            == Bitboard::new(0)
        {
            return false;
        }
        true
    }

    /// Returns no ep square.
    fn no_ep_square() -> Square {
        Square::NONE
    }

    /// Returns the pieces of an empty board.
    fn no_pieces() -> [Bitboard; Nums::PIECES] {
        [
            Bitboard::new(0x0000000000000000),
            Bitboard::new(0x0000000000000000),
            Bitboard::new(0x0000000000000000),
            Bitboard::new(0x0000000000000000),
            Bitboard::new(0x0000000000000000),
            Bitboard::new(0x0000000000000000),
        ]
    }

    /// Returns the sides of an empty board.
    #[rustfmt::skip]
    fn no_sides() -> [Bitboard; Nums::SIDES] {
        [
            Bitboard::new(0x0000000000000000),
            Bitboard::new(0x0000000000000000),
        ]
    }

    /// Returns no side.
    fn no_side() -> Side {
        Side::NONE
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
    /// Adds the given right to the castling rights.
    pub fn add_castling_right(&mut self, right: CastlingRights) {
        self.castling_rights.add_right(right);
    }

    /// Adds a piece to square `square` for side `side`. Assumes there is no
    /// piece on the square to be written to.
    pub fn add_piece(&mut self, side: Side, square: Square, piece: Piece) {
        let square_bb = square.to_bitboard();
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
        self.ep_square = Self::no_ep_square();
        self.side_to_move = Self::no_side();
    }

    /// Pretty-prints the current state of the board.
    pub fn pretty_print(&self) {
        for r in (Rank::RANK1.inner()..=Rank::RANK8.inner()).rev() {
            print!("{} | ", r + 1);
            for f in File::FILE1.inner()..=File::FILE8.inner() {
                print!("{} ", self.char_piece_from_pos(Rank::new(r), File::new(f)));
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
        self.ep_square = Self::no_ep_square();
        self.side_to_move = Self::default_side();
    }
}

impl Board {
    /// Calculates if the given side can castle kingside.
    fn can_castle_kingside<const IS_WHITE: bool>(&self) -> bool {
        self.castling_rights.can_castle_kingside::<IS_WHITE>()
    }

    /// Calculates if the given side can castle queenside.
    fn can_castle_queenside<const IS_WHITE: bool>(&self) -> bool {
        self.castling_rights.can_castle_queenside::<IS_WHITE>()
    }

    /// Finds the piece on the given rank and file and converts it to its
    /// character representation. If no piece is on the square, returns '0'
    /// instead.
    fn char_piece_from_pos(&self, rank: Rank, file: File) -> char {
        let sq_bb = Bitboard::from_pos(rank, file);
        let piece = self.piece_on(Square::from_pos(rank, file));
        if piece == Piece::NONE {
            return '0';
        }
        if self.sides[Side::WHITE.to_index()] & sq_bb != Bitboard::new(0) {
            PIECE_CHARS[Side::WHITE.to_index()][piece.to_index()]
        } else {
            PIECE_CHARS[Side::BLACK.to_index()][piece.to_index()]
        }
    }

    /// Sets the en passant square to [`Square::NONE`].
    fn clear_ep_square(&mut self) {
        self.ep_square = Square::NONE;
    }

    /// Sets the piece on `square` in the piece array to [`Square::NONE`].
    fn clear_piece(&mut self, square: Square) {
        self.piece_board[square.to_index()] = Piece::NONE;
    }

    /// Returns the en passant square, which might be [`Square::NONE`].
    fn ep_square(&self) -> Square {
        self.ep_square
    }

    /// Flip the side to move.
    fn flip_side(&mut self) {
        self.side_to_move = self.side_to_move.flip();
    }

    /// Tests if `square` is attacked by an enemy piece.
    fn is_square_attacked(&self, square: Square) -> bool {
        let occupancies = self.occupancies();
        let us = self.side_to_move();
        let them_bb = self.sides[us.flip().to_index()];

        let pawn_attacks = unsafe { LOOKUPS.pawn_attacks(us, square) };
        let knight_attacks = unsafe { LOOKUPS.knight_attacks(square) };
        let diagonal_attacks = unsafe { LOOKUPS.bishop_attacks(square, occupancies) };
        let orthogonal_attacks = unsafe { LOOKUPS.rook_attacks(square, occupancies) };
        let king_attacks = unsafe { LOOKUPS.king_attacks(square) };

        let pawns = self.pieces::<{ Piece::PAWN.to_index() }>();
        let knights = self.pieces::<{ Piece::KNIGHT.to_index() }>();
        let bishops = self.pieces::<{ Piece::BISHOP.to_index() }>();
        let rooks = self.pieces::<{ Piece::ROOK.to_index() }>();
        let queens = self.pieces::<{ Piece::QUEEN.to_index() }>();
        let kings = self.pieces::<{ Piece::KING.to_index() }>();

        let is_attacked_by_pawns = pawn_attacks & pawns;
        let is_attacked_by_knights = knight_attacks & knights;
        let is_attacked_by_kings = king_attacks & kings;
        let is_attacked_diagonally = diagonal_attacks & (bishops | queens);
        let is_attacked_orthogonally = orthogonal_attacks & (rooks | queens);

        (is_attacked_by_pawns
            | is_attacked_by_knights
            | is_attacked_by_kings
            | is_attacked_diagonally
            | is_attacked_orthogonally)
            & them_bb
            != Bitboard::new(0)
    }

    /// Calculates the square the king is on.
    fn king_square(&self) -> Square {
        (self.pieces::<{ Piece::KING.to_index() }>() & self.sides[self.side_to_move().to_index()])
            .to_square()
    }

    /// Toggles the side and piece bitboard on both `start` and `end`, sets
    /// `start` in the piece array to [`Square::NONE`] and sets `end` in the
    /// piece array to `piece`.
    fn move_piece(&mut self, start: Square, end: Square, side: Side, piece: Piece) {
        let start_bb = start.to_bitboard();
        let end_bb = end.to_bitboard();

        self.toggle_piece_bb(piece, start_bb | end_bb);
        self.toggle_side_bb(side, start_bb | end_bb);
        self.clear_piece(start);
        self.set_piece(end, piece);
    }

    /// Returns all the occupied squares on the board.
    fn occupancies(&self) -> Bitboard {
        self.side::<true>() | self.side::<false>()
    }

    /// Returns the piece on `square`.
    fn piece_on(&self, square: Square) -> Piece {
        self.piece_board[square.to_index()]
    }

    /// Returns the piece bitboard given by `piece`.
    fn pieces<const PIECE: usize>(&self) -> Bitboard {
        self.pieces[PIECE]
    }

    /// Sets the piece on `square` in the piece array to `piece`.
    fn set_piece(&mut self, square: Square, piece: Piece) {
        self.piece_board[square.to_index()] = piece;
    }

    /// Returns the side to move.
    fn side_to_move(&self) -> Side {
        self.side_to_move
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

    /// Unsets right `right` for side `side`.
    fn unset_castling_right(&mut self, side: Side, right: CastlingRights) {
        self.castling_rights.remove_right(side, right);
    }

    /// Clears the castling rights for `side`.
    fn unset_castling_rights(&mut self, side: Side) {
        self.castling_rights.clear_side(side)
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
