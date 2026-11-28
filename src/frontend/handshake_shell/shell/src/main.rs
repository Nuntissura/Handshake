//! The product binary of the kernel-owned shell (WP-KERNEL-012 MT-214 S5, AC214-10, AC214-11).
//!
//! It opens the GPU device only under `device.gpu.shell` and the kernel issuer's binding, through
//! [`handshake_shell::run_product`]. Neither exists yet (`handshake_shell::MISSING`), so today it prints one typed refusal
//! line and exits with [`ExitKind::NoDeviceAuthority`]: no device, window, ring or thread was made. It declares no test
//! authority, its root is an Operator window that takes the focus once at the Operator's launch and whose close ends the
//! process (WP012-SHELL-ROOT-WINDOW-20261127), and it has no flag that opens a window on a desktop; a run to look at is `examples/shell_program.rs`.
//!
//! Options (read only after the gate passed, so a refused start parses nothing): `--diag-ring <path>` is the version 2
//! diagnostics ring's file (without it the launcher's default path for the session is used).
//!
//! After the gate the binary prints its receipt, starts the Palmistry session (`handshake_shell::palmistry::start`: lease,
//! ring, signed launch, handshake marker), prints one `HSK_SHELL_PALMISTRY` state line, and runs the program with the
//! ring and the watcher. A refused start runs none of this. The watcher is watched from the first heartbeat on, so the
//! device open and the window creation before the loop starts are not watched; an ending that never reaches the
//! acknowledgement (no device, exit 4, a failed loop, a refused heartbeat, an unsettled close, a panic) leaves the watcher
//! untold, and it records the end as unexpected.

use std::process::ExitCode;

use handshake_shell::ExitKind;

fn main() -> ExitCode {
    run()
}

#[cfg(not(windows))]
fn run() -> ExitCode {
    eprintln!(
        "HSK_SHELL_END kind={} code={}",
        ExitKind::Unsupported.name(),
        ExitKind::Unsupported.code()
    );
    ExitCode::from(ExitKind::Unsupported.code())
}

#[cfg(windows)]
fn run() -> ExitCode {
    use std::path::Path;

    use handshake_shell::palmistry::{self, Started};
    use handshake_shell::{run_product, run_program, ProgramConfig};

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let outcome = run_product(|authority| {
        println!("{}", authority.receipt());
        let ring_path = arguments
            .iter()
            .position(|argument| argument == "--diag-ring")
            .and_then(|at| arguments.get(at + 1));
        // The Palmistry session: the lease, the version 2 ring, the signed launch and its handshake marker. A refusal at any
        // step is a degrade (no watcher), never a failed start.
        let started = palmistry::start(ring_path.map(Path::new));
        println!("{}", started.state_line());
        let Started {
            ring,
            watcher,
            lease,
            ..
        } = started;
        let outcome = run_program(
            ProgramConfig::operator_root(ring),
            authority,
            Box::new(watcher),
        );
        // The lease is held until the program has returned.
        drop(lease);
        outcome
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
