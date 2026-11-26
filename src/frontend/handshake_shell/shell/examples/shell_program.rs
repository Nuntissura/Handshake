//! The shell program on the Operator's own desktop, as a test binary: one quiet window with a Vulkan frame, the
//! accessibility tree and the heartbeat, closed after N seconds or by its close button. For the Operator to look at,
//! by hand. Tests and agents never run it.
//!
//! ```text
//! cargo run --manifest-path src/frontend/handshake_shell/Cargo.toml -p handshake_shell --example shell_program -- --show-on-my-desktop [--seconds N] [--diag-ring PATH]
//! ```
//!
//! Without the flag it prints how to use it and opens nothing (exit code 2). The window is quiet: it never takes the
//! foreground or the keyboard, has no taskbar button and sits at the bottom of the Z order.
//!
//! Not the product: it declares itself a test binary, opens the device without `device.gpu.shell` and the kernel
//! issuer (NATIVE-RENDER-BOOT-001), and its first line says `product=false device_authority=none`.

use std::process::ExitCode;

use handshake_shell::ExitKind;

const SHOW_FLAG: &str = "--show-on-my-desktop";
const USAGE: &str = "usage: shell_program --show-on-my-desktop [--seconds N] [--diag-ring PATH]
Opens one quiet window on this desktop. It does not take the focus and sits behind other windows.
Without --show-on-my-desktop nothing is opened.";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if !arguments.iter().any(|argument| argument == SHOW_FLAG) {
        eprintln!("{USAGE}");
        return ExitCode::from(ExitKind::Usage.code());
    }
    show(&arguments)
}

#[cfg(not(windows))]
fn show(_: &[String]) -> ExitCode {
    eprintln!("the shell runs on Windows only so far");
    ExitCode::from(ExitKind::Unsupported.code())
}

#[cfg(windows)]
fn show(arguments: &[String]) -> ExitCode {
    use std::time::Duration;

    use handshake_diag_ring::{DiagRingWriter, DEFAULT_CAPACITY};
    use handshake_shell::{
        run_program, DeviceAuthority, EndAfter, NoWatcher, ProgramConfig, TestBinary,
    };

    let value = |name: &str| {
        arguments
            .iter()
            .position(|argument| argument == name)
            .and_then(|at| arguments.get(at + 1))
    };
    let seconds = value("--seconds")
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .unwrap_or(30)
        .clamp(1, 600);
    let ring = value("--diag-ring")
        .and_then(|path| DiagRingWriter::create(std::path::Path::new(path), DEFAULT_CAPACITY).ok());
    let authority = DeviceAuthority::Test(TestBinary::declare("shell_program_example"));
    println!("{}", authority.receipt());
    let mut config = ProgramConfig::new(ring);
    config.end_after = EndAfter {
        time: Some(Duration::from_secs(seconds)),
        frames_presented: None,
    };
    let outcome = run_program(config, authority, Box::new(NoWatcher));
    println!("{}", outcome.line());
    ExitCode::from(outcome.kind.code())
}
