#![allow(dead_code)]

use std::{cmp::Ordering, mem::MaybeUninit};

use crate::{
    bitboard::Bitboard,
    defs::{Piece, PieceType, Rank, Square},
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

/// Gets the item at `index` in `array` without bounds checking.
///
/// In debug mode, it will assert that `index` is within `array`.
#[macro_export]
macro_rules! index_unchecked {
    ($array:expr, $index:expr) => {{
        debug_assert!($index < $array.len(), "Attempted to index out of bounds");
        // SAFETY: we just checked `index` is valid
        unsafe { *$array.get_unchecked($index) }
    }}
}

/// Inserts `item` at `index` into `array` without bounds checking.
///
/// In debug mode, it will assert that `index` is within `array`.
#[macro_export]
macro_rules! index_into_unchecked {
    ($array:expr, $index:expr, $item:expr) => {{
        debug_assert!($index < $array.len(), "Attempted to index out of bounds");
        // SAFETY: we just checked `index` is valid
        unsafe { *$array.get_unchecked_mut($index) = $item }
    }}
}

/// A generic stack.
///
/// The point of this is to custom-make my own methods. Since this is a binary
/// crate, I can do questionable things like `unreachable_unchecked` for some
/// bounds checking without worrying about screwing over users.
pub struct Stack<T: Copy, const SIZE: usize> {
    /// The internal array.
    stack: [MaybeUninit<T>; SIZE],
    /// The first index that can be written to.
    first_empty: usize,
}

impl<T: Copy, const SIZE: usize> Iterator for Stack<T, SIZE> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop()
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
        index_into_unchecked!(self.stack, self.first_empty, MaybeUninit::new(item));
        self.first_empty += 1;
    }

    /// Pops an item off the stack. Returns `Some(move)` if there are `> 0`
    /// items, otherwise returns `None`.
    pub fn pop(&mut self) -> Option<T> {
        (self.first_empty > 0).then(|| {
            self.first_empty -= 1;
            // SAFETY: It is not possible for `first_empty` to index into
            // uninitialised memory
            unsafe { index_unchecked!(self.stack, self.first_empty).assume_init_read() }
        })
    }

    /// Clears `self`.
    pub fn clear(&mut self) {
        self.first_empty = 0;
    }

    /// Sorts the elements in the stack with the comparator function, `cmp`.
    pub fn sort_by<F>(&mut self, mut cmp: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.stack[0..self.first_empty].sort_by(|a, b| {
            // SAFETY: only the initialised elements are sorted
            cmp(&unsafe { a.assume_init_read() }, &unsafe {
                b.assume_init_read()
            })
        });
    }
}

/// Checks if the move is a double pawn push.
pub fn is_double_pawn_push(start: Square, end: Square, piece: Piece) -> bool {
    if PieceType::from(piece) != PieceType::PAWN {
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
