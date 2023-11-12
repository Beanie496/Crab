use std::time::Instant;

use oorandom::Rand64;

use crate::{
    board::Board,
    defs::{Bitboards, Files, Nums, Piece, Pieces, Ranks},
    movegen::Movegen,
    movelist::Movelist,
    util::{file_of, gen_sparse_rand, rank_of, stringify_move},
};

/// Master object that contains all the other major objects.
pub struct Engine {
    board: Board,
    mg: Movegen,
    /// The current move list, from the starting position (set by the user or
    /// the default start pos) to the current position.
    ml: Movelist,
}

impl Engine {
    /// Creates a new [`Engine`] with each member struct initialised to their
    /// default values.
    pub fn new() -> Engine {
        Engine {
            board: Board::new(),
            mg: Movegen::new(),
            ml: Movelist::new(),
        }
    }
}

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

        /* 4096 is the largest number of attacks from a single square: a rook
        attacking from one of the corners. */
        // this stores the attacks for each square
        let mut attacks = [Bitboards::EMPTY; 4096];
        // this is used to check if any collisions are destructive
        let mut lookup_table = [Bitboards::EMPTY; 4096];
        // this is used to store the latest iteration of each index
        let mut epoch = [0u32; 4096];
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

            let mut sparse_rand: u64;
            let mut count = 0;
            // this repeatedly generates a sparse random number and tests it on
            // all different permutations. If the magic number works, it's
            // printed and the loop is exited.
            loop {
                sparse_rand = gen_sparse_rand(&mut rand_gen);
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

    /// Runs perft on the current position. Prints each move followed by the
    /// number of leaf nodes reaches from that move, or just prints "1" if
    /// `depth == 0`.
    pub fn perft_root(&mut self, depth: u8) {
        println!("Result:");
        if depth == 0 {
            println!("1");
            return;
        }

        let time = Instant::now();
        let mut ml = Movelist::new();
        self.mg.generate_moves(&self.board, &mut ml);

        let mut total = 0;
        for mv in ml {
            self.board.make_move(mv, &mut self.ml);
            let moves = self.perft(depth - 1);
            total += moves;
            println!("{}: {moves}", stringify_move(mv));
            self.board.unmake_move(&mut self.ml);
        }
        println!("Total: {total}");
        let elapsed_ms = time.elapsed().as_millis() as u64;
        println!("Time taken: {} ms; NPS: {}", elapsed_ms, 1000 * total / elapsed_ms);
    }

    /// Pretty-prints the current state of the board.
    pub fn pretty_print_board(&self) {
        self.board.pretty_print();
    }
}

impl Engine {
    /// Runs perft on the current position and returns the number of legal
    /// moves.
    fn perft(&mut self, depth: u8) -> u64 {
        if depth == 0 {
            return 1;
        }

        let mut ml = Movelist::new();
        self.mg.generate_moves(&self.board, &mut ml);

        let mut total = 0;
        for mv in ml {
            self.board.make_move(mv, &mut self.ml);
            total += self.perft(depth - 1);
            self.board.unmake_move(&mut self.ml);
        }
        total
    }
}
