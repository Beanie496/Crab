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

#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    bitboard::Bitboard,
    defs::{PieceType, Rank, Square},
};

/// A C-style `for` loop to allow easier looping in `const` functions.
// idea for this is from viridithas
#[macro_export]
macro_rules! cfor {
    ($stmt:stmt; $cond:expr; $expr:expr; $body:block) => {{
        $stmt
        while ($cond) {
            $body;
            $expr;
        }
    }}
}

/// A buffered atomic counter.
///
/// Will only update the internal atomic counter if the internal buffer reaches
/// some value.
pub struct BufferedAtomicU64Counter<'a> {
    /// Where the increments are stored before being flushed to the counter.
    buffer: u64,
    /// The atomic counter.
    counter: &'a AtomicU64,
}

impl BufferedAtomicU64Counter<'_> {
    /// How large the buffer can be before it flushes to the atomic counter.
    const BUFFER_SIZE: u64 = 2048;
}

impl<'a> BufferedAtomicU64Counter<'a> {
    /// Creates a new [`BufferedAtomicU64Counter`].
    pub const fn new(counter: &'a AtomicU64) -> Self {
        Self { buffer: 0, counter }
    }

    /// Increments the buffer, flushing it to the atomic counter if it's too
    /// large.
    pub fn increment(&mut self) {
        self.buffer += 1;
        if self.buffer > Self::BUFFER_SIZE {
            self.flush();
        }
    }

    /// Clears the buffer.
    pub const fn clear(&mut self) {
        self.buffer = 0;
    }

    /// Flushes the buffer to the atomic counter.
    pub fn flush(&mut self) {
        self.counter.fetch_add(self.buffer, Ordering::Relaxed);
        self.buffer = 0;
    }

    /// Returns the total number of increments.
    pub fn count(&self) -> u64 {
        self.counter.load(Ordering::Relaxed) + self.buffer
    }

    /// Checks if the buffer is empty.
    pub const fn has_empty_buffer(&self) -> bool {
        self.buffer == 0
    }
}

/// A wrapper over [`get_unchecked()`], but asserts in debug mode that `index`
/// is within `array`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub fn get_unchecked<T>(array: &[T], index: usize) -> &T {
    debug_assert!(
        index < array.len(),
        "Attempted to index out of bounds: {} >= {}",
        index,
        array.len()
    );
    // SAFETY: we just checked `index` is valid
    unsafe { array.get_unchecked(index) }
}

/// Inserts `item` at `index` into `array` without bounds checking.
///
/// In debug mode, it will assert that `index` is within `array`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub fn insert_unchecked<T>(array: &mut [T], index: usize, item: T) {
    debug_assert!(
        index < array.len(),
        "Attempted to index out of bounds: {} >= {}",
        index,
        array.len()
    );
    // SAFETY: we just checked `index` is valid
    unsafe { *array.get_unchecked_mut(index) = item }
}

/// Checks if the given piece type moving from the given start square to the
/// given end square is a double pawn push.
pub fn is_double_pawn_push(start: Square, end: Square, piece_type: PieceType) -> bool {
    if piece_type != PieceType::PAWN {
        return false;
    }
    let start_bb = Bitboard::from(start);
    let end_bb = Bitboard::from(end);
    if (start_bb & (Bitboard::rank_bb(Rank::RANK2) | Bitboard::rank_bb(Rank::RANK7))).is_empty() {
        return false;
    }
    if (end_bb & (Bitboard::rank_bb(Rank::RANK4) | Bitboard::rank_bb(Rank::RANK5))).is_empty() {
        return false;
    }
    true
}
