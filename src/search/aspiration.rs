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

use super::{Depth, Height, Pv, RootNode, SearchStatus, Worker};
use crate::evaluation::Evaluation;

/// An aspiration window: a set of bounds used for the search and updated if
/// the returned score fails high or low.
pub struct AspirationWindow {
    /// The lower bound.
    alpha: Evaluation,
    /// The upper bound.
    beta: Evaluation,
    /// How much higher or lower the bounds should be set above the returned
    /// score (if the score is a bound).
    margin: Evaluation,
}

impl AspirationWindow {
    /// The highest a margin can be before it increases.
    const MARGIN_LIMIT: Evaluation = Evaluation(700);
}

impl AspirationWindow {
    /// Returns a new [`AspirationWindow`] with infinite bounds and no margin.
    pub fn new() -> Self {
        Self {
            alpha: -Evaluation::INFINITY,
            beta: Evaluation::INFINITY,
            margin: Evaluation::default(),
        }
    }

    /// Adjusts the aspiration window around the score.
    pub(super) fn adjust_around(&mut self, score: Evaluation, depth: Depth) {
        let unbounded_margin =
            Evaluation(60) / Evaluation::from(depth.min(Depth(3))) + score * score / 3_000;

        self.margin = unbounded_margin;
        self.alpha = score - self.margin;
        self.beta = score + self.margin;
    }

    /// Returns the lower bound.
    fn alpha(&self) -> Evaluation {
        self.alpha.max(-Evaluation::INFINITY)
    }

    /// Returns the upper bound.
    fn beta(&self) -> Evaluation {
        self.beta.min(Evaluation::INFINITY)
    }

    /// Increases the upper bound to above the given score.
    fn widen_up(&mut self, score: Evaluation) {
        if self.margin > Self::MARGIN_LIMIT {
            self.beta = Evaluation::INFINITY;
            return;
        }
        self.margin *= 2;

        self.beta = score + self.margin;
    }

    /// Checks if the upper bound can be increased.
    fn can_widen_up(&self) -> bool {
        self.beta() < Evaluation::INFINITY
    }

    /// Lowers the lower bound to below the given score.
    fn widen_down(&mut self, score: Evaluation) {
        if self.margin > Self::MARGIN_LIMIT {
            self.alpha = -Evaluation::INFINITY;
            return;
        }
        self.margin *= 2;

        self.beta = (self.alpha + self.beta) / 2;
        self.alpha = score - self.margin;
    }

    /// Checks if the lower bound can be lowered.
    fn can_widen_down(&self) -> bool {
        self.alpha() > -Evaluation::INFINITY
    }
}

impl Worker<'_> {
    /// Runs the aspiration loop on the given board.
    ///
    /// See <https://www.chessprogramming.org/Aspiration_Windows>.
    /// `pv` does not need to be empty.
    pub fn aspiration_loop(
        &mut self,
        pv: &mut Pv,
        asp_window: &mut AspirationWindow,
        depth: Depth,
    ) -> Evaluation {
        let board = self.board;

        loop {
            let score = self.search::<RootNode>(
                pv,
                &board,
                asp_window.alpha(),
                asp_window.beta(),
                depth,
                Height::default(),
                false,
            );

            if self.can_print {
                self.nodes.flush();
                self.print_report(score, pv, depth);
            }

            if self.check_status() != SearchStatus::Continue {
                break score;
            }

            // fail-low
            if score <= asp_window.alpha() && asp_window.can_widen_down() {
                asp_window.widen_down(score);
                continue;
            }

            // fail-high
            if score >= asp_window.beta() && asp_window.can_widen_up() {
                asp_window.widen_up(score);
                continue;
            }

            // exact score
            break score;
        }
    }
}
