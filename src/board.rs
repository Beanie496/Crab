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

/// Calls the method `set_startpos()` on `board`, prints `msg` using
/// `println!()`, then returns.
macro_rules! reset_board_print_return {
    ($board:expr, $msg:expr) => {{
        $board.set_startpos();
        return println!($msg);
    }};
}

/// The board. It contains information about the current board state and can
/// generate pseudo-legal moves. It is small (134 bytes) so it uses copy-make.
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
    /// The number of halfmoves since the last capture or pawn move.
    halfmoves: u8,
    /// Which move number the current move is. Starts at 1 and is incremented
    /// when Black moves.
    fullmoves: u16,
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

impl Default for Board {
    #[inline]
    fn default() -> Self {
        Self {
            piece_board: Self::default_piece_board(),
            pieces: Self::default_pieces(),
            sides: Self::default_sides(),
            side_to_move: Side::WHITE,
            castling_rights: CastlingRights::new(),
            ep_square: Square::NONE,
            halfmoves: 0,
            fullmoves: 1,
        }
    }
}

impl Board {
    /// Creates a new [`Board`] initialised with the state of the starting
    /// position and initialises the static lookup tables.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Lookup::init();
        Self::default()
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
    const fn empty_piece_board() -> [Piece; Nums::SQUARES] {
        [Piece::NONE; Nums::SQUARES]
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
        [Bitboard::EMPTY; Nums::PIECES]
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
        [Bitboard::EMPTY; Nums::SIDES]
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
        self.side_to_move = Side::NONE;
        self.castling_rights = CastlingRights::none();
        self.ep_square = Square::NONE;
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
        println!();
        println!("FEN: {}", self.current_fen_string());
    }

    /// Gets the current FEN representation of the board state.
    #[inline]
    #[must_use]
    pub fn current_fen_string(&self) -> String {
        let mut ret_str = String::new();

        ret_str.push_str(&self.stringify());
        ret_str.push(' ');

        ret_str.push(self.side_to_move_as_char());
        ret_str.push(' ');

        ret_str.push_str(&self.stringify_castling_rights());
        ret_str.push(' ');

        ret_str.push_str(&self.stringify_ep_square());
        ret_str.push(' ');

        ret_str.push_str(&self.halfmoves().to_string());
        ret_str.push(' ');

        ret_str.push_str(&self.fullmoves().to_string());

        ret_str
    }

    /// Takes a sequence of moves and feeds them to the board. Will stop and
    /// return if any of the moves are incorrect. Not implemented yet.
    #[allow(clippy::unused_self)]
    #[inline]
    pub const fn play_moves(&self, _moves: &str) {}

    /// Sets `self.board` to the given FEN. It will check for basic errors,
    /// like the board having too many ranks, but not many more.
    // this function cannot panic as the only unwrap is on a hardcoded value
    #[allow(clippy::missing_panics_doc)]
    #[allow(clippy::too_many_lines)]
    #[inline]
    pub fn set_pos_to_fen(&mut self, position: &str) {
        self.clear_board();

        let mut iter = position.split(' ');
        let Some(board) = iter.next() else {
            reset_board_print_return!(self, "Error: need to pass a board");
        };
        let side_to_move = iter.next();
        let castling_rights = iter.next();
        let ep_square = iter.next();
        let halfmoves = iter.next();
        let fullmoves = iter.next();

        // 1. the board itself. 1 of each king isn't checked. Hey, garbage in,
        // garbage out!
        // split into 2 to check for overflow easily
        let mut rank_idx = 8u8;
        let mut file_idx = 0;
        let ranks = board.split('/');
        for rank in ranks {
            // if the board has too many ranks, this would eventually underflow
            // and panic, so wrapping sub needed
            rank_idx = rank_idx.wrapping_sub(1);
            for piece in rank.chars() {
                // if it's a number, skip over that many files
                if ('0'..='8').contains(&piece) {
                    // `piece` is from 0 to 8 inclusive so the unwrap cannot
                    // panic
                    #[allow(clippy::unwrap_used)]
                    let empty_squares = piece.to_digit(10).unwrap() as u8;
                    file_idx += empty_squares;
                } else {
                    let piece_num = Piece::from_char(piece.to_ascii_lowercase());
                    let Some(piece_num) = piece_num else {
                        reset_board_print_return!(self, "Error: \"{piece}\" is not a valid piece.");
                    };
                    // 1 if White, 0 if Black
                    let side = Side::from(u8::from(piece.is_ascii_uppercase()));

                    self.add_piece(
                        side,
                        Square::from_pos(Rank::from(rank_idx), File::from(file_idx)),
                        piece_num,
                    );

                    file_idx += 1;
                }
            }
            // if there are too few/many files in that rank, reset and return
            if file_idx != 8 {
                reset_board_print_return!(self, "Error: FEN is invalid");
            }

            file_idx = 0;
        }
        // if there are too many/few ranks in the board, reset and return
        if rank_idx != 0 {
            reset_board_print_return!(self, "Error: FEN is invalid (incorrect number of ranks)");
        }

        // 2. side to move
        if let Some(stm) = side_to_move {
            if stm == "w" {
                self.set_side_to_move(Side::WHITE);
            } else if stm == "b" {
                self.set_side_to_move(Side::BLACK);
            } else {
                reset_board_print_return!(
                    self,
                    "Error: Side to move \"{stm}\" is not \"w\" or \"b\""
                );
            }
        } else {
            // I've decided that everything apart from the board can be omitted
            // and guessed, so if there's nothing given, default to White to
            // move.
            self.set_side_to_move(Side::WHITE);
        }

        // 3. castling rights
        if let Some(cr) = castling_rights {
            for right in cr.chars() {
                match right {
                    'K' => self.add_castling_right(CastlingRights::K),
                    'Q' => self.add_castling_right(CastlingRights::Q),
                    'k' => self.add_castling_right(CastlingRights::k),
                    'q' => self.add_castling_right(CastlingRights::q),
                    '-' => (),
                    _ => {
                        reset_board_print_return!(
                            self,
                            "Error: castling right \"{right}\" is not valid"
                        );
                    }
                }
            }
        } else {
            // KQkq if nothing is given.
            self.set_default_castling_rights();
        }

        // 4. en passant
        self.set_ep_square(if let Some(ep) = ep_square {
            if ep == "-" {
                Square::NONE
            } else if let Some(square) = Square::from_string(ep) {
                square
            } else {
                reset_board_print_return!(
                    self,
                    "Error: En passant square \"{ep}\" is not a valid square"
                );
            }
        } else {
            Square::NONE
        });

        // 5. halfmoves
        self.set_halfmoves(if let Some(hm) = halfmoves {
            if let Ok(hm) = hm.parse::<u8>() {
                hm
            } else {
                reset_board_print_return!(
                    self,
                    "Error: Invalid number (\"hm\") given for halfmove counter"
                );
            }
        } else {
            0
        });

        // 6. fullmoves
        self.set_fullmoves(if let Some(fm) = fullmoves {
            if let Ok(fm) = fm.parse::<u16>() {
                fm
            } else {
                reset_board_print_return!(
                    self,
                    "Error: Invalid number (\"fm\") given for fullmove counter"
                );
            }
        } else {
            0
        });
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
    pub fn stringify(&self) -> String {
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
        *self = Self::default();
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

    /// Returns halfmoves since last capture or pawn move.
    #[inline]
    #[must_use]
    pub const fn halfmoves(&self) -> u8 {
        self.halfmoves
    }

    /// Sets halfmoves. Currently does nothing.
    #[inline]
    pub fn set_halfmoves(&mut self, count: u8) {
        self.halfmoves = count;
    }

    /// Returns the current move number.
    #[inline]
    #[must_use]
    pub const fn fullmoves(&self) -> u16 {
        self.fullmoves
    }

    /// Sets fullmoves. Currently does nothing.
    #[inline]
    pub fn set_fullmoves(&mut self, count: u16) {
        self.fullmoves = count;
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
