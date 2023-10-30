// Move = 0b0000000000000000
//          |--||----||----|
// First 6 bits for start pos (0-63)
// Next 6 bits for end pos (0-63)
// Last 4 bits for flags (unused)
type Move = u16;

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
        self.moves[self.first_empty] = (start & (end << 6)) as Move;
        self.first_empty += 1;
    }
}
