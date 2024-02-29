//! A crate used simply to run [`Uci::main_loop()`].

use backend::uci::Uci;

fn main() {
    Uci::new().main_loop();
}
