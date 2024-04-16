use std::{
    alloc::{alloc_zeroed, Layout},
    mem::{size_of, transmute},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{board::Key, evaluation::Eval, movegen::Move, search::Depth};

/// The bound of a score depending on how it was obtained.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Bound {
    /// A lower bound: `best_score >= beta`.
    Lower,
    /// An exact bound: `alpha < best_score < beta`.
    Exact,
    /// A lower bound: `best_score <= alpha`
    Upper,
}

/// A single entry in a transposition table.
///
/// It contains a key as a checksum and various other fields that are useful in
/// future identical positions.
#[derive(Clone)]
#[repr(C)]
pub struct TranspositionEntry {
    /// The lowest bits of the key, used as a checksum.
    key: u16,
    /// The score of the position.
    score: Eval,
    /// The best move in the position.
    mv: Move,
    /// The depth at which the score was obtained.
    depth: Depth,
    /// The bound of the score.
    bound: Bound,
}

/// A transposition table: a hash of previous board positions and information
/// about each position.
#[allow(clippy::missing_docs_in_private_items)]
pub struct TranspositionTable {
    tt: Vec<AtomicU64>,
}

impl From<u64> for TranspositionEntry {
    fn from(raw_entry: u64) -> Self {
        // SAFETY: there is no `u64` that is an invalid `TranspositionEntry`,
        // even if the entry doesn't make much sense
        unsafe { transmute::<u64, Self>(raw_entry) }
    }
}

impl From<TranspositionEntry> for u64 {
    fn from(entry: TranspositionEntry) -> Self {
        // SAFETY: all fields are integral types
        unsafe { transmute::<TranspositionEntry, Self>(entry) }
    }
}

impl TranspositionEntry {
    /// Creates a new [`TranspositionEntry`] with the given attributes.
    pub const fn new(key: Key, score: Eval, mv: Move, bound: Bound, depth: Depth) -> Self {
        Self {
            key: key as u16,
            score,
            mv,
            bound,
            depth,
        }
    }

    /// Checks if a given key matches the stored key.
    const fn matches(&self, key: Key) -> bool {
        self.key == key as u16
    }

    /// Returns the score.
    pub const fn score(&self) -> Eval {
        self.score
    }

    /// Returns the best move.
    pub const fn mv(&self) -> Move {
        self.mv
    }

    /// Returns the bound of the score.
    pub const fn bound(&self) -> Bound {
        self.bound
    }

    /// Returns the depth at which the score was obtained.
    pub const fn depth(&self) -> Depth {
        self.depth
    }
}

impl TranspositionTable {
    /// Creates a new, empty, zero-sized [`TranspositionTable`].
    pub const fn new() -> Self {
        Self { tt: Vec::new() }
    }

    /// Creates a new, zeroed [`Transposition table`] with the given size in
    /// MiB.
    pub fn with_capacity(size: usize) -> Self {
        let mut tt = Self::new();
        tt.resize(size);
        tt
    }

    /// Resizes the the table to the given size in MiB and zeroes it.
    pub fn resize(&mut self, size_mib: usize) {
        let size_bytes = size_mib * 1024 * 1024 / size_of::<TranspositionEntry>();
        let layout = Layout::array::<AtomicU64>(size_bytes).expect("size of TT is too large");
        // SAFETY: `layout` has a non-zero size
        let ptr = unsafe { alloc_zeroed(layout) }.cast();
        // SAFETY: the pointer is directly from an correct allocation, `size`
        // <= `size` and a too-large size would have caused a panic earlier
        *self.tt_mut() = unsafe { Vec::from_raw_parts(ptr, size_bytes, size_bytes) };
    }

    /// Zeroes the table.
    pub fn clear(&mut self) {
        for entry in self.tt_mut() {
            *entry.get_mut() = 0;
        }
    }

    /// Returns the entry with the given key, or [`None`] if it doesn't exist.
    pub fn load(&self, key: Key) -> Option<TranspositionEntry> {
        // SAFETY: `index()` is guaranteed to be a valid index
        let atomic_entry = unsafe { self.tt().get_unchecked(self.index(key)) };
        let entry = TranspositionEntry::from(atomic_entry.load(Ordering::Relaxed));
        entry.matches(key).then_some(entry)
    }

    /// Stores an entry with the given key.
    pub fn store(&self, key: Key, entry: TranspositionEntry) {
        // SAFETY: `index()` is guaranteed to be a valid index
        let atomic_entry = unsafe { self.tt().get_unchecked(self.index(key)) };
        // this follows the 'always-replace' strategy
        atomic_entry.store(u64::from(entry), Ordering::Relaxed);
    }

    /// Converts a key into a valid index.
    fn index(&self, key: Key) -> usize {
        // this maps the key from range 0..2.pow(64) to 0..self.tt().len(), with
        // the same uniform distribution
        ((u128::from(key) * self.tt().len() as u128) >> 64) as usize
    }

    /// Returns a reference to the internal vector of entries.
    const fn tt(&self) -> &Vec<AtomicU64> {
        &self.tt
    }

    /// Returns a mutable reference to the internal vector of entries.
    fn tt_mut(&mut self) -> &mut Vec<AtomicU64> {
        &mut self.tt
    }
}
