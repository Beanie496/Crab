use std::{
    io::stdin,
    rc::Rc,
    str::FromStr,
    sync::mpsc::{channel, Receiver},
    thread::spawn,
    time::Duration,
};

use crate::{
    board::Board,
    defs::{MoveType, PieceType, Side, Square},
    movegen::generate_moves,
    perft::perft,
    search::{iterative_deepening, Limits, SearchParams},
    uci::UciOptions,
};

/// Master object that contains all the other major objects.
pub struct Engine {
    /// The internal board.
    ///
    /// See [`Board`].
    board: Board,
    /// The current set options.
    options: UciOptions,
    /// A receiver to receive UCI commands from.
    uci_rx: Rc<Receiver<String>>,
}

impl Engine {
    /// Creates a new [`Engine`] and spawns a thread to receive UCI input from.
    ///
    /// Note that the board is completely empty, as UCI specifies that a
    /// `position` command should be given before `go`.
    pub fn new() -> Self {
        let (tx, rx) = channel();

        spawn(move || {
            let stdin = stdin();

            for command in stdin.lines() {
                let command = command.expect("Error while reading from stdin");
                tx.send(command).expect(
                    "It's not possible for this thread to exit later than the main thread.",
                );
            }
        });

        Self {
            board: Board::new(),
            options: UciOptions::new(),
            uci_rx: Rc::new(rx),
        }
    }

    /// Interprets and executes the `go` command.
    pub fn go(&mut self, line: &str) {
        let mut tokens = line.split_whitespace();
        let mut limits = Limits::default();

        if tokens.next() != Some("go") {
            return;
        }

        while let Some(token) = tokens.next() {
            let next = tokens.next();

            match token {
                "wtime" if self.board().side_to_move() == Side::WHITE => {
                    let time = parse_into_nonzero_option(next)
                        .map(Duration::from_millis)
                        .map(|d| d.saturating_sub(self.options().move_overhead()));
                    limits.set_time(time);
                }
                "btime" if self.board().side_to_move() == Side::BLACK => {
                    let time = parse_into_nonzero_option(next)
                        .map(Duration::from_millis)
                        .map(|d| d.saturating_sub(self.options().move_overhead()));
                    limits.set_time(time);
                }
                "winc" if self.board().side_to_move() == Side::WHITE => {
                    let time = parse_into_nonzero_option(next)
                        .map(Duration::from_millis)
                        .map(|d| d.saturating_sub(self.options().move_overhead()));
                    limits.set_inc(time);
                }
                "binc" if self.board().side_to_move() == Side::BLACK => {
                    let time = parse_into_nonzero_option(next)
                        .map(Duration::from_millis)
                        .map(|d| d.saturating_sub(self.options().move_overhead()));
                    limits.set_inc(time);
                }
                "movestogo" => limits.set_moves_to_go(parse_into_nonzero_option(next)),
                "depth" => limits.set_depth(parse_into_nonzero_option(next)),
                "nodes" => limits.set_nodes(parse_into_nonzero_option(next)),
                "movetime" => {
                    limits.set_movetime(parse_into_nonzero_option(next).map(Duration::from_millis));
                }
                // if depth is specified and then `infinite` is give, the
                // latter should override the former
                "infinite" => limits.set_infinite(),
                _ => (),
            }
        }

        let search_params = SearchParams::new(limits, self.options().move_overhead());

        iterative_deepening(search_params, self.board(), self.uci_rx());
    }

    /// Given a `perft` command, run [`perft`] to the specified depth.
    pub fn perft<const SHOULD_PRINT: bool, const IS_TIMED: bool>(&self, line: &str) -> u64 {
        let mut tokens = line.split_whitespace();

        if tokens.next() != Some("perft") {
            return 0;
        }
        let Some(depth) = parse_into_nonzero_option(tokens.next()) else {
            return 0;
        };

        perft::<SHOULD_PRINT, IS_TIMED>(self.board(), depth)
    }

    /// Sets the board to a position specified by the `position` command.
    ///
    /// Will not change anything if the command fails to get parsed
    /// successfully.
    pub fn set_position(&mut self, line: &str) {
        let mut board = Board::new();
        let mut tokens = line.split_whitespace();

        if Some("position") != tokens.next() {
            return;
        }

        match tokens.next() {
            Some("startpos") => board.set_startpos(),
            Some("fen") => {
                // Creating a new `String` is annoying, but probably not too
                // expensive, considering this only happens a few tens of times
                // per game.
                let mut fen_str = String::with_capacity(128);

                // The FEN string should have exactly 6 tokens - more or fewer
                // should raise an error later or now respectively.
                for _ in 0..6 {
                    let Some(token) = tokens.next() else {
                        return;
                    };
                    fen_str.push_str(token);
                    fen_str.push(' ');
                }

                if let Ok(b) = fen_str.parse() {
                    board = b;
                } else {
                    return;
                }
            }
            _ => return,
        };

        // check if we have any moves to parse
        if let Some(token) = tokens.next() {
            if token != "moves" {
                return;
            }
        } else {
            // if there are no moves, that's ok
            *self.board_mut() = board;
            return;
        }

        #[allow(clippy::string_slice)]
        for mv in tokens {
            let mut moves = generate_moves::<{ MoveType::ALL }>(&board);

            let Ok(start) = Square::from_str(&mv[0..=1]) else {
                return;
            };
            let Ok(end) = Square::from_str(&mv[2..=3]) else {
                return;
            };

            // Each move should be exactly 4 characters; if it's a promotion,
            // the last char will be the promotion char.
            let Some(mv) = (if mv.len() == 5 {
                // SAFETY: `mv` has 5 chars so `next_back()` must return `Some`
                let promotion_char = unsafe { mv.chars().next_back().unwrap_unchecked() };
                let Ok(piece_type) = PieceType::try_from(promotion_char) else {
                    return;
                };
                moves.move_with_promo(start, end, piece_type)
            } else {
                moves.move_with(start, end)
            }) else {
                return;
            };

            if !board.make_move(mv) {
                return;
            }
        }

        *self.board_mut() = board;
    }

    /// Sets a UCI option from a `setoption` command.
    pub fn set_option(&mut self, line: &str) {
        let mut tokens = line.split_whitespace();

        if tokens.next() != Some("setoption") {
            return;
        }
        if tokens.next() != Some("name") {
            return;
        }

        let Some(token) = tokens.next() else { return };
        // more options added later, so be quiet, clippy
        #[allow(clippy::single_match)]
        match token {
            "Move" => {
                if tokens.next() != Some("Overhead") {
                    return;
                }
                if tokens.next() != Some("value") {
                    return;
                }

                if let Some(d) = parse_option(tokens.next()) {
                    self.options_mut().set_move_overhead(d);
                }
            }
            _ => (),
        }
    }

    /// Sets the engine to its initial state. Should be called after the
    /// `ucinewgame` command.
    pub fn reset(&mut self) {
        self.board_mut().set_startpos();
    }

    /// Returns a reference to the board.
    pub const fn board(&self) -> &Board {
        &self.board
    }

    /// Returns a mutable reference to the board.
    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    /// Returns a reference to the UCI options.
    pub const fn options(&self) -> &UciOptions {
        &self.options
    }

    /// Returns a mutable reference to the UCI options.
    pub fn options_mut(&mut self) -> &mut UciOptions {
        &mut self.options
    }

    /// Returns a reference-counted receiver to the inputted UCI commands.
    pub fn uci_rx(&self) -> Rc<Receiver<String>> {
        Rc::clone(&self.uci_rx)
    }
}

/// Parses an `Option<&str>` into an `Option<T>`.
///
/// If the parse fails, it will return [`None`].
fn parse_option<T: FromStr>(num: Option<&str>) -> Option<T> {
    num.and_then(|t| t.parse::<T>().ok())
}

/// Parses an `Option<&str>` into an `Option<T>`.
///
/// Returns [`None`] if the result of the parse is 0 or an `Err`.
fn parse_into_nonzero_option<T: FromStr + PartialEq<T> + From<u8>>(num: Option<&str>) -> Option<T> {
    parse_option(num).and_then(|t| if t == T::from(0) { None } else { Some(t) })
}
