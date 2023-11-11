use crate::defs::{ Bitboard, File, Files, Move, Rank, Ranks, Square };
use oorandom::Rand64;

/// Calculates the file that `square` is on.
pub fn file_of(square: Square) -> File {
    square as u8 & 7
}

/// Generates a random number with 1/8 of its bits set on average.
pub fn gen_sparse_rand(rand_gen: &mut Rand64) -> u64 {
    rand_gen.rand_u64() & rand_gen.rand_u64() & rand_gen.rand_u64()
}

// Allowed dead code because this is occasionally useful for debugging.
#[allow(dead_code)]
/// Pretty prints a given bitboard.
pub fn pretty_print(board: Bitboard) {
    for r in (Ranks::RANK1..=Ranks::RANK8).rev() {
        for f in Files::FILE1..=Files::FILE8 {
            if board & (1 << (r * 8 + f)) != 0 {
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
    square as u8 >> 3
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
