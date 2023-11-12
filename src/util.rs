use crate::{
    bits::bitboard_from_pos,
    defs::{Bitboard, Direction, File, Files, Move, Rank, Ranks, Square, Squares},
};
use oorandom::Rand64;

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
pub fn is_valid(square: Square, direction: Direction) -> bool {
    let dest = square.wrapping_add(direction as usize);
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
            if board & bitboard_from_pos(r, f) != 0 {
                print!("1 ");
            } else {
                print!("0 ");
            }
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

#[cfg(test)]
mod tests {
    use super::{horizontal_distance, is_valid};

    use crate::defs::{Directions, Squares};

    #[test]
    fn horizontal_distance_works() {
        assert_eq!(horizontal_distance(Squares::A1, Squares::A8), 0);
        assert_eq!(horizontal_distance(Squares::A1, Squares::H8), 7);
        assert_eq!(horizontal_distance(Squares::A1, Squares::H1), 7);
    }

    #[test]
    fn is_valid_works() {
        assert!(is_valid(Squares::A1, Directions::N));
        assert!(is_valid(Squares::A1, Directions::E));
        assert!(!is_valid(Squares::A1, Directions::S));
        assert!(!is_valid(Squares::A1, Directions::W));
    }
}
