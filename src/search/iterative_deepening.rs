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

use std::sync::atomic::Ordering;

use super::{AspirationWindow, Depth, Height, Pv, Worker};
use crate::{movegen::Move, search::SearchStatus};

impl Worker<'_> {
    /// Performs iterative deepening on the given board.
    ///
    /// Returns the best move.
    pub(super) fn iterative_deepening(&mut self) -> Move {
        let mut asp_window = AspirationWindow::new();
        let mut pv = Pv::new();

        // `Step` is unstable
        for depth in 1..=Depth::MAX.0 {
            let depth = Depth(depth);

            self.seldepth = Height::default();

            let score = self.aspiration_loop(&mut pv, &mut asp_window, depth);

            if self.should_stop(depth) {
                break;
            }

            asp_window.adjust_around(score, depth);
        }

        if self.is_main_thread() {
            self.state
                .status
                .store(SearchStatus::Stop.into(), Ordering::Relaxed);
        }

        self.root_pv.clear();
        self.root_pv.append_pv(&pv);
        self.nodes.flush();

        *self.root_pv.iter().next().expect("null best move")
    }
}
