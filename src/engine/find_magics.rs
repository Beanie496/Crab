use oorandom::Rand64;

use super::Engine;
use crate::{
    board::movegen::{magic::MAX_BLOCKERS, Movegen},
    defs::{Bitboards, Files, Nums, Piece, Pieces, Ranks},
    util::{file_of, gen_sparse_rand, rank_of},
};

impl Engine {
    /// Finds magic numbers for all 64 squares for both the rook and bishop.
    pub fn find_magics(piece: Piece) {
        let piece_str = if piece == Pieces::BISHOP {
            "bishop"
        } else if piece == Pieces::ROOK {
            "rook"
        } else {
            panic!("piece not a rook or bishop");
        };

        // this stores the attacks for each square
        let mut attacks = [Bitboards::EMPTY; MAX_BLOCKERS];
        // this is used to check if any collisions are destructive
        let mut lookup_table = [Bitboards::EMPTY; MAX_BLOCKERS];
        // this is used to store the latest iteration of each index
        let mut epoch = [0u32; MAX_BLOCKERS];
        let mut rand_gen: Rand64 = Rand64::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        );

        for square in 0..Nums::SQUARES {
            let edges = ((Bitboards::FILE_BB[Files::FILE1] | Bitboards::FILE_BB[Files::FILE8])
                & !Bitboards::FILE_BB[file_of(square)])
                | ((Bitboards::RANK_BB[Ranks::RANK1] | Bitboards::RANK_BB[Ranks::RANK8])
                    & !Bitboards::RANK_BB[rank_of(square)]);
            let mask = Movegen::sliding_attacks(square, piece, Bitboards::EMPTY) & !edges;
            let mask_bits = mask.count_ones();
            let perms = 2usize.pow(mask_bits);
            let shift = 64 - mask_bits;
            Movegen::gen_all_sliding_attacks(square, piece, &mut attacks);

            let mut count = 0;
            // this repeatedly generates a sparse random number and tests it on
            // all different permutations. If the magic number works, it's
            // printed and the loop is exited.
            loop {
                let sparse_rand = gen_sparse_rand(&mut rand_gen);
                let mut blockers = mask;
                let mut found = true;

                for attack in attacks.iter().take(perms) {
                    let index = blockers.wrapping_mul(sparse_rand) >> shift;
                    /* Each time an index is made, it's checked to see if it's
                     * collided with one of its previous indexes. If it hasn't
                     * (i.e. epoch[index] < count), the index is marked as
                     * being visited (i.e. epoch[index] = count) and the loop
                     * continues. If it has, it checks to see if the collision
                     * is constructive. If it's not, the magic doesn't work -
                     * discard the magic and start the loop over. I've borrowed
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
                    blockers = blockers.wrapping_sub(1) & mask;
                }
                if found {
                    println!("Found magic for {piece_str}: {sparse_rand}");
                    break;
                }
                count += 1;
            }
        }
    }
}
