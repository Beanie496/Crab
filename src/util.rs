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
