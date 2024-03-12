use std::{
    fmt::{self, Display, Formatter},
    ops::{BitAnd, BitAndAssign, BitOrAssign, Not, Shl},
    str::FromStr,
};

use crate::{
    bitboard::Bitboard,
    defs::{File, MoveType, Piece, PieceType, Rank, Side, Square},
    evaluation::{Score, PHASE_WEIGHTS, PIECE_SQUARE_TABLES},
    movegen::{generate_moves, Lookup, Move, LOOKUPS},
    out_of_bounds_is_unreachable,
    util::is_double_pawn_push,
};

/// Stores castling rights. Encoded as `KQkq`, with one bit for each right.
/// E.g. `0b1101` would be castling rights `KQq`.
#[derive(Clone, Copy, Eq, PartialEq)]
// The inner value of a wrapper does not need to be documented.
pub struct CastlingRights(u8);

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

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for CastlingRights {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOrAssign for CastlingRights {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl Display for CastlingRights {
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

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl Shl<u8> for CastlingRights {
    type Output = Self;

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
        [Bitboard::empty(); PieceType::TOTAL]
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
        [Bitboard::empty(); Side::TOTAL]
    }

    /// Pretty-prints the current state of the board.
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
        println!("FEN: {self}");
    }

    /// Sets `self.board` to the given FEN. It will check for basic errors,
    /// like the board having too many ranks, but not many more.
    // this function cannot panic as the only unwrap is on a hardcoded value
    #[allow(clippy::missing_panics_doc)]
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

        self.refresh_accumulators();
    }

    /// Takes a sequence of moves and feeds them to the board. Will stop and
    /// return if any of the moves are incorrect. Not implemented yet.
    pub fn play_moves(&mut self, moves_str: &str) {
        let mut copy = self.clone();

        // `split()` will always return at least 1 element even if the initial
        // string is empty
        if moves_str.is_empty() {
            return;
        }

        for mv in moves_str.split(' ') {
            let mut moves = generate_moves::<{ MoveType::ALL }>(&copy);

            // I don't particularly want to deal with non-ascii characters
            #[allow(clippy::string_slice)]
            let Ok(start) = Square::from_str(&mv[0..=1]) else {
                return println!("Start square of a move is not valid");
            };
            #[allow(clippy::string_slice)]
            let Ok(end) = Square::from_str(&mv[2..=3]) else {
                return println!("End square of a move is not valid");
            };

            let mv = moves.move_with(start, end);

            let Some(mv) = mv else {
                return println!("Illegal move");
            };

            if !copy.make_move(mv) {
                return println!("Illegal move");
            }
            moves.clear();
        }
        copy.refresh_accumulators();

        *self = copy;
    }

    /// Returns the piece bitboard given by `PIECE`.
    pub const fn piece<const PIECE: usize>(&self) -> Bitboard {
        self.pieces[PIECE]
    }

    /// Returns the board of the side according to `IS_WHITE`.
    pub const fn side<const IS_WHITE: bool>(&self) -> Bitboard {
        if IS_WHITE {
            self.sides[Side::WHITE.to_index()]
        } else {
            self.sides[Side::BLACK.to_index()]
        }
    }

    /// Returns all the occupied squares on the board.
    pub fn occupancies(&self) -> Bitboard {
        self.side::<true>() | self.side::<false>()
    }

    /// Returns the side to move.
    pub const fn side_to_move(&self) -> Side {
        self.side_to_move
    }

    /// Calculates if the given side can castle kingside.
    pub fn can_castle_kingside<const IS_WHITE: bool>(&self) -> bool {
        self.castling_rights.can_castle_kingside::<IS_WHITE>()
    }

    /// Calculates if the given side can castle queenside.
    pub fn can_castle_queenside<const IS_WHITE: bool>(&self) -> bool {
        self.castling_rights.can_castle_queenside::<IS_WHITE>()
    }

    /// Returns the en passant square, which might be [`Square::NONE`].
    pub const fn ep_square(&self) -> Square {
        self.ep_square
    }

    /// Returns halfmoves since last capture or pawn move.
    pub const fn halfmoves(&self) -> u8 {
        self.halfmoves
    }

    /// Returns the current move number.
    pub const fn fullmoves(&self) -> u16 {
        self.fullmoves
    }

    /// Calculates the current material + piece-square table balance.
    ///
    /// Since this value is incrementally upadated, this function is zero-cost
    /// to call.
    pub const fn psq(&self) -> Score {
        self.psq_accumulator
    }

    /// Gets the phase of the game. 0 is midgame and 24 is endgame.
    ///
    /// Since this value is incrementally upadated, this function is zero-cost
    /// to call.
    pub const fn phase(&self) -> u8 {
        self.phase_accumulator
    }

    /// Tests if the king is in check.
    pub fn is_in_check(&self) -> bool {
        self.is_square_attacked(self.king_square())
    }

    /// Makes the given move on the internal board. `mv` is assumed to be a
    /// valid move. Returns `true` if the given move is legal, `false`
    /// otherwise.
    pub fn make_move(&mut self, mv: Move) -> bool {
        let start = mv.start();
        let end = mv.end();
        let is_promotion = mv.is_promotion();
        let is_castling = mv.is_castling();
        let is_en_passant = mv.is_en_passant();

        let piece = self.piece_on(start);
        let piece_type = PieceType::from(piece);
        let captured = self.piece_on(end);
        let captured_type = PieceType::from(captured);
        let us = Side::from(piece);
        let them = us.flip();
        let start_bb = Bitboard::from(start);
        let end_bb = Bitboard::from(end);

        self.halfmoves += 1;
        if us == Side::BLACK {
            self.fullmoves += 1;
        }

        if piece_type == PieceType::PAWN || captured_type != PieceType::NONE {
            self.halfmoves = 0;
        // 75-move rule: if 75 moves have been made by both players, the game
        // is adjucated as a draw. So the 151st move is illegal.
        } else if self.halfmoves > 150 {
            return false;
        }

        self.clear_ep_square();

        self.move_piece(start, end, piece, piece_type, us);

        if captured_type != PieceType::NONE {
            self.update_bb_piece(end_bb, captured_type, them);
            self.remove_psq_piece(end, captured);
            self.remove_phase_piece(captured);

            // check if we need to unset the castling rights if we're capturing
            // a rook
            if captured_type == PieceType::ROOK {
                let us_inner = us.0;
                // this will be 0x81 if we're White (0x81 << 0) and
                // 0x8100000000000000 if we're Black (0x81 << 56). This mask is
                // the starting position of our rooks.
                let rook_squares = Bitboard(0x81) << (us_inner * 56);
                if !(end_bb & rook_squares).is_empty() {
                    // 0 or 56 for queenside -> 0
                    // 7 or 63 for kingside -> 1
                    let is_kingside = end.0 & 1;
                    // queenside -> 0b01
                    // kingside -> 0b10
                    // this replies upon knowing the internal representation of
                    // CastlingRights - 0b01 is queenside, 0b10 is kingside
                    let flag = is_kingside + 1;
                    self.unset_castling_right(them, CastlingRights(flag));
                }
            }
        }

        if is_castling {
            // if the king is castling out of check
            if self.is_square_attacked(start) {
                return false;
            }
            // if the king is castling into check
            if self.is_square_attacked(end) {
                return false;
            }

            let rook_start = Square(end.0.wrapping_add_signed(mv.rook_offset()));
            let rook_end = Square((start.0 + end.0) >> 1);
            // if the king is castling through check
            if self.is_square_attacked(rook_end) {
                return false;
            }

            self.move_piece(
                rook_start,
                rook_end,
                // `captured` is equivalent but slower
                Piece::from_piecetype(PieceType::ROOK, us),
                PieceType::ROOK,
                us,
            );

            self.unset_castling_rights(us);
        } else if is_double_pawn_push(start, end, piece) {
            self.set_ep_square(Square((start.0 + end.0) >> 1));
        } else if is_en_passant {
            let dest = Square(if us == Side::WHITE {
                end.0 - 8
            } else {
                end.0 + 8
            });
            let captured_pawn = Piece::from_piecetype(PieceType::PAWN, them);
            self.remove_piece(dest, captured_pawn, PieceType::PAWN, them);
        } else if is_promotion {
            let promotion_piece_type = mv.promotion_piece();
            let promotion_piece = Piece::from_piecetype(promotion_piece_type, us);

            // overwrite the pawn on the mailbox
            self.set_mailbox_piece(end, promotion_piece);

            // remove the pawn
            self.toggle_piece_bb(piece_type, end_bb);
            self.remove_psq_piece(end, piece);
            self.remove_phase_piece(piece);

            // add the promotion piece
            self.toggle_piece_bb(promotion_piece_type, end_bb);
            self.add_psq_piece(end, promotion_piece);
            self.add_phase_piece(promotion_piece);
        }

        if self.is_square_attacked(self.king_square()) {
            return false;
        }

        // this is basically the same as a few lines ago but with start square
        // instead of end
        if piece_type == PieceType::ROOK {
            let them_inner = them.0;
            let rook_squares = Bitboard(0x81) << (them_inner * 56);
            if !(start_bb & rook_squares).is_empty() {
                let is_kingside = start.0 & 1;
                let flag = is_kingside + 1;
                self.unset_castling_right(us, CastlingRights(flag));
            }
        }
        if piece_type == PieceType::KING {
            self.unset_castling_rights(us);
        }

        self.flip_side();

        true
    }

    /// Converts the current board to a string.
    ///
    /// e.g. the starting position would be
    /// "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR".
    // the unwraps are called on values that logically can not be `None`, so
    // this can not panic
    #[allow(clippy::missing_panics_doc)]
    fn stringify_board(&self) -> String {
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

    /// Clears `self`.
    fn clear_board(&mut self) {
        self.mailbox = Self::empty_mailbox();
        self.pieces = Self::no_pieces();
        self.sides = Self::no_sides();
        self.side_to_move = Side::NONE;
        self.castling_rights = CastlingRights::none();
        self.ep_square = Square::NONE;
        self.halfmoves = 0;
        self.fullmoves = 1;
        self.psq_accumulator = Score(0, 0);
        self.phase_accumulator = 0;
    }

    /// Resets the board.
    fn set_startpos(&mut self) {
        *self = Self::default();
    }

    /// Finds the piece on the given rank and file and converts it to its
    /// character representation. If no piece is on the square, returns '0'
    /// instead.
    fn char_piece_from_pos(&self, rank: Rank, file: File) -> char {
        let square = Square::from_pos(rank, file);
        let piece = self.piece_on(square);
        char::from(piece)
    }

    /// A wrapper over [`move_mailbox_piece`](Board::move_mailbox_piece),
    /// [`update_bb_piece`](Board::update_bb_piece) and
    /// [`move_psq_piece`](Board::move_psq_piece).
    ///
    /// Use the three different functions separately if needed.
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

    /// Returns the piece on `square`.
    fn piece_on(&self, square: Square) -> Piece {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.mailbox.len()) };
        self.mailbox[square.to_index()]
    }

    /// Adds a piece to square `square` for side `side`. Assumes there is no
    /// piece on the square to be written to.
    fn add_piece(&mut self, square: Square, piece: Piece) {
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
    fn remove_piece(&mut self, square: Square, piece: Piece, piece_type: PieceType, side: Side) {
        let bb = Bitboard::from(square);
        self.unset_mailbox_piece(square);
        self.toggle_piece_bb(piece_type, bb);
        self.toggle_side_bb(side, bb);
        self.remove_psq_piece(square, piece);
        self.remove_phase_piece(piece);
    }

    /// Moves `piece` from `start` to `end` in the mailbox.
    ///
    /// `piece` is assumed to exist at the start square: the piece is given as
    /// an argument instead of calculated for reasons of speed.
    fn move_mailbox_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.unset_mailbox_piece(start);
        self.set_mailbox_piece(end, piece);
    }

    /// Sets the piece on `square` in the mailbox to `piece`.
    fn set_mailbox_piece(&mut self, square: Square, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.mailbox.len()) };
        self.mailbox[square.to_index()] = piece;
    }

    /// Sets the piece on `square` in the mailbox to [`Square::NONE`].
    fn unset_mailbox_piece(&mut self, square: Square) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.mailbox.len()) };
        self.mailbox[square.to_index()] = Piece::NONE;
    }

    /// Toggles the bits set in `bb` for the piece bitboard of `piece_type` and
    /// the side bitboard of `side`.
    fn update_bb_piece(&mut self, bb: Bitboard, piece_type: PieceType, side: Side) {
        self.toggle_piece_bb(piece_type, bb);
        self.toggle_side_bb(side, bb);
    }

    /// Toggles the bits set in `bb` of the bitboard of `piece`.
    fn toggle_piece_bb(&mut self, piece: PieceType, bb: Bitboard) {
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

    /// Sets side to move to `side`.
    fn set_side_to_move(&mut self, side: Side) {
        self.side_to_move = side;
    }

    /// Flip the side to move.
    fn flip_side(&mut self) {
        self.side_to_move = self.side_to_move.flip();
    }

    /// Returns the string representation of the current side to move: 'w' if
    /// White and 'b' if Black.
    const fn side_to_move_as_char(&self) -> char {
        (b'b' + self.side_to_move().0 * 21) as char
    }

    /// Converts the current castling rights into their string representation.
    ///
    /// E.g. `KQq` if the White king can castle both ways and the Black king
    /// can only castle queenside.
    fn stringify_castling_rights(&self) -> String {
        self.castling_rights.to_string()
    }

    /// Adds the given right to the castling rights.
    fn add_castling_right(&mut self, right: CastlingRights) {
        self.castling_rights.add_right(right);
    }

    /// Unsets castling the given right for the given side.
    fn unset_castling_right(&mut self, side: Side, right: CastlingRights) {
        self.castling_rights.remove_right(side, right);
    }

    /// Clears the castling rights for the given side.
    fn unset_castling_rights(&mut self, side: Side) {
        self.castling_rights.clear_side(side);
    }

    /// Returns the string representation of the current en passant square: the
    /// square if there is one (e.g. "b3") or "-" if there is none.
    fn stringify_ep_square(&self) -> String {
        let ep_square = self.ep_square();
        if ep_square == Square::NONE {
            "-".to_string()
        } else {
            ep_square.to_string()
        }
    }

    /// Sets the en passant square to `square`.
    fn set_ep_square(&mut self, square: Square) {
        self.ep_square = square;
    }

    /// Sets the en passant square to [`Square::NONE`].
    fn clear_ep_square(&mut self) {
        self.ep_square = Square::NONE;
    }

    /// Sets halfmoves.
    fn set_halfmoves(&mut self, count: u8) {
        self.halfmoves = count;
    }

    /// Sets fullmoves.
    fn set_fullmoves(&mut self, count: u16) {
        self.fullmoves = count;
    }

    /// Recalculates the accumulators from scratch. Prefer to use functions
    /// that incrementally update both if possible.
    fn refresh_accumulators(&mut self) {
        let mut score = Score(0, 0);
        let mut phase = 0;

        for (square, piece) in self.mailbox.iter().enumerate() {
            score += PIECE_SQUARE_TABLES[piece.to_index()][square];
            phase += PHASE_WEIGHTS[piece.to_index()];
        }

        self.psq_accumulator = score;
        self.phase_accumulator = phase;
    }

    /// Updates the piece-square table accumulator by adding the difference
    /// between the psqt value of the start and end square (which can be
    /// negative).
    fn move_psq_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.remove_psq_piece(start, piece);
        self.add_psq_piece(end, piece);
    }

    /// Adds the piece-square table value for `piece` at `square` to the psqt
    /// accumulator.
    fn add_psq_piece(&mut self, square: Square, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PIECE_SQUARE_TABLES.len()) };
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), PIECE_SQUARE_TABLES[0].len()) };
        self.psq_accumulator += PIECE_SQUARE_TABLES[piece.to_index()][square.to_index()];
    }

    /// Removes the piece-square table value for `piece` at `square` from the
    /// psqt accumulator.
    fn remove_psq_piece(&mut self, square: Square, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PIECE_SQUARE_TABLES.len()) };
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), PIECE_SQUARE_TABLES[0].len()) };
        self.psq_accumulator -= PIECE_SQUARE_TABLES[piece.to_index()][square.to_index()];
    }

    /// Adds `piece` to `self.phase`.
    fn add_phase_piece(&mut self, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PHASE_WEIGHTS.len()) };
        self.phase_accumulator += PHASE_WEIGHTS[piece.to_index()];
    }

    /// Removes `piece` from `self.phase`.
    fn remove_phase_piece(&mut self, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PHASE_WEIGHTS.len()) };
        self.phase_accumulator -= PHASE_WEIGHTS[piece.to_index()];
    }

    /// Calculates the square the king is on.
    fn king_square(&self) -> Square {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(self.side_to_move().to_index(), self.sides.len()) };
        Square::from(
            self.piece::<{ PieceType::KING.to_index() }>()
                & self.sides[self.side_to_move().to_index()],
        )
    }

    /// Tests if `square` is attacked by an enemy piece.
    fn is_square_attacked(&self, square: Square) -> bool {
        let occupancies = self.occupancies();
        let us = self.side_to_move();
        let them = us.flip();
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(them.to_index(), self.sides.len()) };
        let them_bb = self.sides[them.to_index()];

        // SAFETY: Instantiating `self` initialises `LOOKUP`.
        let pawn_attacks = unsafe { LOOKUPS.pawn_attacks(us, square) };
        // SAFETY: Ditto.
        let knight_attacks = unsafe { LOOKUPS.knight_attacks(square) };
        // SAFETY: Ditto.
        let diagonal_attacks = unsafe { LOOKUPS.bishop_attacks(square, occupancies) };
        // SAFETY: Ditto.
        let orthogonal_attacks = unsafe { LOOKUPS.rook_attacks(square, occupancies) };
        // SAFETY: Ditto.
        let king_attacks = unsafe { LOOKUPS.king_attacks(square) };

        let pawns = self.piece::<{ PieceType::PAWN.to_index() }>();
        let knights = self.piece::<{ PieceType::KNIGHT.to_index() }>();
        let bishops = self.piece::<{ PieceType::BISHOP.to_index() }>();
        let rooks = self.piece::<{ PieceType::ROOK.to_index() }>();
        let queens = self.piece::<{ PieceType::QUEEN.to_index() }>();
        let kings = self.piece::<{ PieceType::KING.to_index() }>();

        let is_attacked_by_pawns = pawn_attacks & pawns;
        let is_attacked_by_knights = knight_attacks & knights;
        let is_attacked_by_kings = king_attacks & kings;
        let is_attacked_diagonally = diagonal_attacks & (bishops | queens);
        let is_attacked_orthogonally = orthogonal_attacks & (rooks | queens);

        !((is_attacked_by_pawns
            | is_attacked_by_knights
            | is_attacked_by_kings
            | is_attacked_diagonally
            | is_attacked_orthogonally)
            & them_bb)
            .is_empty()
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
