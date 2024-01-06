use oorandom::Rand64;

use crate::defs::{Bitboard, Square};

/// A thin wrapper over a [`Bitboard`] to allow iteration over it.
pub struct BitIter {
    board: Bitboard,
}

impl BitIter {
    /// Wraps a [`Bitboard`] in a [`BitIter`].
    pub fn new(bb: Bitboard) -> BitIter {
        Self { board: bb }
    }
}

impl Iterator for BitIter {
    type Item = Square;

    /// Clears the LSB of the wrapped [`Bitboard`] and returns the position of
    /// that bit. Returns [`None`] if there are no set bits.
    fn next(&mut self) -> Option<Self::Item> {
        if self.board.inner() == 0 {
            None
        } else {
            Some(self.board.pop_next_square())
        }
    }
}

/// Generates a random number with 1/8 of its bits set on average.
pub fn gen_sparse_rand(rand_gen: &mut Rand64) -> u64 {
    rand_gen.rand_u64() & rand_gen.rand_u64() & rand_gen.rand_u64()
}
