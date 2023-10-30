use crate::defs::*;

#[allow(dead_code)]
pub fn pretty_print(board: Bitboard) {
    for r in (Ranks::RANK1..=Ranks::RANK8).rev() {
        for f in Files::FILE1..=Files::FILE8 {
            if board & (1 << (r * 8 + f)) != 0 {
                print!("1 ");
            } else {
                print!("0 ");
            }
        }
        println!("");
    }
    println!("");
}

pub fn pop_next_square(bb: &mut Bitboard) -> u8 {
    let shift: u8 = bb.trailing_zeros() as u8;
    *bb ^= 1u64 << shift;
    return shift;
}
