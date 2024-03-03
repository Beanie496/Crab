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

/// The result of an iteration of an iterative deepening loop.
#[allow(
    clippy::module_name_repetitions,
    clippy::large_enum_variant,
    clippy::exhaustive_enums
)]
#[derive(Clone)]
pub enum SearchResult {
    /// Information about the search so far (time, nodes, etc.)
    Unfinished(WorkingResult),
    /// The best move.
    Finished(Move),
}

/// The principle variation: the current best sequence of moves for both sides.
// non-circular queue, as all the moves are enqueued exactly once before all
// the moves are dequeued exactly once (then it goes out of scope)
// 512 bytes
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Clone)]
pub struct Pv {
    moves: [Move; MAX_PLY],
    first_item: u8,
    first_empty: u8,
}

/// Information about a search.
#[allow(clippy::module_name_repetitions)]
struct SearchInfo {
    /// The depth to be searched.
    pub depth: u8,
    /// How long the search has been going.
    pub time: Duration,
    /// How many positions have been searched.
    pub nodes: u64,
    /// The principle variation: the optimal sequence of moves for both sides.
    pub pv: Pv,
    // ignore this for now
    //_multipv: [[Move]],
    /// The score of the position from the perspective of the side to move.
    pub score: Eval,
    /// Which move is currently being searched at root.
    pub _currmove: Move,
    /// Which move number is currently being searched at root.
    pub _currmovenumber: u8,
    /// How full the transposition table is (in permill).
    pub _hashfull: u16,
    /// How many positions have been reached on average per second.
    pub nps: u64,
    /// A channel to receive the 'stop' command from.
    pub control_rx: Receiver<Stop>,
    /// Whether or not the search has received the 'stop' command.
    pub has_stopped: bool,
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

/// The current result of a search.
// This is almost identical to `SearchInfo` but doesn't have a receiver. When
// I remove the GUI, this struct will be deleted.
#[allow(clippy::exhaustive_structs)]
#[derive(Clone)]
pub struct WorkingResult {
    /// The depth reached.
    pub depth: u8,
    /// The time taken.
    pub time: Duration,
    /// The positions searched.
    pub nodes: u64,
    /// The principle variation: the optimal sequence of moves for both sides.
    pub pv: Pv,
    /// The score of the position from the perspective of the side to move.
    pub score: Eval,
    /// How many positions were searched per second.
    pub nps: u64,
}

/// The maximum depth this engine supports.
const MAX_PLY: usize = u8::MAX as usize;

impl Display for Pv {
    #[inline]
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

impl Iterator for Pv {
    type Item = Move;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.dequeue()
    }
}

impl Display for SearchResult {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Unfinished(ref result) => {
                write!(
                    f,
                    "info depth {} time {} nodes {} pv {} score cp {} nps {}",
                    result.depth,
                    result.time.as_millis(),
                    result.nodes,
                    result.pv,
                    result.score,
                    result.nps,
                )
            }
            Self::Finished(ref mv) => write!(f, "bestmove {mv}"),
        }
    }
}

impl Engine {
    /// Start the search. Runs to infinity if `depth == None`, otherwise runs
    /// to depth `Some(depth)`.
    // this triggers because of `elapsed_us` and `elapsed_ms`, which are
    // obviously different
    #[allow(clippy::similar_names)]
    #[inline]
    #[must_use]
    pub fn start_search(&mut self, depth: Option<u8>) -> Receiver<SearchResult> {
        let board_clone = self.board.clone();
        let depth = depth.unwrap_or(u8::MAX);
        let (info_tx, info_rx) = channel();
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
                iterative_deepening(search_info, &info_tx, &board_clone, depth);
            }),
        ));
        info_rx
    }

    /// Stops the search, if any.
    ///
    /// # Panics
    ///
    /// Panics if `self` couldn't join on the search thread.
    #[inline]
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

impl Pv {
    /// Returns a new 0-initialised [`Pv`].
    const fn new() -> Self {
        Self {
            // TODO: `MaybeUninit` might be faster?
            moves: [Move::null(); MAX_PLY],
            first_item: 0,
            first_empty: 0,
        }
    }

    /// Appends the [`Move`]s from `other_pv` to `self`.
    fn append_pv(&mut self, other_pv: &mut Self) {
        // NOTE: `collect_into()` would be a more ergonomic way to do this,
        // but that's currently nightly
        for mv in other_pv {
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
        }
    }

    /// Turns the information in `self` into a [`SearchResult`]. The result
    /// will be `Unfinished` if the search has not stopped and `Finished` if it
    /// has.
    fn create_result(&self) -> SearchResult {
        if self.has_stopped {
            SearchResult::Finished(self.pv.get(0))
        } else {
            SearchResult::Unfinished(WorkingResult {
                depth: self.depth,
                time: self.time,
                nodes: self.nodes,
                pv: self.pv.clone(),
                score: self.score,
                nps: self.nps,
            })
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
fn iterative_deepening(
    mut search_info: SearchInfo,
    info_tx: &Sender<SearchResult>,
    board: &Board,
    max_depth: u8,
) {
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
        // the initial call to `alpha_beta_search()` counts the starting
        // position, so remove that count
        search_info.nodes -= 1;
        search_info.time = time.elapsed();
        search_info.score = eval;
        search_info.nps = 1_000_000 * search_info.nodes / search_info.time.as_micros() as u64;
        info_tx
            .send(search_info.create_result())
            .expect("Info receiver dropped too early");
    }
    info_tx
        .send(SearchResult::Finished(best_move))
        .expect("Info receiver dropped too early");
}
