use super::Board;
use crate::{
    defs::{Bitboard, Bitboards, Files, Nums, Piece, Pieces, Ranks, Sides, Square, Squares},
    util::{as_bitboard, file_of, pop_lsb, rank_of, to_square, BitIter},
};
use magic::{Magic, BISHOP_MAGICS, MAX_BLOCKERS, ROOK_MAGICS};
use util::{east, gen_all_sliding_attacks, north, pawn_push, sliding_attacks, south, west};

/// Items related to magic bitboards.
pub mod magic;
/// Useful functions for move generation specifically.
pub mod util;

/// Contains lookup tables for each piece.
pub struct Lookup {
    pawn_attacks: [[Bitboard; Nums::SQUARES]; Nums::SIDES],
    knight_attacks: [Bitboard; Nums::SQUARES],
    king_attacks: [Bitboard; Nums::SQUARES],
    bishop_magic_table: [Bitboard; BISHOP_SIZE],
    rook_magic_table: [Bitboard; ROOK_SIZE],
    bishop_magics: [Magic; Nums::SQUARES],
    rook_magics: [Magic; Nums::SQUARES],
}

/**
 * A wrapper for a move and associated methods.
 *
 * From LSB onwards, a [`Move`] is as follows:
 * * Start pos == 6 bits, 0-63
 * * End pos == 6 bits, 0-63
 * * Flags == 2 bits.
 * * Promotion piece == 2 bits. Knight == `0b00`, Bishop == `0b01`, etc.
 */
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Move {
    mv: u16,
}

/// An array of [`Move`]s
pub struct Moves {
    moves: [Move; MAX_LEGAL_MOVES],
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
static mut LOOKUPS: Lookup = Lookup::empty();

impl Move {
    pub const START_MASK: u16 = 0b11_1111;
    pub const START_SHIFT: usize = 0;
    pub const END_MASK: u16 = 0b1111_1100_0000;
    pub const END_SHIFT: usize = 6;
    pub const NO_FLAG: u16 = 0b0000_0000_0000_0000;
    pub const CASTLING_FLAG: u16 = 0b0001_0000_0000_0000;
    pub const EN_PASSANT_FLAG: u16 = 0b0010_0000_0000_0000;
    pub const PROMOTION_FLAG: u16 = 0b0011_0000_0000_0000;
    pub const FLAG_MASK: u16 = 0b0011_0000_0000_0000;
    pub const PIECE_SHIFT: usize = 14;
}

impl Lookup {
    /// Initialises the tables of [`LOOKUPS`].
    pub fn init() {
        unsafe {
            LOOKUPS.init_pawn_attacks();
            LOOKUPS.init_knight_attacks();
            LOOKUPS.init_king_attacks();
            LOOKUPS.init_magics();
        };
    }
}

impl Move {
    /// Creates a [`Move`] given a start square and end square. `FLAG` can be
    /// set to either [`Move::CASTLING_FLAG`] or [`Move::EN_PASSANT_FLAG`], but
    /// cannot be used for [`Move::PROMOTION_FLAG`], since that requires an
    /// additional parameter. See [`new_promo`](Move::new_promo) for a new
    /// promotion [`Move`].
    pub fn new<const FLAG: u16>(start: Square, end: Square) -> Move {
        debug_assert!(FLAG != Move::PROMOTION_FLAG);
        Self {
            mv: (start as u16) << Self::START_SHIFT | (end as u16) << Self::END_SHIFT | FLAG,
        }
    }

    /// Creates a promotion [`Move`] to the given piece.
    pub fn new_promo<const PIECE: Piece>(start: Square, end: Square) -> Move {
        debug_assert!(PIECE != Pieces::PAWN);
        debug_assert!(PIECE != Pieces::KING);
        Self {
            mv: (start as u16) << Self::START_SHIFT
                | (end as u16) << Self::END_SHIFT
                | Self::PROMOTION_FLAG
                | ((PIECE - 1) as u16) << Self::PIECE_SHIFT,
        }
    }

    /// Creates a null [`Move`].
    pub fn null() -> Move {
        Self { mv: 0 }
    }
}

impl Moves {
    /// Creates an empty [`Moves`] object.
    pub fn new() -> Self {
        Self {
            moves: [Move::null(); MAX_LEGAL_MOVES],
            first_empty: 0,
        }
    }
}

impl Lookup {
    /// Returns a [`Lookup`] with empty tables.
    // used to initialise a static `Lookup` variable
    const fn empty() -> Self {
        Self {
            pawn_attacks: [[Bitboards::EMPTY; Nums::SQUARES]; Nums::SIDES],
            knight_attacks: [Bitboards::EMPTY; Nums::SQUARES],
            king_attacks: [Bitboards::EMPTY; Nums::SQUARES],
            bishop_magic_table: [Bitboards::EMPTY; BISHOP_SIZE],
            rook_magic_table: [Bitboards::EMPTY; ROOK_SIZE],
            bishop_magics: [Magic::default(); Nums::SQUARES],
            rook_magics: [Magic::default(); Nums::SQUARES],
        }
    }
}

impl Board {
    /// Generates all legal moves for the current position and puts them in
    /// `moves`.
    pub fn generate_moves(&self, moves: &mut Moves) {
        if self.side_to_move() == Sides::WHITE {
            self.generate_pawn_moves::<true>(moves);
            self.generate_non_sliding_moves::<true>(moves);
            self.generate_sliding_moves::<true>(moves);
            self.generate_castling::<true>(moves);
        } else {
            self.generate_pawn_moves::<false>(moves);
            self.generate_non_sliding_moves::<false>(moves);
            self.generate_sliding_moves::<false>(moves);
            self.generate_castling::<false>(moves);
        }
    }

    /// Makes the given move on the internal board. `mv` is assumed to be a
    /// valid move.
    pub fn make_move(&mut self, mv: Move) {
        let (start, end, is_castling, is_en_passant, is_promotion, promotion_piece) =
            mv.decompose();
        let piece = self.piece_on(start);
        let captured = self.piece_on(end);
        let us = self.side_to_move();
        let them = us ^ 1;
        let end_bb = as_bitboard(end);

        // save the current state before we modify it
        self.played_moves.push_move(
            mv,
            piece,
            captured,
            self.ep_square(),
            self.castling_rights(),
        );

        self.move_piece(start, end, us, piece);
        self.clear_ep_square();

        // these two `if` statements have to be lumped together, annoyingly -
        // otherwise the second one would trigger incorrectly (since the the
        // target square, containing a rook, would count)
        if is_castling {
            let king_square = (start + end + 1) >> 1;
            let rook_square = (start + king_square) >> 1;

            self.move_piece(end, king_square, us, Pieces::KING);
            self.move_piece(end, rook_square, us, Pieces::ROOK);

            self.unset_castling_rights(us);
        } else if captured != Pieces::NONE {
            // if we're capturing a piece, unset the bitboard of the captured
            // piece.
            // By a happy accident, we don't need to check if we're capturing
            // the same piece as we are currently - the bit would have been
            // (wrongly) unset earlier, so this would (wrongly) re-set it.
            // Looks like two wrongs do make a right in binary.
            self.toggle_piece_bb(captured, end_bb);
            self.toggle_side_bb(them, end_bb);
            if captured == Pieces::ROOK {
                // if the captured rook is actually valid
                self.unset_castling_right(them, (end & 1) as u8 + 1);
            }
        }
        if piece == Pieces::ROOK {
            self.unset_castling_right(us, (end & 1) as u8 + 1);
        }

        if Self::is_double_pawn_push(start, end, piece) {
            self.set_ep_square((start + end) >> 1);
        } else if is_en_passant {
            let dest = if us == Sides::WHITE { end - 8 } else { end + 8 };
            self.clear_piece(dest);
            self.toggle_piece_bb(Pieces::PAWN, as_bitboard(dest));
        } else if is_promotion {
            self.set_piece(end, promotion_piece);
            // unset the pawn on the promotion square...
            self.toggle_piece_bb(Pieces::PAWN, end_bb);
            // ...and set the promotion piece on that square
            self.toggle_piece_bb(promotion_piece, end_bb);
        }

        self.flip_side();
    }

    /// Unplays the most recent move. Assumes that a move has been played.
    pub fn unmake_move(&mut self) {
        let (
            start,
            end,
            is_castling,
            is_en_passant,
            is_promotion,
            promotion_piece,
            piece,
            captured,
            ep_square,
            castling_rights,
        ) = self.played_moves.pop_move().decompose();

        self.flip_side();

        let us = self.side_to_move();
        let them = us ^ 1;

        let end_bb = as_bitboard(end);

        self.clear_ep_square();

        if is_castling {
            let king_square = (start + end + 1) >> 1;
            let rook_square = (start + king_square) >> 1;

            self.move_piece(king_square, start, us, Pieces::KING);
            self.move_piece(rook_square, end, us, Pieces::ROOK);

            self.set_castling_rights(us, castling_rights);
        } else {
            self.unmove_piece(start, end, us, piece, captured);
            if captured != Pieces::NONE {
                self.toggle_piece_bb(captured, end_bb);
                self.toggle_side_bb(them, end_bb);
            }
        }

        if is_en_passant {
            let dest = if us == Sides::WHITE { end - 8 } else { end + 8 };
            self.set_piece(dest, Pieces::PAWN);
            self.toggle_piece_bb(Pieces::PAWN, as_bitboard(dest));
            self.set_ep_square(ep_square);
        } else if is_promotion {
            // the pawn would have been wrongly set earlier, so unset it now
            self.toggle_piece_bb(Pieces::PAWN, end_bb);
            self.toggle_piece_bb(promotion_piece, end_bb);
        }
    }
}

impl Move {
    /// Turns a [`Move`] into its components: start square, end square, is
    /// castling, is promotion, is en passant and piece (only set if
    /// `is_promotion`), in that order.
    pub fn decompose(&self) -> (Square, Square, bool, bool, bool, Piece) {
        let start = (self.mv & Self::START_MASK) >> Self::START_SHIFT;
        let end = (self.mv & Self::END_MASK) >> Self::END_SHIFT;
        let is_promotion = self.is_promotion();
        let is_castling = self.is_castling();
        let is_en_passant = self.is_en_passant();
        let piece = (self.mv >> Self::PIECE_SHIFT) + 1;
        (
            start as Square,
            end as Square,
            is_castling,
            is_en_passant,
            is_promotion,
            piece as Piece,
        )
    }

    /// Checks if the move is castling.
    pub fn is_castling(&self) -> bool {
        self.mv & Self::FLAG_MASK == Self::CASTLING_FLAG
    }

    /// Checks if the move is en passant.
    pub fn is_en_passant(&self) -> bool {
        self.mv & Self::FLAG_MASK == Self::EN_PASSANT_FLAG
    }

    /// Checks if the move is a promotion.
    pub fn is_promotion(&self) -> bool {
        self.mv & Self::FLAG_MASK == Self::PROMOTION_FLAG
    }

    /// Returns the piece to be promoted to. Assumes `self.is_promotion()`. Can
    /// only return a value from 1 to 4.
    pub fn promotion_piece(&self) -> Piece {
        (self.mv >> Self::PIECE_SHIFT) as Piece + 1
    }
}

impl Moves {
    /// Returns the number of stored moves.
    pub fn moves(&self) -> usize {
        self.first_empty
    }

    /// Pops a [`Move`] from the array. Returns `Some(move)` if there are `> 0`
    /// moves, otherwise returns `None`.
    pub fn pop_move(&mut self) -> Option<Move> {
        (self.first_empty > 0).then(|| {
            self.first_empty -= 1;
            self.moves[self.first_empty]
        })
    }

    /// Pushes `mv` onto itself. Assumes `self` is not full.
    pub fn push_move(&mut self, mv: Move) {
        self.moves[self.first_empty] = mv;
        self.first_empty += 1;
    }
}

impl Board {
    /// Generates the castling moves for the given side.
    fn generate_castling<const IS_WHITE: bool>(&self, moves: &mut Moves) {
        let occupancies = self.occupancies();

        if IS_WHITE {
            if self.can_castle_kingside::<true>() && occupancies & Bitboards::CASTLING_SPACE_WK == 0
            {
                moves.push_move(Move::new::<{ Move::CASTLING_FLAG }>(
                    Squares::E1,
                    Squares::H1,
                ));
            }
            if self.can_castle_queenside::<true>()
                && occupancies & Bitboards::CASTLING_SPACE_WQ == 0
            {
                moves.push_move(Move::new::<{ Move::CASTLING_FLAG }>(
                    Squares::E1,
                    Squares::A1,
                ));
            }
        } else {
            if self.can_castle_kingside::<false>()
                && occupancies & Bitboards::CASTLING_SPACE_BK == 0
            {
                moves.push_move(Move::new::<{ Move::CASTLING_FLAG }>(
                    Squares::E8,
                    Squares::H8,
                ));
            }
            if self.can_castle_queenside::<false>()
                && occupancies & Bitboards::CASTLING_SPACE_BQ == 0
            {
                moves.push_move(Move::new::<{ Move::CASTLING_FLAG }>(
                    Squares::E8,
                    Squares::A8,
                ));
            }
        }
    }

    /// Generates all legal knight and king moves (excluding castling) for
    /// `board` and puts them in `moves`.
    fn generate_non_sliding_moves<const IS_WHITE: bool>(&self, moves: &mut Moves) {
        let us_bb = self.sides::<IS_WHITE>();

        let knights = BitIter::new(self.pieces::<{ Pieces::KNIGHT }>() & us_bb);
        for knight in knights {
            let targets = BitIter::new(unsafe { LOOKUPS.knight_attacks(knight) } & !us_bb);
            for target in targets {
                moves.push_move(Move::new::<{ Move::NO_FLAG }>(knight, target));
            }
        }

        let kings = BitIter::new(self.pieces::<{ Pieces::KING }>() & us_bb);
        for king in kings {
            let targets = BitIter::new(unsafe { LOOKUPS.king_attacks(king) } & !us_bb);
            for target in targets {
                moves.push_move(Move::new::<{ Move::NO_FLAG }>(king, target));
            }
        }
    }

    /// Generates all legal pawn moves for `board` and puts them in `moves`.
    fn generate_pawn_moves<const IS_WHITE: bool>(&self, moves: &mut Moves) {
        let us_bb = self.sides::<IS_WHITE>();
        let occupancies = self.occupancies();
        let them_bb = occupancies ^ us_bb;
        let ep_square_bb = if self.ep_square() == Squares::NONE {
            0
        } else {
            as_bitboard(self.ep_square())
        };
        let empty = !occupancies;

        let mut pawns = self.pieces::<{ Pieces::PAWN }>() & us_bb;
        while pawns != 0 {
            let pawn = pop_lsb(&mut pawns);
            let pawn_sq = to_square(pawn);

            let single_push = pawn_push::<IS_WHITE>(pawn) & empty;

            let double_push_rank = if IS_WHITE {
                Bitboards::RANK_BB[Ranks::RANK4]
            } else {
                Bitboards::RANK_BB[Ranks::RANK5]
            };
            let double_push = pawn_push::<IS_WHITE>(single_push) & empty & double_push_rank;

            let all_captures = unsafe { LOOKUPS.pawn_attacks::<IS_WHITE>(pawn_sq) };
            let normal_captures = all_captures & them_bb;
            let ep_captures = all_captures & ep_square_bb;

            let targets = single_push | normal_captures | double_push;
            let promotion_targets =
                targets & (Bitboards::RANK_BB[Ranks::RANK1] | Bitboards::RANK_BB[Ranks::RANK8]);
            let normal_targets = targets ^ promotion_targets;

            for target in BitIter::new(normal_targets) {
                moves.push_move(Move::new::<{ Move::NO_FLAG }>(pawn_sq, target));
            }
            for target in BitIter::new(promotion_targets) {
                moves.push_move(Move::new_promo::<{ Pieces::KNIGHT }>(pawn_sq, target));
                moves.push_move(Move::new_promo::<{ Pieces::BISHOP }>(pawn_sq, target));
                moves.push_move(Move::new_promo::<{ Pieces::ROOK }>(pawn_sq, target));
                moves.push_move(Move::new_promo::<{ Pieces::QUEEN }>(pawn_sq, target));
            }
            for target in BitIter::new(ep_captures) {
                moves.push_move(Move::new::<{ Move::EN_PASSANT_FLAG }>(pawn_sq, target));
            }
        }
    }

    /// Generates all legal bishop, rook and queen moves for `board` and puts
    /// them in `moves`.
    fn generate_sliding_moves<const IS_WHITE: bool>(&self, moves: &mut Moves) {
        let us_bb = self.sides::<IS_WHITE>();
        let occupancies = self.occupancies();

        let bishops = BitIter::new(self.pieces::<{ Pieces::BISHOP }>() & us_bb);
        for bishop in bishops {
            let targets =
                BitIter::new(unsafe { LOOKUPS.bishop_attacks(bishop, occupancies) } & !us_bb);
            for target in targets {
                moves.push_move(Move::new::<{ Move::NO_FLAG }>(bishop, target));
            }
        }

        let rooks = BitIter::new(self.pieces::<{ Pieces::ROOK }>() & us_bb);
        for rook in rooks {
            let targets = BitIter::new(unsafe { LOOKUPS.rook_attacks(rook, occupancies) } & !us_bb);
            for target in targets {
                moves.push_move(Move::new::<{ Move::NO_FLAG }>(rook, target));
            }
        }

        let queens = BitIter::new(self.pieces::<{ Pieces::QUEEN }>() & us_bb);
        for queen in queens {
            let targets =
                BitIter::new(unsafe { LOOKUPS.queen_attacks(queen, occupancies) } & !us_bb);
            for target in targets {
                moves.push_move(Move::new::<{ Move::NO_FLAG }>(queen, target));
            }
        }
    }
}

impl Lookup {
    /// Finds the bishop attacks from `square` with the given blockers.
    fn bishop_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        self.bishop_magic_table[self.bishop_magics[square].get_table_index(blockers)]
    }

    /// Initialises king attack lookup table.
    fn init_king_attacks(&mut self) {
        for (square, bb) in self.king_attacks.iter_mut().enumerate() {
            let king = as_bitboard(square);
            let mut attacks = east(king) | west(king) | king;
            attacks |= north(attacks) | south(attacks);
            attacks ^= king;
            *bb = attacks;
        }
    }

    /// Initialises knight attack lookup table.
    fn init_knight_attacks(&mut self) {
        for (square, bb) in self.knight_attacks.iter_mut().enumerate() {
            let knight = as_bitboard(square);
            // shortened name to avoid collisions with the function
            let mut e = east(knight);
            let mut w = west(knight);
            let mut attacks = north(north(e | w));
            attacks |= south(south(e | w));
            e = east(e);
            w = west(w);
            attacks |= north(e | w);
            attacks |= south(e | w);
            *bb = attacks
        }
    }

    /// Initialises the magic lookup tables with attacks and initialises a
    /// [`Magic`] object for each square.
    fn init_magics(&mut self) {
        let mut b_offset = 0;
        let mut r_offset = 0;

        for square in 0..Nums::SQUARES {
            let mut attacks = [Bitboards::EMPTY; MAX_BLOCKERS];

            let edges = ((Bitboards::FILE_BB[Files::FILE1] | Bitboards::FILE_BB[Files::FILE8])
                & !Bitboards::FILE_BB[file_of(square)])
                | ((Bitboards::RANK_BB[Ranks::RANK1] | Bitboards::RANK_BB[Ranks::RANK8])
                    & !Bitboards::RANK_BB[rank_of(square)]);
            let b_mask = sliding_attacks::<{ Pieces::BISHOP }>(square, Bitboards::EMPTY) & !edges;
            let r_mask = sliding_attacks::<{ Pieces::ROOK }>(square, Bitboards::EMPTY) & !edges;
            let b_mask_bits = b_mask.count_ones();
            let r_mask_bits = r_mask.count_ones();
            let b_perms = 2usize.pow(b_mask_bits);
            let r_perms = 2usize.pow(r_mask_bits);
            let b_magic = Magic::new(BISHOP_MAGICS[square], b_mask, b_offset, 64 - b_mask_bits);
            let r_magic = Magic::new(ROOK_MAGICS[square], r_mask, r_offset, 64 - r_mask_bits);

            gen_all_sliding_attacks::<{ Pieces::BISHOP }>(square, &mut attacks);
            let mut blockers = b_mask;
            for attack in attacks.iter().take(b_perms) {
                let index = b_magic.get_table_index(blockers);
                self.bishop_magic_table[index] = *attack;
                blockers = blockers.wrapping_sub(1) & b_mask;
            }
            self.bishop_magics[square] = b_magic;
            b_offset += b_perms;

            gen_all_sliding_attacks::<{ Pieces::ROOK }>(square, &mut attacks);
            let mut blockers = r_mask;
            for attack in attacks.iter().take(r_perms) {
                let index = r_magic.get_table_index(blockers);
                self.rook_magic_table[index] = *attack;
                blockers = blockers.wrapping_sub(1) & r_mask;
            }
            self.rook_magics[square] = r_magic;
            r_offset += r_perms;
        }
    }

    /// Initialises pawn attack lookup table. First and last rank are ignored.
    fn init_pawn_attacks(&mut self) {
        for (side, table) in self.pawn_attacks.iter_mut().enumerate() {
            // take() ignores the last rank, since pawns can't advance past.
            // enumerate() gives both tables a value (0 or 1).
            // skip() ignores the first rank, since pawns can't start there.
            // It's very important that the skip is done _after_ the enumerate,
            // as otherwise the enumerate would give A2 value 0, B2 value 1,
            // and so on.
            for (square, bb) in table
                .iter_mut()
                .take(Nums::SQUARES - Nums::FILES)
                .enumerate()
                .skip(Nums::FILES)
            {
                // adds 8 if the side is White (1) or subtracts 8 if Black (0)
                let pushed = as_bitboard(square - 8 + side * 16);
                *bb = east(pushed) | west(pushed);
            }
        }
    }

    /// Finds the king attacks from `square`.
    fn king_attacks(&self, square: Square) -> Bitboard {
        self.king_attacks[square]
    }

    /// Finds the knight attacks from `square`.
    fn knight_attacks(&self, square: Square) -> Bitboard {
        self.knight_attacks[square]
    }

    /// Finds the pawn attacks from `square`.
    fn pawn_attacks<const IS_WHITE: bool>(&self, square: Square) -> Bitboard {
        if IS_WHITE {
            self.pawn_attacks[Sides::WHITE][square]
        } else {
            self.pawn_attacks[Sides::BLACK][square]
        }
    }

    /// Finds the queen attacks from `square` with the given blockers.
    fn queen_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        self.bishop_attacks(square, blockers) | self.rook_attacks(square, blockers)
    }

    /// Finds the rook attacks from `square` with the given blockers.
    fn rook_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        self.rook_magic_table[self.rook_magics[square].get_table_index(blockers)]
    }
}

impl Iterator for Moves {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop_move()
    }
}

#[cfg(test)]
mod tests {
    use super::{Board, Move};

    use crate::defs::{Pieces, Sides, Squares};

    #[test]
    fn make_and_unmake() {
        let mut board = Board::new();

        let mv = Move::new::<{ Move::NO_FLAG }>(Squares::A1, Squares::A3);
        board.make_move(mv);
        assert_eq!(board.sides[Sides::WHITE], 0x000000000001fffe);
        assert_eq!(board.pieces[Pieces::ROOK], 0x8100000000010080);
        board.unmake_move();
        assert_eq!(board.sides[Sides::WHITE], 0x000000000000ffff);
        assert_eq!(board.pieces[Pieces::ROOK], 0x8100000000000081);
    }
}
