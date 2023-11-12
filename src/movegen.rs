use crate::{
    bits::{as_bitboard, east, north, pop_lsb, pop_next_square, ray_attack, south, to_square, west},
    board::Board,
    defs::{Bitboard, Bitboards, Directions, Files, Nums, Piece, Pieces, Ranks, Square},
    movelist::Movelist,
    util::{file_of, rank_of},
};
use magic::Magic;
use util::create_move;

/// Items relating to magic bitboards.
mod magic;
/// Useful functions for move generation specifically.
pub mod util;

/// Generates legal moves. Contains lookup tables for each piece which are
/// initialised at startup.
pub struct Movegen {
    pawn_attacks: [[Bitboard; Nums::SQUARES]; Nums::SIDES],
    knight_attacks: [Bitboard; Nums::SQUARES],
    king_attacks: [Bitboard; Nums::SQUARES],
    bishop_magic_lookup: [Bitboard; BISHOP_SIZE],
    rook_magic_lookup: [Bitboard; ROOK_SIZE],
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
    pub fn new() -> Movegen {
        let mut mg = Movegen {
            pawn_attacks: [[Bitboards::EMPTY; Nums::SQUARES]; Nums::SIDES],
            knight_attacks: [Bitboards::EMPTY; Nums::SQUARES],
            king_attacks: [Bitboards::EMPTY; Nums::SQUARES],
            bishop_magic_lookup: [Bitboards::EMPTY; BISHOP_SIZE],
            rook_magic_lookup: [Bitboards::EMPTY; ROOK_SIZE],
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
    pub fn gen_all_sliding_attacks(square: Square, piece: Piece, attacks: &mut [Bitboard; 4096]) {
        let edges =
            ((Bitboards::FILE_BB[Files::FILE1] | Bitboards::FILE_BB[Files::FILE8])
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
        let bishop_directions = [ Directions::NE, Directions::SE, Directions::SW, Directions::NW ];
        let rook_directions = [ Directions::N, Directions::E, Directions::S, Directions::W ];
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
        self.generate_pawn_moves(board, ml);
        self.generate_non_sliding_moves(board, ml);
        self.generate_sliding_moves(board, ml);
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
                // adds 8 if the side is White (0) or subtracts 8 if Black (1)
                let pushed = as_bitboard(square + 8 - side * 16);
                *bb = east(pushed) | west(pushed);
            }
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

    /// Initialises the magic lookup tables with attacks and initialises a
    /// [`Magic`] object for each square.
    fn init_magics(&mut self) {}
}

impl Movegen {
    /// Generates all legal knight and king moves for `board` and puts them in
    /// `ml`.
    fn generate_non_sliding_moves(&self, board: &Board, ml: &mut Movelist) {
        let us = board.side_to_move;
        let us_bb = board.sides[us];

        let mut knights = board.pieces[Pieces::KNIGHT] & us_bb;
        while knights != 0 {
            let knight = pop_lsb(&mut knights);
            let mut targets = self.knight_attacks[to_square(knight)] & !us_bb;
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(to_square(knight), target, Pieces::KNIGHT, us));
            }
        }

        let mut kings = board.pieces[Pieces::KING] & us_bb;
        while kings != 0 {
            let king = pop_lsb(&mut kings);
            let mut targets = self.king_attacks[to_square(king)] & !us_bb;
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(to_square(king), target, Pieces::KING, us));
            }
        }
    }

    /// Generates all legal pawn moves for `board` and puts them in `ml`.
    fn generate_pawn_moves(&self, board: &Board, ml: &mut Movelist) {
        let us = board.side_to_move;
        let us_bb = board.sides[us];
        let them_bb = board.sides[1 - us];
        let empty = !(us_bb | them_bb);
        let mut pawns = board.pieces[Pieces::PAWN] & us_bb;
        while pawns != 0 {
            let pawn = pop_lsb(&mut pawns);
            /* Learned this rotate left trick from Rustic -
             * <https://github.com/mvanthoor/rustic>
             * Since `0xdeadbeef.rotate_left(64 + x) == 0xdeadbeef` where
             * x = 0, you can set x to any positive or negative number to
             * effectively shift left or right respectively by x's magnitude,
             * as long as it doesn't overflow. I'm unsure how fast this is
             * compared to C++-style generics, but performance is not an issue
             * yet.
             */
            let single_push = pawn.rotate_left(72 - (us as u32) * 16) & empty;
            let captures = self.pawn_attacks[us][to_square(pawn)] & them_bb;
            let mut targets = single_push | captures;
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(to_square(pawn), target, Pieces::PAWN, us));
            }
        }
    }

    /// Generates all legal bishop, rook and queen moves for `board` and puts
    /// them in `ml`.
    fn generate_sliding_moves(&self, board: &Board, ml: &mut Movelist) {
        let us = board.side_to_move;
        let us_bb = board.sides[us];
        let them_bb = board.sides[1 - us];
        let occupancies = us_bb | them_bb;

        let mut bishops = board.pieces[Pieces::BISHOP] & us_bb;
        while bishops != 0 {
            let bishop = pop_lsb(&mut bishops);
            let bishop_sq = to_square(bishop);
            let mut targets = self.bishop_magic_lookup
                [self.bishop_magics[bishop_sq].get_table_index(occupancies)]
                & !us_bb;
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(bishop_sq, target, Pieces::BISHOP, us));
            }
        }

        let mut rooks = board.pieces[Pieces::ROOK] & us_bb;
        while rooks != 0 {
            let rook = pop_lsb(&mut rooks);
            let rook_sq = to_square(rook);
            let mut targets = self.rook_magic_lookup
                [self.rook_magics[rook_sq].get_table_index(occupancies)]
                & !us_bb;
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(rook_sq, target, Pieces::ROOK, us));
            }
        }

        let mut queens = board.pieces[Pieces::ROOK] & us_bb;
        while queens != 0 {
            let queen = pop_lsb(&mut queens);
            let queen_sq = to_square(queen);
            let mut targets = self.rook_magic_lookup
                [self.rook_magics[queen_sq].get_table_index(occupancies)]
                | self.bishop_magic_lookup
                    [self.bishop_magics[queen_sq].get_table_index(occupancies)]
                    & !us_bb;
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(queen_sq, target, Pieces::QUEEN, us));
            }
        }
    }
}
