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

pub fn pretty_move(mv: Move) -> String {
    let start = (mv as u8) & 0x3f;
    let end  = ((mv >> 6) as u8) & 0x3f;
    let mut ret = String::with_capacity(4);
    ret += &square_as_str(start);
    ret += &square_as_str(end);
    ret
}

pub fn square_as_str(sq: u8) -> String {
    let mut ret = String::with_capacity(2);
    ret.push(('a' as u8 + (sq & 7)) as char);
    ret.push(('1' as u8 + (sq >> 3)) as char);
    ret
}

pub fn pop_next_square(bb: &mut Bitboard) -> u8 {
    let shift: u8 = bb.trailing_zeros() as u8;
    *bb ^= 1u64 << shift;
    return shift;
}
