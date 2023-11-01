use crate::defs::Move;

const MAX_MOVES: usize = 250;

/// A wrapper struct for the current move list, from the starting position
/// (set by the user or the default start pos) to the current position.
pub struct Movelist {
    moves: [Move; MAX_MOVES],
    first_empty: usize,
}

impl Movelist {
    /// Returns a Movelist object with an empty move list.
    pub fn new() -> Movelist {
        Movelist {
            moves: [0; MAX_MOVES],
            first_empty: 0,
        }
    }

    /// Pushes a move onto the move list. Panics if the move list is already
    /// full.
    pub fn push_move(&mut self, start: u8, end: u8) {
        if self.first_empty < MAX_MOVES {
            self.moves[self.first_empty] = start as Move| ((end as Move) << 6);
            self.first_empty += 1;
        } else {
            panic!("Pushing a move onto an already-full move list.");
        }
    }

    /// Pops a move from the move list. Returns `Some(move)` if there are `> 0`
    /// moves, otherwise returns `None`.
    fn pop_move(&mut self) -> Option<Move> {
        if self.first_empty > 0 {
            self.first_empty -= 1;
            Some(self.moves[self.first_empty])
        } else {
            None
        }
    }
}
