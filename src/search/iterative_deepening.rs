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

use std::process::exit;

use super::{
    alpha_beta_search::search, print_report, Pv, RootNode, SearchReferences, SearchStatus,
};
use crate::{board::Board, evaluation::INF_EVAL, movegen::Move};

/// Performs iterative deepening on the given board.
///
/// Returns the number of positions searched.
pub fn iterative_deepening(mut search_refs: SearchReferences<'_>, board: Board) -> u64 {
    let mut pv = Pv::new();
    let mut best_move = Move::null();

    for depth in 1_u8.. {
        search_refs.depth = depth;
        search_refs.seldepth = 0;
        search_refs.status = SearchStatus::Continue;

        let score = search::<RootNode>(
            &mut search_refs,
            &mut pv,
            &board,
            -INF_EVAL,
            INF_EVAL,
            depth,
            0,
        );

        // the root search guarantees that there will always be 1 valid move in
        // the PV
        best_move = pv.get(0);
        let time = search_refs.start.elapsed();

        print_report(&search_refs, time, score, &pv);

        if search_refs.should_stop() {
            break;
        }

        pv.clear();
    }

    println!("bestmove {best_move}");

    if search_refs.check_status() == SearchStatus::Quit {
        exit(0);
    }

    search_refs.nodes
}
