/*
 * Crab, a UCI-compatible chess engine
 * Copyright (C) 2024 Jasper Shovelton
 *
 * Crab is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Crab is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Crab. If not, see <https://www.gnu.org/licenses/>.
 */

use std::{
    mem::{size_of, transmute},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    board::Key,
    evaluation::{Eval, MATE_BOUND},
    movegen::Move,
    search::Depth,
};

/// The bound of a score depending on how it was obtained.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Bound {
    /// A lower bound: `best_score >= beta`.
    Lower,
    /// An exact bound: `alpha < best_score < beta`.
    Exact,
    /// An upper bound: `best_score <= alpha`.
    Upper,
}

/// A single entry in a transposition table.
///
/// It contains a key as a checksum and various other fields that are useful in
/// future identical positions.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct TranspositionEntry {
    /// The key, used as a checksum.
    key: Key,
    /// The score of the position.
    score: Eval,
    /// The best move in the position.
    mv: Move,
    /// The depth at which the score was obtained.
    depth: Depth,
    /// The bound of the score.
    bound: Bound,
    /// The compiler would do this anyway but I want the location to be
    /// explicit.
    _padding: u16,
}

/// The information from a successful transposition table lookup.
#[derive(Clone, Copy)]
pub struct TranspositionHit {
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
    tt: Vec<[AtomicU64; 2]>,
}

impl From<[u64; 2]> for TranspositionEntry {
    fn from(raw_entry: [u64; 2]) -> Self {
        // SAFETY: there is no `[u64; 2]` that is an invalid
        // `TranspositionEntry`, even if the entry doesn't make much sense
        unsafe { transmute::<[u64; 2], Self>(raw_entry) }
    }
}

impl From<TranspositionEntry> for [u64; 2] {
    fn from(entry: TranspositionEntry) -> Self {
        // SAFETY: all fields are integral types
        unsafe { transmute::<TranspositionEntry, Self>(entry) }
    }
}

impl TranspositionEntry {
    /// Creates a new [`TranspositionEntry`] with the given attributes.
    pub fn new(key: Key, score: Eval, mv: Move, depth: Depth, bound: Bound, height: Depth) -> Self {
        Self {
            key,
            score: normalise(score, height),
            mv,
            depth,
            bound,
            _padding: 0,
        }
    }

    /// Checks if a given key matches the stored key.
    const fn matches(self, key: Key) -> bool {
        self.key == key
    }
}

impl TranspositionHit {
    /// Creates a new [`TranspositionHit`].
    fn new(score: Eval, mv: Move, depth: Depth, bound: Bound, height: Depth) -> Self {
        Self {
            score: denormalise(score, height),
            mv,
            depth,
            bound,
        }
    }

    /// Returns the score.
    pub const fn score(self) -> Eval {
        self.score
    }

    /// Returns the best move.
    pub const fn mv(self) -> Move {
        self.mv
    }

    /// Returns the depth at which the score was obtained.
    pub const fn depth(self) -> Depth {
        self.depth
    }

    /// Returns the bound of the score.
    pub const fn bound(self) -> Bound {
        self.bound
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
        let entries = size_mib * 1024 * 1024 / size_of::<TranspositionEntry>();
        *self.tt_mut() = Vec::with_capacity(entries);
        for _ in 0..entries {
            self.tt_mut().push([AtomicU64::new(0), AtomicU64::new(0)]);
        }
    }

    /// Zeroes the table.
    pub fn clear(&mut self) {
        for entry in self.tt_mut() {
            *entry[0].get_mut() = 0;
            *entry[1].get_mut() = 0;
        }
    }

    /// Returns the entry with the given key, or [`None`] if it doesn't exist.
    pub fn load(&self, key: Key, height: Depth) -> Option<TranspositionHit> {
        // SAFETY: `index()` is guaranteed to be a valid index
        let atomic_entry = unsafe { self.tt().get_unchecked(self.index(key)) };
        let upper_bits = atomic_entry[0].load(Ordering::Relaxed);
        let lower_bits = atomic_entry[1].load(Ordering::Relaxed);
        // XOR trick again - see comments in `Self::store()`
        let entry = TranspositionEntry::from([upper_bits ^ lower_bits, lower_bits]);

        entry.matches(key).then_some(TranspositionHit::new(
            entry.score,
            entry.mv,
            entry.depth,
            entry.bound,
            height,
        ))
    }

    /// Stores an entry with the given key.
    ///
    /// It uses the 'always-replace' strategy.
    pub fn store(&self, key: Key, entry: TranspositionEntry) {
        // SAFETY: `index()` is guaranteed to be a valid index
        let atomic_entry = unsafe { self.tt().get_unchecked(self.index(key)) };
        let bits: [u64; 2] = entry.into();
        let upper_bits = bits[0];
        let lower_bits = bits[1];

        // This uses the XOR trick to avoid hits on corrupted entries:
        // https://web.archive.org/web/20201106232343/https://www.craftychess.com/hyatt/hashing.html
        atomic_entry[0].store(upper_bits ^ lower_bits, Ordering::Relaxed);
        atomic_entry[1].store(lower_bits, Ordering::Relaxed);
    }

    /// Estimates how full the hash is, per mille.
    pub fn estimate_hashfull(&self) -> usize {
        self.tt()
            .iter()
            .take(1000)
            // the non-key information in an entry is never 0: if the move is
            // `Move::null()`, the bound must be `Bound::Upper` which is > 0,
            // and if the bound is `Bound::Lower` (0), the move cannot be
            // `Move::null()`
            .filter(|entry| entry[1].load(Ordering::Relaxed) != 0)
            .count()
    }

    /// Converts a key into a valid index.
    fn index(&self, key: Key) -> usize {
        // this maps the key from range 0..2.pow(64) to 0..self.tt().len(), with
        // the same uniform distribution
        ((u128::from(key) * self.tt().len() as u128) >> 64) as usize
    }

    /// Returns a reference to the internal vector of entries.
    const fn tt(&self) -> &Vec<[AtomicU64; 2]> {
        &self.tt
    }

    /// Returns a mutable reference to the internal vector of entries.
    fn tt_mut(&mut self) -> &mut Vec<[AtomicU64; 2]> {
        &mut self.tt
    }
}

/// If `score` is a mate score, assume it is a mate score relative to the root
/// node and turn it in to a mate score relative to the current node.
fn normalise(score: Eval, height: Depth) -> Eval {
    if score <= -MATE_BOUND {
        score - Eval::from(height)
    } else if score >= MATE_BOUND {
        score + Eval::from(height)
    } else {
        score
    }
}

/// If `score` is a mate score, assume it is a mate score relative to the
/// current node and turn it into a mate score relative to the root node.
fn denormalise(score: Eval, height: Depth) -> Eval {
    if score <= -MATE_BOUND {
        score + Eval::from(height)
    } else if score >= MATE_BOUND {
        score - Eval::from(height)
    } else {
        score
    }
}
