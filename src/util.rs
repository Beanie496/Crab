use crate::defs::{ Bitboard, File, Files, Move, Ranks, Square };

/// Returns the file of a given Square.
pub fn file_of(square: Square) -> File {
    square as u8 & 7
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

/// Returns a string representation of a move.
pub fn stringify_move(mv: Move) -> String {
    let start = mv & 0x3f;
    let end = (mv >> 6) & 0x3f;
    let mut ret = String::with_capacity(4);
    ret += &stringify_square(start as Square);
    ret += &stringify_square(end as Square);
    ret
}

/// Returns a string representation of a square.
pub fn stringify_square(sq: Square) -> String {
    let mut ret = String::with_capacity(2);
    ret.push((b'a' + (sq as u8 & 7)) as char);
    ret.push((b'1' + (sq as u8 >> 3)) as char);
    ret
}
