fn main() {
    if std::env::args().any(|arg| arg == "--version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if let Err(error) = battery_deck_lib::helper::run_daemon() {
        eprintln!("battery-helper error: {}", error);
        std::process::exit(1);
    }
}
