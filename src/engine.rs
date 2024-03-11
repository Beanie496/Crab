use std::{
    sync::mpsc::{channel, Sender},
    thread::{spawn, JoinHandle},
};

use crate::{
    board::Board,
    perft::perft,
    search::{iterative_deepening, SearchInfo, Stop},
    uci::Limits,
};

/// Master object that contains all the other major objects.
pub struct Engine {
    /// The internal board.
    ///
    /// See [`Board`].
    board: Board,
    /// A tramsmitter to the search thread to tell it to stop and a join handle
    /// to the same thread.
    search_thread_state: Option<ThreadState<Stop, ()>>,
}

/// Used to lump together a transmitter and a join handle into the same
/// [`Option`].
#[allow(clippy::missing_docs_in_private_items)]
pub struct ThreadState<Tx, Handle> {
    tx: Sender<Tx>,
    handle: JoinHandle<Handle>,
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

    /// Start the search. Runs to infinity if `depth == None`, otherwise runs
    /// to depth `Some(depth)`.
    pub fn start_search(&mut self, limits: Limits) {
        let board_clone = self.board.clone();
        let (control_tx, control_rx) = channel();

        let search_info = SearchInfo::new(control_rx, limits);

        self.stop_search();

        self.search_thread_state = Some(ThreadState::new(
            control_tx,
            spawn(move || {
                iterative_deepening(search_info, &board_clone);
            }),
        ));
    }

    /// Stops the search, if any.
    ///
    /// # Panics
    ///
    /// Panics if `self` couldn't join on the search thread.
    pub fn stop_search(&mut self) {
        // we don't particularly care if it's already stopped, we just want it
        // to stop.
        #[allow(unused_must_use)]
        if let Some(state) = self.search_thread_state.take() {
            state.tx.send(Stop);
            #[allow(clippy::unwrap_used)]
            state.handle.join().unwrap();
        }
    }

    /// Calls [`pretty_print`](Board::pretty_print) on the internal board.
    pub fn pretty_print_board(&self) {
        self.board.pretty_print();
    }

    /// Runs [`perft`] with the given parameters and the current
    /// board.
    pub fn perft<const SHOULD_PRINT: bool, const IS_TIMED: bool>(&self, depth: u8) -> u64 {
        perft::<SHOULD_PRINT, IS_TIMED>(&self.board.clone(), depth)
    }
}

impl<T, U> ThreadState<T, U> {
    /// Creates a new [`ThreadState`] from a transmitter and handle.
    pub const fn new(tx: Sender<T>, handle: JoinHandle<U>) -> Self {
        Self { tx, handle }
    }
}
