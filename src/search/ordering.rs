use super::SearchInfo;
use crate::{
    evaluation::Eval,
    movegen::{Move, Moves, MAX_LEGAL_MOVES},
    util::Stack,
};

/// An ordered [`ScoredMoves`] instance.
#[allow(clippy::missing_docs_in_private_items)]
pub struct OrderedMoves {
    ordered_moves: ScoredMoves,
}

/// A [`Move`] that has been given a certain score.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy)]
pub struct ScoredMove {
    score: Eval,
    mv: Move,
}

/// A stack of [`ScoredMove`]s.
pub type ScoredMoves = Stack<ScoredMove, MAX_LEGAL_MOVES>;

impl Iterator for OrderedMoves {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop().map(|scored_move| scored_move.mv)
    }
}

impl Moves {
    /// Scores the moves in `moves`, given the information in `search_info` and
    /// the current height.
    pub fn score(self, search_info: &SearchInfo, height: u8) -> ScoredMoves {
        self.map(|mv| ScoredMove::new(mv, search_info, height))
            .collect()
    }
}

impl OrderedMoves {
    /// Sorts the moves in [`ScoredMoves`] based on their score.
    fn new(mut scored_moves: ScoredMoves) -> Self {
        scored_moves.sort_by(|mv1, mv2| mv1.score.cmp(&mv2.score));
        Self {
            ordered_moves: scored_moves,
        }
    }

    /// Pops a [`ScoredMove`] off the stack.
    fn pop(&mut self) -> Option<ScoredMove> {
        self.ordered_moves.pop()
    }
}

impl ScoredMove {
    /// Scores a [`Move`] based off the information in `search_info` and
    /// `height`.
    pub fn new(mv: Move, search_info: &SearchInfo, height: u8) -> Self {
        // always search the PV first
        // technically this will be reading from 1 past the end of the PV if
        // we're at a leaf node, but since it will just be a null move, it can
        // safely be compared against
        let score = if search_info.history.get(usize::from(height)) == mv {
            Eval::MAX
        } else {
            0
        };
        Self { score, mv }
    }
}

impl ScoredMoves {
    /// Sorts the moves in the stack based on their score.
    pub fn sort(self) -> OrderedMoves {
        OrderedMoves::new(self)
    }
}
