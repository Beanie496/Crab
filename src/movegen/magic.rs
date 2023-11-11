use crate::defs::Bitboard;

/// Stores magic information for a square:
#[derive(Clone, Copy, Default)]
pub struct Magic {
    /// The magic number.
    pub magic: u64,
    /// The relevant attacked squares, excluding the edge.
    pub mask: Bitboard,
    /// Where in the table the lookups are.
    // u16 (0-65535) is slightly too small for the rook table (102,400)
    pub offset: u32,
    /// The bits required to index into the lookup table - it's the number of
    /// permutations of blockers, excluding the edge (since it makes no
    /// difference whether or not there is a piece on the edge).
    pub shift: u32,
}

impl Magic {
    /// Calculates the index into the table it is for. See
    /// <https://www.chessprogramming.org/Magic_Bitboards> for an explanation.
    pub fn get_table_index(&self, mut occupancies: Bitboard) -> usize {
        occupancies &= self.mask;
        occupancies = occupancies.wrapping_mul(self.magic);
        occupancies >>= self.shift;
        (occupancies + self.offset as u64) as usize
    }
}
