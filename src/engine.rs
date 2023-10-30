use crate::{
    board::*,
    util::pretty_move,
};

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

    pub fn perft(&mut self, depth: u8) {
        println!("Result:");
        if depth == 0 {
            println!("1");
            return;
        }

        self.board.generate_moves();

        while let Some(result) = self.board.next_move() {
            //make_move();
            println!("{}: {}",
                pretty_move(result),
                self.board.perft(depth - 1));
            //unmake_move();
        }
    }
}
