use backend::{
    board::{Move, Moves},
    defs::{MoveType, Piece, Side, Square},
};

use super::Gui;

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
        self.engine()
            .board
            .generate_moves::<{ MoveType::ALL }>(&mut moves);

        let mv = moves.move_with(start, end);
        let mv = if mv.is_none() {
            return false;
        } else {
            // SAFETY: We just checked that `mv` is not `None`.
            unsafe { mv.unwrap_unchecked() }
        };

        let mut copy = self.engine().board.clone();
        if !copy.make_move(mv) {
            return false;
        }

        self.engine().board = copy;

        self.regenerate_mailboxes();
        self.check_position_is_over();

        true
    }

    /// Finds the piece on `square`.
    pub const fn piece_on(&self, square: Square) -> Piece {
        self.piece_mailbox[square.to_index()]
    }

    /// Refreshes the mailboxe of `self` from `self.engine.board`.
    pub fn regenerate_mailboxes(&mut self) {
        let mailbox = self.engine().board.clone_mailbox();
        self.piece_mailbox = mailbox;
    }

    /// Makes the move `mv`, assuming it's the engine to move.
    pub fn make_engine_move(&mut self, mv: Move) {
        assert!(
            self.engine().board.make_move(mv),
            "Error: best move is illegal"
        );
        self.regenerate_mailboxes();
        self.info_rx = None;
        self.state.is_player_turn = true;

        self.check_position_is_over();
    }

    /// Checks if the position is over, setting `self.state.side_has_won` to
    /// the winner, if any.
    fn check_position_is_over(&mut self) {
        let mut moves = Moves::new();
        self.engine()
            .board
            .generate_moves::<{ MoveType::ALL }>(&mut moves);

        for mv in moves {
            let mut copy = self.engine().board.clone();
            if copy.make_move(mv) {
                return;
            }
        }

        if self.engine().board.is_in_check() {
            let side_has_won = Some(self.engine().board.side_to_move().flip());
            self.state.side_has_won = side_has_won;
        } else {
            self.state.side_has_won = Some(Side::NONE);
        }
    }
}
