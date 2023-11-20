use crate::{
    bits::{
        as_bitboard, east, north, pawn_push, pop_lsb, ray_attack, south, to_square, west, BitIter,
    },
    board::Board,
    defs::{Bitboard, Bitboards, Directions, Files, Nums, Piece, Pieces, Ranks, Sides, Square},
    movelist::Movelist,
    util::{file_of, rank_of},
};
use magic::{Magic, BISHOP_MAGICS, MAX_BLOCKERS, ROOK_MAGICS};
use util::create_move;

/// Items related to magic bitboards.
pub mod magic;
/// Useful functions for move generation specifically.
pub mod util;

/// Generates legal moves. Contains lookup tables for each piece which are
/// initialised at startup.
pub struct Movegen {
    pawn_attacks: [[Bitboard; Nums::SQUARES]; Nums::SIDES],
    knight_attacks: [Bitboard; Nums::SQUARES],
    king_attacks: [Bitboard; Nums::SQUARES],
    bishop_magic_table: [Bitboard; BISHOP_SIZE],
    rook_magic_table: [Bitboard; ROOK_SIZE],
    bishop_magics: [Magic; Nums::SQUARES],
    rook_magics: [Magic; Nums::SQUARES],
}

/// The number of bitboards required to store all bishop attacks, where each
/// element corresponds to one permutation of blockers. (This means some
/// elements will be duplicates, as different blockers can have the same
/// attacks.) Repeated once per quadrant: `2.pow(6)` blocker permutations for
/// the corner, `2.pow(5)` for each non-corner edge and each square adjacent to
/// an edge, `2.pow(7)` for the squares adjacent or diagonal to a corner and
/// `2.pow(9)` for the corners themselves.
const BISHOP_SIZE: usize = 5_248;
/// The number of bitboards required to store all rook attacks, where each
/// element corresponds to one permutation of blockers. (This means some
/// elements will be duplicates, as different blockers can have the same
/// attacks.) There are `2.pow(12)` blocker permutations for each corner,
/// `2.pow(11)` for each non-corner edge and `2.pow(10)` for all others.
const ROOK_SIZE: usize = 102_400;

impl Movegen {
    /// Creates a new [`Movegen`] with empty lookup tables, then initialises
    /// each of the tables.
    pub fn new() -> Self {
        let mut mg = Self {
            pawn_attacks: [[Bitboards::EMPTY; Nums::SQUARES]; Nums::SIDES],
            knight_attacks: [Bitboards::EMPTY; Nums::SQUARES],
            king_attacks: [Bitboards::EMPTY; Nums::SQUARES],
            bishop_magic_table: [Bitboards::EMPTY; BISHOP_SIZE],
            rook_magic_table: [Bitboards::EMPTY; ROOK_SIZE],
            bishop_magics: [Magic::default(); Nums::SQUARES],
            rook_magics: [Magic::default(); Nums::SQUARES],
        };
        mg.init_pawn_attacks();
        mg.init_knight_attacks();
        mg.init_king_attacks();
        mg.init_magics();
        mg
    }

    /// Generates all combinations of attacks from `square` and puts them in
    /// `attacks`. It starts with a full blocker board that goes from the
    /// square to the edge exclusive and uses the Carry-Rippler trick to
    /// generate each subsequent attack.
    pub fn gen_all_sliding_attacks(
        square: Square,
        piece: Piece,
        attacks: &mut [Bitboard; MAX_BLOCKERS],
    ) {
        let edges = ((Bitboards::FILE_BB[Files::FILE1] | Bitboards::FILE_BB[Files::FILE8])
            & !Bitboards::FILE_BB[file_of(square)])
            | ((Bitboards::RANK_BB[Ranks::RANK1] | Bitboards::RANK_BB[Ranks::RANK8])
                & !Bitboards::RANK_BB[rank_of(square)]);
        let mask = Self::sliding_attacks(square, piece, 0) & !edges;

        let mut blockers = mask;
        let mut first_empty = 0;
        while blockers != 0 {
            attacks[first_empty] = Self::sliding_attacks(square, piece, blockers);
            first_empty += 1;
            blockers = (blockers - 1) & mask;
        }
        attacks[first_empty] = Self::sliding_attacks(square, piece, 0);
    }

    /// Generates the attack set for `piece` on `square` up to and including the
    /// given blockers. Includes the edge.
    pub fn sliding_attacks(square: Square, piece: Piece, blockers: Bitboard) -> Bitboard {
        let bishop_directions = [
            Directions::NE,
            Directions::SE,
            Directions::SW,
            Directions::NW,
        ];
        #[rustfmt::skip]
        let rook_directions = [
            Directions::N,
            Directions::E,
            Directions::S,
            Directions::W
        ];
        let directions = if piece == Pieces::BISHOP {
            bishop_directions
        } else {
            rook_directions
        };

        let mut ray = Bitboards::EMPTY;
        for direction in directions {
            ray |= ray_attack(square, direction, blockers);
        }
        ray
    }
}

impl Movegen {
    /// Generates all legal moves for `board` and puts them in `ml`.
    pub fn generate_moves(&self, board: &Board, ml: &mut Movelist) {
        if board.side_to_move() == Sides::WHITE {
            self.generate_pawn_moves::<true>(board, ml);
            self.generate_non_sliding_moves::<true>(board, ml);
            self.generate_sliding_moves::<true>(board, ml);
        } else {
            self.generate_pawn_moves::<false>(board, ml);
            self.generate_non_sliding_moves::<false>(board, ml);
            self.generate_sliding_moves::<false>(board, ml);
        }
    }
}

impl Movegen {
    /// Finds the bishop attacks from `square` with the given blockers.
    fn bishop_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        self.bishop_magic_table[self.bishop_magics[square].get_table_index(blockers)]
    }

    /// Generates all legal knight and king moves for `board` and puts them in
    /// `ml`.
    fn generate_non_sliding_moves<const IS_WHITE: bool>(&self, board: &Board, ml: &mut Movelist) {
        let us_bb = board.sides::<IS_WHITE>();

        let knights = BitIter::new(board.pieces::<{ Pieces::KNIGHT }>() & us_bb);
        for knight in knights {
            let targets = BitIter::new(self.knight_attacks(knight) & !us_bb);
            for target in targets {
                ml.push_move(create_move::<IS_WHITE, { Pieces::KNIGHT }>(knight, target));
            }
        }

        let kings = BitIter::new(board.pieces::<{ Pieces::KING }>() & us_bb);
        for king in kings {
            let targets = BitIter::new(self.king_attacks(king) & !us_bb);
            for target in targets {
                ml.push_move(create_move::<IS_WHITE, { Pieces::KING }>(king, target));
            }
        }
    }

    /// Generates all legal pawn moves for `board` and puts them in `ml`.
    fn generate_pawn_moves<const IS_WHITE: bool>(&self, board: &Board, ml: &mut Movelist) {
        let us_bb = board.sides::<IS_WHITE>();
        let occupancies = board.occupancies();
        let them_bb = occupancies ^ us_bb;
        let empty = !occupancies;

        let mut pawns = board.pieces::<{ Pieces::PAWN }>() & us_bb;
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

            let captures = self.pawn_attacks::<IS_WHITE>(pawn_sq) & them_bb;

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
    fn generate_sliding_moves<const IS_WHITE: bool>(&self, board: &Board, ml: &mut Movelist) {
        let us_bb = board.sides::<IS_WHITE>();
        let occupancies = board.occupancies();

        let bishops = BitIter::new(board.pieces::<{ Pieces::BISHOP }>() & us_bb);
        for bishop in bishops {
            let targets = BitIter::new(self.bishop_attacks(bishop, occupancies) & !us_bb);
            for target in targets {
                ml.push_move(create_move::<IS_WHITE, { Pieces::BISHOP }>(bishop, target));
            }
        }

        let rooks = BitIter::new(board.pieces::<{ Pieces::ROOK }>() & us_bb);
        for rook in rooks {
            let targets = BitIter::new(self.rook_attacks(rook, occupancies) & !us_bb);
            for target in targets {
                ml.push_move(create_move::<IS_WHITE, { Pieces::ROOK }>(rook, target));
            }
        }

        let queens = BitIter::new(board.pieces::<{ Pieces::QUEEN }>() & us_bb);
        for queen in queens {
            let targets = BitIter::new(self.queen_attacks(queen, occupancies) & !us_bb);
            for target in targets {
                ml.push_move(create_move::<IS_WHITE, { Pieces::QUEEN }>(queen, target));
            }
        }
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
            let b_mask =
                Movegen::sliding_attacks(square, Pieces::BISHOP, Bitboards::EMPTY) & !edges;
            let r_mask = Movegen::sliding_attacks(square, Pieces::ROOK, Bitboards::EMPTY) & !edges;
            let b_mask_bits = b_mask.count_ones();
            let r_mask_bits = r_mask.count_ones();
            let b_perms = 2usize.pow(b_mask_bits);
            let r_perms = 2usize.pow(r_mask_bits);
            let b_magic = Magic::new(BISHOP_MAGICS[square], b_mask, b_offset, 64 - b_mask_bits);
            let r_magic = Magic::new(ROOK_MAGICS[square], r_mask, r_offset, 64 - r_mask_bits);

            Movegen::gen_all_sliding_attacks(square, Pieces::BISHOP, &mut attacks);
            let mut blockers = b_mask;
            for attack in attacks.iter().take(b_perms) {
                let index = b_magic.get_table_index(blockers);
                self.bishop_magic_table[index] = *attack;
                blockers = blockers.wrapping_sub(1) & b_mask;
            }
            self.bishop_magics[square] = b_magic;
            b_offset += b_perms;

            Movegen::gen_all_sliding_attacks(square, Pieces::ROOK, &mut attacks);
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
