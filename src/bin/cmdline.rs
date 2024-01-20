//! A crate used simply to run [`backend::uci::Uci::main_loop`].

use backend::uci::Uci;

fn main() {
    Uci::main_loop();
}
