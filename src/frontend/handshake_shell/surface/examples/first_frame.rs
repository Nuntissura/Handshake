//! The first Handshake frame without egui, on the Operator's own desktop: one quiet window, cleared to the dark theme's
//! background by the central GPU owner and presented through Vulkan, then read back from the swapchain and compared
//! texel by texel. For the Operator to look at, by hand. Tests and agents never run it.
//!
//! ```text
//! cargo run --manifest-path src/frontend/handshake_shell/Cargo.toml -p handshake_shell_surface --example first_frame -- --show-on-my-desktop [--seconds N]
//! ```
//!
//! Without the flag it prints how to use it and opens nothing. The window is quiet: it never takes the foreground or
//! the keyboard, has no taskbar button and sits at the bottom of the Z order, so other windows may cover it. It closes
//! after N seconds (30 unless given), or when its close button is used.
//!
//! Not the product: it opens the device without the `device.gpu.shell` capability and the kernel issuer
//! (NATIVE-RENDER-BOOT-001, MT-214 S5), with its own quota policy, and prints so in its receipt.

use std::process::ExitCode;

const SHOW_FLAG: &str = "--show-on-my-desktop";
const USAGE: &str = "usage: first_frame --show-on-my-desktop [--seconds N]
Opens one quiet window on this desktop with a Vulkan frame in it. It does not take the focus and sits behind other
windows. Without --show-on-my-desktop nothing is opened.";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if !arguments.iter().any(|argument| argument == SHOW_FLAG) {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let seconds = arguments
        .iter()
        .position(|argument| argument == "--seconds")
        .and_then(|at| arguments.get(at + 1))
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30)
        .clamp(1, 600);
    show(seconds)
}

#[cfg(not(windows))]
fn show(_: u64) -> ExitCode {
    eprintln!("the shell presents on Windows only so far");
    ExitCode::from(3)
}

#[cfg(windows)]
fn show(seconds: u64) -> ExitCode {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use handshake_shell_surface::{ClearFrame, PresentConfig, PresentHost, Shell, ShellApp};
    use handshake_shell_window::{
        CommandSource, DispatchedWake, Event, LogicalSize, LoopConfig, Placement, WakeLane,
        WindowClass, WindowCommand, WindowEvent, WindowKey, WindowSpec,
    };
    use handshake_vulkan_renderer::{ModuleId, NullDiagnostics, PolicyRevision, QuotaPolicy};

    struct FirstFrame {
        lane: RefCell<Option<WakeLane>>,
        window: Cell<Option<WindowKey>>,
        until: Instant,
        captured: Cell<bool>,
        ending: Cell<bool>,
        frame: ClearFrame,
    }

    impl FirstFrame {
        fn tick(&self) {
            if let Some(lane) = self.lane.borrow().as_ref() {
                let _ = lane.request(0, Duration::from_millis(100));
            }
        }

        fn end(&self, shell: &Shell<'_>) {
            if self.ending.replace(true) {
                return;
            }
            let report = shell.report();
            println!(
                "HSK_SHELL_END frames_due={} presented={} failed={} not_ready={} configures={} errors={}",
                report.frames_due,
                report.frames_presented,
                report.frames_failed,
                report.not_ready,
                report.configures,
                report.last_error.map_or("none", |error| error.code())
            );
            let _ = shell.shutdown(0);
        }
    }

    impl ShellApp for FirstFrame {
        fn on_event(&self, shell: &Shell<'_>, event: &Event) {
            match event {
                Event::LoopStarted => {
                    *self.lane.borrow_mut() = shell.control().open_wake_lane().ok();
                    let placement = shell
                        .control()
                        .monitors()
                        .into_iter()
                        .find(|monitor| monitor.primary)
                        .map_or(Placement::System, |monitor| Placement::Centered(monitor.key));
                    let spec = WindowSpec {
                        title: "Handshake shell: first Vulkan frame".to_owned(),
                        inner: LogicalSize::new(640.0, 400.0),
                        min_inner: None,
                        placement,
                        class: WindowClass::Quiet,
                    };
                    match shell.open_window(&CommandSource::host(1), spec) {
                        Ok(window) => self.window.set(Some(window)),
                        Err(refusal) => {
                            eprintln!("HSK_SHELL_NO_WINDOW refusal={}", refusal.code());
                            let _ = shell.shutdown(5);
                        }
                    }
                }
                Event::Window {
                    window,
                    event: WindowEvent::Created { .. },
                } => {
                    if let Some(presentation) = shell.presentation(*window) {
                        if let Some(info) = presentation.swapchain {
                            println!(
                                "HSK_SHELL_SWAPCHAIN format={} mode={} images={} extent={}x{}",
                                info.format.key(),
                                info.mode.key(),
                                info.images,
                                info.extent.width,
                                info.extent.height
                            );
                        }
                        if let Some(error) = presentation.last_error {
                            eprintln!("HSK_SHELL_ERROR code={}", error.code());
                        }
                    }
                    let shown = shell.control().command(
                        &CommandSource::host(2),
                        *window,
                        WindowCommand::Show { activate: false },
                    );
                    if let Err(refusal) = shown {
                        eprintln!("HSK_SHELL_NOT_SHOWN refusal={}", refusal.code());
                    }
                    self.tick();
                }
                Event::Window {
                    event: WindowEvent::CloseRequested,
                    ..
                } => self.end(shell),
                _ => {}
            }
        }

        fn on_wake(&self, shell: &Shell<'_>, _: &DispatchedWake<'_>) {
            let Some(window) = self.window.get() else {
                return;
            };
            if !self.captured.get() && shell.report().frames_presented >= 1 {
                self.captured.set(true);
                let _ = shell.capture_next(window);
            }
            if let Some(capture) = shell.take_capture(window) {
                match capture.map(|readback| readback.compare(&self.frame)) {
                    Ok(Ok(comparison)) => println!(
                        "HSK_SHELL_READBACK texels={} mismatched={} matches={}",
                        comparison.texels,
                        comparison.mismatched,
                        comparison.matches()
                    ),
                    Ok(Err(refusal)) => eprintln!("HSK_SHELL_READBACK refused={refusal:?}"),
                    Err(error) => eprintln!("HSK_SHELL_READBACK error={}", error.code()),
                }
            }
            if Instant::now() >= self.until {
                self.end(shell);
            } else {
                self.tick();
            }
        }
    }

    // The example's own policy: one window's claim and room for its frames. The product's comes from Core.
    let policy = QuotaPolicy::Configured {
        revision: PolicyRevision(1),
        limit_bytes: 512 << 20,
        max_claims: 4,
        max_in_flight_per_claim: 16,
    };
    let config = PresentConfig::new(policy, ModuleId(214));
    let app = Rc::new(FirstFrame {
        lane: RefCell::new(None),
        window: Cell::new(None),
        until: Instant::now() + Duration::from_secs(seconds),
        captured: Cell::new(false),
        ending: Cell::new(false),
        frame: config.frame,
    });
    let host = match PresentHost::open(config, Arc::new(NullDiagnostics), app) {
        Ok(host) => host,
        Err(reason) => {
            eprintln!(
                "HSK_SHELL_UNAVAILABLE reason={} number={}",
                reason.key(),
                reason.number()
            );
            return ExitCode::from(4);
        }
    };
    let identity = host.owner().identity();
    println!(
        "HSK_SHELL_RECEIPT binary=example product=false device_authority=none device=\"{}\" driver=\"{}\" api={}.{}.{}",
        identity.name,
        identity.driver_name,
        identity.api_version[0],
        identity.api_version[1],
        identity.api_version[2]
    );
    let config = LoopConfig {
        launched_by_operator: false,
        ..LoopConfig::default()
    };
    match handshake_shell_window::run(config, host.clone()) {
        Ok(report) => {
            println!(
                "HSK_SHELL_LOOP exit={} windows_created={} windows_destroyed={} quiet_broken={}",
                report.exit_code, report.windows_created, report.windows_destroyed, report.quiet_broken
            );
            ExitCode::from(u8::try_from(report.exit_code).unwrap_or(1))
        }
        Err(error) => {
            eprintln!("HSK_SHELL_LOOP error={}", error.code());
            ExitCode::from(6)
        }
    }
}
