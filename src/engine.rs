use crate::{
    board::{find_magics, Board},
    defs::Piece,
};

mod perft;
mod search;

/// The result of parsing a FEN string.
pub enum FENResult {
    /// No errors.
    Ok,
}

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

    /// Sets `self.board` to the given FEN. The result will be a [`FENResult`].
    pub fn set_pos_to_fen(&mut self, position: &str, moves: &str) -> FENResult {
        println!("Setting FEN to {position} and applying moves {moves}");
        FENResult::Ok
    }

    /// Resets `self.board`.
    pub fn set_startpos(&mut self) {
        self.board.set_startpos();
    }
}
