use crate::{
    board::Board,
    defs::{ Bitboard, Bitboards, Nums, Pieces, Sides },
    movelist::Movelist,
    util::{ create_move, east, pop_lsb, pop_next_square, square_of, to_square, west },
};

/// Generates and stores all legal moves on the current board state.
pub struct Movegen {
    pawn_attacks: [[Bitboard; Nums::SQUARES as usize]; Nums::SIDES as usize],
}

impl Movegen {
    /// Returns a new Movegen object with an empty list.
    pub fn new() -> Movegen {
        let mut mg = Movegen {
            pawn_attacks: [[Bitboards::EMPTY; Nums::SQUARES as usize]; Nums::SIDES as usize],
        };
        mg.init_pawn_attacks();
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
                    .take((Nums::SQUARES - Nums::FILES) as usize)
                    .enumerate()
                    .skip(Nums::FILES as usize) {
                // adds 8 if the side is White (0) or subtracts 8 if Black (1)
                let push = 1u64 << (square + 8 - side * 16);
                *bb = east(push) | west(push);
            }
        }
    }
}

impl Movegen {
    /// Given an immutable reference to a Board object, generate all legal
    /// moves and put them in the given movelist.
    pub fn generate_moves(&self, board: &Board, ml: &mut Movelist) {
        self.generate_pawn_moves(board, ml);
    }

    fn generate_pawn_moves(&self, board: &Board, ml: &mut Movelist) {
        let us = board.side_to_move;
        let them_bb = board.sides[(1 - us) as usize];
        let empty = !(board.sides[Sides::WHITE as usize] | board.sides[Sides::BLACK as usize]);
        let mut pawns = board.pieces[Pieces::PAWN as usize] & board.sides[us as usize];
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
            let single_push = pawn.rotate_left((72 - us * 16) as u32) & empty;
            let captures = self.pawn_attacks[us as usize][square_of(pawn) as usize] & them_bb;
            let mut targets = single_push | captures;
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(to_square(pawn), target, Pieces::PAWN, us));
            }
        }
    }
}
