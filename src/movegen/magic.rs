use crate::defs::Bitboard;

/// Stores magic information for a square:
/// - `magic` is the magic number
/// - `mask` masks the occupancies of the whole board to only the attacked
/// squares
/// - offset is where in the table the lookups are
/// - shift is the bits required to index that lookup - it's the number of
/// squares attacked.
#[derive(Clone, Copy, Default)]
pub struct Magic {
    // calling the magic a u64 since it's just a number, not a bitboard
    pub magic: u64,
    pub mask: Bitboard,
    // u16 (0-65535) is slightly too small for the rook table (102,400)
    pub offset: u32,
    pub shift: u8,
}

impl Magic {
    /// Uses the magic information for a square to give the index into the
    /// table it is for. See <https://www.chessprogramming.org/Magic_Bitboards>
    /// for an explanation.
    pub fn get_table_index(&self, mut occupancies: Bitboard) -> usize {
        occupancies &= self.mask;
        occupancies *= self.magic;
        occupancies >>= self.shift as u32;
        (occupancies + self.offset as u64) as usize
    }
}
