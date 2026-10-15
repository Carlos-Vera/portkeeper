#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `portkeeper --dump` prints the scan as JSON and exits. Handy for debugging platform quirks.
    if std::env::args().any(|a| a == "--dump") {
        portkeeper_lib::dump();
        return;
    }
    portkeeper_lib::run()
}
