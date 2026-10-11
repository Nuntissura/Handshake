//! WP-1 — `cwd-probe`
//!
//! Minimal, real executable that prints its own working directory to stdout.
//!
//! # Why this binary exists
//!
//! `operator_chat_cli_lane_runs_in_operator_selected_cwd` proves a real product
//! guarantee: an operator-selected working directory must survive the whole
//! plumbing chain (`SpawnRequest.working_dir` -> `CliBridgeConfig.working_dir`
//! -> the actual spawned OS process) and be the cwd the child really runs in.
//!
//! That test previously proved it by spawning `cmd /c cd`. It could never pass:
//! `reject_command_interpreter` in
//! `model_runtime/cloud/official_cli_bridge.rs` denylists `cmd` / `cmd.exe`
//! (along with the other generic shells and script interpreters) as CLI
//! entrypoints, matching on the file name. So PATH resolution does not rescue
//! it either — the name itself is refused, by design. Making that test pass by
//! removing `cmd` from the denylist would delete a security control to satisfy
//! a fixture, which is backwards.
//!
//! A generic interpreter is exactly what the product refuses to launch, but the
//! *behaviour under test* is legitimate and worth proving at runtime. So the
//! test needs a real, non-interpreter executable whose only job is to report
//! where it was started. That is this binary.
//!
//! Deliberately dependency-free and side-effect-free: it reads no input, writes
//! no files, opens no sockets, and cannot be used as a shell. It prints one
//! line and exits, which keeps it useless as an execution-escape hatch while
//! still being a genuine spawned process.
//!
//! Usage:
//!   cwd-probe
//!
//! Output:
//!   the absolute current working directory, one line, on stdout.
//!
//! Exit codes:
//!   0 — cwd resolved and printed
//!   1 — cwd could not be resolved (printed to stderr)

fn main() {
    match std::env::current_dir() {
        Ok(dir) => {
            println!("{}", dir.display());
        }
        Err(error) => {
            eprintln!("cwd-probe: cannot resolve current directory: {error}");
            std::process::exit(1);
        }
    }
}
