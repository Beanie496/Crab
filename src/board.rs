use std::ops::{BitAnd, BitAndAssign, BitOrAssign, Not, Shl};

use crate::{
    bitboard::Bitboard,
    defs::{piece_to_char, File, Nums, Piece, Rank, Side, Square},
    out_of_bounds_is_unreachable,
};
use movegen::Lookup;

pub use movegen::{magic::find_magics, Move, Moves};

/// Stores castling rights. Encoded as `KQkq`, with one bit for each right.
/// E.g. `0b1101` would be castling rights `KQq`.
#[derive(Clone, Copy, Eq, PartialEq)]
// The inner value of a wrapper does not need to be documented.
#[allow(clippy::missing_docs_in_private_items)]
pub struct CastlingRights {
    cr: u8,
}

/// Items related to move generation.
mod movegen;

/// The board. It contains information about the current board state and can
/// generate pseudo-legal moves. It is small (131 bytes) so it uses copy-make.
#[derive(Clone)]
pub struct Board {
    /// An array of piece values, used for seeing which pieces are on the start
    /// and end square.
    /// `piece_board[SQUARE] == piece on that square`.
    piece_board: [Piece; Nums::SQUARES],
    /// `pieces[0]` is the intersection of all pawns on the board, `pieces[1]`
    /// is the knights, and so on, as according to the order set by
    /// [`Piece`].
    pieces: [Bitboard; Nums::PIECES],
    /// `sides[1]` is the intersection of all White piece bitboards; `sides[0]`
    /// is is the intersection of all Black piece bitboards.
    sides: [Bitboard; Nums::SIDES],
    /// The current side to move.
    side_to_move: Side,
    /// Castling rights.
    castling_rights: CastlingRights,
    /// The current en passant square. Is [`Square::NONE`] if there is no ep
    /// square.
    ep_square: Square,
}

impl BitAnd for CastlingRights {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self::from(self.inner() & rhs.inner())
    }
}

impl BitAndAssign for CastlingRights {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.cr &= rhs.inner();
    }
}

impl BitOrAssign for CastlingRights {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.cr |= rhs.inner();
    }
}

impl Not for CastlingRights {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self::from(!self.inner())
    }
}

impl Shl<u8> for CastlingRights {
    type Output = Self;

    #[inline]
    fn shl(self, rhs: u8) -> Self::Output {
        Self::from(self.inner() << rhs)
    }
}

#[allow(non_upper_case_globals)]
/// Flags. It's fine to use `&`, `^` and `|` on these.
impl CastlingRights {
    /// The flag `K`.
    pub const K: Self = Self::from(0b1000);
    /// The flag `Q`.
    pub const Q: Self = Self::from(0b0100);
    /// The flag `k`.
    pub const k: Self = Self::from(0b0010);
    /// The flag `q`.
    pub const q: Self = Self::from(0b0001);
    /// The flags `KQkq`, i.e. all flags.
    pub const KQkq: Self = Self::from(0b1111);
    /// No flags.
    pub const NONE: Self = Self::from(0b0000);
}

impl Board {
    /// Creates a new [`Board`] initialised with the state of the starting
    /// position and initialises the static lookup tables.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Lookup::init();
        Self {
            piece_board: Self::default_piece_board(),
            pieces: Self::default_pieces(),
            sides: Self::default_sides(),
            side_to_move: Self::default_side(),
            castling_rights: CastlingRights::new(),
            ep_square: Self::default_ep_square(),
        }
    }

    /// Returns the [`Piece`] board of the starting position.
    #[rustfmt::skip]
    #[allow(clippy::many_single_char_names)]
    const fn default_piece_board() -> [Piece; Nums::SQUARES] {
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

    /// Returns an empty [`Piece`] board.
    #[rustfmt::skip]
    const fn empty_piece_board() -> [Piece; Nums::SQUARES] {
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

    /// Returns the piece [`Bitboard`]s of the starting position.
    const fn default_pieces() -> [Bitboard; Nums::PIECES] {
        [
            Bitboard::from(0x00ff_0000_0000_ff00), // Pawns
            Bitboard::from(0x4200_0000_0000_0042), // Knights
            Bitboard::from(0x2400_0000_0000_0024), // Bishops
            Bitboard::from(0x8100_0000_0000_0081), // Rooks
            Bitboard::from(0x0800_0000_0000_0008), // Queens
            Bitboard::from(0x1000_0000_0000_0010), // Kings
        ]
    }

    /// Returns the piece [`Bitboard`]s of an empty board.
    const fn no_pieces() -> [Bitboard; Nums::PIECES] {
        [
            Bitboard::from(0x0000_0000_0000_0000),
            Bitboard::from(0x0000_0000_0000_0000),
            Bitboard::from(0x0000_0000_0000_0000),
            Bitboard::from(0x0000_0000_0000_0000),
            Bitboard::from(0x0000_0000_0000_0000),
            Bitboard::from(0x0000_0000_0000_0000),
        ]
    }

    /// Returns the side [`Bitboard`]s of the starting position.
    const fn default_sides() -> [Bitboard; Nums::SIDES] {
        [
            Bitboard::from(0xffff_0000_0000_0000), // Black
            Bitboard::from(0x0000_0000_0000_ffff), // White
        ]
    }

    /// Returns the side [`Bitboard`]s of an empty board.
    const fn no_sides() -> [Bitboard; Nums::SIDES] {
        [
            Bitboard::from(0x0000_0000_0000_0000),
            Bitboard::from(0x0000_0000_0000_0000),
        ]
    }

    /// Returns the side to move from the starting position. Unless chess 1.1
    /// has been released, this will be [`Side::WHITE`].
    const fn default_side() -> Side {
        Side::WHITE
    }

    /// Returns the default en passant square.
    const fn default_ep_square() -> Square {
        Square::NONE
    }

    /// Copies and returns its mailbox board array.
    #[inline]
    #[must_use]
    pub const fn clone_piece_board(&self) -> [Piece; Nums::SQUARES] {
        self.piece_board
    }

    /// Clears `self`.
    #[inline]
    pub fn clear_board(&mut self) {
        self.piece_board = Self::empty_piece_board();
        self.pieces = Self::no_pieces();
        self.sides = Self::no_sides();
        self.castling_rights = CastlingRights::none();
        self.ep_square = Square::NONE;
        self.side_to_move = Side::NONE;
    }

    /// Pretty-prints the current state of the board.
    #[inline]
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

    /// Converts the current board to a string.
    ///
    /// e.g. the starting position would be
    /// "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR".
    // the unwraps are called on values that logically can not be `None`, so
    // this can not panic
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    #[must_use]
    pub fn stringify_board(&self) -> String {
        let mut ret_str = String::new();
        let mut empty_squares = 0;
        // I can't just iterate over the piece board normally: the board goes
        // from a1 to h8, rank-file, whereas the FEN goes from a8 to h1, also
        // rank-file
        for rank in (0..Nums::RANKS).rev() {
            for file in 0..Nums::FILES {
                let square = Square::from_pos(Rank::from(rank as u8), File::from(file as u8));
                let piece = self.piece_on(square);

                if piece == Piece::NONE {
                    empty_squares += 1;
                } else {
                    if empty_squares != 0 {
                        // `empty_squares` logically can not be greater than 8,
                        // so it's impossible for this to panic
                        #[allow(clippy::unwrap_used)]
                        ret_str.push(char::from_digit(empty_squares, 10).unwrap());
                        empty_squares = 0;
                    }
                    let side = self.side_of(square);
                    ret_str.push(piece_to_char(side, piece));
                }
            }
            if empty_squares != 0 {
                // same reason as before - this can not panic
                #[allow(clippy::unwrap_used)]
                ret_str.push(char::from_digit(empty_squares, 10).unwrap());
                empty_squares = 0;
            }
            ret_str.push('/');
        }
        // remove the trailing slash
        ret_str.pop();

        ret_str
    }

    /// Resets the board.
    #[inline]
    pub fn set_startpos(&mut self) {
        self.piece_board = Self::default_piece_board();
        self.pieces = Self::default_pieces();
        self.sides = Self::default_sides();
        self.castling_rights = CastlingRights::new();
        self.ep_square = Self::default_ep_square();
        self.side_to_move = Self::default_side();
    }

    /// Returns the piece on `square`.
    #[inline]
    #[must_use]
    pub fn piece_on(&self, square: Square) -> Piece {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.piece_board.len()) };
        self.piece_board[square.to_index()]
    }

    /// Returns the [`Side`] of `square`.
    #[inline]
    #[must_use]
    pub fn side_of(&self, square: Square) -> Side {
        let square_bb = Bitboard::from_square(square);
        if !(self.side::<{ Side::WHITE.to_bool() }>() & square_bb).is_empty() {
            Side::WHITE
        } else if !(self.side::<{ Side::BLACK.to_bool() }>() & square_bb).is_empty() {
            Side::BLACK
        } else {
            Side::NONE
        }
    }

    /// Adds a piece to square `square` for side `side`. Assumes there is no
    /// piece on the square to be written to.
    #[inline]
    pub fn add_piece(&mut self, side: Side, square: Square, piece: Piece) {
        let square_bb = Bitboard::from_square(square);
        self.set_piece(square, piece);
        self.toggle_piece_bb(piece, square_bb);
        self.toggle_side_bb(side, square_bb);
    }

    /// Sets the piece on `square` in the piece array to `piece`.
    #[inline]
    pub fn set_piece(&mut self, square: Square, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.piece_board.len()) };
        self.piece_board[square.to_index()] = piece;
    }

    /// Sets the piece on `square` in the piece array to [`Square::NONE`].
    #[inline]
    pub fn unset_piece(&mut self, square: Square) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.piece_board.len()) };
        self.piece_board[square.to_index()] = Piece::NONE;
    }

    /// Returns the side to move.
    #[inline]
    #[must_use]
    pub const fn side_to_move(&self) -> Side {
        self.side_to_move
    }

    /// Sets side to move to the default side.
    #[inline]
    pub fn set_default_side_to_move(&mut self) {
        self.side_to_move = Self::default_side();
    }

    /// Sets side to move to `side`.
    #[inline]
    pub fn set_side_to_move(&mut self, side: Side) {
        self.side_to_move = side;
    }

    /// Flip the side to move.
    #[inline]
    pub fn flip_side(&mut self) {
        self.side_to_move = self.side_to_move.flip();
    }

    /// Returns the string representation of the current side to move: 'w' if
    /// White and 'b' if Black.
    #[inline]
    #[must_use]
    pub const fn side_to_move_as_char(&self) -> char {
        (b'b' + self.side_to_move().inner() * 21) as char
    }

    /// Calculates if the given side can castle kingside.
    #[inline]
    #[must_use]
    pub fn can_castle_kingside<const IS_WHITE: bool>(&self) -> bool {
        self.castling_rights.can_castle_kingside::<IS_WHITE>()
    }

    /// Calculates if the given side can castle queenside.
    #[inline]
    #[must_use]
    pub fn can_castle_queenside<const IS_WHITE: bool>(&self) -> bool {
        self.castling_rights.can_castle_queenside::<IS_WHITE>()
    }

    /// Adds the given right to the castling rights.
    #[inline]
    pub fn add_castling_right(&mut self, right: CastlingRights) {
        self.castling_rights.add_right(right);
    }

    /// Sets the default castling rights.
    #[inline]
    pub fn set_default_castling_rights(&mut self) {
        self.castling_rights.set_to_default();
    }

    /// Unsets castling the given right for the given side.
    #[inline]
    pub fn unset_castling_right(&mut self, side: Side, right: CastlingRights) {
        self.castling_rights.remove_right(side, right);
    }

    /// Clears the castling rights for the given side.
    #[inline]
    pub fn unset_castling_rights(&mut self, side: Side) {
        self.castling_rights.clear_side(side);
    }

    /// Converts the current castling rights into their string representation.
    ///
    /// E.g. `KQq` if the White king can castle both ways and the Black king
    /// can only castle queenside.
    #[inline]
    #[must_use]
    pub fn stringify_castling_rights(&self) -> String {
        self.castling_rights.stringify()
    }

    /// Returns the en passant square, which might be [`Square::NONE`].
    #[inline]
    #[must_use]
    pub const fn ep_square(&self) -> Square {
        self.ep_square
    }

    /// Sets the en passant square to [`Square::NONE`].
    #[inline]
    pub fn clear_ep_square(&mut self) {
        self.ep_square = Square::NONE;
    }

    /// Sets the en passant square to `square`.
    #[inline]
    pub fn set_ep_square(&mut self, square: Square) {
        self.ep_square = square;
    }

    /// Returns the string representation of the current en passant square: the
    /// square if there is one (e.g. "b3") or "-" if there is none.
    #[inline]
    #[must_use]
    pub fn stringify_ep_square(&self) -> String {
        let ep_square = self.ep_square();
        if ep_square == Square::NONE {
            "-".to_string()
        } else {
            ep_square.stringify()
        }
    }

    /// Sets fullmoves. Currently does nothing.
    #[inline]
    pub fn set_fullmoves(&mut self, _count: u32) {
        /* unused */
    }

    /// Sets halfmoves. Currently does nothing.
    #[inline]
    pub fn set_halfmoves(&mut self, _count: u32) {
        /* unused */
    }

    /// Finds the piece on the given rank and file and converts it to its
    /// character representation. If no piece is on the square, returns '0'
    /// instead.
    #[inline]
    #[must_use]
    fn char_piece_from_pos(&self, rank: Rank, file: File) -> char {
        let sq_bb = Bitboard::from_pos(rank, file);
        let piece = self.piece_on(Square::from_pos(rank, file));
        if piece == Piece::NONE {
            return '0';
        }
        if (self.side::<{ Side::WHITE.to_bool() }>() & sq_bb).is_empty() {
            piece_to_char(Side::BLACK, piece)
        } else {
            piece_to_char(Side::WHITE, piece)
        }
    }

    /// Returns the board of the side according to `IS_WHITE`.
    const fn side<const IS_WHITE: bool>(&self) -> Bitboard {
        if IS_WHITE {
            self.sides[Side::WHITE.to_index()]
        } else {
            self.sides[Side::BLACK.to_index()]
        }
    }

    /// Toggles the bits set in `bb` of the bitboard of `piece`.
    fn toggle_piece_bb(&mut self, piece: Piece, bb: Bitboard) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), self.pieces.len()) };
        self.pieces[piece.to_index()] ^= bb;
    }

    /// Toggles the bits set in `bb` of the bitboard of `side`.
    fn toggle_side_bb(&mut self, side: Side, bb: Bitboard) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(side.to_index(), self.sides.len()) };
        self.sides[side.to_index()] ^= bb;
    }
}

impl CastlingRights {
    /// Returns new [`CastlingRights`] with contents `cr`.
    const fn from(cr: u8) -> Self {
        Self { cr }
    }

    /// Returns new [`CastlingRights`] with the default castling rights.
    const fn new() -> Self {
        Self::KQkq
    }

    /// Returns empty [`CastlingRights`].
    const fn none() -> Self {
        Self::NONE
    }

    /// Returns the contents of `self`.
    const fn inner(self) -> u8 {
        self.cr
    }

    /// Calculates if the given side can castle kingside.
    fn can_castle_kingside<const IS_WHITE: bool>(self) -> bool {
        if IS_WHITE {
            self & Self::K == Self::K
        } else {
            self & Self::k == Self::k
        }
    }

    /// Calculates if the given side can castle queenside.
    fn can_castle_queenside<const IS_WHITE: bool>(self) -> bool {
        if IS_WHITE {
            self & Self::Q == Self::Q
        } else {
            self & Self::q == Self::q
        }
    }

    /// Adds the given right to `self`.
    fn add_right(&mut self, right: Self) {
        *self |= right;
    }

    /// Sets the rights of `self` to default.
    fn set_to_default(&mut self) {
        *self = Self::new();
    }

    /// Removes the given right from `self`. `right` does not already have to
    /// be set to be removed.
    fn remove_right(&mut self, side: Side, right: Self) {
        // `side.inner() * 2` is 0 for Black and 2 for White. Thus, if `right`
        // is `0brr`, `right << side.inner()` is `0brr00` for White and `0brr`
        // for Black.
        *self &= !(right << (side.inner() * 2));
    }

    /// Clears the rights for `side`.
    fn clear_side(&mut self, side: Side) {
        debug_assert_eq!(
            Side::WHITE.inner(),
            1,
            "This function relies on White being 1 and Black 0"
        );
        // `side * 2` is 2 for White and 0 for Black. `0b11 << (side * 2)` is
        // a mask for the bits for White or Black. `&`ing the rights with
        // `!(0b11 << (side * 2))` will clear the bits on the given side.
        *self &= Self::from(!(0b11 << (side.inner() * 2)));
    }

    /// Converts `self` to its string representation.
    fn stringify(self) -> String {
        let mut ret_str = String::new();
        if self.can_castle_kingside::<true>() {
            ret_str.push('K');
        }
        if self.can_castle_queenside::<true>() {
            ret_str.push('Q');
        }
        if self.can_castle_kingside::<false>() {
            ret_str.push('k');
        }
        if self.can_castle_queenside::<false>() {
            ret_str.push('q');
        }
        ret_str
    }
}
