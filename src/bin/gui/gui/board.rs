use super::Gui;

use backend::{
    board::Moves,
    defs::{Piece, Side, Square},
};

impl Gui {
    /// Attempts to move a piece from `start` to `end`. Returns `true` if the
    /// given move is legal; false otherwise.
    // there are many ways to do this. The method I've chosen is
    // - hold a list of legal moves in `self`
    // - when this function is called, create the `Move`
    // - check if it's in the list (linear search), returning if it isn't
    // - play it on a copy, returning if it's illegal
    // - set the current board to the copy
    // - regenerate the legal move list
    // It's not very efficient, but I doubt it takes more than a few hundred
    // nanoseconds, so who cares.
    pub fn move_piece(&mut self, start: Square, end: Square) -> bool {
        let mut moves = Moves::new();
        self.engine.board.generate_moves(&mut moves);

        let mv = moves.move_with(start, end);
        let mv = if mv.is_none() {
            return false;
        } else {
            unsafe { mv.unwrap_unchecked() }
        };

        let mut copy = self.engine.board.clone();
        if !copy.make_move(mv) {
            return false;
        }

        self.engine.board = copy;
        // not the most efficient, but considering how fast it is anyway, who
        // cares
        self.regenerate_mailboxes();

        true
    }

    /// Finds the piece on `square`.
    pub fn piece_on(&self, square: Square) -> Piece {
        self.piece_mailbox[square.to_index()]
    }

    /// Finds the side of the piece on `square`. If there is no piece on
    /// `square`, it returns [`Side::NONE`].
    pub fn side_of(&self, square: Square) -> Side {
        self.side_mailbox[square.to_index()]
    }

    /// Refreshes the piece and side mailboxes of `self` from
    /// `self.engine.board`. The piece mailbox probably takes a matter of
    /// cycles but the side mailbox is a little more expensive due to the 64
    /// unpredictable branches.
    pub fn regenerate_mailboxes(&mut self) {
        self.piece_mailbox = self.engine.board.clone_piece_board();
        for (square, side) in self.side_mailbox.iter_mut().enumerate() {
            *side = self.engine.board.side_of(Square::from(square as u8));
        }
    }
}
