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

use super::{aspiration::AspirationWindow, Depth, Pv, SearchStatus, Worker};

impl Worker<'_> {
    /// Performs iterative deepening on the given board.
    ///
    /// Returns the number of positions searched.
    pub fn iterative_deepening(&mut self) {
        let mut asp_window = AspirationWindow::new();
        let mut pv = Pv::new();

        for depth in 1..Depth::MAX {
            self.seldepth = 0;
            self.status = SearchStatus::Continue;

            let score = self.aspiration_loop(&mut pv, &mut asp_window, depth);

            if self.should_stop(depth) {
                break;
            }

            asp_window.adjust_around(score, depth);
        }

        self.root_pv.clear();
        self.root_pv.append_pv(&mut pv);

        if self.can_print {
            println!("bestmove {}", self.root_pv.get(0));
        }

        if self.check_status() == SearchStatus::Quit {
            exit(0);
        }
    }
}
