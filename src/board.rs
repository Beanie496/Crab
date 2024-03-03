use std::{
    fmt::{self, Display, Formatter},
    ops::{BitAnd, BitAndAssign, BitOrAssign, Not, Shl},
    slice::Iter,
    str::FromStr,
};

use crate::{
    bitboard::Bitboard,
    defs::{File, MoveType, Piece, PieceType, Rank, Side, Square},
    evaluation::{Score, PHASE_WEIGHTS, PIECE_SQUARE_TABLES},
    out_of_bounds_is_unreachable,
};
use movegen::Lookup;

pub use movegen::{magic::find_magics, Move, Moves};

/// Stores castling rights. Encoded as `KQkq`, with one bit for each right.
/// E.g. `0b1101` would be castling rights `KQq`.
#[derive(Clone, Copy, Eq, PartialEq)]
// The inner value of a wrapper does not need to be documented.
#[allow(clippy::missing_docs_in_private_items)]
pub struct CastlingRights(u8);

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
    /// An array of piece values, used for quick lookup of which piece is on a
    /// given square.
    mailbox: [Piece; Square::TOTAL],
    /// `pieces[0]` is the intersection of all pawns on the board, `pieces[1]`
    /// is the knights, and so on, as according to the order set by
    /// [`Piece`].
    pieces: [Bitboard; PieceType::TOTAL],
    /// `sides[1]` is the intersection of all White piece bitboards; `sides[0]`
    /// is is the intersection of all Black piece bitboards.
    sides: [Bitboard; Side::TOTAL],
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
    /// The current material balance weighted with piece-square tables, from
    /// the perspective of White.
    ///
    /// It is incrementally updated.
    psq_accumulator: Score,
    /// The current phase of the game, where 0 means the midgame and 24 means
    /// the endgame.
    ///
    /// `psq_val` uses this value to lerp between its midgame and
    /// endgame values. It is incrementally updated.
    phase_accumulator: u8,
}

impl Display for Board {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {}",
            &self.stringify_board(),
            &self.side_to_move_as_char(),
            &self.stringify_castling_rights(),
            &self.stringify_ep_square(),
            &self.halfmoves(),
            &self.fullmoves(),
        )
    }
}

impl BitAnd for CastlingRights {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for CastlingRights {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOrAssign for CastlingRights {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl Display for CastlingRights {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
        if ret_str.is_empty() {
            ret_str.push('-');
        }
        f.write_str(&ret_str)
    }
}

impl Not for CastlingRights {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl Shl<u8> for CastlingRights {
    type Output = Self;

    #[inline]
    fn shl(self, rhs: u8) -> Self::Output {
        Self(self.0 << rhs)
    }
}

#[allow(non_upper_case_globals)]
/// Flags. It's fine to use `&`, `^` and `|` on these.
impl CastlingRights {
    /// The flag `K`.
    pub const K: Self = Self(0b1000);
    /// The flag `Q`.
    pub const Q: Self = Self(0b0100);
    /// The flag `k`.
    pub const k: Self = Self(0b0010);
    /// The flag `q`.
    pub const q: Self = Self(0b0001);
    /// The flags `KQkq`, i.e. all flags.
    pub const KQkq: Self = Self(0b1111);
    /// No flags.
    pub const NONE: Self = Self(0b0000);
}

impl Default for Board {
    #[inline]
    fn default() -> Self {
        let mut board = Self {
            mailbox: Self::default_mailbox(),
            pieces: Self::default_pieces(),
            sides: Self::default_sides(),
            side_to_move: Side::WHITE,
            castling_rights: CastlingRights::new(),
            ep_square: Square::NONE,
            halfmoves: 0,
            fullmoves: 1,
            psq_accumulator: Score(0, 0),
            phase_accumulator: 0,
        };
        board.refresh_accumulators();
        board
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

    /// Returns the mailbox of the starting position.
    #[rustfmt::skip]
    #[allow(clippy::many_single_char_names, non_snake_case)]
    const fn default_mailbox() -> [Piece; Square::TOTAL] {
        let p = Piece::BPAWN;
        let n = Piece::BKNIGHT;
        let b = Piece::BBISHOP;
        let r = Piece::BROOK;
        let q = Piece::BQUEEN;
        let k = Piece::BKING;
        let P = Piece::WPAWN;
        let N = Piece::WKNIGHT;
        let B = Piece::WBISHOP;
        let R = Piece::WROOK;
        let Q = Piece::WQUEEN;
        let K = Piece::WKING;
        let e = Piece::NONE;
        [
            R, N, B, Q, K, B, N, R,
            P, P, P, P, P, P, P, P,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            e, e, e, e, e, e, e, e,
            p, p, p, p, p, p, p, p,
            r, n, b, q, k, b, n, r,
        ]
    }

    /// Returns an mailbox.
    const fn empty_mailbox() -> [Piece; Square::TOTAL] {
        [Piece::NONE; Square::TOTAL]
    }

    /// Returns the piece [`Bitboard`]s of the starting position.
    const fn default_pieces() -> [Bitboard; PieceType::TOTAL] {
        [
            Bitboard(0x00ff_0000_0000_ff00), // Pawns
            Bitboard(0x4200_0000_0000_0042), // Knights
            Bitboard(0x2400_0000_0000_0024), // Bishops
            Bitboard(0x8100_0000_0000_0081), // Rooks
            Bitboard(0x0800_0000_0000_0008), // Queens
            Bitboard(0x1000_0000_0000_0010), // Kings
        ]
    }

    /// Returns the piece [`Bitboard`]s of an empty board.
    const fn no_pieces() -> [Bitboard; PieceType::TOTAL] {
        [Bitboard::EMPTY; PieceType::TOTAL]
    }

    /// Returns the side [`Bitboard`]s of the starting position.
    const fn default_sides() -> [Bitboard; Side::TOTAL] {
        [
            Bitboard(0xffff_0000_0000_0000), // Black
            Bitboard(0x0000_0000_0000_ffff), // White
        ]
    }

    /// Returns the side [`Bitboard`]s of an empty board.
    const fn no_sides() -> [Bitboard; Side::TOTAL] {
        [Bitboard::EMPTY; Side::TOTAL]
    }

    /// Returns an iterator over the internal mailbox. a1 b1, etc.
    #[inline]
    pub fn mailbox_iter(&self) -> Iter<'_, Piece> {
        self.mailbox.iter()
    }

    /// Copies and returns the mailbox of `self`.
    #[inline]
    #[must_use]
    pub const fn clone_mailbox(&self) -> [Piece; Square::TOTAL] {
        self.mailbox
    }

    /// Clears `self`.
    #[inline]
    pub fn clear_board(&mut self) {
        self.mailbox = Self::empty_mailbox();
        self.pieces = Self::no_pieces();
        self.sides = Self::no_sides();
        self.side_to_move = Side::NONE;
        self.castling_rights = CastlingRights::none();
        self.ep_square = Square::NONE;
    }

    /// Pretty-prints the current state of the board.
    #[inline]
    pub fn pretty_print(&self) {
        for r in (0..Rank::TOTAL as u8).rev() {
            print!("{} | ", r + 1);
            for f in 0..File::TOTAL as u8 {
                print!("{} ", self.char_piece_from_pos(Rank(r), File(f)));
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
        self.to_string()
    }

    /// Takes a sequence of moves and feeds them to the board. Will stop and
    /// return if any of the moves are incorrect. Not implemented yet.
    #[inline]
    pub fn play_moves(&mut self, moves_str: &str) {
        let mut moves = Moves::new();
        let mut copy = self.clone();

        // `split()` will always return at least 1 element even if the initial
        // string is empty
        if moves_str.is_empty() {
            return;
        }

        for mv in moves_str.split(' ') {
            copy.generate_moves::<{ MoveType::ALL }>(&mut moves);

            // I don't particularly want to deal with non-ascii characters
            #[allow(clippy::string_slice)]
            let Ok(start) = Square::from_str(&mv[0..=1]) else {
                return println!("Start square of a move is not valid");
            };
            #[allow(clippy::string_slice)]
            let Ok(end) = Square::from_str(&mv[2..=3]) else {
                return println!("End square of a move is not valid");
            };
            // Each move should be exactly 4 characters; if it's a promotion,
            // the last char will be the promotion char.
            let mv = if mv.len() == 5 {
                // SAFETY: It's not possible for it to be `None`.
                let promotion_char = unsafe { mv.chars().next_back().unwrap_unchecked() };
                moves.move_with_promo(start, end, PieceType::from(promotion_char))
            } else {
                moves.move_with(start, end)
            };

            let Some(mv) = mv else {
                return println!("Illegal move");
            };

            if !copy.make_move(mv) {
                return println!("Illegal move");
            }
            moves.clear();
        }
        *self = copy;
    }

    /// Sets `self.board` to the given FEN. It will check for basic errors,
    /// like the board having too many ranks, but not many more.
    // this function cannot panic as the only unwrap is on a hardcoded value
    #[allow(clippy::missing_panics_doc, clippy::too_many_lines)]
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
                    let piece_num = Piece::from(piece);
                    if piece_num == Piece::NONE {
                        reset_board_print_return!(self, "Error: \"{piece}\" is not a valid piece.");
                    }

                    self.add_piece(Square::from_pos(Rank(rank_idx), File(file_idx)), piece_num);

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
        }

        // 4. en passant
        self.set_ep_square(if let Some(ep) = ep_square {
            if let Ok(square) = ep.parse::<Square>() {
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
                    "Error: Invalid number (\"{hm}\") given for halfmove counter"
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
                    "Error: Invalid number (\"{fm}\") given for fullmove counter"
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
    pub fn stringify_board(&self) -> String {
        let mut ret_str = String::new();
        let mut empty_squares = 0;
        // I can't just iterate over the piece board normally: the board goes
        // from a1 to h8, rank-file, whereas the FEN goes from a8 to h1, also
        // rank-file
        for rank in (0..Rank::TOTAL).rev() {
            for file in 0..File::TOTAL {
                let square = Square::from_pos(Rank(rank as u8), File(file as u8));
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
                    ret_str.push(char::from(piece));
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
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.mailbox.len()) };
        self.mailbox[square.to_index()]
    }

    /// Returns the piece bitboard given by `PIECE`.
    #[inline]
    #[must_use]
    pub const fn piece<const PIECE: usize>(&self) -> Bitboard {
        self.pieces[PIECE]
    }

    /// Adds a piece to square `square` for side `side`. Assumes there is no
    /// piece on the square to be written to.
    #[inline]
    pub fn add_piece(&mut self, square: Square, piece: Piece) {
        let square_bb = Bitboard::from(square);
        let side = Side::from(piece);
        self.set_mailbox_piece(square, piece);
        self.toggle_piece_bb(PieceType::from(piece), square_bb);
        self.toggle_side_bb(side, square_bb);
        self.add_psq_piece(square, piece);
        self.add_phase_piece(piece);
    }

    /// Removes `piece` from `square`.
    ///
    /// Technically most of these parameters could be calculated instead of
    /// passed by argument, but it resulted in a noticeable slowdown when they
    /// were removed.
    #[inline]
    pub fn remove_piece(
        &mut self,
        square: Square,
        piece: Piece,
        piece_type: PieceType,
        side: Side,
    ) {
        let bb = Bitboard::from(square);
        self.unset_mailbox_piece(square);
        self.toggle_piece_bb(piece_type, bb);
        self.toggle_side_bb(side, bb);
        self.remove_psq_piece(square, piece);
        self.remove_phase_piece(piece);
    }

    /// A wrapper over [`move_mailbox_piece`](Board::move_mailbox_piece),
    /// [`update_bb_piece`](Board::update_bb_piece) and
    /// [`move_psq_piece`](Board::move_psq_piece).
    ///
    /// Use the three different functions separately if needed.
    #[inline]
    fn move_piece(
        &mut self,
        start: Square,
        end: Square,
        piece: Piece,
        piece_type: PieceType,
        side: Side,
    ) {
        // this _is_ faster to calculate on the fly, since the alternative is
        // passing a full `u64` by argument
        let bb = Bitboard::from(start) | Bitboard::from(end);
        self.move_mailbox_piece(start, end, piece);
        self.update_bb_piece(bb, piece_type, side);
        self.move_psq_piece(start, end, piece);
    }

    /// Sets the piece on `square` in the mailbox to `piece`.
    #[inline]
    pub fn set_mailbox_piece(&mut self, square: Square, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.mailbox.len()) };
        self.mailbox[square.to_index()] = piece;
    }

    /// Sets the piece on `square` in the mailbox to [`Square::NONE`].
    #[inline]
    pub fn unset_mailbox_piece(&mut self, square: Square) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.mailbox.len()) };
        self.mailbox[square.to_index()] = Piece::NONE;
    }

    /// Moves `piece` from `start` to `end` in the mailbox.
    ///
    /// `piece` is assumed to exist at the start square: the piece is given as
    /// an argument instead of calculated for reasons of speed.
    #[inline]
    pub fn move_mailbox_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.unset_mailbox_piece(start);
        self.set_mailbox_piece(end, piece);
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

    /// Returns the bitboard of the given side.
    #[inline]
    #[must_use]
    pub const fn side_any(&self, side: Side) -> Bitboard {
        self.sides[side.to_index()]
    }

    /// Returns the board of the side according to `IS_WHITE`.
    #[inline]
    #[must_use]
    pub const fn side<const IS_WHITE: bool>(&self) -> Bitboard {
        if IS_WHITE {
            self.sides[Side::WHITE.to_index()]
        } else {
            self.sides[Side::BLACK.to_index()]
        }
    }

    /// Returns the string representation of the current side to move: 'w' if
    /// White and 'b' if Black.
    #[inline]
    #[must_use]
    pub const fn side_to_move_as_char(&self) -> char {
        (b'b' + self.side_to_move().0 * 21) as char
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
        self.castling_rights.to_string()
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
            ep_square.to_string()
        }
    }

    /// Returns halfmoves since last capture or pawn move.
    #[inline]
    #[must_use]
    pub const fn halfmoves(&self) -> u8 {
        self.halfmoves
    }

    /// Sets halfmoves.
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

    /// Sets fullmoves.
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
        let square = Square::from_pos(rank, file);
        let piece = self.piece_on(square);
        char::from(piece)
    }

    /// Toggles the bits set in `bb` of the bitboard of `piece`.
    #[inline]
    fn toggle_piece_bb(&mut self, piece: PieceType, bb: Bitboard) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), self.pieces.len()) };
        self.pieces[piece.to_index()] ^= bb;
    }

    /// Toggles the bits set in `bb` of the bitboard of `side`.
    #[inline]
    fn toggle_side_bb(&mut self, side: Side, bb: Bitboard) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(side.to_index(), self.sides.len()) };
        self.sides[side.to_index()] ^= bb;
    }

    /// Toggles the bits set in `bb` for the piece bitboard of `piece_type` and
    /// the side bitboard of `side`.
    #[inline]
    fn update_bb_piece(&mut self, bb: Bitboard, piece_type: PieceType, side: Side) {
        self.toggle_piece_bb(piece_type, bb);
        self.toggle_side_bb(side, bb);
    }

    /// Recalculates the accumulators from scratch. Prefer to use functions
    /// that incrementally update both if possible.
    fn refresh_accumulators(&mut self) {
        let mut score = Score(0, 0);
        let mut phase = 0;

        for (square, piece) in self.mailbox_iter().enumerate() {
            score += PIECE_SQUARE_TABLES[piece.to_index()][square];
            phase += PHASE_WEIGHTS[piece.to_index()];
        }

        self.psq_accumulator = score;
        self.phase_accumulator = phase;
    }

    /// Calculates the current material + piece-square table balance.
    ///
    /// Since this value is incrementally upadated, this function is zero-cost
    /// to call.
    #[inline]
    #[must_use]
    pub const fn psq(&self) -> Score {
        self.psq_accumulator
    }

    /// Adds the piece-square table value for `piece` at `square` to the psqt
    /// accumulator.
    #[inline]
    fn add_psq_piece(&mut self, square: Square, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PIECE_SQUARE_TABLES.len()) };
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), PIECE_SQUARE_TABLES[0].len()) };
        self.psq_accumulator += PIECE_SQUARE_TABLES[piece.to_index()][square.to_index()];
    }

    /// Removes the piece-square table value for `piece` at `square` from the
    /// psqt accumulator.
    #[inline]
    fn remove_psq_piece(&mut self, square: Square, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PIECE_SQUARE_TABLES.len()) };
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), PIECE_SQUARE_TABLES[0].len()) };
        self.psq_accumulator -= PIECE_SQUARE_TABLES[piece.to_index()][square.to_index()];
    }

    /// Updates the piece-square table accumulator by adding the difference
    /// between the psqt value of the start and end square (which can be
    /// negative).
    #[inline]
    fn move_psq_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.remove_psq_piece(start, piece);
        self.add_psq_piece(end, piece);
    }

    /// Gets the phase of the game. 0 is midgame and 24 is endgame.
    ///
    /// Since this value is incrementally upadated, this function is zero-cost
    /// to call.
    #[inline]
    #[must_use]
    pub const fn phase(&self) -> u8 {
        self.phase_accumulator
    }

    /// Adds `piece` to `self.phase`.
    #[inline]
    fn add_phase_piece(&mut self, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PHASE_WEIGHTS.len()) };
        self.phase_accumulator += PHASE_WEIGHTS[piece.to_index()];
    }

    /// Removes `piece` from `self.phase`.
    #[inline]
    fn remove_phase_piece(&mut self, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PHASE_WEIGHTS.len()) };
        self.phase_accumulator -= PHASE_WEIGHTS[piece.to_index()];
    }

    /// Tests if the king is in check.
    #[inline]
    #[must_use]
    pub fn is_in_check(&self) -> bool {
        self.is_square_attacked(self.king_square())
    }

    /// Calculates the square the king is on.
    fn king_square(&self) -> Square {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(self.side_to_move().to_index(), self.sides.len()) };
        (self.piece::<{ PieceType::KING.to_index() }>()
            & self.sides[self.side_to_move().to_index()])
        .to_square()
    }
}

impl CastlingRights {
    /// Returns new [`CastlingRights`] with the default castling rights.
    const fn new() -> Self {
        Self::KQkq
    }

    /// Returns empty [`CastlingRights`].
    const fn none() -> Self {
        Self::NONE
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
        // `side.0 * 2` is 0 for Black and 2 for White. Thus, if `right` is
        // `0brr`, `right << side.0` is `0brr00` for White and `0brr` for
        // Black.
        *self &= !(right << (side.0 * 2));
    }

    /// Clears the rights for `side`.
    fn clear_side(&mut self, side: Side) {
        debug_assert_eq!(
            Side::WHITE.0,
            1,
            "This function relies on White being 1 and Black 0"
        );
        // `side * 2` is 2 for White and 0 for Black. `0b11 << (side * 2)` is
        // a mask for the bits for White or Black. `&`ing the rights with
        // `!(0b11 << (side * 2))` will clear the bits on the given side.
        *self &= Self(!(0b11 << (side.0 * 2)));
    }
}
