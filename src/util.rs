use std::mem::MaybeUninit;

use crate::out_of_bounds_is_unreachable;

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

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.pop()
    }
}

impl<T: Copy, const SIZE: usize> Stack<T, SIZE> {
    /// Creates an empty [`Stack`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            stack: [MaybeUninit::uninit(); SIZE],
            first_empty: 0,
        }
    }

    /// Pushes an item onto the stack.
    #[inline]
    pub fn push(&mut self, item: T) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(self.first_empty, self.stack.len()) };
        self.stack[self.first_empty] = MaybeUninit::new(item);
        self.first_empty += 1;
    }

    /// Pops an item off the stack. Returns `Some(move)` if there are `> 0`
    /// items, otherwise returns `None`.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        (self.first_empty > 0).then(|| {
            self.first_empty -= 1;
            // SAFETY: If it does get reached, it will panic in debug.
            unsafe { out_of_bounds_is_unreachable!(self.first_empty, self.stack.len()) };
            // SAFETY: It is not possible for `first_empty` to index into
            // uninitialised memory
            unsafe { self.stack[self.first_empty].assume_init_read() }
        })
    }

    /// Clears `self`.
    #[inline]
    pub fn clear(&mut self) {
        self.first_empty = 0;
    }

    /// Returns a mutable slice to all of the elements of the stack.
    // TODO: make this return a slice of T
    pub fn get_mut_slice(&mut self) -> &mut [MaybeUninit<T>] {
        &mut self.stack[0..self.first_empty]
    }
}
