//! What the GPU tests rely on, without a GPU: a test process that hangs is told apart from one that fails, and a Vulkan
//! device is required unless somebody says otherwise.

mod support;

use std::ffi::OsStr;
use std::time::Duration;
use support::gpu_env::{is_required, run_child, Ended, REQUIRE_GPU};

/// What the test of this name does when it runs in a process of its own.
const PLAY: &str = "HANDSHAKE_VULKAN_HARNESS_PLAY";

fn plays(part: &str) -> bool {
    std::env::var(PLAY).is_ok_and(|played| played == part)
}

#[test]
fn a_test_process_that_hangs_is_killed_and_reported_as_a_timeout() {
    if plays("hang") {
        std::thread::sleep(Duration::from_secs(600));
    }
    let name = "a_test_process_that_hangs_is_killed_and_reported_as_a_timeout";
    let deadline = Duration::from_secs(3);
    let run = run_child(name, deadline, &[(PLAY, "hang")]);
    assert_eq!(run.ended, Ended::TimedOut);
    let verdict = run.verdict(name, deadline).expect("not a pass");
    assert!(verdict.starts_with("TIMEOUT: "), "{verdict}");
    assert!(!verdict.contains("FAILED"), "{verdict}");
}

#[test]
fn a_test_process_that_fails_is_reported_as_failed() {
    assert!(!plays("fail"), "as it should");
    let name = "a_test_process_that_fails_is_reported_as_failed";
    let deadline = Duration::from_secs(60);
    let run = run_child(name, deadline, &[(PLAY, "fail")]);
    assert_eq!(run.ended, Ended::Failed);
    let verdict = run.verdict(name, deadline).expect("not a pass");
    assert!(verdict.starts_with("FAILED: "), "{verdict}");
    assert!(run.output.contains("0 passed; 1 failed"), "{}", run.output);
}

#[test]
fn a_test_process_that_passes_has_no_verdict() {
    if plays("pass") {
        return;
    }
    let name = "a_test_process_that_passes_has_no_verdict";
    let deadline = Duration::from_secs(60);
    let run = run_child(name, deadline, &[(PLAY, "pass")]);
    assert_eq!(
        (run.ended, run.verdict(name, deadline)),
        (Ended::Passed, None)
    );
    assert!(run.output.contains("1 passed"), "{}", run.output);
}

#[test]
fn a_vulkan_device_is_required_unless_the_variable_says_zero() {
    for value in [None, Some(""), Some("1"), Some("no"), Some("00")] {
        assert!(is_required(value.map(OsStr::new)), "{value:?}");
    }
    assert!(!is_required(Some(OsStr::new("0"))));
    // And the configuration that is checked in says so to everything cargo runs here.
    let config = include_str!("../.cargo/config.toml");
    let entry = format!("{REQUIRE_GPU} = {{ value = \"1\", force = false }}");
    assert!(config.lines().any(|line| line == entry), "{entry}");
}
