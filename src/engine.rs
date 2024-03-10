use crate::board::Board;
use search::{Stop, ThreadState};

/// For perft, as it's counting leaf nodes, not searching.
mod perft;
/// For the search.
mod search;

/// Master object that contains all the other major objects.
#[non_exhaustive]
pub struct Engine {
    /// The internal board.
    ///
    /// See [`Board`].
    board: Board,
    /// A tramsmitter to the search thread to tell it to stop and a join handle
    /// to the same thread.
    search_thread_state: Option<ThreadState<Stop, ()>>,
}

impl Clone for Engine {
    fn clone(&self) -> Self {
        Self {
            board: self.board.clone(),
            search_thread_state: None,
        }
    }
}

impl Engine {
    /// Creates a new [`Engine`] with each member struct initialised to their
    /// default values.
    pub fn new() -> Self {
        Self {
            board: Board::new(),
            search_thread_state: None,
        }
    }

    /// Sets `self.board` to the given FEN and moves, as given by the
    /// `position` UCI command. Unexpected/incorrect tokens will be ignored.
    pub fn set_position(&mut self, position: &str, moves: &str) {
        self.board.set_pos_to_fen(position);
        self.board.play_moves(moves);
    }

    /// Calls [`pretty_print`](Board::pretty_print) on the internal board.
    pub fn pretty_print_board(&self) {
        self.board.pretty_print();
    }
}
