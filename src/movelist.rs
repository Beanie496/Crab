use crate::defs::*;

const MAX_MOVES: usize = 250;

pub struct Movelist {
    moves: [Move; MAX_MOVES],
    first_empty: usize,
}

impl Movelist {
    pub fn new() -> Movelist {
        Movelist {
            moves: [0; MAX_MOVES],
            first_empty: 0,
        }
    }

    pub fn push_move(&mut self, start: u8, end: u8) {
        self.moves[self.first_empty] = start as Move| ((end as Move) << 6);
        self.first_empty += 1;
    }

    fn pop_move(&mut self) -> Option<Move> {
        if self.first_empty > 0 {
            self.first_empty -= 1;
            Some(self.moves[self.first_empty])
        } else {
            None
        }
    }
}

impl Iterator for Movelist {
    type Item = Move;
    fn next (&mut self) -> Option<Self::Item> {
        self.pop_move()
    }
}
