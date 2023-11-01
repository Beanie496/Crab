use crate::{
    board::Board,
};

pub struct Engine {
    board: Board,
}

impl Engine {
    /// Returns a new Engine object initialised with default values of each
    /// member struct.
    pub fn new() -> Engine {
        Engine {
            board: Board::new(),
        }
    }

    pub fn pretty_print_board(&self) {
        self.board.pretty_print();
    }

    /// Runs perft on the current position. It gives the number of positions for
    /// each legal move on the current board or just prints "1" if it's called
    /// on depth 0.
    pub fn perft(&mut self, depth: u8) {
        println!("Result:");
        if depth == 0 {
            println!("1");
            return;
        }
        self.board.perft(depth - 1);
    }
}
