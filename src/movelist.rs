use crate::defs::Move;

/// There is no basis to this number other than 'yeah that seems good enough`.
const MAX_GAME_MOVES: usize = 250;

/// A wrapper around an array of moves.
pub struct Movelist {
    moves: [Move; MAX_GAME_MOVES],
    first_empty: usize,
}

impl Movelist {
    /// Creates an empty [`Movelist`].
    pub fn new() -> Movelist {
        Movelist {
            moves: [0; MAX_GAME_MOVES],
            first_empty: 0,
        }
    }
}

impl Movelist {
    /// Pops a [`Move`] from the move list. Returns `Some(move)` if there are `> 0`
    /// moves, otherwise returns `None`.
    pub fn pop_move(&mut self) -> Option<Move> {
        (self.first_empty > 0).then(|| {
            self.first_empty -= 1;
            self.moves[self.first_empty]
        })
    }

    /// Pushes `mv` onto itself. Panics if it is already full.
    pub fn push_move(&mut self, mv: Move) {
        if self.first_empty < MAX_GAME_MOVES {
            self.moves[self.first_empty] = mv;
            self.first_empty += 1;
        } else {
            panic!("Pushing a move onto an already-full move list.");
        }
    }
}

impl Iterator for Movelist {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop_move()
    }
}
