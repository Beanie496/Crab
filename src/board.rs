use crate::defs::{Bitboard, File, Nums, Piece, Rank, Side, Square, PIECE_CHARS};
use movegen::{Lookup, Move, LOOKUPS};

pub use movegen::magic::find_magics;

/// 4 bits: KQkq.
pub type CastlingRights = u8;

/// Items related to move generation.
pub mod movegen;

/// Stores information about the current state of the board.
pub struct Board {
    played_moves: Movelist,
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

/// A [`Move`] with metadata used for quickly unmaking moves.
#[derive(Clone, Copy)]
struct ChessMove {
    mv: Move,
    piece: Piece,
    captured: Piece,
    ep_square: Square,
    castling_rights: CastlingRights,
}

/// The history of the board.
struct Movelist {
    moves: [ChessMove; MAX_GAME_MOVES],
    first_empty: usize,
}

/// There is no basis to this number other than 'yeah that seems good enough`.
const MAX_GAME_MOVES: usize = 250;

#[allow(non_upper_case_globals)]
impl Board {
    pub const CASTLE_FLAGS_K: CastlingRights = 0b1000;
    pub const CASTLE_FLAGS_Q: CastlingRights = 0b0100;
    pub const CASTLE_FLAGS_k: CastlingRights = 0b0010;
    pub const CASTLE_FLAGS_q: CastlingRights = 0b0001;
    pub const CASTLE_FLAGS_KQkq: CastlingRights = 0b1111;
    pub const CASTLE_FLAGS_NONE: CastlingRights = 0b0000;
}

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
            castling_rights: Self::default_castling_rights(),
            ep_square: Square::NONE,
            side_to_move: Self::default_side(),
        }
    }
}

impl ChessMove {
    /// Creates a [`ChessMove`] with the data set to the parameters given.
    pub fn new(
        mv: Move,
        piece: Piece,
        captured: Piece,
        ep_square: Square,
        castling_rights: CastlingRights,
    ) -> Self {
        Self {
            mv,
            piece,
            captured,
            ep_square,
            castling_rights,
        }
    }

    /// Returns a 0-initialised [`ChessMove`].
    pub fn null() -> Self {
        Self {
            mv: Move::null(),
            piece: Piece::NONE,
            captured: Piece::NONE,
            ep_square: Square::NULL,
            castling_rights: Board::CASTLE_FLAGS_NONE,
        }
    }
}

impl Movelist {
    /// Creates an empty [`Movelist`].
    pub fn new() -> Self {
        Self {
            moves: [ChessMove::null(); MAX_GAME_MOVES],
            first_empty: 0,
        }
    }
}

impl Board {
    /// Returns the default castling rights: `KQkq`.
    fn default_castling_rights() -> CastlingRights {
        Self::CASTLE_FLAGS_KQkq
    }

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

    /// Returns empty castling rights.
    fn no_castling_rights() -> CastlingRights {
        Self::CASTLE_FLAGS_NONE
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

impl Board {
    /// Adds a piece to square `square` for side `side`. Assumes there is no
    /// piece on the square to be written to.
    pub fn add_piece(&mut self, side: Side, square: Square, piece: Piece) {
        let square_bb = square.to_bitboard();
        self.set_piece(square, piece);
        self.toggle_piece_bb(piece, square_bb);
        self.toggle_side_bb(side, square_bb);
    }

    pub fn clear_board(&mut self) {
        self.played_moves.clear();
        self.piece_board = Self::empty_piece_board();
        self.pieces = Self::no_pieces();
        self.sides = Self::no_sides();
        self.castling_rights = Self::no_castling_rights();
        //self.ep_square = Self::no_ep_square();
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

    /// Sets the castling rights of `side` to `rights`. `rights` should be 2
    /// bits in the format `KQkq`, where the absence of a bit signifies the
    /// lack of a right.
    pub fn set_castling_rights(&mut self, rights: CastlingRights) {
        self.castling_rights |= rights;
    }

    /// Sets the default castling rights.
    pub fn set_default_castling_rights(&mut self) {
        self.castling_rights = Self::default_castling_rights();
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
        self.played_moves.clear();
        self.piece_board = Self::default_piece_board();
        self.pieces = Self::default_pieces();
        self.sides = Self::default_sides();
        self.castling_rights = Self::default_castling_rights();
        //self.ep_square = Self::no_ep_square();
        self.side_to_move = Self::default_side();
    }
}

impl ChessMove {
    /// Seperates a [`ChessMove`] into a [`Move`] with its metadata:
    /// [all the fields of [`Move::decompose`]], piece being moved, piece
    /// being captured, and en passant, in that order.
    pub fn decompose(
        &self,
    ) -> (
        Square,
        Square,
        bool,
        bool,
        bool,
        Piece,
        Piece,
        Piece,
        Square,
        CastlingRights,
    ) {
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
            self.ep_square,
            self.castling_rights,
        )
    }
}

impl Movelist {
    /// Clears the list. Note: this does not zero out the old data.
    pub fn clear(&mut self) {
        self.first_empty = 0;
    }

    /// Pops a [`Move`] with its metadata from the list. Assumes that `self`
    /// contains at least one element.
    pub fn pop_move(&mut self) -> ChessMove {
        self.first_empty -= 1;
        self.moves[self.first_empty]
    }

    /// Pushes a move with metadata onto the list. Assumes that `self` will not
    /// overflow.
    pub fn push_move(
        &mut self,
        mv: Move,
        piece: Piece,
        captured: Piece,
        ep_square: Square,
        castling_rights: CastlingRights,
    ) {
        self.moves[self.first_empty] =
            ChessMove::new(mv, piece, captured, ep_square, castling_rights);
        self.first_empty += 1;
    }
}

impl Board {
    /// Calculates if the given side can castle kingside.
    fn can_castle_kingside<const IS_WHITE: bool>(&self) -> bool {
        if IS_WHITE {
            self.castling_rights & Self::CASTLE_FLAGS_K == Self::CASTLE_FLAGS_K
        } else {
            self.castling_rights & Self::CASTLE_FLAGS_k == Self::CASTLE_FLAGS_k
        }
    }

    /// Calculates if the given side can castle queenside.
    fn can_castle_queenside<const IS_WHITE: bool>(&self) -> bool {
        if IS_WHITE {
            self.castling_rights & Self::CASTLE_FLAGS_Q == Self::CASTLE_FLAGS_Q
        } else {
            self.castling_rights & Self::CASTLE_FLAGS_q == Self::CASTLE_FLAGS_q
        }
    }
    /// Returns castling rights.
    fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
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
        let them_bb = self.sides[self.side_to_move().flip().to_index()];
        let diagonal_attacks = unsafe { LOOKUPS.bishop_attacks(square, occupancies) };
        let orthogonal_attacks = unsafe { LOOKUPS.rook_attacks(square, occupancies) };
        let queens = self.pieces::<{ Piece::QUEEN.to_index() }>();
        let rooks = self.pieces::<{ Piece::ROOK.to_index() }>();
        let bishops = self.pieces::<{ Piece::BISHOP.to_index() }>();
        (diagonal_attacks & (bishops | queens) | orthogonal_attacks & (rooks | queens)) & them_bb
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

    /// Toggles the side and piece bitboard on both `start` and `end`, sets
    /// `start` in the piece array to `piece` and sets `end` in the piece array
    /// to `captured`.
    fn unmove_piece(
        &mut self,
        start: Square,
        end: Square,
        side: Side,
        piece: Piece,
        captured: Piece,
    ) {
        let start_bb = start.to_bitboard();
        let end_bb = end.to_bitboard();

        self.toggle_piece_bb(piece, start_bb | end_bb);
        self.toggle_side_bb(side, start_bb | end_bb);
        self.set_piece(end, captured);
        self.set_piece(start, piece);
    }

    /// Unsets right `right` for side `side`. `right` is either `0b01` or
    /// `0b10`.
    fn unset_castling_right(&mut self, side: Side, right: CastlingRights) {
        // `side * 2` is 2 for White and 0 for Black. `0b11 << (side * 2)` is
        // a mask for the bits for White or Black. `&`ing the rights with
        // `!(0b11 << (side * 2))` will clear the bits on the given side.
        self.castling_rights &= !(right << (side.inner() * 2));
    }

    /// Clears the castling rights for `side`.
    fn unset_castling_rights(&mut self, side: Side) {
        // `side * 2` is 2 for White and 0 for Black. `0b11 << (side * 2)` is
        // a mask for the bits for White or Black. `&`ing the rights with
        // `!(0b11 << (side * 2))` will clear the bits on the given side.
        self.castling_rights &= !(0b11 << (side.inner() * 2));
    }
}
