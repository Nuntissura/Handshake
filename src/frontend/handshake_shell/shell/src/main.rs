//! The product binary of the kernel-owned shell (WP-KERNEL-012 MT-214 S5, AC214-10, AC214-11).
//!
//! It opens the GPU device only under `device.gpu.shell` and the kernel issuer's binding, through
//! [`handshake_shell::run_product`]. Neither exists yet (`handshake_shell::MISSING`), so today it prints one typed refusal
//! line and exits with [`ExitKind::NoDeviceAuthority`]: no device, window, ring or thread was made. It declares no test
//! authority and has no flag that opens a window on a desktop; a run to look at is `examples/shell_program.rs`.
//!
//! Options (read only after the gate passed, so a refused start parses nothing): `--diag-ring <path>` makes the
//! diagnostics ring the heartbeat writes.

use std::process::ExitCode;

use handshake_shell::ExitKind;

fn main() -> ExitCode {
    run()
}

#[cfg(not(windows))]
fn run() -> ExitCode {
    eprintln!("HSK_SHELL_END kind={} code={}", ExitKind::Unsupported.name(), ExitKind::Unsupported.code());
    ExitCode::from(ExitKind::Unsupported.code())
}

#[cfg(windows)]
fn run() -> ExitCode {
    use handshake_diag_ring::{DiagRingWriter, DEFAULT_CAPACITY};
    use handshake_shell::{run_product, run_program, NoWatcher, ProgramConfig};

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let outcome = run_product(|authority| {
        println!("{}", authority.receipt());
        let ring = arguments
            .iter()
            .position(|argument| argument == "--diag-ring")
            .and_then(|at| arguments.get(at + 1))
            .and_then(|path| DiagRingWriter::create(std::path::Path::new(path), DEFAULT_CAPACITY).ok());
        run_program(ProgramConfig::new(ring), authority, Box::new(NoWatcher))
    });
    match outcome {
        Err(refusal) => {
            eprintln!("{}", refusal.line());
            ExitCode::from(ExitKind::NoDeviceAuthority.code())
        }
        Ok(outcome) => {
            if let Some(state) = &outcome.unavailable {
                println!("{}", state.line());
                eprintln!("{}", state.text());
            }
            println!("{}", outcome.line());
            ExitCode::from(outcome.kind.code())
        }
    }
}
