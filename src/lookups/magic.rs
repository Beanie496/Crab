/*
 * Crab, a UCI-compatible chess engine
 * Copyright (C) 2024 Jasper Shovelton
 *
 * Crab is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Crab is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Crab. If not, see <https://www.gnu.org/licenses/>.
 */

use std::time::{SystemTime, UNIX_EPOCH};

use oorandom::Rand64;

use super::util::{gen_all_sliding_attacks, sliding_attacks};
use crate::{
    bitboard::Bitboard,
    defs::{PieceType, Square},
};

/// Magic information about a square.
#[derive(Clone, Copy)]
pub struct Magic {
    /// The relevant attacked squares, excluding the edge.
    mask: Bitboard,
    /// The magic number.
    #[allow(clippy::struct_field_names)]
    magic: u64,
    /// The bits required to index into the lookup table.
    ///
    /// It's the number of permutations of blockers, excluding the edge.
    shift: u32,
    /// Where in the table the lookups are.
    offset: u32,
}

/// The hardcoded magic numbers for the bishop. Generated using [`find_magics()`].
#[allow(clippy::unreadable_literal)]
pub const BISHOP_MAGICS: [u64; Square::TOTAL] = [
    18017181921083777,
    2459251561629761536,
    292753294555095040,
    4756931506604097536,
    325390588419506176,
    1163421009871873,
    586182643679872,
    164104318360814626,
    288265697979597056,
    72415141542694432,
    17609373271040,
    4483954245642,
    4543526717112832,
    433473699698968580,
    288232652635652162,
    29273552566354945,
    4845890933042135552,
    2314850277255061632,
    1125934807650816,
    145170003722240,
    1153352530355290113,
    4616330407351191616,
    10088415017757836288,
    2324011374789005312,
    38284995418131968,
    180709753360762880,
    11534017717204959745,
    1200420682408722944,
    845525015347200,
    9297717991649165832,
    5632256920055809,
    93450517189559296,
    20883144609630212,
    11863062030699495681,
    38104950114304,
    2201238372480,
    218425183222957312,
    73484805228464128,
    1126492646180864,
    28148614966559779,
    10455113338795601936,
    9558610074181050380,
    18027593857016834,
    9246211166542235648,
    9259683477685537152,
    2626181342223663616,
    148658404492058688,
    18335483830469124,
    2306129500781478400,
    1153027624927764486,
    288249071088437280,
    1008810724946345986,
    10416869989473714206,
    36134376072940544,
    4508002103205888,
    9808858732329730048,
    282033366500352,
    37384536852480,
    36592924632354834,
    1243099462589519872,
    38280599248700952,
    9232379923841221184,
    4620763597834879264,
    72638153366733312,
];
/// The maximum number of blocker permutations from a single square: a rook
/// attacking from one of the corners.
pub const MAX_BLOCKERS: usize = 4096;
/// The hardcoded magic numbers for the rook. Generated using [`find_magics()`].
#[allow(clippy::unreadable_literal)]
pub const ROOK_MAGICS: [u64; Square::TOTAL] = [
    36033333578174594,
    10394312406808535040,
    144152572550217736,
    144124052959789088,
    1224981366456189184,
    2449959331228155920,
    9511611219905102082,
    72058693570625574,
    4925951679021057,
    1225612486133161988,
    612630699136020482,
    2392640389795840,
    182536573988372608,
    1442559272822507264,
    2919458493105917956,
    9241949430484648194,
    108089414715965633,
    4503874643697666,
    3459051486628110336,
    4786175325769729,
    1297319267305263108,
    13835340629837613066,
    5352422623023112,
    1152923703656645700,
    1152991875498549376,
    2306125046833233927,
    45036065261879808,
    4507450068699136,
    11260102879481864,
    3096263398654976,
    6053981717696913921,
    4081423669543168,
    6896961571524772,
    4652218552533647424,
    4611827924155244550,
    4644405843593216,
    4611967527780618436,
    562984346726468,
    1162491963086210049,
    36029925018304596,
    9259405369648627712,
    2306197056261324834,
    13519599874154496,
    1297074077193338896,
    1162492068548182024,
    1252563668105822216,
    873702734346387712,
    36337348560158732,
    325730596552832,
    9878287350309120,
    142249317892224,
    144328494139066880,
    617134333115499392,
    577624037753356416,
    255095562896384,
    985163509006464,
    9210059156422721,
    10448633160574042306,
    9367680911313010753,
    4513499795464213,
    1171498887708410882,
    4647996436519061505,
    4556382779486756,
    5836806168129044614,
];

impl Magic {
    /// Returns a [`Magic`] with the given parameters.
    pub const fn new(magic: u64, mask: Bitboard, offset: usize, shift: u32) -> Self {
        Self {
            mask,
            magic,
            shift,
            offset: offset as u32,
        }
    }

    /// Returns a 0-initialised [`Magic`].
    pub const fn null() -> Self {
        Self {
            mask: Bitboard::empty(),
            magic: 0,
            shift: 0,
            offset: 0,
        }
    }

    /// Calculates the index into the table it is for.
    ///
    /// See <https://www.chessprogramming.org/Magic_Bitboards> for an
    /// explanation.
    pub const fn get_table_index(&self, occupancies: Bitboard) -> usize {
        let mut occupancies = occupancies.0;
        occupancies &= self.mask.0;
        occupancies = occupancies.wrapping_mul(self.magic);
        occupancies >>= self.shift;
        occupancies as usize + self.offset as usize
    }
}

/// Finds magic numbers for all 64 squares for both the rook and bishop.
///
/// Panics if the value given for the generic parameter does not match the
/// inner value of a [`PieceType::BISHOP`] or a [`PieceType::ROOK`].
pub fn find_magics<const PIECE: u8>() {
    let piece = PieceType(PIECE);
    let piece_str = if piece == PieceType::BISHOP {
        "bishop"
    } else if piece == PieceType::ROOK {
        "rook"
    } else {
        panic!("piece not a rook or bishop");
    };

    // this stores the attacks for each square
    let mut attacks = [Bitboard::empty(); MAX_BLOCKERS];
    // this is used to check if any collisions are destructive
    let mut lookup_table = [Bitboard::empty(); MAX_BLOCKERS];
    // this is used to store the latest iteration of each index
    let mut epoch = [0_u32; MAX_BLOCKERS];
    let mut rand_gen = Rand64::new(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Your clock is _very_ outdated")
            .as_nanos(),
    );

    for square in 0..Square::TOTAL {
        let square = Square(square as u8);
        let edges = Bitboard::edges_without(square);
        let mask = sliding_attacks::<PIECE>(square, Bitboard::empty()) & !edges;
        let mask_bits = mask.count_ones();
        let perms = 2_usize.pow(mask_bits);
        let shift = 64 - mask_bits;
        gen_all_sliding_attacks::<PIECE>(square, &mut attacks);

        let mut count = 0;
        // this repeatedly generates a sparse random number and tests it on all
        // different permutations. If the magic number works, it's printed and
        // the loop is exited.
        loop {
            // 1/8 of bits set on average
            let sparse_rand = rand_gen.rand_u64() & rand_gen.rand_u64() & rand_gen.rand_u64();
            let mut blockers = mask;
            let mut found = true;

            for attack in attacks.iter().take(perms) {
                let index = blockers.wrapping_mul(sparse_rand) >> shift;
                /* Each time an index is made, it's checked to see if it's
                 * collided with one of its previous indexes. If it hasn't
                 * (i.e. epoch[index] < count), the index is marked as being
                 * visited (i.e. epoch[index] = count) and the loop continues.
                 * If it has, it checks to see if the collision is
                 * constructive. If it's not, the magic doesn't work - it
                 * discards the magic and start the loop over. I've borrowed
                 * this epoch trick from Stockfish.
                 */
                if epoch[index as usize] < count {
                    epoch[index as usize] = count;
                    lookup_table[index as usize] = *attack;
                } else if lookup_table[index as usize] != *attack {
                    found = false;
                    break;
                }
                // Carry-Rippler trick
                blockers = Bitboard(blockers.wrapping_sub(1)) & mask;
            }
            if found {
                println!("Found magic for {piece_str}: {sparse_rand}");
                break;
            }
            count += 1;
        }
    }
}
