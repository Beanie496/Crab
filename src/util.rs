use oorandom::Rand64;

use crate::defs::{Bitboard, Direction, File, Files, Move, Rank, Ranks, Square, Squares};

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
        if self.board == 0 {
            None
        } else {
            Some(pop_next_square(&mut self.board))
        }
    }
}

/// Calculates the file that `square` is on.
pub fn file_of(square: Square) -> File {
    square & 7
}

/// Generates a random number with 1/8 of its bits set on average.
pub fn gen_sparse_rand(rand_gen: &mut Rand64) -> u64 {
    rand_gen.rand_u64() & rand_gen.rand_u64() & rand_gen.rand_u64()
}

/// Finds the horizontal distance between `square_1` and `square_2`
pub fn horizontal_distance(square_1: Square, square_2: Square) -> u8 {
    (file_of(square_1) as i8 - file_of(square_2) as i8).unsigned_abs()
}

/// Checks if `square` can go in the given direction.
pub fn is_valid<const DIRECTION: Direction>(square: Square) -> bool {
    let dest = square.wrapping_add(DIRECTION as usize);
    // credit to Stockfish, as I didn't come up with this code.
    // It checks if `square` is still within the board, and if it is, it checks
    // if it hasn't wrapped (because if it has wrapped, the distance will be
    // larger than 2).
    is_valid_square(dest) && horizontal_distance(square, dest) <= 1
}

/// Checks if `square` is within the board.
pub fn is_valid_square(square: Square) -> bool {
    // `square` is a usize so it can't be less than 0.
    square <= Squares::H8
}

// Allowed dead code because this is occasionally useful for debugging.
#[allow(dead_code)]
/// Pretty prints a given bitboard.
pub fn pretty_print(board: Bitboard) {
    for r in (Ranks::RANK1..=Ranks::RANK8).rev() {
        for f in Files::FILE1..=Files::FILE8 {
            print!("{} ", (board & bitboard_from_pos(r, f) != 0) as u8);
        }
        println!();
    }
    println!();
}

/// Calculates the rank that `square` is on.
pub fn rank_of(square: Square) -> Rank {
    square >> 3
}

/// Converts `mv` into its string representation.
pub fn stringify_move(mv: Move) -> String {
    let start = mv & 0x3f;
    let end = (mv >> 6) & 0x3f;
    let mut ret = String::with_capacity(4);
    ret += &stringify_square(start as Square);
    ret += &stringify_square(end as Square);
    ret
}

/// Converts `sq` into its string representation.
pub fn stringify_square(sq: Square) -> String {
    let mut ret = String::with_capacity(2);
    ret.push((b'a' + (sq as u8 & 7)) as char);
    ret.push((b'1' + (sq as u8 >> 3)) as char);
    ret
}

/// Converts `square` into its corresponding bitboard.
pub fn as_bitboard(square: Square) -> Bitboard {
    1 << square
}

/// Converts `rank` and `file` into a bitboard with the bit in the given
/// position set.
pub fn bitboard_from_pos(rank: Rank, file: File) -> Bitboard {
    1 << (rank * 8 + file)
}

/// Clears the least significant bit of `bb` and returns it.
pub fn pop_lsb(bb: &mut Bitboard) -> Bitboard {
    let popped_bit = *bb & bb.wrapping_neg();
    *bb ^= popped_bit;
    popped_bit
}

/// Clears the least significant bit of `bb` and returns the position of it.
pub fn pop_next_square(bb: &mut Bitboard) -> Square {
    let shift = bb.trailing_zeros();
    *bb ^= 1 << shift;
    shift as Square
}

/// Finds the position of the least significant bit of `bb`.
pub fn to_square(bb: Bitboard) -> Square {
    bb.trailing_zeros() as Square
}
