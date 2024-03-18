use std::{
    fmt::{self, Display, Formatter},
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not, Shl},
    str::FromStr,
};

use crate::{
    bitboard::Bitboard,
    defs::{File, MoveType, Piece, PieceType, Rank, Side, Square},
    evaluation::Score,
    index_into_unchecked, index_unchecked,
    movegen::{generate_moves, Lookup, Move, LOOKUPS},
    util::is_double_pawn_push,
};
use zobrist::Key;

/// Evaluation accumulators.
mod accumulators;
/// Functions for zobrist hashing.
mod zobrist;

/// Stores castling rights. Encoded as `KQkq`, with one bit for each right.
/// E.g. `0b1101` would be castling rights `KQq`.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CastlingRights(u8);

/// The board. It contains information about the current board state and can
/// generate pseudo-legal moves. It is small (134 bytes) so it uses copy-make.
#[derive(Clone, Copy)]
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
    /// The current phase of the game, where 0 means the midgame and 24 means
    /// the endgame.
    ///
    /// `psq_val` uses this value to lerp between its midgame and
    /// endgame values. It is incrementally updated.
    phase_accumulator: u8,
    /// The current material balance weighted with piece-square tables, from
    /// the perspective of White.
    ///
    /// It is incrementally updated.
    psq_accumulator: Score,
    /// The current zobrist key of the board.
    zobrist: Key,
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
            phase_accumulator: 0,
            psq_accumulator: Score(0, 0),
            zobrist: 0,
        };
        board.refresh_accumulators();
        board.refresh_zobrist();
        board
    }
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

impl BitOr for CastlingRights {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
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
        println!("Zobrist key: {}", self.zobrist());
    }

    /// Sets `self.board` to the given FEN.
    ///
    /// This function does practically no error checking because that's up to
    /// the GUI. If it does catch an error, it will panic.
    #[allow(clippy::unwrap_used)]
    pub fn set_pos_to_fen(&mut self, position: &str) {
        if position.is_empty() {
            return;
        }

        self.clear_board();

        let mut iter = position.split(' ');
        let board = iter.next().unwrap();
        let side_to_move = iter.next();
        let castling_rights = iter.next();
        let ep_square = iter.next();
        let halfmoves = iter.next();
        let fullmoves = iter.next();

        // 1. the board itself
        let mut square = 56;
        let ranks = board.split('/');
        for rank in ranks {
            for piece in rank.chars() {
                // if it's a number, skip over that many files
                if ('0'..='8').contains(&piece) {
                    // `piece` is from 0 to 8 inclusive so the unwrap cannot
                    // panic
                    let empty_squares = piece.to_digit(10).unwrap() as u8;
                    square += empty_squares;
                } else {
                    let piece = Piece::from(piece);

                    self.add_piece(Square(square), piece);

                    square += 1;
                }
            }
            square = square.wrapping_sub(16);
        }

        // 2. side to move
        if let Some(stm) = side_to_move {
            if stm == "w" {
                self.set_side_to_move(Side::WHITE);
            } else {
                self.set_side_to_move(Side::BLACK);
            }
        }

        // 3. castling rights
        if let Some(cr) = castling_rights {
            for right in cr.chars() {
                match right {
                    'K' => self.add_castling_right(CastlingRights::K),
                    'Q' => self.add_castling_right(CastlingRights::Q),
                    'k' => self.add_castling_right(CastlingRights::k),
                    'q' => self.add_castling_right(CastlingRights::q),
                    _ => (),
                }
            }
        }

        // 4. en passant
        let ep_square = ep_square.map_or(Square::NONE, |ep| ep.parse::<Square>().unwrap());
        self.set_ep_square(ep_square);

        // 5. halfmoves
        let halfmoves = halfmoves.map_or(0, |hm| hm.parse::<u8>().unwrap());
        self.set_halfmoves(halfmoves);

        // 6. fullmoves
        let fullmoves = fullmoves.map_or(1, |fm| fm.parse::<u16>().unwrap());
        self.set_fullmoves(fullmoves);

        self.refresh_zobrist();
    }

    /// Takes a sequence of moves in long algebraic notation and feeds them to
    /// the board. If a move is invalid or illegal, it panics.
    #[allow(clippy::unwrap_used)]
    pub fn play_moves(&mut self, moves_str: &str) {
        // UCI says it's ok to have no moves
        if moves_str.is_empty() {
            return;
        }

        let mut copy = *self;

        #[allow(clippy::string_slice)]
        for mv in moves_str.split(' ') {
            let mut moves = generate_moves::<{ MoveType::ALL }>(&copy);

            let start = Square::from_str(&mv[0..=1]).unwrap();
            let end = Square::from_str(&mv[2..=3]).unwrap();

            // Each move should be exactly 4 characters; if it's a promotion,
            // the last char will be the promotion char.
            let mv = if mv.len() == 5 {
                // SAFETY: It's not possible for it to be `None`.
                let promotion_char = unsafe { mv.chars().next_back().unwrap_unchecked() };
                moves.move_with_promo(start, end, PieceType::from(promotion_char))
            } else {
                moves.move_with(start, end)
            };

            assert!(copy.make_move(mv.unwrap()), "Illegal move");
        }

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

    /// Returns the board of the side according to `IS_WHITE`.
    pub fn side_any(&self, side: Side) -> Bitboard {
        index_unchecked!(self.sides, side.to_index())
    }

    /// Returns all the occupied squares on the board.
    pub fn occupancies(&self) -> Bitboard {
        self.side::<true>() | self.side::<false>()
    }

    /// Returns the side to move.
    pub const fn side_to_move(&self) -> Side {
        self.side_to_move
    }

    /// Returns the castling rights.
    const fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
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
        self.toggle_zobrist_ep_square(self.ep_square());
        // it's easiest just to unset them now and then re-set them later
        // rather than doing additional checks
        self.toggle_zobrist_castling_rights(self.castling_rights());

        self.move_piece(start, end, piece, piece_type, us);

        if captured_type != PieceType::NONE {
            self.update_bb_piece(end_bb, captured_type, them);
            self.remove_phase_piece(captured);
            self.remove_psq_piece(end, captured);
            self.toggle_zobrist_piece(end, piece);

            // check if we need to unset the castling rights if we're capturing
            // a rook
            if captured_type == PieceType::ROOK {
                match end {
                    Square::A1 => {
                        self.remove_castling_right(CastlingRights::Q);
                    }
                    Square::H1 => {
                        self.remove_castling_right(CastlingRights::K);
                    }
                    Square::A8 => {
                        self.remove_castling_right(CastlingRights::q);
                    }
                    Square::H8 => {
                        self.remove_castling_right(CastlingRights::k);
                    }
                    _ => (),
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

            self.remove_castling_rights(us);
        } else if is_double_pawn_push(start, end, piece) {
            let ep_square = Square((start.0 + end.0) >> 1);
            self.set_ep_square(ep_square);
            self.toggle_zobrist_ep_square(ep_square);
        } else if is_en_passant {
            let dest = Square(if us == Side::WHITE {
                end.0 - 8
            } else {
                end.0 + 8
            });
            let captured_pawn = Piece::from_piecetype(PieceType::PAWN, them);
            self.remove_piece(dest, captured_pawn, PieceType::PAWN, them);
            self.toggle_zobrist_piece(dest, captured_pawn);
        } else if is_promotion {
            let promotion_piece_type = mv.promotion_piece();
            let promotion_piece = Piece::from_piecetype(promotion_piece_type, us);

            // overwrite the pawn on the mailbox
            self.set_mailbox_piece(end, promotion_piece);

            // remove the pawn
            self.toggle_piece_bb(PieceType::PAWN, end_bb);
            self.remove_phase_piece(piece);
            self.remove_psq_piece(end, piece);
            self.toggle_zobrist_piece(end, piece);

            // add the promotion piece
            self.toggle_piece_bb(promotion_piece_type, end_bb);
            self.add_phase_piece(promotion_piece);
            self.add_psq_piece(end, promotion_piece);
            self.toggle_zobrist_piece(end, promotion_piece);
        }

        if self.is_square_attacked(self.king_square()) {
            return false;
        }

        if piece_type == PieceType::ROOK {
            match start {
                Square::A1 => {
                    self.remove_castling_right(CastlingRights::Q);
                }
                Square::H1 => {
                    self.remove_castling_right(CastlingRights::K);
                }
                Square::A8 => {
                    self.remove_castling_right(CastlingRights::q);
                }
                Square::H8 => {
                    self.remove_castling_right(CastlingRights::k);
                }
                _ => (),
            }
        }
        if piece_type == PieceType::KING {
            self.remove_castling_rights(us);
        }

        self.toggle_zobrist_castling_rights(self.castling_rights());
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
        self.clear_mailbox();
        self.clear_pieces();
        self.clear_sides();
        self.reset_side();
        self.clear_castling_rights();
        self.clear_ep_square();
        self.set_halfmoves(0);
        self.set_fullmoves(1);
        self.clear_accumulators();
        self.clear_zobrist();
    }

    /// Finds the piece on the given rank and file and converts it to its
    /// character representation. If no piece is on the square, returns '0'
    /// instead.
    fn char_piece_from_pos(&self, rank: Rank, file: File) -> char {
        let square = Square::from_pos(rank, file);
        let piece = self.piece_on(square);
        char::from(piece)
    }

    /// Moves `piece` from `start` to `end`, updating all relevant fields.
    ///
    /// `piece == Piece::from_piecetype(piece_type, side)`. Having the two
    /// redundant arguments is faster though.
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
        self.move_zobrist_piece(start, end, piece);
    }

    /// Returns the piece on `square`.
    fn piece_on(&self, square: Square) -> Piece {
        index_unchecked!(self.mailbox, square.to_index())
    }

    /// Adds a piece to square `square` for side `side`. Assumes there is no
    /// piece on the square to be written to.
    fn add_piece(&mut self, square: Square, piece: Piece) {
        let square_bb = Bitboard::from(square);
        let side = Side::from(piece);
        self.set_mailbox_piece(square, piece);
        self.toggle_piece_bb(PieceType::from(piece), square_bb);
        self.toggle_side_bb(side, square_bb);
        self.add_phase_piece(piece);
        self.add_psq_piece(square, piece);
        self.toggle_zobrist_piece(square, piece);
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
        self.remove_phase_piece(piece);
        self.remove_psq_piece(square, piece);
        self.toggle_zobrist_piece(square, piece);
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
        index_into_unchecked!(self.mailbox, square.to_index(), piece);
    }

    /// Sets the piece on `square` in the mailbox to [`Square::NONE`].
    fn unset_mailbox_piece(&mut self, square: Square) {
        index_into_unchecked!(self.mailbox, square.to_index(), Piece::NONE);
    }

    /// Clears the mailbox.
    fn clear_mailbox(&mut self) {
        for square in 0..(Square::TOTAL as u8) {
            self.unset_mailbox_piece(Square(square));
        }
    }

    /// Toggles the bits set in `bb` for the piece bitboard of `piece_type` and
    /// the side bitboard of `side`.
    fn update_bb_piece(&mut self, bb: Bitboard, piece_type: PieceType, side: Side) {
        self.toggle_piece_bb(piece_type, bb);
        self.toggle_side_bb(side, bb);
    }

    /// Toggles the bits set in `bb` of the bitboard of `piece`.
    fn toggle_piece_bb(&mut self, piece: PieceType, bb: Bitboard) {
        let old_bb = index_unchecked!(self.pieces, piece.to_index());
        index_into_unchecked!(self.pieces, piece.to_index(), old_bb ^ bb);
    }

    /// Clears the piece bitboards.
    fn clear_pieces(&mut self) {
        self.pieces = Self::no_pieces();
    }

    /// Toggles the bits set in `bb` of the bitboard of `side`.
    fn toggle_side_bb(&mut self, side: Side, bb: Bitboard) {
        let old_bb = index_unchecked!(self.sides, side.to_index());
        index_into_unchecked!(self.sides, side.to_index(), old_bb ^ bb);
    }

    /// Clears the side bitboards.
    fn clear_sides(&mut self) {
        self.sides = Self::no_sides();
    }

    /// Sets side to move to `side`.
    fn set_side_to_move(&mut self, side: Side) {
        self.side_to_move = side;
    }

    /// Flip the side to move.
    fn flip_side(&mut self) {
        self.toggle_zobrist_side();
        self.side_to_move = self.side_to_move.flip();
    }

    /// Sets the side to [`Side::NONE`].
    fn reset_side(&mut self) {
        self.side_to_move = Side::NONE;
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
    fn remove_castling_right(&mut self, right: CastlingRights) {
        self.castling_rights.remove_right(right);
    }

    /// Clears the castling rights for the given side.
    fn remove_castling_rights(&mut self, side: Side) {
        self.castling_rights.clear_side(side);
    }

    /// Clears all castling rights.
    fn clear_castling_rights(&mut self) {
        self.castling_rights = CastlingRights::none();
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

    /// Calculates the square the king is on.
    fn king_square(&self) -> Square {
        Square::from(
            self.piece::<{ PieceType::KING.to_index() }>() & self.side_any(self.side_to_move()),
        )
    }

    /// Tests if `square` is attacked by an enemy piece.
    fn is_square_attacked(&self, square: Square) -> bool {
        let occupancies = self.occupancies();
        let us = self.side_to_move();
        let them = us.flip();
        let them_bb = self.side_any(them);

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

    /// Removes the given right from `self`.
    fn remove_right(&mut self, right: Self) {
        debug_assert!(
            right.0.count_ones() == 1,
            "`right` contains multiple rights"
        );
        *self &= !right;
    }

    /// Clears the rights for `side`.
    fn clear_side(&mut self, side: Side) {
        if side == Side::WHITE {
            *self &= Self::k | Self::q;
        } else {
            *self &= Self::K | Self::Q;
        }
    }
}
