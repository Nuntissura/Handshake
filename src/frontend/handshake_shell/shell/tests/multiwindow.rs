//! Several windows of the shell program on this machine's real device, as a test binary, quiet, on a private desktop that
//! receives no input, in a process of its own (MT-214 S6, AC214-8): the root window, two detached panes and the detached
//! Settings window share one device and one owner ledger; closing one detached pane releases only its surface and claim,
//! and the windows that stay present a frame after it.
//!
//! Never on the Operator's desktop: `on_private_desktop` runs the loop on a thread bound to a desktop of its own. A
//! machine without a device skips, with the owner's reason, unless `HANDSHAKE_REQUIRE_VULKAN_GPU` says a device is
//! required.
#![cfg(windows)]

#[path = "../../surface/tests/support/mod.rs"]
mod support;

#[path = "../../../handshake_host_ports/ui_heartbeat/tests/support/limits.rs"]
mod limits;

use std::time::Duration;

use handshake_shell::{
    run_program, Action, DetachedSpec, DeviceAuthority, EndAfter, ExitKind, NoWatcher, ProgramConfig,
    TestBinary, WindowKind,
};
use handshake_shell_window::{LogicalSize, PhysicalPosition, Placement};

use limits::TimeLimit;
use support::desktop::on_private_desktop;
use support::gpu_env::{in_own_process, is_required, ran_in_its_own_process, REQUIRE_GPU};

const LIMIT: Duration = Duration::from_secs(100);
const TIME_LIMIT: Duration = Duration::from_secs(90);
/// Long enough to create four windows, close one (up to two seconds to prove its presents ended without
/// `VK_EXT_swapchain_maintenance1`) and see the others present after it.
const RUN_FOR: Duration = Duration::from_millis(6000);

fn detached(kind: WindowKind, title: &str) -> Action {
    let spec = DetachedSpec::new(kind, title, LogicalSize::new(200.0, 120.0)).expect("a detached kind");
    Action::Open(spec)
}

fn config() -> ProgramConfig {
    let mut config = ProgramConfig::new(None);
    config.title = "multiwindow root".to_owned();
    config.size = LogicalSize::new(240.0, 140.0);
    config.placement = Some(Placement::At(PhysicalPosition::new(40, 40)));
    config.end_after = EndAfter {
        time: Some(RUN_FOR),
        frames_presented: None,
    };
    config.plan = vec![
        detached(WindowKind::DetachedPane, "detached pane one"),
        detached(WindowKind::DetachedSettings, "detached settings"),
        detached(WindowKind::DetachedPane, "detached pane two"),
        // Every window exists and shows its first frame, then the first detached pane is closed and the rest must present again.
        Action::AwaitCreated(4),
        Action::AwaitPresented(4),
        Action::CloseDetached(0),
        Action::AwaitSurvivors,
    ];
    config
}

#[test]
fn the_root_detached_panes_and_settings_share_one_device_and_closing_one_keeps_the_others() {
    in_own_process!();
    let _limit = TimeLimit::new(TIME_LIMIT);
    let outcome = on_private_desktop(LIMIT, move || {
        let authority = DeviceAuthority::Test(TestBinary::declare("shell_multiwindow_test"));
        assert!(authority.receipt().contains("product=false"));
        run_program(config(), authority, Box::new(NoWatcher))
    });
    if outcome.kind == ExitKind::DeviceUnavailable {
        assert!(
            !is_required(std::env::var_os(REQUIRE_GPU).as_deref()),
            "no device to run on ({:?}), and {REQUIRE_GPU} is not 0",
            outcome.detail
        );
        let state = outcome.unavailable.expect("a missing device is a typed, readable state");
        assert_eq!(state.code(), outcome.detail.expect("the owner's key"));
        println!("HSK_GPU_SKIP reason={}", outcome.detail.unwrap_or("none"));
        return;
    }
    assert_eq!(outcome.kind, ExitKind::Clean, "{}", outcome.line());
    assert!(outcome.unavailable.is_none(), "a clean run has no unavailable state");
    let report = outcome.report.expect("the loop ran");

    // The windows the contract names, all opened, each of its kind, none with a key used before.
    assert_eq!(report.windows.of(WindowKind::Root), 1);
    assert_eq!(report.windows.of(WindowKind::DetachedPane), 2);
    assert_eq!(report.windows.of(WindowKind::DetachedSettings), 1);
    assert_eq!(report.run.windows_created, 4, "{:?}", report.run);
    assert_eq!(report.key_reuse, 0, "a window key was used twice");
    assert_eq!(report.plan_refused, 0, "a step named a window or a baseline that did not exist");
    assert!(report.plan_done, "the plan did not finish in {RUN_FOR:?}: {report:?}");

    // One device, one owner ledger: every window's claim was admitted by the same ledger, on the one generation, and
    // they were open at once.
    assert_eq!(report.present.surfaces_made, 4, "{:?}", report.present);
    assert_eq!(report.ledger_admitted, 4, "one claim per window in the one ledger");
    assert_eq!(report.ledger_generations, 1, "one device generation served every window");
    assert_eq!(report.claims_peak, 4, "all four claims were open in that ledger at once");

    // Closing one detached pane: it was destroyed before the shutdown, and only it. The windows that stayed presented a
    // frame after it, with no error.
    assert_eq!(report.windows_closed_early, 1, "{report:?}");
    assert_eq!(report.survivors_presented, Some(true), "the others did not present after the close");
    assert_eq!(report.present.errors, 0, "{:?}", report.present.last_error);
    assert_eq!(report.present.device_losses, 0);

    // Everything ended: every window destroyed, every claim settled, the owner closed with nothing held, nothing
    // activated a quiet window.
    assert_eq!(report.run.windows_destroyed, 4);
    assert_eq!(report.run.quiet_broken, 0, "a window took the foreground");
    assert!(report.settled, "{report:?}");
    assert_eq!(report.present.claims_abandoned, 0);
    assert_eq!(report.present.claims_settled, report.present.surfaces_made);
    assert!(
        report.present.shutdown.is_some_and(|owner| owner.is_settled()),
        "the owner closed with nothing held: {:?}",
        report.present.shutdown
    );

    // One accessibility tree per window, each attached before it was shown.
    assert_eq!(report.attach_refused, 0, "{:?}", report.access_code);
    assert_eq!(report.access.frames, 4, "{:?}", report.access);
    assert_eq!(report.access.frames_refused, 0);
    assert_eq!(report.watch_refused, 0);
}
