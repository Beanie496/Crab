use crate::{
    board::{Board, CastlingRights},
    defs::{File, Piece, Rank, Side, Square},
};

/// For perft, as it's counting leaf nodes, not searching.
mod perft;
/// For search-related code.
mod search;

/// Master object that contains all the other major objects.
#[non_exhaustive]
#[derive(Clone)]
pub struct Engine {
    /// The internal board.
    ///
    /// See [`Board`].
    pub board: Board,
}

impl Engine {
    /// Creates a new [`Engine`] with each member struct initialised to their
    /// default values.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            board: Board::new(),
        }
    }

    /// Sets `self.board` to the given FEN and moves, as given by the
    /// `position` UCI command. Unexpected/incorrect tokens will be ignored.
    #[inline]
    pub fn set_position(&mut self, position: &str, moves: &str) {
        self.set_pos_to_fen(position);
        self.play_moves(moves);
    }

    /// Takes a sequence of moves and feeds them to the board. Will stop and
    /// return if any of the moves are incorrect. Not implemented yet.
    #[allow(clippy::unused_self)]
    const fn play_moves(&self, _moves: &str) {}

    /// Sets `self.board` to the given FEN. It will check for basic errors,
    /// like the board having too many ranks, but not many more.
    #[allow(clippy::too_many_lines)]
    fn set_pos_to_fen(&mut self, position: &str) {
        self.board.clear_board();

        let mut iter = position.split(' ');
        let Some(board) = iter.next() else {
            self.board.set_startpos();
            return println!("Need to pass a board");
        };
        let side_to_move = iter.next();
        let castling_rights = iter.next();
        let ep_square = iter.next();
        let halfmoves = iter.next();
        let fullmoves = iter.next();

        // 1. the board itself. 1 of each king isn't checked. Hey, garbage in,
        // garbage out!
        // split into 2 to check for overflow easily
        let mut rank_idx = 8u8;
        let mut file_idx = 0;
        let ranks = board.split('/');
        for rank in ranks {
            // if the board has too many ranks, this would eventually underflow
            // and panic, so wrapping sub needed
            rank_idx = rank_idx.wrapping_sub(1);
            for piece in rank.chars() {
                // if it's a number, skip over that many files
                if ('0'..='8').contains(&piece) {
                    file_idx += piece.to_digit(10).expect("Hardcoded values cannot fail.") as u8;
                } else {
                    let piece_num = Piece::from_char(piece.to_ascii_lowercase());
                    let Some(piece_num) = piece_num else {
                        self.board.set_startpos();
                        return println!("Error: \"{piece}\" is not a valid piece.");
                    };
                    // 1 if White, 0 if Black
                    let side = Side::from(u8::from(piece.is_ascii_uppercase()));

                    self.board.add_piece(
                        side,
                        Square::from_pos(Rank::from(rank_idx), File::from(file_idx)),
                        piece_num,
                    );

                    file_idx += 1;
                }
            }
            // if there are too few/many files in that rank, reset and return
            if file_idx != 8 {
                self.board.set_startpos();
                return println!("Error: FEN is invalid");
            }

            file_idx = 0;
        }
        // if there are too many/few ranks in the board, reset and return
        if rank_idx != 0 {
            self.board.set_startpos();
            return println!("Error: FEN is invalid (incorrect number of ranks)");
        }

        // 2. side to move
        if let Some(stm) = side_to_move {
            if stm == "w" {
                self.board.set_side_to_move(Side::WHITE);
            } else if stm == "b" {
                self.board.set_side_to_move(Side::BLACK);
            } else {
                self.board.set_startpos();
                return println!("Error: Side to move \"{stm}\" is not \"w\" or \"b\"");
            }
        } else {
            // I've decided that everything apart from the board can be omitted
            // and guessed, so if there's nothing given, default to White to
            // move.
            self.board.set_default_side_to_move();
        }

        // 3. castling rights
        if let Some(cr) = castling_rights {
            for right in cr.chars() {
                match right {
                    'K' => self.board.add_castling_right(CastlingRights::K),
                    'Q' => self.board.add_castling_right(CastlingRights::Q),
                    'k' => self.board.add_castling_right(CastlingRights::k),
                    'q' => self.board.add_castling_right(CastlingRights::q),
                    '-' => (),
                    _ => {
                        self.board.set_startpos();
                        return println!("Error: castling right \"{right}\" is not valid");
                    }
                }
            }
        } else {
            // KQkq if nothing is given.
            self.board.set_default_castling_rights();
        }

        // 4. en passant
        self.board.set_ep_square(if let Some(ep) = ep_square {
            if ep == "-" {
                Square::NONE
            } else if let Some(square) = Square::from_string(ep) {
                square
            } else {
                self.board.set_startpos();
                return println!("Error: En passant square \"{ep}\" is not a valid square");
            }
        } else {
            Square::NONE
        });

        // 5. halfmoves
        self.board.set_halfmoves(if let Some(hm) = halfmoves {
            if let Ok(hm) = hm.parse::<u32>() {
                hm
            } else {
                self.board.set_startpos();
                return println!("Error: Invalid number (\"hm\") given for halfmove counter");
            }
        } else {
            0
        });

        // 6. fullmoves
        self.board.set_fullmoves(if let Some(fm) = fullmoves {
            if let Ok(fm) = fm.parse::<u32>() {
                fm
            } else {
                return println!("Error: Invalid number (\"fm\") given for fullmove counter");
            }
        } else {
            0
        });
    }
}
