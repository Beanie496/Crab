// Move = 0b0000000000000000
//          |--||----||----|
// First 6 bits for start pos (0-63)
// Next 6 bits for end pos (0-63)
// Last 4 bits for flags (unused)
type Move = u16;

const MAX_MOVES: usize = 250;

pub struct Movelist {
    moves: [Move; MAX_MOVES],
}

impl Movelist {
    pub fn new() -> Movelist {
        Movelist {
            moves: [0; MAX_MOVES]
        }
    }
}
