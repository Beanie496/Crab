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

use std::{cmp::Ordering, mem::MaybeUninit};

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

/// An iterator over the elements of a [`Stack`].
pub struct Iter<'a, T: Copy, const SIZE: usize> {
    /// The stack being iterated over.
    stack: &'a Stack<T, SIZE>,
    /// The index of the first item.
    ///
    /// Equal to `first_empty` if there are no items.
    first_item: usize,
    /// The first index out of the stack.
    ///
    /// Equal to `first_item` if there are no items.
    first_empty: usize,
}

/// A generic stack.
///
/// The point of this is to custom-make my own methods. Since this is a binary
/// crate, I can do questionable things like `unreachable_unchecked` for some
/// bounds checking without worrying about screwing over users.
#[derive(Clone)]
pub struct Stack<T: Copy, const SIZE: usize> {
    /// The internal array.
    stack: [MaybeUninit<T>; SIZE],
    /// The first index that can be written to.
    first_empty: usize,
}

impl<T: Copy, const SIZE: usize> DoubleEndedIterator for Iter<'_, T, SIZE> {
    fn next_back(&mut self) -> Option<Self::Item> {
        (self.first_item < self.first_empty).then(|| {
            self.first_empty -= 1;
            self.stack.get(self.first_empty)
        })
    }
}

impl<T: Copy, const SIZE: usize> Iterator for Iter<'_, T, SIZE> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        (self.first_item < self.first_empty).then(|| {
            let item = self.stack.get(self.first_item);
            self.first_item += 1;
            item
        })
    }
}

impl<T: Copy, const SIZE: usize> FromIterator<T> for Stack<T, SIZE> {
    fn from_iter<U: IntoIterator<Item = T>>(other_stack: U) -> Self {
        let mut stack = Self::new();

        for item in other_stack {
            stack.push(item);
        }

        stack
    }
}

impl<T: Copy, const SIZE: usize> Iterator for Stack<T, SIZE> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop()
    }
}

impl<'a, T: Copy, const SIZE: usize> Iter<'a, T, SIZE> {
    /// Creates a new [`Iter`] over a stack.
    const fn new(stack: &'a Stack<T, SIZE>) -> Self {
        Self {
            stack,
            first_item: 0,
            first_empty: stack.len(),
        }
    }
}

impl<T: Copy, const SIZE: usize> Stack<T, SIZE> {
    /// Creates an empty [`Stack`].
    pub const fn new() -> Self {
        Self {
            stack: [MaybeUninit::uninit(); SIZE],
            first_empty: 0,
        }
    }

    /// Pushes an item onto the stack.
    pub fn push(&mut self, item: T) {
        insert_unchecked(&mut self.stack, self.first_empty, MaybeUninit::new(item));
        self.first_empty += 1;
    }

    /// Pops an item off the stack. Returns `Some(move)` if there are `> 0`
    /// items, otherwise returns `None`.
    pub fn pop(&mut self) -> Option<T> {
        (self.first_empty > 0).then(|| {
            self.first_empty -= 1;
            let item = *get_unchecked(&self.stack, self.first_empty);
            // SAFETY: It is not possible for `first_empty` to index into
            // uninitialised memory
            unsafe { item.assume_init_read() }
        })
    }

    /// Returns the top item of the stack.
    ///
    /// Assumes that there is at least one item in the stack.
    pub fn peek(&self) -> T {
        let item = *get_unchecked(&self.stack, self.first_empty - 1);
        // SAFETY: `get_unchecked()` makes sure that the index is to within the
        // stack (i.e. initialised memory)
        unsafe { item.assume_init_read() }
    }

    /// Gets the item at the given index.
    ///
    /// Will panic in debug if the index is invalid.
    fn get(&self, index: usize) -> T {
        let item = *get_unchecked(&self.stack, index);
        // SAFETY: `get_unchecked()` makes sure that the index is to within the
        // stack (i.e. initialised memory)
        unsafe { item.assume_init_read() }
    }

    /// Clears the stack.
    pub fn clear(&mut self) {
        self.first_empty = 0;
    }

    /// Returns the height of the stack.
    pub const fn len(&self) -> usize {
        self.first_empty
    }

    /// Sorts the items in the stack with the comparator function, `cmp`.
    pub fn sort_by<F>(&mut self, mut cmp: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.stack[0..self.first_empty].sort_by(|a, b| {
            // SAFETY: only the initialised items are sorted
            cmp(&unsafe { a.assume_init_read() }, &unsafe {
                b.assume_init_read()
            })
        });
    }

    /// Returns a non-consuming iterator over the stack.
    pub const fn iter(&self) -> Iter<'_, T, SIZE> {
        Iter::new(self)
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
