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

use core::arch::x86_64::{_mm_prefetch, _MM_HINT_ET0};
use std::{
    mem::{size_of, transmute},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    board::Key,
    evaluation::{CompressedEvaluation, Evaluation},
    movegen::Move,
    search::{CompressedDepth, Depth, Height},
    util::get_unchecked,
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
    /// The static evaluation of the position.
    static_eval: CompressedEvaluation,
    /// The score of the position.
    score: CompressedEvaluation,
    /// The best move in the position.
    mv: Option<Move>,
    /// The depth at which the score was obtained.
    depth: CompressedDepth,
    /// The bound of the score.
    bound: Bound,
}

/// The information from a successful transposition table lookup.
#[derive(Clone, Copy)]
pub struct TranspositionHit {
    /// The static evaluation of the position.
    static_eval: Evaluation,
    /// The score of the position.
    score: Evaluation,
    /// The best move in the position.
    mv: Option<Move>,
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
    pub fn new(
        key: Key,
        static_eval: Evaluation,
        score: Evaluation,
        mv: Option<Move>,
        depth: Depth,
        bound: Bound,
        height: Height,
    ) -> Self {
        Self {
            key,
            static_eval: static_eval.into(),
            score: normalise(score, height).into(),
            mv,
            depth: depth.into(),
            bound,
        }
    }

    /// Checks if a given key matches the stored key.
    const fn matches(self, key: Key) -> bool {
        self.key == key
    }
}

impl TranspositionHit {
    /// Creates a new [`TranspositionHit`].
    fn new(
        static_eval: Evaluation,
        score: Evaluation,
        mv: Option<Move>,
        depth: Depth,
        bound: Bound,
        height: Height,
    ) -> Self {
        Self {
            static_eval,
            score: denormalise(score, height),
            mv,
            depth,
            bound,
        }
    }

    /// Returns the static evaluation.
    pub const fn static_eval(self) -> Evaluation {
        self.static_eval
    }

    /// Returns the score.
    pub const fn score(self) -> Evaluation {
        self.score
    }

    /// Returns the best move.
    pub const fn mv(self) -> Option<Move> {
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

    /// Prefetches the entry with the given key into all levels of cache.
    pub fn prefetch(&self, key: Key) {
        #[cfg(target_arch = "x86_64")]
        {
            let index = self.index(key);
            let pointer = get_unchecked(self.tt(), index).as_ptr();
            // SAFETY: there isn't anything particularly dangerous about this,
            // and `pointer` is always valid
            unsafe { _mm_prefetch(pointer.cast(), _MM_HINT_ET0) }
        }
    }

    /// Returns the entry with the given key, or [`None`] if it doesn't exist.
    pub fn load(&self, key: Key, height: Height) -> Option<TranspositionHit> {
        let atomic_entry = get_unchecked(self.tt(), self.index(key));
        let upper_bits = atomic_entry[0].load(Ordering::Relaxed);
        let lower_bits = atomic_entry[1].load(Ordering::Relaxed);
        // XOR trick again - see comments in `Self::store()`
        let entry = TranspositionEntry::from([upper_bits ^ lower_bits, lower_bits]);

        entry.matches(key).then_some(TranspositionHit::new(
            entry.static_eval.into(),
            entry.score.into(),
            entry.mv,
            entry.depth.into(),
            entry.bound,
            height,
        ))
    }

    /// Stores an entry.
    ///
    /// It uses the 'always-replace' strategy.
    pub fn store(&self, entry: TranspositionEntry) {
        let atomic_entry = get_unchecked(self.tt(), self.index(entry.key));
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
fn normalise(score: Evaluation, height: Height) -> Evaluation {
    if score <= -Evaluation::MATE_BOUND {
        score - Evaluation::from(height)
    } else if score >= Evaluation::MATE_BOUND {
        score + Evaluation::from(height)
    } else {
        score
    }
}

/// If `score` is a mate score, assume it is a mate score relative to the
/// current node and turn it into a mate score relative to the root node.
fn denormalise(score: Evaluation, height: Height) -> Evaluation {
    if score <= -Evaluation::MATE_BOUND {
        score + Evaluation::from(height)
    } else if score >= Evaluation::MATE_BOUND {
        score - Evaluation::from(height)
    } else {
        score
    }
}
