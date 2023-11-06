use crate::{
    bits::util::{ east, north, pop_lsb, pop_next_square, south, square_of, to_square, west },
    board::Board,
    defs::{ Bitboard, Bitboards, Nums, Pieces },
    movelist::Movelist,
};
use util::create_move;

pub mod util;

/// Generates and stores all legal moves on the current board state.
pub struct Movegen {
    pawn_attacks: [[Bitboard; Nums::SQUARES]; Nums::SIDES],
    knight_attacks: [Bitboard; Nums::SQUARES],
    king_attacks: [Bitboard; Nums::SQUARES],
}

impl Movegen {
    /// Returns a new Movegen object with an empty list.
    pub fn new() -> Movegen {
        let mut mg = Movegen {
            pawn_attacks: [[Bitboards::EMPTY; Nums::SQUARES]; Nums::SIDES],
            knight_attacks: [Bitboards::EMPTY; Nums::SQUARES],
            king_attacks: [Bitboards::EMPTY; Nums::SQUARES],
        };
        mg.init_pawn_attacks();
        mg.init_knight_attacks();
        mg.init_king_attacks();
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
}

impl Movegen {
    /// Given an immutable reference to a Board object, generate all legal
    /// moves and put them in the given movelist.
    pub fn generate_moves(&self, board: &Board, ml: &mut Movelist) {
        self.generate_pawn_moves(board, ml);
        self.generate_non_sliding_moves(board, ml);
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
}
