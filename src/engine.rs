use crate::{
    board::Board,
    movegen::Movegen,
    movelist::Movelist,
    util::stringify_move,
};

/// Master object that contains all the other major objects.
pub struct Engine {
    board: Board,
    mg: Movegen,
    /// The current move list, from the starting position (set by the user or
    /// the default start pos) to the current position.
    ml: Movelist,
}

impl Engine {
    /// Returns a new Engine object initialised with default values of each
    /// member struct.
    pub fn new() -> Engine {
        Engine {
            board: Board::new(),
            mg: Movegen::new(),
            ml: Movelist::new(),
        }
    }

    pub fn pretty_print_board(&self) {
        self.board.pretty_print();
    }

    /// Runs perft on the current position. It gives the number of positions for
    /// each legal move on the current board or just prints "1" if it's called
    /// on depth 0.
    pub fn perft_root(&mut self, depth: u8) {
        println!("Result:");
        if depth == 0 {
            println!("1");
            return;
        }

        let mut ml = Movelist::new();
        self.mg.generate_moves(&self.board, &mut ml);

        let mut total = 0;
        for mv in ml {
            self.board.make_move(mv, &mut self.ml);
            let moves = self.perft(depth - 1);
            total += moves;
            println!("{}: {moves}", stringify_move(mv));
            self.board.unmake_move(&mut self.ml);
        }
        println!("Total: {total}");
    }

    /// Runs perft on the current position and returns the number of legal
    /// moves.
    pub fn perft(&mut self, depth: u8) -> u64 {
        if depth == 0 {
            return 1;
        }

        let mut ml = Movelist::new();
        self.mg.generate_moves(&self.board, &mut ml);

        let mut total = 0;
        for mv in ml {
            self.board.make_move(mv, &mut self.ml);
            total += self.perft(depth - 1);
            self.board.unmake_move(&mut self.ml);
        }
        total
    }
}
