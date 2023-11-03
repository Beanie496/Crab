use crate::{
    board::Board,
    defs::{ Pieces, Sides },
    movelist::Movelist,
    util::{ create_move, pop_lsb, pop_next_square, to_square },
};

/// Generates and stores all legal moves on the current board state.
pub struct Movegen {}

impl Movegen {
    /// Returns a new Movegen object with an empty list.
    pub fn new() -> Movegen {
        Movegen {}
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
            let mut targets = single_push; // more targets to come
            while targets != 0 {
                let target = pop_next_square(&mut targets);
                ml.push_move(create_move(to_square(pawn), target, Pieces::PAWN, us));
            }
        }
    }
}
