use crate::{
    board::{find_magics, Board},
    defs::Piece,
};

mod perft;
mod search;

/// Master object that contains all the other major objects.
pub struct Engine {
    board: Board,
}

impl Engine {
    /// Creates a new [`Engine`] with each member struct initialised to their
    /// default values.
    pub fn new() -> Self {
        Self {
            board: Board::new(),
        }
    }

    /// Wrapper for [`find_magics`].
    pub fn find_magics<const PIECE: Piece>() {
        find_magics::<PIECE>();
    }
}

impl Engine {
    /// Pretty-prints the current state of the board.
    pub fn pretty_print_board(&self) {
        self.board.pretty_print();
    }

    /// Resets `self.board`.
    pub fn set_startpos(&mut self) {
        self.board.set_startpos();
    }
}
