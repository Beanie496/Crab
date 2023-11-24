use super::{
    bits::{east, gen_all_sliding_attacks, north, pawn_push, sliding_attacks, south, west},
    Board,
};
use crate::{
    defs::{Bitboard, Bitboards, Files, Move, Nums, Pieces, Ranks, Sides, Square},
    movelist::Movelist,
    util::{as_bitboard, file_of, pop_lsb, rank_of, to_square, BitIter},
};
use magic::{Magic, BISHOP_MAGICS, MAX_BLOCKERS, ROOK_MAGICS};
use util::{create_move, decompose_move};

/// Items related to magic bitboards.
pub mod magic;
/// Useful functions for move generation specifically.
mod util;

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

/// The lookup tables used at runtime.
// initialised at runtime
static mut LOOKUPS: Lookup = Lookup::empty();

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

impl Board {
    /// Generates all legal moves for the current position and puts them in
    /// `ml`.
    pub fn generate_moves(&self, ml: &mut Movelist) {
        if self.side_to_move() == Sides::WHITE {
            self.generate_pawn_moves::<true>(ml);
            self.generate_non_sliding_moves::<true>(ml);
            self.generate_sliding_moves::<true>(ml);
        } else {
            self.generate_pawn_moves::<false>(ml);
            self.generate_non_sliding_moves::<false>(ml);
            self.generate_sliding_moves::<false>(ml);
        }
    }

    /// Makes the given move on the internal board. `mv` is assumed to be a
    /// valid move.
    pub fn make_move(&mut self, mv: Move) {
        self.played_moves.push_move(mv);
        let (start, end, piece, side) = decompose_move(mv);
        self.pieces[piece] ^= as_bitboard(start) | as_bitboard(end);
        self.sides[side] ^= as_bitboard(start) | as_bitboard(end);
        self.side_to_move ^= 1;
    }

    /// Unplays the most recent move. Assumes that a move has been played.
    pub fn unmake_move(&mut self) {
        let mv = self.played_moves.pop_move().unwrap();
        let (start, end, piece, side) = decompose_move(mv);
        self.pieces[piece] ^= as_bitboard(start) | as_bitboard(end);
        self.sides[side] ^= as_bitboard(start) | as_bitboard(end);
        self.side_to_move ^= 1;
    }
}

impl Board {
    /// Generates all legal knight and king moves for `board` and puts them in
    /// `ml`.
    fn generate_non_sliding_moves<const IS_WHITE: bool>(&self, ml: &mut Movelist) {
        let us_bb = self.sides::<IS_WHITE>();

        let knights = BitIter::new(self.pieces::<{ Pieces::KNIGHT }>() & us_bb);
        for knight in knights {
            let targets = BitIter::new(unsafe { LOOKUPS.knight_attacks(knight) } & !us_bb);
            for target in targets {
                ml.push_move(create_move::<IS_WHITE, { Pieces::KNIGHT }>(knight, target));
            }
        }

        let kings = BitIter::new(self.pieces::<{ Pieces::KING }>() & us_bb);
        for king in kings {
            let targets = BitIter::new(unsafe { LOOKUPS.king_attacks(king) } & !us_bb);
            for target in targets {
                ml.push_move(create_move::<IS_WHITE, { Pieces::KING }>(king, target));
            }
        }
    }

    /// Generates all legal pawn moves for `board` and puts them in `ml`.
    fn generate_pawn_moves<const IS_WHITE: bool>(&self, ml: &mut Movelist) {
        let us_bb = self.sides::<IS_WHITE>();
        let occupancies = self.occupancies();
        let them_bb = occupancies ^ us_bb;
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

            let captures = unsafe { LOOKUPS.pawn_attacks::<IS_WHITE>(pawn_sq) } & them_bb;

            let targets = single_push | double_push | captures;
            let promotion_targets =
                targets & (Bitboards::RANK_BB[Ranks::RANK1] | Bitboards::RANK_BB[Ranks::RANK8]);
            let normal_targets = targets ^ promotion_targets;
            for target in BitIter::new(normal_targets) {
                ml.push_move(create_move::<IS_WHITE, { Pieces::PAWN }>(pawn_sq, target));
            }
            for target in BitIter::new(promotion_targets) {
                ml.push_move(create_move::<IS_WHITE, { Pieces::KNIGHT }>(pawn_sq, target));
                ml.push_move(create_move::<IS_WHITE, { Pieces::BISHOP }>(pawn_sq, target));
                ml.push_move(create_move::<IS_WHITE, { Pieces::ROOK }>(pawn_sq, target));
                ml.push_move(create_move::<IS_WHITE, { Pieces::QUEEN }>(pawn_sq, target));
            }
        }
    }

    /// Generates all legal bishop, rook and queen moves for `board` and puts
    /// them in `ml`.
    fn generate_sliding_moves<const IS_WHITE: bool>(&self, ml: &mut Movelist) {
        let us_bb = self.sides::<IS_WHITE>();
        let occupancies = self.occupancies();

        let bishops = BitIter::new(self.pieces::<{ Pieces::BISHOP }>() & us_bb);
        for bishop in bishops {
            let targets =
                BitIter::new(unsafe { LOOKUPS.bishop_attacks(bishop, occupancies) } & !us_bb);
            for target in targets {
                ml.push_move(create_move::<IS_WHITE, { Pieces::BISHOP }>(bishop, target));
            }
        }

        let rooks = BitIter::new(self.pieces::<{ Pieces::ROOK }>() & us_bb);
        for rook in rooks {
            let targets = BitIter::new(unsafe { LOOKUPS.rook_attacks(rook, occupancies) } & !us_bb);
            for target in targets {
                ml.push_move(create_move::<IS_WHITE, { Pieces::ROOK }>(rook, target));
            }
        }

        let queens = BitIter::new(self.pieces::<{ Pieces::QUEEN }>() & us_bb);
        for queen in queens {
            let targets =
                BitIter::new(unsafe { LOOKUPS.queen_attacks(queen, occupancies) } & !us_bb);
            for target in targets {
                ml.push_move(create_move::<IS_WHITE, { Pieces::QUEEN }>(queen, target));
            }
        }
    }
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

    /// Returns a [`Lookup`] with empty tables.
    // used to declare a static `Lookup` variable
    pub const fn empty() -> Self {
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
