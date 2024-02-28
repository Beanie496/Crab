use std::fmt::{self, Display, Formatter};

use super::Board;
use crate::{
    bitboard::Bitboard,
    board::CastlingRights,
    defs::{File, MoveType, Piece, PieceType, Rank, Side, Square},
    out_of_bounds_is_unreachable,
};
use magic::{Magic, BISHOP_MAGICS, MAX_BLOCKERS, ROOK_MAGICS};
use util::{gen_all_sliding_attacks, is_double_pawn_push, sliding_attacks};

use getrandom::getrandom;

/// Items related to magic bitboards.
pub mod magic;
/// Useful functions for move generation specifically.
pub mod util;

/// Contains lookup tables for each piece.
pub struct Lookup {
    /// The pawn attack table. `pawn_attacks[side][square] == attack bitboard
    /// for that square`.
    pawn_attacks: [[Bitboard; Square::TOTAL]; Side::TOTAL],
    /// The knight attack table. `knight_attacks[square] == attack bitboard for
    /// that square`.
    knight_attacks: [Bitboard; Square::TOTAL],
    /// The king attack table. `king_attacks[square] == attack bitboard for
    /// that square`.
    king_attacks: [Bitboard; Square::TOTAL],
    /// The magic lookup table for rooks and bishops.
    ///
    /// The rook attacks are before all the bishop attacks. It uses the 'fancy'
    /// approach. See <https://www.chessprogramming.org/Magic_Bitboards>.
    magic_table: [Bitboard; ROOK_SIZE + BISHOP_SIZE],
    /// The (wrapped) magic numbers for the bishop. One per square. See
    /// <https://www.chessprogramming.org/Magic_Bitboards>.
    bishop_magics: [Magic; Square::TOTAL],
    /// The (wrapped) magic numbers for the rook. One per square. See
    /// <https://www.chessprogramming.org/Magic_Bitboards>.
    rook_magics: [Magic; Square::TOTAL],
}

/// A wrapper for a move and associated methods.
///
/// From LSB onwards, a [`Move`] is as follows:
/// * Start pos == 6 bits, 0-63
/// * End pos == 6 bits, 0-63
/// * Flags == 2 bits.
/// * Promotion piece == 2 bits. Knight == `0b00`, Bishop == `0b01`, etc.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Move(u16);

/// An stack of `Move`s.
pub struct Moves {
    /// The internal array.
    moves: [Move; MAX_LEGAL_MOVES],
    /// The first index that can be written to.
    first_empty: usize,
}

/// The number of bitboards required to store all bishop attacks, where each
/// element corresponds to one permutation of blockers. (This means some
/// elements will be duplicates, as different blockers can have the same
/// attacks.) Repeated once per quadrant: `2.pow(6)` blocker permutations for
/// the corner, `2.pow(5)` for each non-corner edge and each square adjacent to
/// an edge, `2.pow(7)` for the squares adjacent or diagonal to a corner and
/// `2.pow(9)` for the centre.
const BISHOP_SIZE: usize = 5_248;
/// The number of bitboards required to store all rook attacks, where each
/// element corresponds to one permutation of blockers. (This means some
/// elements will be duplicates, as different blockers can have the same
/// attacks.) There are `2.pow(12)` blocker permutations for each corner,
/// `2.pow(11)` for each non-corner edge and `2.pow(10)` for all others.
const ROOK_SIZE: usize = 102_400;
/// Maximum number of legal moves that can be reached in a standard chess game.
///
/// Example: `R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1`
const MAX_LEGAL_MOVES: usize = 218;
/// The lookup tables used at runtime.
// initialised at runtime
pub static mut LOOKUPS: Lookup = Lookup::empty();

impl Display for Move {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let start = Square(((self.0 & Self::START_MASK) >> Self::START_SHIFT) as u8);
        let end = Square(((self.0 & Self::END_MASK) >> Self::END_SHIFT) as u8);
        if self.is_promotion() {
            // we want the lowercase letter here
            write!(f, "{start}{end}{}", char::from(self.promotion_piece()))
        } else {
            write!(f, "{start}{end}")
        }
    }
}

impl Iterator for Moves {
    type Item = Move;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.pop_move()
    }
}

/// Flags used for [`new`](Move::new) and [`new_promo`](Move::new_promo).
///
/// Note that it isn't safe to do bitwise operations on them, as they're
/// mutually exclusive.
impl Move {
    /// Flag for castling.
    pub const CASTLING: u16 = 0b0001_0000_0000_0000;
    /// Flag for en passant.
    pub const EN_PASSANT: u16 = 0b0010_0000_0000_0000;
    /// Flag for promotion.
    pub const PROMOTION: u16 = 0b0011_0000_0000_0000;
    /// No flags.
    pub const NORMAL: u16 = 0b0000_0000_0000_0000;
    /// Mask for the start square.
    const START_MASK: u16 = 0b11_1111;
    /// Shift for the start square, after it has been masked.
    const START_SHIFT: usize = 0;
    /// Mask for the end square.
    const END_MASK: u16 = 0b1111_1100_0000;
    /// Shift for the end square, after it has been masked.
    const END_SHIFT: usize = 6;
    /// Mask for both the start and end square.
    const SQUARE_MASK: u16 = 0b0000_1111_1111_1111;
    /// Shift for the start and end square, after they have been masked.
    const SQUARE_SHIFT: usize = 0;
    /// Mask for the flags. They do not need a shift because they simply need
    /// to be set or unset.
    const FLAG_MASK: u16 = 0b0011_0000_0000_0000;
    /// Shift for the promotion piece. It does not need a mask because shifting
    /// already removes unwanted bits.
    const PIECE_SHIFT: usize = 14;
}

impl Board {
    /// Generates all legal moves for the current position and puts them in
    /// `moves`.
    #[inline]
    pub fn generate_moves<const MOVE_TYPE: u8>(&self, moves: &mut Moves) {
        if self.side_to_move() == Side::WHITE {
            self.generate_pawn_moves::<true, MOVE_TYPE>(moves);
            self.generate_non_sliding_moves::<true, MOVE_TYPE>(moves);
            self.generate_sliding_moves::<true, MOVE_TYPE>(moves);
            if MOVE_TYPE == MoveType::ALL {
                self.generate_castling::<true>(moves);
            }
        } else {
            self.generate_pawn_moves::<false, MOVE_TYPE>(moves);
            self.generate_non_sliding_moves::<false, MOVE_TYPE>(moves);
            self.generate_sliding_moves::<false, MOVE_TYPE>(moves);
            if MOVE_TYPE == MoveType::ALL {
                self.generate_castling::<false>(moves);
            }
        }
    }

    /// Generates the castling moves for the given side.
    fn generate_castling<const IS_WHITE: bool>(&self, moves: &mut Moves) {
        let occupancies = self.occupancies();

        if IS_WHITE {
            if self.can_castle_kingside::<true>()
                && (occupancies & Bitboard::CASTLING_SPACE_WK).is_empty()
            {
                moves.push_move(Move::new::<{ Move::CASTLING }>(Square::E1, Square::H1));
            }
            if self.can_castle_queenside::<true>()
                && (occupancies & Bitboard::CASTLING_SPACE_WQ).is_empty()
            {
                moves.push_move(Move::new::<{ Move::CASTLING }>(Square::E1, Square::A1));
            }
        } else {
            if self.can_castle_kingside::<false>()
                && (occupancies & Bitboard::CASTLING_SPACE_BK).is_empty()
            {
                moves.push_move(Move::new::<{ Move::CASTLING }>(Square::E8, Square::H8));
            }
            if self.can_castle_queenside::<false>()
                && (occupancies & Bitboard::CASTLING_SPACE_BQ).is_empty()
            {
                moves.push_move(Move::new::<{ Move::CASTLING }>(Square::E8, Square::A8));
            }
        }
    }

    /// Generates all legal knight and king moves (excluding castling) for
    /// `board` and puts them in `moves`.
    fn generate_non_sliding_moves<const IS_WHITE: bool, const MOVE_TYPE: u8>(
        &self,
        moves: &mut Moves,
    ) {
        let us_bb = self.side::<IS_WHITE>();
        let target_squares = if MOVE_TYPE == MoveType::ALL {
            // all squares that aren't us
            !us_bb
        } else if MOVE_TYPE == MoveType::CAPTURES {
            // the opponent's piece
            if IS_WHITE {
                self.side::<false>()
            } else {
                self.side::<true>()
            }
        } else {
            panic!("Unknown movetype");
        };

        let knights = self.piece::<{ PieceType::KNIGHT.to_index() }>() & us_bb;
        for knight in knights {
            // SAFETY: Instantiating `self` initialises `LOOKUP`.
            let targets = unsafe { LOOKUPS.knight_attacks(knight) } & target_squares;
            for target in targets {
                moves.push_move(Move::new::<{ Move::NORMAL }>(knight, target));
            }
        }

        let kings = self.piece::<{ PieceType::KING.to_index() }>() & us_bb;
        for king in kings {
            // SAFETY: Instantiating `self` initialises `LOOKUP`.
            let targets = unsafe { LOOKUPS.king_attacks(king) } & target_squares;
            for target in targets {
                moves.push_move(Move::new::<{ Move::NORMAL }>(king, target));
            }
        }
    }

    /// Generates all legal pawn moves for `board` and puts them in `moves`.
    fn generate_pawn_moves<const IS_WHITE: bool, const MOVE_TYPE: u8>(&self, moves: &mut Moves) {
        let us_bb = self.side::<IS_WHITE>();
        let occupancies = self.occupancies();
        let them_bb = occupancies ^ us_bb;
        let ep_square_bb = if self.ep_square() == Square::NONE {
            Bitboard::EMPTY
        } else {
            Bitboard::from(self.ep_square())
        };
        let empty = !occupancies;

        let mut pawns = self.piece::<{ PieceType::PAWN.to_index() }>() & us_bb;
        while !pawns.is_empty() {
            let pawn = pawns.pop_lsb();
            let pawn_sq = pawn.to_square();

            let potential_captures = if IS_WHITE {
                // SAFETY: Instantiating `self` initialises `LOOKUP`.
                unsafe { LOOKUPS.pawn_attacks(Side::WHITE, pawn_sq) }
            } else {
                // SAFETY: Same thing.
                unsafe { LOOKUPS.pawn_attacks(Side::BLACK, pawn_sq) }
            };
            let normal_captures = potential_captures & them_bb;
            let ep_targets = potential_captures & ep_square_bb;

            // if we're just looking at captures, loop through all captures
            // early. Otherwise, wait a bit longer to loop through pushes and
            // captures in the same loop.
            if MOVE_TYPE == MoveType::CAPTURES {
                for target in normal_captures {
                    moves.push_move(Move::new::<{ Move::NORMAL }>(pawn_sq, target));
                }
                for target in ep_targets {
                    moves.push_move(Move::new::<{ Move::EN_PASSANT }>(pawn_sq, target));
                }
                continue;
            }

            let single_push = pawn.pawn_push::<IS_WHITE>() & empty;

            let double_push_rank = if IS_WHITE {
                Bitboard::rank_bb(Rank::RANK4)
            } else {
                Bitboard::rank_bb(Rank::RANK5)
            };
            let double_push = single_push.pawn_push::<IS_WHITE>() & empty & double_push_rank;

            let targets = single_push | normal_captures | double_push;
            let promotion_targets =
                targets & (Bitboard::rank_bb(Rank::RANK1) | Bitboard::rank_bb(Rank::RANK8));
            let normal_targets = targets ^ promotion_targets;

            for target in normal_targets {
                moves.push_move(Move::new::<{ Move::NORMAL }>(pawn_sq, target));
            }
            for target in ep_targets {
                moves.push_move(Move::new::<{ Move::EN_PASSANT }>(pawn_sq, target));
            }
            for target in promotion_targets {
                moves.push_move(Move::new_promo::<{ PieceType::KNIGHT.0 }>(pawn_sq, target));
                moves.push_move(Move::new_promo::<{ PieceType::BISHOP.0 }>(pawn_sq, target));
                moves.push_move(Move::new_promo::<{ PieceType::ROOK.0 }>(pawn_sq, target));
                moves.push_move(Move::new_promo::<{ PieceType::QUEEN.0 }>(pawn_sq, target));
            }
        }
    }

    /// Generates all legal bishop, rook and queen moves for `board` and puts
    /// them in `moves`.
    fn generate_sliding_moves<const IS_WHITE: bool, const MOVE_TYPE: u8>(&self, moves: &mut Moves) {
        let us_bb = self.side::<IS_WHITE>();
        let occupancies = self.occupancies();
        let target_squares = if MOVE_TYPE == MoveType::ALL {
            !us_bb
        } else if MOVE_TYPE == MoveType::CAPTURES {
            us_bb ^ occupancies
        } else {
            panic!("Unknown movetype");
        };

        let bishops = self.piece::<{ PieceType::BISHOP.to_index() }>() & us_bb;
        for bishop in bishops {
            // SAFETY: Instantiating `self` initialises `LOOKUP`.
            let targets = unsafe { LOOKUPS.bishop_attacks(bishop, occupancies) } & target_squares;
            for target in targets {
                moves.push_move(Move::new::<{ Move::NORMAL }>(bishop, target));
            }
        }

        let rooks = self.piece::<{ PieceType::ROOK.to_index() }>() & us_bb;
        for rook in rooks {
            // SAFETY: Instantiating `self` initialises `LOOKUP`.
            let targets = unsafe { LOOKUPS.rook_attacks(rook, occupancies) } & target_squares;
            for target in targets {
                moves.push_move(Move::new::<{ Move::NORMAL }>(rook, target));
            }
        }

        let queens = self.piece::<{ PieceType::QUEEN.to_index() }>() & us_bb;
        for queen in queens {
            // SAFETY: Instantiating `self` initialises `LOOKUP`.
            let targets = unsafe { LOOKUPS.queen_attacks(queen, occupancies) } & target_squares;
            for target in targets {
                moves.push_move(Move::new::<{ Move::NORMAL }>(queen, target));
            }
        }
    }

    /// Makes the given move on the internal board. `mv` is assumed to be a
    /// valid move. Returns `true` if the given move is legal, `false`
    /// otherwise.
    #[inline]
    pub fn make_move(&mut self, mv: Move) -> bool {
        let (start, end, is_castling, is_en_passant, is_promotion, promotion_piece_type) =
            mv.decompose();
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

        // since castling is encoded as king takes rook, castling has to be
        // checked before checking for captures
        if is_castling {
            // if the king is castling out of check
            if self.is_square_attacked(start) {
                return false;
            }
            let king_square = Square((start.0 + end.0 + 1) >> 1);
            let rook_square = Square((start.0 + king_square.0) >> 1);

            // if the king is castling through check
            if self.is_square_attacked(rook_square) {
                return false;
            }
            // if the king is castling into check
            if self.is_square_attacked(king_square) {
                return false;
            }

            self.move_piece(start, king_square, piece, PieceType::KING, us);
            self.move_piece(
                end,
                rook_square,
                // `captured` is equivalent but slower
                Piece::from_piecetype(PieceType::ROOK, us),
                PieceType::ROOK,
                us,
            );

            self.unset_castling_rights(us);
        } else {
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
        }

        if is_en_passant {
            let dest = Square(if us == Side::WHITE {
                end.0 - 8
            } else {
                end.0 + 8
            });
            let captured_pawn = Piece::from_piecetype(PieceType::PAWN, them);
            self.remove_piece(dest, captured_pawn, PieceType::PAWN, them);
        } else if is_double_pawn_push(start, end, piece) {
            self.set_ep_square(Square((start.0 + end.0) >> 1));
        } else if is_promotion {
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

    /// Tests if `square` is attacked by an enemy piece.
    #[inline]
    #[must_use]
    pub fn is_square_attacked(&self, square: Square) -> bool {
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

    /// Returns all the occupied squares on the board.
    fn occupancies(&self) -> Bitboard {
        self.side::<true>() | self.side::<false>()
    }
}

impl Lookup {
    /// Initialises the tables of [`LOOKUPS`].
    // for some reason clippy _doesn't_ complain if I remove this
    // attribute. I'll add it anyway though.
    #[inline]
    pub fn init() {
        // SAFETY: These functions write to a mutable static before anything
        // else reads from it.
        #[allow(clippy::multiple_unsafe_ops_per_block)]
        unsafe {
            LOOKUPS.init_pawn_attacks();
            LOOKUPS.init_knight_attacks();
            LOOKUPS.init_king_attacks();
            LOOKUPS.init_magics();
        };
    }

    /// Returns a [`Lookup`] with empty tables.
    // used to initialise a static `Lookup` variable
    #[allow(clippy::large_stack_frames)]
    const fn empty() -> Self {
        Self {
            pawn_attacks: [[Bitboard::EMPTY; Square::TOTAL]; Side::TOTAL],
            knight_attacks: [Bitboard::EMPTY; Square::TOTAL],
            king_attacks: [Bitboard::EMPTY; Square::TOTAL],
            // allowed because, after testing, a vector was slightly slower
            #[allow(clippy::large_stack_arrays)]
            magic_table: [Bitboard::EMPTY; ROOK_SIZE + BISHOP_SIZE],
            bishop_magics: [Magic::default(); Square::TOTAL],
            rook_magics: [Magic::default(); Square::TOTAL],
        }
    }

    /// Initialises pawn attack lookup table. First and last rank are ignored.
    fn init_pawn_attacks(&mut self) {
        for (square, bb) in self.pawn_attacks[Side::WHITE.to_index()]
            .iter_mut()
            .enumerate()
            .take(Square::TOTAL - File::TOTAL)
        {
            let pushed = Bitboard::from(Square(square as u8 + 8));
            *bb = pushed.east() | pushed.west();
        }
        for (square, bb) in self.pawn_attacks[Side::BLACK.to_index()]
            .iter_mut()
            .enumerate()
            .skip(File::TOTAL)
        {
            let pushed = Bitboard::from(Square(square as u8 - 8));
            *bb = pushed.east() | pushed.west();
        }
    }

    /// Initialises knight attack lookup table.
    fn init_knight_attacks(&mut self) {
        for (square, bb) in self.knight_attacks.iter_mut().enumerate() {
            let square = Square(square as u8);
            let knight = Bitboard::from(square);
            // shortened name to avoid collisions with the function
            let mut e = knight.east();
            let mut w = knight.west();
            let mut attacks = (e | w).north().north();
            attacks |= (e | w).south().south();
            e = e.east();
            w = w.west();
            attacks |= (e | w).north();
            attacks |= (e | w).south();
            *bb = attacks;
        }
    }

    /// Initialises king attack lookup table.
    fn init_king_attacks(&mut self) {
        for (square, bb) in self.king_attacks.iter_mut().enumerate() {
            let square = Square(square as u8);
            let king = Bitboard::from(square);
            let mut attacks = king.east() | king.west() | king;
            attacks |= attacks.north() | attacks.south();
            attacks ^= king;
            *bb = attacks;
        }
    }

    /// Initialises the magic lookup tables with attacks and initialises a
    /// [`Magic`] object for each square.
    fn init_magics(&mut self) {
        let mut b_offset = ROOK_SIZE;
        let mut r_offset = 0;

        for square in 0..Square::TOTAL {
            let square = Square(square as u8);
            let mut attacks = [Bitboard::EMPTY; MAX_BLOCKERS];
            let edges = Bitboard::edges_without(square);
            let b_mask =
                sliding_attacks::<{ PieceType::BISHOP.0 }>(square, Bitboard::EMPTY) & !edges;
            let r_mask = sliding_attacks::<{ PieceType::ROOK.0 }>(square, Bitboard::EMPTY) & !edges;
            let b_mask_bits = b_mask.0.count_ones();
            let r_mask_bits = r_mask.0.count_ones();
            let b_perms = 2usize.pow(b_mask_bits);
            let r_perms = 2usize.pow(r_mask_bits);
            let b_magic = Magic::new(
                BISHOP_MAGICS[square.to_index()],
                b_mask,
                b_offset,
                64 - b_mask_bits,
            );
            let r_magic = Magic::new(
                ROOK_MAGICS[square.to_index()],
                r_mask,
                r_offset,
                64 - r_mask_bits,
            );

            gen_all_sliding_attacks::<{ PieceType::BISHOP.0 }>(square, &mut attacks);
            let mut blockers = b_mask;
            for attack in attacks.iter().take(b_perms) {
                let index = b_magic.get_table_index(blockers);
                self.magic_table[index] = *attack;
                blockers = Bitboard(blockers.0.wrapping_sub(1)) & b_mask;
            }
            self.bishop_magics[square.to_index()] = b_magic;
            b_offset += b_perms;

            gen_all_sliding_attacks::<{ PieceType::ROOK.0 }>(square, &mut attacks);
            let mut blockers = r_mask;
            for attack in attacks.iter().take(r_perms) {
                let index = r_magic.get_table_index(blockers);
                self.magic_table[index] = *attack;
                blockers = Bitboard(blockers.0.wrapping_sub(1)) & r_mask;
            }
            self.rook_magics[square.to_index()] = r_magic;
            r_offset += r_perms;
        }
    }

    /// Finds the pawn attacks from `square`.
    #[inline]
    pub fn pawn_attacks(&self, side: Side, square: Square) -> Bitboard {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(side.to_index(), self.pawn_attacks.len()) };
        // SAFETY: Ditto.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.pawn_attacks[0].len()) };
        self.pawn_attacks[side.to_index()][square.to_index()]
    }

    /// Finds the knight attacks from `square`.
    #[inline]
    pub fn knight_attacks(&self, square: Square) -> Bitboard {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.knight_attacks.len()) };
        self.knight_attacks[square.to_index()]
    }

    /// Finds the king attacks from `square`.
    #[inline]
    pub fn king_attacks(&self, square: Square) -> Bitboard {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.king_attacks.len()) };
        self.king_attacks[square.to_index()]
    }

    /// Finds the bishop attacks from `square` with the given blockers.
    #[inline]
    pub fn bishop_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.bishop_magics.len()) };
        let index = self.bishop_magics[square.to_index()].get_table_index(blockers);
        // SAFETY: Ditto.
        unsafe { out_of_bounds_is_unreachable!(index, self.magic_table.len()) };
        self.magic_table[index]
    }

    /// Finds the rook attacks from `square` with the given blockers.
    #[inline]
    pub fn rook_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.rook_magics.len()) };
        let index = self.rook_magics[square.to_index()].get_table_index(blockers);
        // SAFETY: Ditto.
        unsafe { out_of_bounds_is_unreachable!(index, self.magic_table.len()) };
        self.magic_table[index]
    }

    /// Finds the queen attacks from `square` with the given blockers.
    #[inline]
    pub fn queen_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        self.bishop_attacks(square, blockers) | self.rook_attacks(square, blockers)
    }
}

impl Move {
    /// Creates a [`Move`] given a start square and end square. `FLAG` can be
    /// set to either [`CASTLING`](Move::CASTLING) or
    /// [`EN_PASSANT`](Move::EN_PASSANT), but cannot be used for
    /// [`PROMOTION`](Move::PROMOTION), since that requires an additional
    /// parameter. See [`new_promo`](Move::new_promo) for a new promotion
    /// [`Move`].
    #[inline]
    #[must_use]
    pub const fn new<const FLAG: u16>(start: Square, end: Square) -> Self {
        debug_assert!(
            FLAG != Self::PROMOTION,
            "Tried to make a new promotion `Move` with the wrong function"
        );
        Self((start.0 as u16) << Self::START_SHIFT | (end.0 as u16) << Self::END_SHIFT | FLAG)
    }

    /// Creates a promotion [`Move`] to the given piece.
    #[inline]
    #[must_use]
    pub const fn new_promo<const PIECE: u8>(start: Square, end: Square) -> Self {
        debug_assert!(
            PIECE != PieceType::PAWN.0,
            "Tried to make a new promotion `Move` into a pawn"
        );
        debug_assert!(
            PIECE != PieceType::KING.0,
            "Tried to make a new promotion `Move` into a king"
        );
        Self(
            (start.0 as u16) << Self::START_SHIFT
                | (end.0 as u16) << Self::END_SHIFT
                | Self::PROMOTION
                | ((PIECE - 1) as u16) << Self::PIECE_SHIFT,
        )
    }

    /// Creates a promotion [`Move`] to the given piece type.
    #[inline]
    #[must_use]
    pub fn new_promo_any(start: Square, end: Square, promotion_piece: PieceType) -> Self {
        debug_assert!(
            promotion_piece != PieceType::PAWN,
            "Tried to make a new promotion `Move` into a pawn"
        );
        debug_assert!(
            promotion_piece != PieceType::KING,
            "Tried to make a new promotion `Move` into a king"
        );
        Self(
            u16::from(start.0) << Self::START_SHIFT
                | u16::from(end.0) << Self::END_SHIFT
                | Self::PROMOTION
                | u16::from(promotion_piece.0 - 1) << Self::PIECE_SHIFT,
        )
    }

    /// Creates a null [`Move`].
    #[inline]
    #[must_use]
    pub const fn null() -> Self {
        Self(0)
    }

    /// Turns a [`Move`] into its components: start square, end square, is
    /// castling, is promotion, is en passant and piece (only set if
    /// `is_promotion`), in that order.
    #[inline]
    #[must_use]
    pub const fn decompose(&self) -> (Square, Square, bool, bool, bool, PieceType) {
        let start = self.start();
        let end = self.end();
        let is_promotion = self.is_promotion();
        let is_castling = self.is_castling();
        let is_en_passant = self.is_en_passant();
        let piece_type = PieceType((self.0 >> Self::PIECE_SHIFT) as u8 + 1);
        (
            start,
            end,
            is_castling,
            is_en_passant,
            is_promotion,
            piece_type,
        )
    }

    /// Calculates the start square of `self`.
    #[inline]
    #[must_use]
    pub const fn start(&self) -> Square {
        Square(((self.0 & Self::START_MASK) >> Self::START_SHIFT) as u8)
    }

    /// Calculates the end square of `self`.
    #[inline]
    #[must_use]
    pub const fn end(&self) -> Square {
        Square(((self.0 & Self::END_MASK) >> Self::END_SHIFT) as u8)
    }

    /// Checks if the move is castling.
    #[inline]
    #[must_use]
    pub const fn is_castling(&self) -> bool {
        self.0 & Self::FLAG_MASK == Self::CASTLING
    }

    /// Checks if the move is en passant.
    #[inline]
    #[must_use]
    pub const fn is_en_passant(&self) -> bool {
        self.0 & Self::FLAG_MASK == Self::EN_PASSANT
    }

    /// Checks if the given start and end square match the start and end square
    /// contained within `self`.
    #[inline]
    #[must_use]
    pub const fn is_moving_from_to(&self, start: Square, end: Square) -> bool {
        let other = Self::new::<{ Self::NORMAL }>(start, end);
        (other.0 & Self::SQUARE_MASK) >> Self::SQUARE_SHIFT
            == (self.0 & Self::SQUARE_MASK) >> Self::SQUARE_SHIFT
    }

    /// Checks if the given start square, end square and promotion piece match
    /// the start, end square and promotion piece contained within `self`.
    #[inline]
    #[must_use]
    pub fn is_moving_from_to_promo(
        &self,
        start: Square,
        end: Square,
        promotion_piece: PieceType,
    ) -> bool {
        let other = Self::new_promo_any(start, end, promotion_piece);
        *self == other
    }

    /// Checks if the move is a promotion.
    #[inline]
    #[must_use]
    pub const fn is_promotion(&self) -> bool {
        self.0 & Self::FLAG_MASK == Self::PROMOTION
    }

    /// Returns the piece to be promoted to. Assumes `self.is_promotion()`. Can
    /// only return a value from 1 to 4.
    #[inline]
    #[must_use]
    pub const fn promotion_piece(&self) -> PieceType {
        PieceType((self.0 >> Self::PIECE_SHIFT) as u8 + 1)
    }
}

impl Moves {
    /// Creates an empty [`Moves`] object.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            moves: [Move::null(); MAX_LEGAL_MOVES],
            first_empty: 0,
        }
    }

    /// Returns if it's empty
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.first_empty == 0
    }

    /// Returns its length.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.first_empty
    }

    /// Finds and returns, if it exists, the move that has start square `start`
    /// and end square `end`.
    ///
    /// Returns `Some(mv)` if a `Move` does match the start and end square;
    /// returns `None` otherwise.
    #[inline]
    pub fn move_with(&mut self, start: Square, end: Square) -> Option<Move> {
        self.moves
            .into_iter()
            .find(|&mv| mv.is_moving_from_to(start, end))
    }

    /// Finds and returns, if it exists, the move that has start square
    /// `start`, end square `end` and promotion piece `piece_type`.
    ///
    /// Returns `Some(mv)` if a `Move` does match the criteria; returns `None`
    /// otherwise.
    #[inline]
    pub fn move_with_promo(
        &mut self,
        start: Square,
        end: Square,
        piece_type: PieceType,
    ) -> Option<Move> {
        self.moves
            .into_iter()
            .find(|&mv| mv.is_moving_from_to_promo(start, end, piece_type))
    }

    /// Returns a random move.
    ///
    /// # Panics
    ///
    /// Will panic if `self` is empty.
    #[inline]
    #[must_use]
    pub fn random_move(&self) -> Move {
        assert!(
            !self.is_empty(),
            "Tried to get a random move from an empty `Moves` list"
        );
        let mut rand = [0u8; 1];
        if getrandom(&mut rand).is_ok() {
            let rand = rand[0] as usize;
            self.moves[rand % self.len()]
        } else {
            self.moves[0]
        }
    }

    /// Pushes `mv` onto itself. Assumes `self` is not full.
    #[inline]
    pub fn push_move(&mut self, mv: Move) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(self.first_empty, self.moves.len()) };
        self.moves[self.first_empty] = mv;
        self.first_empty += 1;
    }

    /// Pops a `Move` from the array. Returns `Some(move)` if there are `> 0`
    /// moves, otherwise returns `None`.
    #[inline]
    pub fn pop_move(&mut self) -> Option<Move> {
        (self.first_empty > 0).then(|| {
            self.first_empty -= 1;
            // SAFETY: If it does get reached, it will panic in debug.
            unsafe { out_of_bounds_is_unreachable!(self.first_empty, self.moves.len()) };
            self.moves[self.first_empty]
        })
    }

    /// Clears `self`. This doesn't actually zero any bits: it just resets the
    /// head pointer, so it's O(1).
    #[inline]
    pub fn clear(&mut self) {
        self.first_empty = 0;
    }
}
