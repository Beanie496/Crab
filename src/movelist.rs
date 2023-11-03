use crate::{
    defs::Move,
};

const MAX_GAME_MOVES: usize = 250;

/// A wrapper around an array of moves.
pub struct Movelist {
    moves: [Move; MAX_GAME_MOVES],
    first_empty: usize,
}

impl Movelist {
    /// Returns a Movelist object with an empty move list.
    pub fn new() -> Movelist {
        Movelist {
            moves: [0; MAX_GAME_MOVES],
            first_empty: 0,
        }
    }

    /// Pushes a move onto the move list. Panics if the move list is already
    /// full.
    pub fn push_move(&mut self, mv: Move) {
        if self.first_empty < MAX_GAME_MOVES {
            self.moves[self.first_empty] = mv;
            self.first_empty += 1;
        } else {
            panic!("Pushing a move onto an already-full move list.");
        }
    }

    /// Pops a move from the move list. Returns `Some(move)` if there are `> 0`
    /// moves, otherwise returns `None`.
    pub fn pop_move(&mut self) -> Option<Move> {
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

    fn next(&mut self) -> Option<Self::Item> {
        self.pop_move()
    }
}
