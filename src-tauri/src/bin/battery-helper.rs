fn main() {
    if let Err(error) = battery_deck_lib::helper::run_daemon() {
        eprintln!("battery-helper error: {}", error);
        std::process::exit(1);
    }
}
