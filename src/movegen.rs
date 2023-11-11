use crate::{
    bits::{ east, north, pop_lsb, pop_next_square, south, square_of, to_square, west },
    board::Board,
    defs::{ Bitboard, Bitboards, Direction, Directions, Files, Nums, Piece, Pieces, Ranks, Square },
    movelist::Movelist,
    util::{ file_of, rank_of },
};
use magic::Magic;
use util::create_move;

mod magic;
pub mod util;

/// Generates and stores all legal moves on the current board state.
pub struct Movegen {
    pawn_attacks: [[Bitboard; Nums::SQUARES]; Nums::SIDES],
    knight_attacks: [Bitboard; Nums::SQUARES],
    king_attacks: [Bitboard; Nums::SQUARES],
    bishop_magic_lookup: [Bitboard; BISHOP_SIZE],
    rook_magic_lookup: [Bitboard; ROOK_SIZE],
    bishop_magics: [Magic; Nums::SQUARES],
    rook_magics: [Magic; Nums::SQUARES],
}

/// 12 bits for each corner, 11 for each non-corner edge, 10 for all others.
const ROOK_SIZE: usize = 102_400;
/// Repeated once per quadrant: 6 bits for corner, 5 bits for each non-corner
/// edge and each square adjacent to an edge, 7 bits for the squares adjacent
/// or diagonal to a corner, 9 bits for the corners themselves.
const BISHOP_SIZE: usize = 5_248;

impl Movegen {
    /// Returns a new Movegen object with an empty list.
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
}

impl Movegen {
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
                let push = 1u64 << (square + 8 - side * 16);
                *bb = east(push) | west(push);
            }
        }
    }

    /// Initialises knight attack lookup table.
    fn init_knight_attacks(&mut self) {
        for (square, bb) in self.knight_attacks.iter_mut().enumerate() {
            let knight = 1 << square;
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
            let king = 1 << square;
            let mut attacks = east(king) | west(king) | king;
            attacks |= north(attacks) | south(attacks);
            attacks ^= king;
            *bb = attacks;
        }
    }

    fn init_magics(&mut self) {
    }
}

impl Movegen {
    /// Given an immutable reference to a Board object, generate all legal
    /// moves and put them in the given movelist.
    pub fn generate_moves(&self, board: &Board, ml: &mut Movelist) {
        self.generate_pawn_moves(board, ml);
        self.generate_non_sliding_moves(board, ml);
        self.generate_sliding_moves(board, ml);
    }

    fn generate_pawn_moves(&self, board: &Board, ml: &mut Movelist) {
        let us = board.side_to_move;
        let us_bb = board.sides[us];
        let them_bb = board.sides[1 - us];
        let empty = !(us_bb | them_bb);
        let mut pawns = board.pieces[Pieces::PAWN] & us_bb;
        while pawns != 0 {
            let pawn = pop_lsb(&mut pawns);
            /* Learned this rotate left trick from Rustic -
             * https://github.com/mvanthoor/rustic
             * Since `0xdeadbeef.rotate_left(64 + x) == 0xdeadbeef` where
             * x = 0, you can set x to any positive or negative number to
             * effectively shift left or right respectively by x's magnitude,
             * as long as it doesn't overflow. I'm unsure how fast this is
             * compared to C++-style generics, but performance is not an issue
             * yet.
             */
            let single_push = pawn.rotate_left(72 - (us as u32) * 16) & empty;
            let captures = self.pawn_attacks[us][square_of(pawn)] & them_bb;
            let mut targets = single_push | captures;
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(to_square(pawn), target, Pieces::PAWN, us));
            }
        }
    }

    fn generate_non_sliding_moves(&self, board: &Board, ml: &mut Movelist) {
        let us = board.side_to_move;
        let us_bb = board.sides[us];

        let mut knights = board.pieces[Pieces::KNIGHT] & us_bb;
        while knights != 0 {
            let knight = pop_lsb(&mut knights);
            let mut targets = self.knight_attacks[square_of(knight)] & !us_bb;
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(to_square(knight), target, Pieces::KNIGHT, us));
            }
        }

        let mut kings = board.pieces[Pieces::KING] & us_bb;
        while kings != 0 {
            let king = pop_lsb(&mut kings);
            let mut targets = self.king_attacks[square_of(king)] & !us_bb;
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(to_square(king), target, Pieces::KING, us));
            }
        }
    }

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

impl Movegen {
    /// Generates attacks from a square to the edge in each diagonal direction,
    /// excluding the square and the edge.
    pub fn bishop_mask(
        square: Square,
        ray_attacks: &[[Bitboard; Nums::SQUARES]; Nums::DIRECTIONS]
    ) -> Bitboard {
        let mut rays =
            ray_attacks[Directions::NE][square]
            | ray_attacks[Directions::SE][square]
            | ray_attacks[Directions::SW][square]
            | ray_attacks[Directions::NW][square];
        rays &=
            !(Bitboards::FILE_BB[Files::FILE1 as usize]
              | Bitboards::FILE_BB[Files::FILE8 as usize])
            | Bitboards::FILE_BB[file_of(square) as usize];
        rays &=
            !(Bitboards::RANK_BB[Ranks::RANK1 as usize]
              | Bitboards::RANK_BB[Ranks::RANK8 as usize])
            | Bitboards::RANK_BB[rank_of(square) as usize];
        rays
    }

    /// Generates attacks from a square to the edge in each cardinal direction,
    /// excluding the square and the edge.
    pub fn rook_mask(
        square: Square,
        ray_attacks: &[[Bitboard; Nums::SQUARES]; Nums::DIRECTIONS]
    ) -> Bitboard {
        let mut rays =
            ray_attacks[Directions::N][square]
            | ray_attacks[Directions::E][square]
            | ray_attacks[Directions::S][square]
            | ray_attacks[Directions::W][square];
        rays &=
            !(Bitboards::FILE_BB[Files::FILE1 as usize]
              | Bitboards::FILE_BB[Files::FILE8 as usize])
            | Bitboards::FILE_BB[file_of(square) as usize];
        rays &=
            !(Bitboards::RANK_BB[Ranks::RANK1 as usize]
              | Bitboards::RANK_BB[Ranks::RANK8 as usize])
            | Bitboards::RANK_BB[rank_of(square) as usize];
        rays
    }
}

impl Movegen {
    fn ray_attack(
        square: Square,
        direction: Direction,
        blockers: Bitboard,
        ray_attacks: &[[Bitboard; Nums::SQUARES]; Nums::DIRECTIONS],
    ) -> Bitboard {
        let mut ray = ray_attacks[direction][square];
        let blocker_direction = blockers & ray;
        if blocker_direction == Bitboards::EMPTY {
            return ray;
        }
        // See <https://www.chessprogramming.org/Classical_Approach> - NW to E
        // inclusive have the lsb closest to the square, whereas the other four
        // have the msb closest to the square
        ray ^= if direction <= Directions::E || direction == Directions::NW {
            ray_attacks[direction][blocker_direction.trailing_zeros() as usize]
        } else {
            ray_attacks[direction][blocker_direction.leading_zeros() as usize]
        };
        ray
    }

    pub fn generate_all_ray_attacks(
        square: Square,
        piece: Piece,
        ray_attacks: &[[Bitboard; Nums::SQUARES]; Nums::DIRECTIONS],
        attack_buffer: &mut [Bitboard; 4096],
    ) {
        let mut mask = Bitboards::EMPTY;
        let start = if piece == Pieces::BISHOP { Directions::NE } else { Directions::N };
        for d in (start..(start + Nums::DIRECTIONS)).step_by(2) {
            mask |= ray_attacks[d][square];
        }

        let mut first_empty = 0;
        let mut blockers = mask;
        while blockers != 0 {
            let mut attacks = 0;
            for direction in (start..(start + 8)).step_by(2) {
                attacks |= Self::ray_attack(square, direction, blockers, ray_attacks);
            }
            attack_buffer[first_empty] = attacks;
            // Carry-Rippler trick
            blockers = blockers.wrapping_sub(1) & mask;
            first_empty += 1;
        }
        // the loop above doesn't take into account when `blockers == 0`, so
        // manually add it here: no blockers means the attacks are just the
        // full mask
        attack_buffer[first_empty] = mask;
    }
}
