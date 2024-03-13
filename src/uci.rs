use std::{
    io,
    process::exit,
    str::{FromStr, Split},
    time::Duration,
};

use crate::{
    defs::{PieceType, Side},
    engine::{Engine, STARTPOS},
    movegen::magic::find_magics,
    search::Limits,
};

/// Used to store state for [`Self::main_loop()`].
#[allow(clippy::missing_docs_in_private_items)]
pub struct Uci {
    engine: Engine,
    options: UciOptions,
}

/// The UCI options this engine supports.
#[derive(Clone, Copy)]
pub struct UciOptions {
    /// The overhead of sending a move from the engine to the GUI.
    pub move_overhead: Duration,
}

/// The name of this engine.
const ID_NAME: &str = "Crab";
/// The version of this engine.
const ID_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The name of the author of this engine.
const ID_AUTHOR: &str = "Beanie";

impl Default for UciOptions {
    fn default() -> Self {
        Self {
            move_overhead: Duration::from_millis(1),
        }
    }
}

impl Uci {
    /// Creates a new instance of [`Uci`](Self).
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            options: UciOptions::default(),
        }
    }

    /// Repeatedly waits for a command and executes it according to the UCI
    /// protocol.
    ///
    /// # Panics
    ///
    /// Panics if [`read_line()`](`std::io::BufRead::read_line`) returns an
    /// [`Err`].
    pub fn main_loop(&mut self) -> ! {
        let mut input = String::new();
        loop {
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read from stdin");
            self.handle_input_line(&input);
            input.clear();
        }
    }

    /// Dissects `line` according to the UCI protocol.
    fn handle_input_line(&mut self, line: &str) {
        let mut line = line.trim().split(' ');

        // handle each UCI option
        if let Some(command) = line.next() {
            if command.is_empty() {
                return;
            }
            #[allow(clippy::match_same_arms)]
            match command {
                // Ignored commands
                "debug" | "ponderhit" => {
                    /* "debug": Sets debug to "on" or "off". Default "off". */
                    /* "ponderhit": The user has played the expected move. */
                }
                "go" => {
                    /* Start calculating from the current position,
                     * as specified by the "position" command.
                     * The next element should be one of the following:
                     * - searchmoves: restrict search to the specified moves
                     * - ponder: start searching in pondering mode.  Don't
                     *   implement this.
                     * - wtime: White has x ms left
                     * - btime: Black has x ms left
                     * - winc: White has x ms inc
                     * - binc: Black has x ms inc
                     * - movestogo: x moves until next tc, otherwise sudden death
                     * - depth: search x plies only
                     * - nodes: search x nodes only
                     * - mate: search for mate in x
                     * - movetime: search for exactly x ms
                     * - infinite: search until "stop" command received. Do not
                     * exit search otherwise.
                     */
                    self.go(&mut line);
                }
                "isready" => {
                    /* Immediately print "readyok" */
                    println!("readyok");
                }
                "position" => {
                    /* Next element should be "fen" or "startpos".  If the next
                     * element is "fen", a FEN string should be given (spanning
                     * multiple elements).  The element after that should be
                     * "moves", followed by a series of moves, one per element.
                     * The moves should look like, for example, "e2e4".
                     */
                    self.handle_position(&mut line);
                }
                "setoption" => {
                    self.handle_option(&mut line);
                }
                "stop" => {
                    /* Stop calculating immediately. */
                    self.engine.stop_search();
                }
                "uci" => {
                    /* Print ID, all options and "uciok" */
                    UciOptions::print();
                    println!("uciok");
                }
                "ucinewgame" => {
                    self.engine.reset();
                }
                "q" | "quit" => {
                    /* Quit as soon as possible */
                    self.engine.stop_search();
                    exit(0);
                }

                /* non-standard commands */
                /* "f" - find magic numbers for each square for bishop and rook */
                "f" => {
                    find_magics::<{ PieceType::BISHOP.0 }>();
                    find_magics::<{ PieceType::ROOK.0 }>();
                }
                /* "p" - prints current position */
                "p" => {
                    self.engine.pretty_print_board();
                }
                /* "perft n", where n is a number - run perft to depth n */
                "perft" => {
                    if let Some(depth) = line.next() {
                        match depth.parse::<u8>() {
                            Ok(result) => _ = self.engine.perft::<true, true>(result),
                            Err(result) => println!("{result}; must give 0-255"),
                        }
                    }
                }

                other => {
                    println!("Unrecognised option \"{other}\".");
                }
            }
        } else {
            unreachable!("Each line should have at least 1 iterable element.");
        }
    }

    /// Starts the search, given the rest of the tokens after `go`.
    fn go(&mut self, line: &mut Split<'_, char>) {
        let mut limits = Limits::default();

        while let Some(token) = line.next() {
            match token {
                "wtime" => {
                    if self.engine.side_to_move() == Side::WHITE {
                        limits.set_time(
                            parse_next_num(line)
                                .and_then(|d| if d == 0 { None } else { Some(d) })
                                .map(Duration::from_millis),
                        );
                    }
                }
                "btime" => {
                    if self.engine.side_to_move() == Side::BLACK {
                        limits.set_time(parse_next_num(line).map(Duration::from_millis));
                    }
                }
                "winc" => {
                    if self.engine.side_to_move() == Side::WHITE {
                        limits.set_inc(parse_next_num(line).map(Duration::from_millis));
                    }
                }
                "binc" => {
                    if self.engine.side_to_move() == Side::BLACK {
                        limits.set_inc(parse_next_num(line).map(Duration::from_millis));
                    }
                }
                "movestogo" => limits.set_moves_to_go(parse_next_num(line)),
                "depth" => limits.set_depth(parse_next_num(line)),
                "nodes" => limits.set_nodes(parse_next_num(line)),
                "movetime" => limits.set_movetime(parse_next_num(line).map(Duration::from_millis)),
                // if depth is specified and then `infinite` is give, the
                // latter should override the former
                "infinite" => limits.set_infinite(),
                _ => (),
            }
        }

        self.engine.start_search(limits, self.options);
    }

    /// Given an iterator over the remaining space-delimited tokens of a `position`
    /// command, removes all empty strings and concatenate the remaining tokens
    /// into a [`String`] for the FEN and moves each with a space between each
    /// token.
    fn handle_position(&mut self, line: &mut Split<'_, char>) {
        let fen = match line.next() {
            Some("startpos") => {
                // ensure the next token is "moves" if there is one
                if let Some(token) = line.next() {
                    if token != "moves" {
                        return;
                    }
                }
                STARTPOS.to_string()
            }
            Some("fen") => {
                let mut fen = String::new();
                line.take_while(|token| *token != "moves")
                    .filter(|token| !token.is_empty())
                    // I COULD use `map()` then `collect()` but that's an unnecessary heap
                    // allocation for each token
                    .for_each(|token| {
                        fen.push_str(token);
                        fen.push(' ');
                    });
                // remove the trailing space
                fen.pop();
                fen
            }
            _ => return,
        };

        let mut moves = String::new();
        line.filter(|token| !token.is_empty()).for_each(|token| {
            moves.push_str(token);
            moves.push(' ');
        });
        // remove the trailing space
        moves.pop();

        self.engine.set_position(&fen, &moves);
    }

    /// Sets the UCI options given by the remaining space-delimited tokens of
    /// the `setoption` command.
    fn handle_option(&mut self, line: &mut Split<'_, char>) {
        if line.next() != Some("name") {
            return;
        }

        // more options added later, so be quiet, clippy
        #[allow(clippy::single_match)]
        match line.next() {
            Some("Move") => {
                if line.next() != Some("Overhead") {
                    return;
                }
                if line.next() != Some("value") {
                    return;
                }
                self.options.move_overhead = line
                    .next()
                    .and_then(|result| result.parse::<u64>().ok())
                    .map_or(UciOptions::default_move_overhead(), Duration::from_millis);
            }
            _ => (),
        }
    }
}

impl UciOptions {
    /// The default overhead of sending a move.
    const fn default_move_overhead() -> Duration {
        Duration::from_millis(1)
    }

    /// Prints the identification of this engine and all the UCI options it
    /// supports.
    fn print() {
        println!("id name {ID_NAME}-{ID_VERSION}");
        println!("id author {ID_AUTHOR}");
        println!(
            "option name Move Overhead type spin default {} min 0 max 1000",
            Self::default_move_overhead().as_millis()
        );
    }
}

/// Parses the next unsigned integer.
///
/// If the next token is a valid number, isn't 0, and fits within `T`, it
/// returns `Some(T)`; otherwise, it returns `None`.
fn parse_next_num<T: TryFrom<u128> + FromStr>(line: &mut Split<'_, char>) -> Option<T> {
    line.next()
        .and_then(|result| result.parse::<u128>().ok())
        .and_then(|result| (result != 0).then_some(result))
        .and_then(|result| result.try_into().ok())
}
