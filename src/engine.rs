use crate::board::*;

pub struct Engine {
    board: Board,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            board: Board::new(),
        }
    }

    pub fn pretty_print_board(&self) {
        self.board.pretty_print();
    }
}

