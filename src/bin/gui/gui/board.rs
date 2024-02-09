use super::Gui;

use backend::{
    board::Moves,
    defs::{MoveType, Piece, Square},
};

impl Gui {
    /// Attempts to move a piece from `start` to `end`. Returns `true` if the
    /// given move is legal; false otherwise.
    // there are many ways to do this. The method I've chosen is
    // - when this function is called, generate a list of legal moves
    // - check if a move from the start square to the end is in the list,
    //   returning if it isn't
    // - play the move on a copy, returning if it's illegal
    // - set the current board to the copy
    pub fn move_piece(&mut self, start: Square, end: Square) -> bool {
        let mut moves = Moves::new();
        self.engine
            .board
            .generate_moves::<{ MoveType::ALL }>(&mut moves);

        let mv = moves.move_with(start, end);
        let mv = if mv.is_none() {
            return false;
        } else {
            // SAFETY: We just checked that `mv` is not `None`.
            unsafe { mv.unwrap_unchecked() }
        };

        let mut copy = self.engine.board.clone();
        if !copy.make_move(mv) {
            return false;
        }

        self.engine.board = copy;

        // let the engine respond with a random move. TODO: when this starts
        // doing an actual search, make this happen on a new thread.
        self.engine.board = loop {
            let mut copy = self.engine.board.clone();
            moves.clear();
            copy.generate_moves::<{ MoveType::ALL }>(&mut moves);
            let rand_move = moves.random_move();
            // if we've selected a legal move, break. Otherwise, keep trying
            // other random moves.
            if copy.make_move(rand_move) {
                break copy;
            }
        };

        self.regenerate_mailboxes();

        true
    }

    /// Finds the piece on `square`.
    pub const fn piece_on(&self, square: Square) -> Piece {
        self.piece_mailbox[square.to_index()]
    }

    /// Refreshes the mailboxe of `self` from `self.engine.board`.
    pub fn regenerate_mailboxes(&mut self) {
        self.piece_mailbox = self.engine.board.clone_mailbox();
    }
}
