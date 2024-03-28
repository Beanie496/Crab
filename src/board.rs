use std::{
    fmt::{self, Display, Formatter},
    num::ParseIntError,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not, Shl},
    str::FromStr,
};

use crate::{
    bitboard::Bitboard,
    defs::{File, Piece, PieceType, Rank, Side, Square},
    error::ParseError,
    evaluation::Score,
    index_into_unchecked, index_unchecked,
    movegen::{Move, LOOKUPS},
    util::is_double_pawn_push,
};

/// Accumulated, incrementally-updated fields.
mod accumulators;

/// The type of a zobrist key.
pub type Key = u64;

/// All the errors that can occur while parsing a `position` command into a
/// [`Board`].
#[derive(Debug)]
pub enum BoardParseError {
    /// A generic parsing error occured.
    ParseError(ParseError),
    /// An integer couldn't be parsed.
    ParseIntError(ParseIntError),
}

/// The board. It contains information about the current board state and can
/// generate pseudo-legal moves. It uses copy-make.
#[derive(Clone, Copy)]
pub struct Board {
    /// An array of piece values, used for quick lookup of which piece is on a
    /// given square.
    mailbox: [Piece; Square::TOTAL],
    /// `pieces[0]` is the intersection of all pawns on the board, `pieces[1]`
    /// is the knights, and so on, as according to the order set by [`Piece`].
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
    /// `psq` uses this value to lerp between its midgame and endgame values.
    /// It is incrementally updated.
    phase: u8,
    /// The current material balance weighted with piece-square tables, from
    /// the perspective of White.
    ///
    /// It is incrementally updated.
    psq: Score,
    /// The current zobrist key of the board.
    ///
    /// It is incrementally updated.
    zobrist: Key,
}

/// Stores castling rights.
///
/// Encoded as `KQkq`, with one bit for each right.  E.g. `0b1101` would be
/// castling rights `KQq`.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CastlingRights(u8);

#[allow(non_upper_case_globals)]
impl CastlingRights {
    /// The flag `K`.
    const K: Self = Self(0b1000);
    /// The flag `Q`.
    const Q: Self = Self(0b0100);
    /// The flag `k`.
    const k: Self = Self(0b0010);
    /// The flag `q`.
    const q: Self = Self(0b0001);
    /// The flags `KQkq`, i.e. all flags.
    const KQkq: Self = Self(0b1111);
    /// No flags.
    const NONE: Self = Self(0b0000);
}

impl Default for Board {
    /// Returns a [`Board`] with the starting position.
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
            phase: 0,
            psq: Score(0, 0),
            zobrist: Self::new_zobrist(),
        };
        board.refresh_accumulators();
        board
    }
}

impl Display for Board {
    /// Converts the board into a FEN string.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut board = String::new();
        let mut empty_squares = 0;
        // I can't just iterate over the mailbox normally: the board goes from
        // a1 to h8, rank-file, whereas the FEN goes from a8 to h1, also
        // rank-file
        for rank in (0..Rank::TOTAL).rev() {
            for file in 0..File::TOTAL {
                let square = Square::from_pos(Rank(rank as u8), File(file as u8));
                let piece = self.piece_on(square);

                if piece == Piece::NONE {
                    empty_squares += 1;
                } else {
                    if empty_squares != 0 {
                        // `from_digit`, unwrapping then mapping is a lot more
                        // verbose and does pointless error checking
                        board.push(char::from(b'0' + empty_squares));
                        empty_squares = 0;
                    }
                    board.push(char::from(piece));
                }
            }
            if empty_squares != 0 {
                board.push(char::from(b'0' + empty_squares));
                empty_squares = 0;
            }
            board.push('/');
        }
        // remove the trailing slash
        board.pop();

        let side_to_move = self.side_to_move();
        let rights = self.castling_rights();
        let ep_square = self.ep_square();
        let halfmoves = self.halfmoves();
        let fullmoves = self.fullmoves();

        write!(
            f,
            "{} {} {rights} {ep_square} {halfmoves} {fullmoves}",
            &board,
            char::from(side_to_move),
        )
    }
}

impl FromStr for Board {
    type Err = BoardParseError;

    /// Parses a full `position` command.
    ///
    /// It will return with an [`Err`] if the FEN string cannot be parsed (e.g.
    /// if it's too short) but does not check if the FEN string actually makes
    /// sense (e.g. if it contains a row with 14 pieces).
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut board = Self::new();
        let mut tokens = string.split_whitespace();

        let board_str = tokens.next().ok_or(ParseError::ExpectedToken)?;
        let side_to_move = tokens.next().ok_or(ParseError::ExpectedToken)?;
        let castling_rights = tokens.next().ok_or(ParseError::ExpectedToken)?;
        let ep_square = tokens.next().ok_or(ParseError::ExpectedToken)?;
        let halfmoves = tokens.next().ok_or(ParseError::ExpectedToken)?;
        let fullmoves = tokens.next().ok_or(ParseError::ExpectedToken)?;

        // 1. the board itself
        let mut square = 56;
        let ranks = board_str.split('/');
        for rank in ranks {
            for piece in rank.chars() {
                // if it's a number, skip over that many files
                if ('0'..='8').contains(&piece) {
                    let empty_squares = piece as u8 - b'0';
                    square += empty_squares;
                } else {
                    board.add_piece(Square(square), Piece::try_from(piece)?);
                    square += 1;
                }
            }
            square = square.wrapping_sub(16);
        }

        // 2. side to move
        if side_to_move == "w" {
            board.set_side_to_move(Side::WHITE);
        } else {
            board.set_side_to_move(Side::BLACK);
            board.toggle_zobrist_side();
        }

        // 3. castling rights
        for right in castling_rights.chars() {
            match right {
                'K' => board.castling_rights_mut().add_right(CastlingRights::K),
                'Q' => board.castling_rights_mut().add_right(CastlingRights::Q),
                'k' => board.castling_rights_mut().add_right(CastlingRights::k),
                'q' => board.castling_rights_mut().add_right(CastlingRights::q),
                _ => (),
            }
        }
        board.toggle_zobrist_castling_rights(board.castling_rights());

        // 4. en passant
        let ep_square = ep_square.parse::<Square>()?;
        board.set_ep_square(ep_square);
        board.toggle_zobrist_ep_square(ep_square);

        // 5. halfmoves
        let halfmoves = halfmoves.parse::<u8>()?;
        board.set_halfmoves(halfmoves);

        // 6. fullmoves
        let fullmoves = fullmoves.parse::<u16>()?;
        board.set_fullmoves(fullmoves);

        Ok(board)
    }
}

impl From<ParseError> for BoardParseError {
    fn from(parse_error: ParseError) -> Self {
        Self::ParseError(parse_error)
    }
}

impl From<ParseIntError> for BoardParseError {
    fn from(parse_int_error: ParseIntError) -> Self {
        Self::ParseIntError(parse_int_error)
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

/// Converts the current castling rights into their string representation.
///
/// E.g. `KQq` if the White king can castle both ways and the Black king
/// can only castle queenside.
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

impl Board {
    /// Creates a new, empty [`Board`].
    pub const fn new() -> Self {
        Self {
            mailbox: Self::no_mailbox(),
            pieces: Self::no_pieces(),
            sides: Self::no_sides(),
            side_to_move: Side::NONE,
            castling_rights: CastlingRights::none(),
            ep_square: Square::NONE,
            halfmoves: 0,
            fullmoves: 1,
            phase: 0,
            psq: Score(0, 0),
            zobrist: Self::new_zobrist(),
        }
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

    /// Returns an empty mailbox.
    const fn no_mailbox() -> [Piece; Square::TOTAL] {
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
        println!("Zobrist key: {}", self.zobrist());
    }

    /// Returns the piece bitboard given by `PIECE`.
    pub const fn piece<const PIECE: usize>(&self) -> Bitboard {
        self.pieces[PIECE]
    }

    /// Returns the side bitboard according to `IS_WHITE`.
    pub const fn side<const IS_WHITE: bool>(&self) -> Bitboard {
        if IS_WHITE {
            self.sides[Side::WHITE.to_index()]
        } else {
            self.sides[Side::BLACK.to_index()]
        }
    }

    /// Returns the side bitboard of the given side.
    pub fn side_any(&self, side: Side) -> Bitboard {
        index_unchecked!(self.sides, side.to_index())
    }

    /// Calculates the bitboard with all occupancies set.
    pub fn occupancies(&self) -> Bitboard {
        self.side::<true>() | self.side::<false>()
    }

    /// Returns the side to move.
    pub const fn side_to_move(&self) -> Side {
        self.side_to_move
    }

    /// Returns the castling rights.
    pub const fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
    }

    /// Returns a mutable reference to the castling rights.
    pub fn castling_rights_mut(&mut self) -> &mut CastlingRights {
        &mut self.castling_rights
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

    /// Sets the board to the starting position.
    pub fn set_startpos(&mut self) {
        *self = Self::default();
    }

    /// Makes the given move on the internal board. `mv` is assumed to be a
    /// valid move. Returns `true` if the given move is legal and `false`
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
        }

        self.toggle_zobrist_ep_square(self.ep_square());
        // it's easiest just to unset them now and then re-set them later
        // rather than doing additional checks
        self.toggle_zobrist_castling_rights(self.castling_rights());
        self.clear_ep_square();

        self.move_piece(start, end, piece, piece_type, us);

        if captured_type != PieceType::NONE {
            self.update_bb_piece(end_bb, captured_type, them);
            self.remove_accumulated_piece(end, captured);

            // check if we need to unset the castling rights if we're capturing
            // a rook
            if captured_type == PieceType::ROOK {
                match end {
                    Square::A1 => {
                        self.castling_rights_mut().remove_right(CastlingRights::Q);
                    }
                    Square::H1 => {
                        self.castling_rights_mut().remove_right(CastlingRights::K);
                    }
                    Square::A8 => {
                        self.castling_rights_mut().remove_right(CastlingRights::q);
                    }
                    Square::H8 => {
                        self.castling_rights_mut().remove_right(CastlingRights::k);
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
                Piece::from_piecetype(PieceType::ROOK, us),
                PieceType::ROOK,
                us,
            );

            self.castling_rights_mut().clear_side(us);
        } else if is_double_pawn_push(start, end, piece_type) {
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
        } else if is_promotion {
            let promotion_piece_type = mv.promotion_piece();
            let promotion_piece = Piece::from_piecetype(promotion_piece_type, us);

            // overwrite the pawn on the mailbox
            self.set_mailbox_piece(end, promotion_piece);

            // remove the pawn
            self.toggle_piece_bb(PieceType::PAWN, end_bb);
            self.remove_accumulated_piece(end, piece);

            // add the promotion piece
            self.toggle_piece_bb(promotion_piece_type, end_bb);
            self.add_accumulated_piece(end, promotion_piece);
        }

        if self.is_square_attacked(self.king_square()) {
            return false;
        }

        if piece_type == PieceType::ROOK {
            match start {
                Square::A1 => {
                    self.castling_rights_mut().remove_right(CastlingRights::Q);
                }
                Square::H1 => {
                    self.castling_rights_mut().remove_right(CastlingRights::K);
                }
                Square::A8 => {
                    self.castling_rights_mut().remove_right(CastlingRights::q);
                }
                Square::H8 => {
                    self.castling_rights_mut().remove_right(CastlingRights::k);
                }
                _ => (),
            }
        }
        if piece_type == PieceType::KING {
            self.castling_rights_mut().clear_side(us);
        }

        self.toggle_zobrist_castling_rights(self.castling_rights());
        self.flip_side();

        true
    }

    /// Finds the piece on the given rank and file and converts it to its
    /// character representation.
    ///
    /// If no piece is on the square, returns '0' instead.
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
        self.move_accumulated_piece(start, end, piece);
    }

    /// Returns the piece on `square`.
    fn piece_on(&self, square: Square) -> Piece {
        index_unchecked!(self.mailbox, square.to_index())
    }

    /// Adds a piece to square `square` for side `side`.
    ///
    /// Assumes there is no piece on the square to be written to.
    fn add_piece(&mut self, square: Square, piece: Piece) {
        let square_bb = Bitboard::from(square);
        let side = Side::from(piece);
        self.set_mailbox_piece(square, piece);
        self.toggle_piece_bb(PieceType::from(piece), square_bb);
        self.toggle_side_bb(side, square_bb);
        self.add_accumulated_piece(square, piece);
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
        self.remove_accumulated_piece(square, piece);
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

    /// Toggles the bits set in `bb` of the bitboard of `side`.
    fn toggle_side_bb(&mut self, side: Side, bb: Bitboard) {
        let old_bb = index_unchecked!(self.sides, side.to_index());
        index_into_unchecked!(self.sides, side.to_index(), old_bb ^ bb);
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

        let pawn_attacks = LOOKUPS.pawn_attacks(us, square);
        let knight_attacks = LOOKUPS.knight_attacks(square);
        let diagonal_attacks = LOOKUPS.bishop_attacks(square, occupancies);
        let orthogonal_attacks = LOOKUPS.rook_attacks(square, occupancies);
        let king_attacks = LOOKUPS.king_attacks(square);

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
    pub fn can_castle_kingside<const IS_WHITE: bool>(self) -> bool {
        if IS_WHITE {
            self & Self::K == Self::K
        } else {
            self & Self::k == Self::k
        }
    }

    /// Calculates if the given side can castle queenside.
    pub fn can_castle_queenside<const IS_WHITE: bool>(self) -> bool {
        if IS_WHITE {
            self & Self::Q == Self::Q
        } else {
            self & Self::q == Self::q
        }
    }

    /// Adds the given right to the castling rights.
    fn add_right(&mut self, right: Self) {
        *self |= right;
    }

    /// Removes the given right from the castling rights.
    fn remove_right(&mut self, right: Self) {
        debug_assert!(
            right.0.count_ones() == 1,
            "`right` contains multiple rights"
        );
        *self &= !right;
    }

    /// Clears the rights of the given side.
    fn clear_side(&mut self, side: Side) {
        if side == Side::WHITE {
            *self &= Self::k | Self::q;
        } else {
            *self &= Self::K | Self::Q;
        }
    }
}
