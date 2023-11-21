use crate::board::{movegen::Movegen, Board};

mod find_magics;
mod perft;
mod search;

/// Master object that contains all the other major objects.
pub struct Engine {
    board: Board,
    mg: Movegen,
}

impl Engine {
    /// Creates a new [`Engine`] with each member struct initialised to their
    /// default values.
    pub fn new() -> Self {
        Self {
            board: Board::new(),
            mg: Movegen::new(),
        }
    }
}

impl Engine {
    /// Pretty-prints the current state of the board.
    pub fn pretty_print_board(&self) {
        self.board.pretty_print();
    }

    /// Sets `self.board` to its starting position.
    pub fn set_startpos(&mut self) {
        self.board = Board::new();
    }
}
