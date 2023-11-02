use crate::{
    board::Board,
    movelist::Movelist,
};

/// Generates and stores all legal moves on the current board state.
pub struct Movegen {
}

impl Movegen {
    /// Returns a new Movegen object with an empty list.
    pub fn new() -> Movegen {
        Movegen {
        }
    }
}

impl Movegen {
    /// Given an immutable reference to a Board object, generate all legal
    /// moves and put them in the given movelist.
    pub fn generate_moves(&self, board: &Board, ml: &mut Movelist) {
    }
}
