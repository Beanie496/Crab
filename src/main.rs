use std::{
    io,
    process::exit,
};

use crate::engine::Engine;

mod bits;
mod board;
mod defs;
mod engine;
mod movegen;
mod movelist;
mod util;

fn main() {
    let engine = Engine::new();
    uci_main_loop(engine);
}

fn uci_main_loop(mut engine: Engine) {
    let mut input = String::new();
    loop {
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read from stdin");
        handle_input_line(&input, &mut engine);
        input.clear();
    }
}

fn handle_input_line(line: &str, engine: &mut Engine) {
    let mut line = line.trim().split(' ');

    // handle each UCI option
    if let Some(command) = line.next() {
        match command {
            "debug" => {
                /* Sets debug to "on" or "off". Default "off". */
                /* This can be ignored. */
            }
            "go" => {
                /* Start calculating from the current position,
                 * as specified by the "position" command.
                 * The next element should be one of the following:
                 * - searchmoves: restrict search to one of the
                 *   specified moves
                 * - ponder: start searching in pondering mode.
                 *   Don't implement this.
                 * - wtime: White has x ms left
                 * - btime: Black has x ms left
                 * - winc: White has x ms inc
                 * - binc: Black has x ms inc
                 * - movestogo: x moves until next tc, otherwise
                 *   sudden death
                 * - depth: search x plies only
                 * - nodes: search x nodes only
                 * - mate: search for mate in x
                 * - movetime: search for exactly x ms
                 * - infinite: search until "stop" command
                 *   received. Do not exit search otherwise.
                 * - perft [unofficial]: run perft to x plies
                 */
            }
            "isready" => {
                /* Immediately print "readyok" */
                println!("readyok");
            }
            "ponderhit" => {
                /* The user has played the expected move. */
                /* Don't implement. */
            }
            "position" => {
                /* Next element should be "fen" or "startpos".
                 * If the next element is "fen", a FEN string
                 * should be given (spanning multiple elements).
                 * The element after that should be "moves",
                 * followed by a series of moves, one per element.
                 * The moves should look like, for example,
                 * "e2e4".
                 */
            }
            "setoption" => {
                /* Next element of line_iter should be "name".
                 * Element after "name" should be one of the
                 * options specified from "uci" command.
                 */
            }
            "stop" => {
                /* Stop calculating immediately. */
            }
            "uci" => {
                /* Print ID, all options and "uciok" */
                println!("uciok");
            }
            "ucinewgame" => {
                /* What it sounds like. Set pos to start pos, etc. */
            }
            "q" | "quit" => {
                /* Quit as soon as possible */
                exit(0);
            }

            /* non-standard commands */
            /* "p" - prints current position */
            "p" => {
                engine.pretty_print_board();
            }
            /* "perft n", where n is a number - run perft to depth n */
            "perft" => {
                if let Some(num) = line.next() {
                    if let Ok(result) = num.parse::<u8>() {
                        engine.perft_root(result);
                    } else {
                        println!("Must give a number between 0 and 255.");
                    }
                }
            }

            other => {
                println!("Unrecognised option \"{other}\".");
            }
        }
    } else {
        println!("Unreachable code reached. (Each line should have at least 1 iterable element.)");
        exit(1);
    }
}
