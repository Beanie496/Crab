/*
 * Crab, a UCI-compatible chess engine
 * Copyright (C) 2024 Jasper Shovelton
 *
 * Crab is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Crab is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Crab. If not, see <https://www.gnu.org/licenses/>.
 */

use std::{
    fmt::{self, Display, Formatter},
    hint::unreachable_unchecked,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not, Shl},
    str::FromStr,
};

use crate::{
    bitboard::Bitboard,
    defs::{Direction, File, Piece, PieceType, Rank, Side, Square},
    error::ParseError,
    evaluation::{evaluate, Phase, Score},
    lookups::{ray_between, ATTACK_LOOKUPS},
    movegen::Move,
    util::{get_unchecked, insert_unchecked, is_double_pawn_push},
};

/// Accumulated, incrementally-updated fields.
mod accumulators;

/// The type of a zobrist key.
pub type Key = u64;

/// A chessboard.
///
/// It contains all the necessary information about a chess position, plus some
/// extra accumulators for rapid lookup. It uses copy-make to make moves.
#[derive(Clone, Copy)]
pub struct Board {
    /// An array of pieces, used for quick lookup of which piece is on a given
    /// square.
    mailbox: [Piece; Square::TOTAL],
    /// An array of all six piece types, where one bitboard represents the
    /// locations of all pieces of that type.
    ///
    /// Index `PieceType::PAWN.to_index()` is all the pawns, etc.
    pieces: [Bitboard; PieceType::TOTAL],
    /// An array of both sides, where one bitboard represents the locations of
    /// all pieces belonging to that side.
    ///
    /// Index `Side::WHITE.to_index()` is all White pieces, etc.
    sides: [Bitboard; Side::TOTAL],
    /// The current side to move.
    side_to_move: Side,
    /// Castling rights.
    castling_rights: CastlingRights,
    /// The en passant square.
    ///
    /// Is [`Square::NONE`] if there is no ep square.
    ep_square: Square,
    /// The number of halfmoves since the last capture or pawn move.
    halfmoves: u8,
    /// Which move number the current move is. Starts at 1 and is incremented
    /// when Black moves.
    fullmoves: u16,
    /// The current phase.
    ///
    /// It is incrementally updated.
    phase: Phase,
    /// The current score from the perspective of White.
    ///
    /// It is incrementally updated.
    score: Score,
    /// The current zobrist key of the board.
    ///
    /// It is incrementally updated.
    key: Key,
    /// The zobrist key of the pawns only.
    ///
    /// It is incrementally updated.
    pawn_key: Key,
}

/// Castling rights.
///
/// Encoded as `KQkq`, with one bit for each right.  E.g. `0b1101` would be
/// castling rights `KQq`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Copy, PartialEq)]
pub struct CastlingRights(u8);

/// The FEN string of the starting position.
pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

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
    /// No flags.
    const NONE: Self = Self(0b0000);
}

impl Default for Board {
    /// Returns a [`Board`] with the starting position.
    fn default() -> Self {
        // SAFETY: `STARTPOS` is hardcoded, therefore it will always parse
        // correctly
        unsafe { STARTPOS.parse().unwrap_unchecked() }
    }
}

impl Display for Board {
    /// Converts the board into a FEN string.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut board = String::with_capacity(128);
        let mut empty_squares = 0;

        let mut square = 56;
        for _ in 0..Rank::TOTAL {
            for _ in 0..File::TOTAL {
                let piece = self.piece_on(Square(square));

                if piece == Piece::NONE {
                    empty_squares += 1;
                } else {
                    if empty_squares != 0 {
                        // SAFETY: `empty_squares` is less than 10
                        let empty_squares_char =
                            unsafe { char::from_digit(empty_squares, 10).unwrap_unchecked() };
                        board.push(empty_squares_char);
                        empty_squares = 0;
                    }
                    board.push(char::from(piece));
                }
                square += 1;
            }
            if empty_squares != 0 {
                // SAFETY: `empty_squares` is less than 10
                let empty_squares_char =
                    unsafe { char::from_digit(empty_squares, 10).unwrap_unchecked() };
                board.push(empty_squares_char);
                empty_squares = 0;
            }

            board.push('/');
            square = square.wrapping_sub(16);
        }
        // remove the trailing slash
        board.pop();

        let side_to_move = char::from(self.side_to_move());
        let rights = self.castling_rights();
        let ep_square = self.ep_square();
        let halfmoves = self.halfmoves();
        let fullmoves = self.fullmoves();

        write!(
            f,
            "{board} {side_to_move} {rights} {ep_square} {halfmoves} {fullmoves}",
        )
    }
}

impl FromStr for Board {
    type Err = ParseError;

    /// Parses a full `position` command.
    ///
    /// It will return with an [`Err`] if the FEN string cannot be parsed (e.g.
    /// if it's too short) but does not check if the FEN string actually makes
    /// sense (e.g. if it contains a row with 14 pieces). The castling rights,
    /// ep square, halfmoves and fullmoves can be omitted but the board itself
    /// and side must be present.
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut board = Self::new();
        let mut tokens = string.split_whitespace();

        let board_str = tokens.next().ok_or(ParseError)?;
        let side_to_move = tokens.next().ok_or(ParseError)?;
        let castling_rights = tokens.next().unwrap_or("-");
        let ep_square = tokens.next().unwrap_or("-");
        let halfmoves = tokens.next().unwrap_or("0");
        let fullmoves = tokens.next().unwrap_or("1");

        // 1. the board itself
        let mut square = 56;
        let ranks = board_str.split('/');
        for rank in ranks {
            for piece in rank.chars() {
                // if it's a number, skip over that many files
                if ('0'..='8').contains(&piece) {
                    // SAFETY: we just checked `piece` is in the valid range
                    square += unsafe { piece.to_digit(10).unwrap_unchecked() as u8 };
                } else {
                    board.add_piece(Square(square), Piece::try_from(piece)?);
                    square += 1;
                }
            }
            square = square.wrapping_sub(16);
        }

        // 2. side to move
        let side_to_move = side_to_move.parse()?;
        board.set_side_to_move(side_to_move);

        // 3. castling rights
        for right in castling_rights.chars() {
            match right {
                'K' => board.add_castling_rights(CastlingRights::K),
                'Q' => board.add_castling_rights(CastlingRights::Q),
                'k' => board.add_castling_rights(CastlingRights::k),
                'q' => board.add_castling_rights(CastlingRights::q),
                _ => (),
            }
        }

        // 4. en passant
        let ep_square = ep_square.parse::<Square>()?;
        board.set_ep_square(ep_square);

        // 5. halfmoves
        let halfmoves = halfmoves.parse::<u8>()?;
        board.set_halfmoves(halfmoves);

        // 6. fullmoves
        let fullmoves = fullmoves.parse::<u16>()?;
        board.set_fullmoves(fullmoves);

        Ok(board)
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
    /// Converts the current castling rights into their string representation.
    ///
    /// E.g. `KQq` if the White king can castle both ways and the Black king
    /// can only castle queenside.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if *self == Self::NONE {
            return f.write_str("-");
        }

        if self.can_castle_kingside::<true>() {
            f.write_str("K")?;
        }
        if self.can_castle_queenside::<true>() {
            f.write_str("Q")?;
        }
        if self.can_castle_kingside::<false>() {
            f.write_str("k")?;
        }
        if self.can_castle_queenside::<false>() {
            f.write_str("q")?;
        }
        Ok(())
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
    pub fn new() -> Self {
        Self {
            mailbox: [Piece::NONE; Square::TOTAL],
            pieces: [Bitboard::empty(); PieceType::TOTAL],
            sides: [Bitboard::empty(); Side::TOTAL],
            side_to_move: Side::NONE,
            castling_rights: CastlingRights::new(),
            ep_square: Square::NONE,
            halfmoves: 0,
            fullmoves: 1,
            phase: Phase::default(),
            score: Score::default(),
            key: 0,
            pawn_key: 0,
        }
    }

    /// Pretty-prints the current state of the board.
    pub fn pretty_print(&self) {
        for rank in (0..Rank::TOTAL as u8).rev() {
            print!("{} | ", rank + 1);
            for file in 0..File::TOTAL as u8 {
                let square = Square::from_pos(Rank(rank), File(file));
                let piece = self.piece_on(square);
                print!("{} ", char::from(piece));
            }
            println!();
        }
        println!("    ---------------");
        println!("    a b c d e f g h");
        println!();
        println!("FEN: {self}");
        println!("Zobrist key: {}", self.key());
        println!("Pawn zobrist key: {}", self.pawn_key());
        println!("Static evaluation: {}", evaluate(self));
    }

    /// Returns the piece bitboard given by `PIECE`.
    pub const fn piece<const PIECE: usize>(&self) -> Bitboard {
        self.pieces[PIECE]
    }

    /// Returns the piece bitboard of the given piece type.
    pub fn piece_any(&self, piece_type: PieceType) -> Bitboard {
        *get_unchecked(&self.pieces, piece_type.to_index())
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
        *get_unchecked(&self.sides, side.to_index())
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

        self.increment_halfmoves();
        if us == Side::BLACK {
            self.increment_fullmoves();
        }

        if piece_type == PieceType::PAWN || captured_type != PieceType::NONE {
            self.reset_halfmoves();
        }

        // it's easiest just to unset them now and then re-set them later
        // rather than doing additional checks
        self.toggle_castling_rights_key(self.castling_rights());
        self.clear_ep_square();

        self.move_piece(start, end, piece, piece_type, us);

        if captured_type != PieceType::NONE {
            self.update_piece_bb(end_bb, captured_type, them);
            self.remove_accumulated_piece(end, captured);

            // check if we need to unset the castling rights if we're capturing
            // a rook
            if captured_type == PieceType::ROOK {
                match end {
                    Square::A1 => {
                        self.remove_castling_rights(CastlingRights::Q);
                    }
                    Square::H1 => {
                        self.remove_castling_rights(CastlingRights::K);
                    }
                    Square::A8 => {
                        self.remove_castling_rights(CastlingRights::q);
                    }
                    Square::H8 => {
                        self.remove_castling_rights(CastlingRights::k);
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
        } else if is_double_pawn_push(start, end, piece_type) {
            let ep_square = Square((start.0 + end.0) >> 1);
            self.set_ep_square(ep_square);
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

        if self.is_in_check() {
            return false;
        }

        if piece_type == PieceType::ROOK {
            match start {
                Square::A1 => {
                    self.remove_castling_rights(CastlingRights::Q);
                }
                Square::H1 => {
                    self.remove_castling_rights(CastlingRights::K);
                }
                Square::A8 => {
                    self.remove_castling_rights(CastlingRights::q);
                }
                Square::H8 => {
                    self.remove_castling_rights(CastlingRights::k);
                }
                _ => (),
            }
        }
        if piece_type == PieceType::KING {
            let removed_rights = if us == Side::WHITE {
                CastlingRights::K | CastlingRights::Q
            } else {
                CastlingRights::k | CastlingRights::q
            };
            self.remove_castling_rights(removed_rights);
        }

        self.toggle_castling_rights_key(self.castling_rights());
        self.flip_side();

        true
    }

    /// Makes a null move.
    ///
    /// This flips the side to move, clears the en passant square and does
    /// nothing else.
    pub fn make_null_move(&mut self) {
        self.clear_ep_square();
        self.flip_side();
    }

    /// Moves `piece` from `start` to `end`, updating all relevant fields.
    ///
    /// `piece == Piece::from_piecetype(piece_type, side)`. Having the two
    /// redundant arguments is faster though.
    pub fn move_piece(
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
        self.update_piece_bb(bb, piece_type, side);
        self.move_accumulated_piece(start, end, piece);
    }

    /// Returns the piece on `square`.
    pub fn piece_on(&self, square: Square) -> Piece {
        *get_unchecked(&self.mailbox, square.to_index())
    }

    /// Adds a piece to square `square` for side `side`.
    ///
    /// Assumes there is no piece on the square to be written to.
    pub fn add_piece(&mut self, square: Square, piece: Piece) {
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
        insert_unchecked(&mut self.mailbox, square.to_index(), piece);
    }

    /// Sets the piece on `square` in the mailbox to [`Square::NONE`].
    fn unset_mailbox_piece(&mut self, square: Square) {
        insert_unchecked(&mut self.mailbox, square.to_index(), Piece::NONE);
    }

    /// Toggles the bits set in `bb` for the piece bitboard of `piece_type` and
    /// the side bitboard of `side`.
    fn update_piece_bb(&mut self, bb: Bitboard, piece_type: PieceType, side: Side) {
        self.toggle_piece_bb(piece_type, bb);
        self.toggle_side_bb(side, bb);
    }

    /// Toggles the bits set in `bb` of the bitboard of `piece`.
    fn toggle_piece_bb(&mut self, piece: PieceType, bb: Bitboard) {
        let old_bb = *get_unchecked(&self.pieces, piece.to_index());
        insert_unchecked(&mut self.pieces, piece.to_index(), old_bb ^ bb);
    }

    /// Toggles the bits set in `bb` of the bitboard of `side`.
    fn toggle_side_bb(&mut self, side: Side, bb: Bitboard) {
        let old_bb = *get_unchecked(&self.sides, side.to_index());
        insert_unchecked(&mut self.sides, side.to_index(), old_bb ^ bb);
    }

    /// Sets side to move to `side`.
    fn set_side_to_move(&mut self, side: Side) {
        if side == Side::BLACK {
            self.toggle_side_key();
        }
        self.side_to_move = side;
    }

    /// Flip the side to move.
    fn flip_side(&mut self) {
        self.toggle_side_key();
        self.side_to_move = self.side_to_move.flip();
    }

    /// Sets the en passant square to `square`.
    fn set_ep_square(&mut self, square: Square) {
        self.toggle_ep_square_key(square);
        self.ep_square = square;
    }

    /// Sets the en passant square to [`Square::NONE`].
    fn clear_ep_square(&mut self) {
        self.toggle_ep_square_key(self.ep_square());
        self.ep_square = Square::NONE;
    }

    /// Adds the given castling rights, assuming none of the rights already
    /// exist, and updates the zobrist key.
    ///
    /// Will panic in debug if any of the rights already exist.
    fn add_castling_rights(&mut self, rights: CastlingRights) {
        debug_assert!(
            self.castling_rights() & rights == CastlingRights::NONE,
            "Adding rights that already exist"
        );
        self.castling_rights.add_rights(rights);
        self.toggle_castling_rights_key(rights);
    }

    /// Removes all of the given rights, whether or not they already exist.
    /// Does not update the zobrist key.
    fn remove_castling_rights(&mut self, rights: CastlingRights) {
        self.castling_rights.remove_rights(rights);
    }

    /// Increments the halfmove counter.
    fn increment_halfmoves(&mut self) {
        self.halfmoves += 1;
    }

    /// Sets halfmoves.
    fn set_halfmoves(&mut self, count: u8) {
        self.halfmoves = count;
    }

    /// Zeroes the halfmove counter.
    fn reset_halfmoves(&mut self) {
        self.halfmoves = 0;
    }

    /// Increments the fullmove counter.
    fn increment_fullmoves(&mut self) {
        self.fullmoves += 1;
    }

    /// Sets fullmoves.
    fn set_fullmoves(&mut self, count: u16) {
        self.fullmoves = count;
    }

    /// Tests if the king is in check.
    pub fn is_in_check(&self) -> bool {
        self.is_square_attacked(self.king_square())
    }

    /// Calculates the square the king is on.
    fn king_square(&self) -> Square {
        Square::from(
            self.piece::<{ PieceType::KING.to_index() }>() & self.side_any(self.side_to_move()),
        )
    }

    /// Checks if there are pieces on the board that aren't pawns (or kings).
    #[rustfmt::skip]
    pub fn has_non_pawn_pieces(&self) -> bool {
        self.piece::<{ PieceType::KNIGHT.to_index() }>().count_ones()
            + self.piece::<{ PieceType::BISHOP.to_index() }>().count_ones()
            + self.piece::<{ PieceType::ROOK.to_index() }>().count_ones()
            + self.piece::<{ PieceType::QUEEN.to_index() }>().count_ones()
            > 0
    }

    /// Returns all the attackers from the given side to move to the given
    /// square.
    fn square_attackers(&self, side_to_move: Side, square: Square) -> Bitboard {
        let occupancies = self.occupancies();

        let pawn_attacks = ATTACK_LOOKUPS.pawn_attacks(side_to_move, square);
        let knight_attacks = ATTACK_LOOKUPS.knight_attacks(square);
        let diagonal_attacks = ATTACK_LOOKUPS.bishop_attacks(square, occupancies);
        let orthogonal_attacks = ATTACK_LOOKUPS.rook_attacks(square, occupancies);
        let king_attacks = ATTACK_LOOKUPS.king_attacks(square);

        let pawns = self.piece::<{ PieceType::PAWN.to_index() }>();
        let knights = self.piece::<{ PieceType::KNIGHT.to_index() }>();
        let bishops = self.piece::<{ PieceType::BISHOP.to_index() }>();
        let rooks = self.piece::<{ PieceType::ROOK.to_index() }>();
        let queens = self.piece::<{ PieceType::QUEEN.to_index() }>();
        let kings = self.piece::<{ PieceType::KING.to_index() }>();

        pawn_attacks & pawns
            | knight_attacks & knights
            | king_attacks & kings
            | diagonal_attacks & (bishops | queens)
            | orthogonal_attacks & (rooks | queens)
    }

    /// Checks if the given move is quiet.
    ///
    /// This means it's not capturing any piece (this includes en passant) and
    /// it's not a queen promotion.
    pub fn is_quiet(&self, mv: Move) -> bool {
        self.piece_on(mv.end()) == Piece::NONE
            && !mv.is_en_passant()
            && !(mv.is_promotion() && mv.promotion_piece() == PieceType::QUEEN)
    }

    /// Tests if `square` is attacked by an enemy piece.
    fn is_square_attacked(&self, square: Square) -> bool {
        let us = self.side_to_move();
        let them = us.flip();
        let them_bb = self.side_any(them);

        !(self.square_attackers(us, square) & them_bb).is_empty()
    }

    /// Performs Static Exchange Evaluation (SEE) on the destination square of
    /// the given move. Returns whether or not the resulting exchange is a net
    /// material win.
    ///
    /// If the move isn't capturing anything, it will return `true` even if
    /// would be a losing exchange.
    pub fn is_winning_exchange(&self, mv: Move) -> bool {
        let origin = mv.start();
        let target = mv.end();
        let mut us = self.side_to_move();

        let mut see_value = if mv.is_en_passant() {
            PieceType::PAWN
        } else {
            let captured_piece = PieceType::from(self.piece_on(target));
            if captured_piece == PieceType::NONE {
                return true;
            }
            captured_piece
        }
        .see_bonus();

        if mv.is_promotion() {
            // swap the pawn vaue with the promotion piece value
            see_value += mv.promotion_piece().see_bonus() - PieceType::PAWN.see_bonus();
        }

        let mut attacker_type = if mv.is_promotion() {
            mv.promotion_piece()
        } else {
            PieceType::from(self.piece_on(origin))
        };
        let mut attacker = Bitboard::empty();

        see_value -= attacker_type.see_bonus();
        // if we're up material even if they recapture
        if see_value >= 0 {
            return true;
        }

        // NOTE: the other engines I looked at `|` this with the bitboard of
        // the target square (and then check if the move was an en passant to
        // xor it). I can't see why this would make a difference (and it makes
        // no difference to my bench) so I don't do it.
        let mut occupancies = self.occupancies() ^ Bitboard::from(origin);
        let mut attackers = self.square_attackers(us, target) & occupancies;
        let diagonal_attackers = self.piece::<{ PieceType::BISHOP.to_index() }>()
            | self.piece::<{ PieceType::QUEEN.to_index() }>();
        let orthogonal_attackers = self.piece::<{ PieceType::ROOK.to_index() }>()
            | self.piece::<{ PieceType::QUEEN.to_index() }>();

        us = us.flip();

        // recapturing make `see_value` positive (for the time being), so find
        // the cheapest piece to recapture with
        loop {
            let our_attackers = attackers & self.side_any(us);
            // if we don't have any pieces to recapture with
            if our_attackers.is_empty() {
                break;
            }

            for piece_type in 0..PieceType::TOTAL as u8 {
                attacker_type = PieceType(piece_type);
                attacker = self.piece_any(attacker_type) & our_attackers;
                if !attacker.is_empty() {
                    break;
                }
            }

            let next_attacker = attacker.pop_lsb();
            occupancies ^= next_attacker;

            // if the attacker moves diagonally (pawn, bishop or queen), it can
            // reveal diagonal sliders behind it
            if attacker_type.0 & 1 == 0 {
                attackers |=
                    ATTACK_LOOKUPS.bishop_attacks(target, occupancies) & diagonal_attackers;
            }
            // if the attacker moves orthogonally (rook or queen), it can
            // reveal orthogonal sliders behind it. The condition does include
            // kings, but most of the time the king isn't involved, making the
            // comparision a net speedup over checking the rook and queen
            // separately.
            if attacker_type.0 >= PieceType::ROOK.0 {
                attackers |=
                    ATTACK_LOOKUPS.rook_attacks(target, occupancies) & orthogonal_attackers;
            }
            attackers &= occupancies;

            us = us.flip();
            see_value += attacker_type.see_bonus();

            // if we're down material even if we recapture
            if see_value < 0 {
                // Idea from Ethereal: if the last attacker was a king and the
                // we side still have attackers remaining, we automatically win
                // becuase their move was illegal
                if attacker_type == PieceType::KING && !(attackers & self.side_any(us)).is_empty() {
                    us = us.flip();
                }
                break;
            }

            // it's important to do `- 1` because 0 wouldn't be properly
            // negated
            see_value = -see_value - 1;
        }

        // return whether or not we're not the loser
        self.side_to_move() != us
    }

    /// Checks if `mv` is a pseudolegal move on `self`.
    // implementation, for the most part, yoinked from viridithas
    pub fn is_pseudolegal(&self, mv: Move) -> bool {
        let start = mv.start();
        let end = mv.end();
        let is_promotion = mv.is_promotion();
        let is_castling = mv.is_castling();
        let is_en_passant = mv.is_en_passant();

        let piece = self.piece_on(start);
        let piece_type = PieceType::from(piece);
        // this might be wrong so it needs to be checked before it's used
        let piece_side = Side::from(piece);
        let captured = self.piece_on(end);
        // this also might be wrong
        let captured_side = Side::from(captured);
        let occupancies = self.occupancies();
        let us = self.side_to_move();

        // the piece exists and hasn't been captured
        if piece == Piece::NONE || piece_side != us {
            return false;
        }

        // we aren't capturing a friendly piece
        if captured != Piece::NONE && captured_side == us {
            return false;
        }

        // we aren't moving a non-pawn as a pawn
        if piece_type != PieceType::PAWN && (is_en_passant || is_promotion) {
            return false;
        }

        if is_castling {
            let is_kingside = File::from(end) >= File::FILE5;

            // we're castling with the king
            if piece_type != PieceType::KING {
                return false;
            }

            // we're allowed to castle (this includes if the rook still exists
            // and hasn't moved)
            if !self
                .castling_rights()
                .can_castle_any(piece_side == Side::WHITE, is_kingside)
            {
                return false;
            }

            // there is space to castle
            if !Bitboard::is_clear_to_castle(occupancies, piece_side == Side::WHITE, is_kingside) {
                return false;
            }

            // checked in `make_move()`: castling out of check, castling
            // through check and castling into check
            return true;
        }

        if piece_type == PieceType::PAWN {
            let end_bb = Bitboard::from(end);
            let first_rank = Bitboard::rank_bb(Rank::RANK8) | Bitboard::rank_bb(Rank::RANK1);

            // if the pawn is reaching the final rank, it's promoting
            if !(end_bb & first_rank).is_empty() && !mv.is_promotion() {
                return false;
            }

            // the en passant is legal on the board
            if is_en_passant {
                return self.ep_square() == end;
            }

            let (push, double_push) = if piece_side == Side::WHITE {
                (start + Direction::N, start + Direction::N + Direction::N)
            } else {
                (start + Direction::S, start + Direction::S + Direction::S)
            };

            if double_push == end {
                let between = Square((start.0 + end.0) >> 1);
                let start_bb = Bitboard::from(start);
                let first_rank = Bitboard::rank_bb(Rank::RANK2) | Bitboard::rank_bb(Rank::RANK7);

                // we're starting on the correct rank
                if (start_bb & first_rank).is_empty() {
                    return false;
                }

                // the double push isn't blocked
                if captured != Piece::NONE {
                    return false;
                }

                // there's nothing between the start and end
                return self.piece_on(between) == Piece::NONE;
            }

            if captured == Piece::NONE {
                // either it's a pawn push and there's nothing blocking it, or
                // it's a pawn capture and *not* a pawn push
                return push == end;
            }
        }

        let attacks = match piece_type {
            PieceType::PAWN => ATTACK_LOOKUPS.pawn_attacks(piece_side, start),
            PieceType::BISHOP => ATTACK_LOOKUPS.bishop_attacks(start, occupancies),
            PieceType::KNIGHT => ATTACK_LOOKUPS.knight_attacks(start),
            PieceType::ROOK => ATTACK_LOOKUPS.rook_attacks(start, occupancies),
            PieceType::QUEEN => ATTACK_LOOKUPS.queen_attacks(start, occupancies),
            PieceType::KING => ATTACK_LOOKUPS.king_attacks(start),
            // SAFETY: if the piece type was `NONE`, this function would have
            // already exited
            _ => unsafe { unreachable_unchecked() },
        };
        !(Bitboard::from(end) & attacks).is_empty()
    }

    /// Checks if `mv` is a pseudolegal killer on the board, assuming it was
    /// legal in the previous same-depth search.
    pub fn is_pseudolegal_killer(&self, mv: Move) -> bool {
        let start = mv.start();
        let end = mv.end();

        let piece = self.piece_on(start);
        let piece_type = PieceType::from(piece);
        // this might be wrong so it needs to be checked before it's used
        let piece_side = Side::from(piece);
        let captured = self.piece_on(end);
        let captured_type = PieceType::from(captured);
        // this also might be wrong
        let captured_side = Side::from(captured);
        let occupancies = self.occupancies();

        // the piece still exists (en passant can delete it) and hasn't been
        // captured
        if piece == Piece::NONE || piece_side != self.side_to_move() {
            return false;
        }

        // we aren't capturing a friendly piece
        if captured != Piece::NONE && captured_side == self.side_to_move() {
            return false;
        }

        // we weren't blocked
        if !(ray_between(start, end) & occupancies).is_empty() {
            return false;
        }

        // we aren't capturing a king
        if captured_type == PieceType::KING {
            return false;
        }

        // if the piece is a pawn, do some additional checks
        if piece_type == PieceType::PAWN && !self.is_pseudolegal_pawn_killer(mv) {
            return false;
        }

        if mv.is_castling() {
            let rook_start = Square(end.0.wrapping_add_signed(mv.rook_offset()));
            let is_kingside = File::from(end) >= File::FILE5;

            // we have space to castle
            if !Bitboard::is_clear_to_castle(occupancies, piece_side == Side::WHITE, is_kingside) {
                return false;
            }

            // the rook hasn't been captured
            if self.piece_on(rook_start) != Piece::from_piecetype(PieceType::ROOK, piece_side) {
                return false;
            }
        }

        true
    }

    /// Checks if `mv` is a pseudolegal pawn killer, given the same assumptions
    /// as [`Self::is_pseudolegal_killer()`] and assuming the move is a pawn
    /// move.
    fn is_pseudolegal_pawn_killer(&self, mv: Move) -> bool {
        // small optimisation: if the best response to the first move was en
        // passant, it is impossible for that same en passant move to be legal
        // after any other move
        if mv.is_en_passant() {
            return false;
        }

        let start = mv.start();
        let end = mv.end();
        let diff = start.0.abs_diff(end.0);
        // a piece getting between the start and end of a double push was already
        // checked
        let is_push = diff == 8 || diff == 16;
        let is_piece_on_end = self.piece_on(end) != Piece::NONE;

        // check that there isn't a piece blocking us if we're pushing or that
        // there is a piece if we're capturing
        is_push && !is_piece_on_end || !is_push && is_piece_on_end
    }
}

impl CastlingRights {
    /// Returns new, empty [`CastlingRights`].
    const fn new() -> Self {
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

    /// Calculates if the given board and piece side can castle.
    fn can_castle_any(self, is_white: bool, is_kingside: bool) -> bool {
        #[allow(clippy::collapsible_else_if)]
        if is_white {
            if is_kingside {
                self & Self::K == Self::K
            } else {
                self & Self::Q == Self::Q
            }
        } else {
            if is_kingside {
                self & Self::k == Self::k
            } else {
                self & Self::q == Self::q
            }
        }
    }

    /// Adds the given rights to the castling rights.
    ///
    /// If the rights already exist, nothing will happen.
    fn add_rights(&mut self, rights: Self) {
        *self |= rights;
    }

    /// Removes the given rights from the castling rights.
    ///
    /// If the rights do not already exist, nothing will happen.
    fn remove_rights(&mut self, rights: Self) {
        *self &= !rights;
    }
}
