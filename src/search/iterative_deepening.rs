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
    aspiration::{aspiration_loop, AspirationWindow},
    Pv, SearchReferences, SearchStatus,
};
use crate::board::Board;

/// Performs iterative deepening on the given board.
///
/// Returns the number of positions searched.
pub fn iterative_deepening(mut search_refs: SearchReferences<'_>, board: Board) -> u64 {
    let mut asp_window = AspirationWindow::new();
    let mut pv = Pv::new();

    for depth in 1_u8.. {
        search_refs.depth = depth;
        search_refs.seldepth = 0;
        search_refs.status = SearchStatus::Continue;

        let score = aspiration_loop(&mut search_refs, &mut pv, &board, &mut asp_window);

        if search_refs.should_stop() {
            break;
        }

        asp_window.adjust_around(score, depth);
    }

    println!("bestmove {}", pv.get(0));

    if search_refs.check_status() == SearchStatus::Quit {
        exit(0);
    }

    search_refs.nodes
}
