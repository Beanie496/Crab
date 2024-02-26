use std::{
    io,
    str::Split,
    sync::{Arc, Mutex},
    thread::spawn,
};

use crate::{board::find_magics, defs::PieceType, engine::Engine};

/// The starting position as a FEN string.
const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Repeatedly waits for a command and executes it according to the UCI
/// protocol. It is not yet concurrent, i.e. it cannot process commands
/// while not idle.
///
/// # Panics
///
/// Panics if [`read_line()`](`std::io::BufRead::read_line`) returns an
/// [`Err`].
#[inline]
pub fn main_loop() {
    let engine = Arc::new(Mutex::new(Engine::new()));
    let mut input = String::new();
    loop {
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read from stdin");
        handle_input_line(&input, &engine);
        input.clear();
    }
}

/// Starts the search, given the rest of the tokens after `go`.
fn go(line: &mut Split<'_, char>, engine: &Arc<Mutex<Engine>>) {
    let mut depth = None;

    while let Some(token) = line.next() {
        // just depth for now
        #[allow(clippy::single_match)]
        match token {
            "depth" => {
                if let Some(result) = line.next() {
                    if let Ok(d) = result.parse::<u8>() {
                        if d != 0 {
                            depth = Some(d);
                        }
                    }
                }
            }
            _ => (),
        }
    }

    let engine = Arc::clone(engine);
    spawn(move || {
        #[allow(clippy::unwrap_used)]
        let info_rx = engine.lock().unwrap().start_search(depth);
        for result in info_rx {
            println!("{result}");
        }
    });
}

/// Given an iterator over the remaining space-delimited tokens of a `position`
/// command, removes all empty strings and concatenate the remaining tokens
/// into a [`String`] for the FEN and moves each with a space between each
/// token.
fn handle_position(line: &mut Split<'_, char>, engine: &Arc<Mutex<Engine>>) {
    let fen = match line.next() {
        Some("startpos") => STARTPOS.to_string(),
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

    // the moves need to be preceeded with the token "moves"
    if line.next() != Some("moves") {
        return;
    }

    let mut moves = String::new();
    line.filter(|token| !token.is_empty()).for_each(|token| {
        moves.push_str(token);
        moves.push(' ');
    });
    // remove the trailing space
    moves.pop();

    #[allow(clippy::unwrap_used)]
    engine.lock().unwrap().set_position(&fen, &moves);
}

/// Dissects `line` according to the UCI protocol.
fn handle_input_line(line: &str, engine: &Arc<Mutex<Engine>>) {
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
                 * - searchmoves: restrict search to one of the specified moves
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
                go(&mut line, engine);
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
                handle_position(&mut line, engine);
            }
            "setoption" => {
                /* Next element of line_iter should be "name".  Element after
                 * "name" should be one of the options specified from "uci"
                 * command.
                 */
            }
            "stop" => {
                /* Stop calculating immediately. */
                #[allow(clippy::unwrap_used)]
                engine.lock().unwrap().stop_search();
            }
            "uci" => {
                /* Print ID, all options and "uciok" */
                println!("uciok");
            }
            "ucinewgame" => { /* What it sounds like. Set pos to start pos, etc. */ }
            "q" | "quit" => {
                /* Quit as soon as possible */
                #[allow(clippy::unwrap_used)]
                engine.lock().unwrap().exit();
            }

            /* non-standard commands */
            /* "f" - find magic numbers for each square for bishop and rook */
            "f" => {
                find_magics::<{ PieceType::BISHOP.0 }>();
                find_magics::<{ PieceType::ROOK.0 }>();
            }
            /* "p" - prints current position */
            "p" => {
                #[allow(clippy::unwrap_used)]
                engine.lock().unwrap().pretty_print_board();
            }
            /* "perft n", where n is a number - run perft to depth n */
            "perft" => {
                if let Some(depth) = line.next() {
                    match depth.parse::<u8>() {
                        #[allow(clippy::unwrap_used)]
                        Ok(result) => _ = engine.lock().unwrap().perft::<true, true>(result),
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
