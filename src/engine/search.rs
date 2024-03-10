use std::{
    fmt::{self, Display, Formatter},
    sync::mpsc::{channel, Receiver, Sender},
    thread::{spawn, JoinHandle},
    time::{Duration, Instant},
};

use super::Engine;
use crate::{
    board::{Board, Move},
    evaluation::{Eval, INF_EVAL},
};
use alpha_beta::alpha_beta_search;

/// For carrying out the search.
mod alpha_beta;
/// Move ordering.
mod ordering;

/// The principle variation: the current best sequence of moves for both sides.
// 512 bytes
#[derive(Clone, Copy)]
struct Pv {
    /// A non-circular queue of moves.
    ///
    /// All the moves are enqueued exactly once before all the moves are
    /// dequeued exactly once (then it goes out of scope)
    moves: [Move; MAX_PLY],
    /// Index of the first [`Move`]. This will be equal to `first_empty` if
    /// there are no [`Move`]s.
    first_item: u8,
    /// Index of the first empty space.
    first_empty: u8,
}

/// An iterator over a [`Pv`].
#[allow(clippy::missing_docs_in_private_items)]
struct PvIter {
    pv: Pv,
}

/// Information about a search.
pub struct SearchInfo {
    /// The depth to be searched.
    depth: u8,
    /// How long the search has been going.
    time: Duration,
    /// How many positions have been searched.
    nodes: u64,
    /// The principle variation: the optimal sequence of moves for both sides.
    pv: Pv,
    // ignore this for now
    //_multipv: [[Move]],
    /// The score of the position from the perspective of the side to move.
    score: Eval,
    /// Which move is currently being searched at root.
    _currmove: Move,
    /// Which move number is currently being searched at root.
    _currmovenumber: u8,
    /// How full the transposition table is (in permill).
    _hashfull: u16,
    /// How many positions have been reached on average per second.
    nps: u64,
    /// A channel to receive the 'stop' command from.
    control_rx: Receiver<Stop>,
    /// Whether or not the search has received the 'stop' command.
    has_stopped: bool,
    /// The previous PV.
    history: Pv,
}

/// Used to tell the search thread to stop.
pub struct Stop;

/// Used to lump together a transmitter and a join handle into the same
/// [`Option`].
#[allow(clippy::missing_docs_in_private_items)]
pub struct ThreadState<Tx, Handle> {
    tx: Sender<Tx>,
    handle: JoinHandle<Handle>,
}

/// The maximum depth this engine supports.
const MAX_PLY: usize = u8::MAX as usize;

impl Display for Pv {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut ret_str = String::with_capacity(self.len());
        for mv in 0..self.len() {
            ret_str.push_str(&self.get(mv).to_string());
            ret_str.push(' ');
        }
        ret_str.pop();
        write!(f, "{ret_str}")
    }
}

impl IntoIterator for Pv {
    type Item = Move;
    type IntoIter = PvIter;

    fn into_iter(self) -> Self::IntoIter {
        PvIter::new(self)
    }
}

impl Iterator for PvIter {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.pv.dequeue()
    }
}

impl Display for SearchInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "info depth {} time {} nodes {} pv {} score cp {} nps {}",
            self.depth,
            self.time.as_millis(),
            self.nodes,
            self.pv,
            self.score,
            self.nps,
        )
    }
}

impl Engine {
    /// Start the search. Runs to infinity if `depth == None`, otherwise runs
    /// to depth `Some(depth)`.
    pub fn start_search(&mut self, depth: Option<u8>) {
        let board_clone = self.board.clone();
        let depth = depth.unwrap_or(u8::MAX);
        let (control_tx, control_rx) = channel();

        let search_info = SearchInfo::new(control_rx);

        // The inner thread spawned runs the iterative deepening loop. It sends
        // the information to `info_rx`. The outer thread spawned prints any
        // information received through `info_rx`, blocking until it does. When
        // the search finishes, both threads terminate.
        // I don't like spawning two threads, but I don't really have a choice
        // if I don't want users of my API not to have to implement
        // parallelism.
        self.search_thread_state = Some(ThreadState::new(
            control_tx,
            spawn(move || {
                iterative_deepening(search_info, &board_clone, depth);
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
}

impl PvIter {
    /// Creates a new [`PvIter`].
    // this function will be inlined anyway
    #[allow(clippy::large_types_passed_by_value)]
    const fn new(pv: Pv) -> Self {
        Self { pv }
    }
}

impl Pv {
    /// Returns a new 0-initialised [`Pv`].
    const fn new() -> Self {
        Self {
            moves: [Move::null(); MAX_PLY],
            first_item: 0,
            first_empty: 0,
        }
    }

    /// Appends the [`Move`]s from `other_pv` to `self`.
    fn append_pv(&mut self, other_pv: &Self) {
        // NOTE: `collect_into()` would be a more ergonomic way to do this,
        // but that's currently nightly
        for mv in other_pv.into_iter() {
            self.enqueue(mv);
        }
    }

    /// Adds a [`Move`] to the back of `self`.
    fn enqueue(&mut self, mv: Move) {
        self.moves[self.first_empty as usize] = mv;
        self.first_empty += 1;
    }

    /// Removes a [`Move`] from the front of `self`.
    fn dequeue(&mut self) -> Option<Move> {
        (self.first_item < self.first_empty).then(|| {
            let mv = self.moves[self.first_item as usize];
            self.first_item += 1;
            mv
        })
    }

    /// Clears all moves from `self`.
    ///
    /// This only sets a couple of variables, so it's basically free.
    fn clear(&mut self) {
        self.first_item = 0;
        self.first_empty = 0;
    }

    /// Gets the [`Move`] at the given index.
    ///
    /// Useful for read-only iteration.
    const fn get(&self, index: usize) -> Move {
        self.moves[index]
    }

    /// Calculates the length of `self`.
    fn len(&self) -> usize {
        usize::from(self.first_empty - self.first_item)
    }
}

impl SearchInfo {
    /// Creates a new [`SearchInfo`] with the initial information that searches
    /// start with.
    const fn new(control_rx: Receiver<Stop>) -> Self {
        Self {
            depth: 0,
            time: Duration::ZERO,
            nodes: 0,
            pv: Pv::new(),
            score: 0,
            _currmove: Move::null(),
            _currmovenumber: 1,
            _hashfull: 0,
            nps: 0,
            control_rx,
            has_stopped: false,
            history: Pv::new(),
        }
    }

    /// If the search needs to stop.
    fn should_stop(&mut self) -> bool {
        if self.control_rx.try_recv().is_ok() {
            self.has_stopped = true;
        }
        self.has_stopped
    }
}

impl<T, U> ThreadState<T, U> {
    /// Creates a new [`ThreadState`] from a transmitter and handle.
    const fn new(tx: Sender<T>, handle: JoinHandle<U>) -> Self {
        Self { tx, handle }
    }
}

/// Performs iterative deepening.
///
/// Since there is no move ordering or TT, this is currently a slowdown.
fn iterative_deepening(mut search_info: SearchInfo, board: &Board, max_depth: u8) {
    let time = Instant::now();
    let alpha = -INF_EVAL;
    let beta = INF_EVAL;
    let mut best_move = Move::null();

    for depth in 1..=max_depth {
        search_info.pv.clear();
        search_info.depth += 1;

        let eval = alpha_beta_search(&mut search_info, &board.clone(), -beta, -alpha, depth);

        if search_info.has_stopped {
            // if the search gets stopped before even depth 1 gets finished,
            // make the best move whatever the best move in the search was
            if depth == 1 {
                best_move = search_info.pv.get(0);
            }
            break;
        }

        best_move = search_info.pv.get(0);
        search_info.time = time.elapsed();
        search_info.score = eval;
        search_info.nps = 1_000_000 * search_info.nodes / search_info.time.as_micros() as u64;

        println!("{search_info}");

        search_info.history = search_info.pv;
    }
    println!("bestmove {best_move}");
}
