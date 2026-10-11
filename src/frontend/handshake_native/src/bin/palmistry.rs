fn main() {
    if let Err(error) = handshake_native::palmistry::run_from_env() {
        eprintln!("palmistry failed: {error}");
        std::process::exit(1);
    }
}
