use crate::{
    defs::Move,
};

const MAX_LEGAL_MOVES: usize = 512;

/// Generates and stores all legal moves on the current board state.
pub struct Movegen {
    moves: [Move; MAX_LEGAL_MOVES],
    first_empty: usize,
}

impl Movegen {
    /// Returns a new Movegen object with an empty list.
    pub fn new() -> Movegen {
        Movegen {
            moves: [0; MAX_LEGAL_MOVES],
            first_empty: 0,
        }
    }
}
