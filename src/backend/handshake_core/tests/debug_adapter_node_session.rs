// WP-KERNEL-009 / MT-254 — REAL Node debug session over the V8 Inspector.
//
// This test launches an ACTUAL `node` child process under --inspect-brk, drives
// it through the Handshake DebugAdapter (CDP), and asserts every capability with
// REAL values read from the running process:
//   * set a breakpoint and assert verified == true (REAL binding via CDP),
//   * run, hit the breakpoint, read the call stack (REAL frame line),
//   * read scopes + variables and find the REAL local values (a=2, b=40),
//   * evaluate("a + b") at the paused frame == REAL 42,
//   * step over advances the current line,
//   * continue to completion, Terminated with exit code 0.
//
// No mock, no canned stack, no faked verified flag: if `node` is not on PATH the
// test reports ENVIRONMENT_BLOCKED rather than passing falsely.

use std::io::Write;

use handshake_core::debug_adapter::{
    launch, DebugAdapter, LaunchRequest, SourceBreakpoint, StepKind,
};

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A tiny deterministic script with a function whose body has a stable
/// breakpoint line. Lines (1-based):
///   1: function add(a, b) {
///   2:   const sum = a + b;     <- breakpoint here; a=2, b=40 are in scope
///   3:   return sum;
///   4: }
///   5: const result = add(2, 40);
///   6: console.log("result=" + result);
const FIXTURE: &str = "function add(a, b) {\n  const sum = a + b;\n  return sum;\n}\nconst result = add(2, 40);\nconsole.log(\"result=\" + result);\n";

#[tokio::test]
async fn node_inspector_real_breakpoint_stack_variables_eval_step_continue() {
    if !node_available() {
        panic!("ENVIRONMENT_BLOCKED: MT-254 node debug session test requires `node` on PATH");
    }

    // Write the real fixture script to a temp file.
    let dir = tempfile::tempdir().expect("temp dir");
    let script = dir.path().join("mt254_fixture.js");
    let mut f = std::fs::File::create(&script).expect("create fixture");
    f.write_all(FIXTURE.as_bytes()).expect("write fixture");
    f.sync_all().ok();
    let script_path = script.to_string_lossy().to_string();

    // Launch the REAL node process under the inspector. The session is paused at
    // entry after launch returns.
    let session = launch(LaunchRequest::node(script_path.clone()))
        .await
        .expect("launch real node session");

    // Set a breakpoint on line 2 (`const sum = a + b;`) and assert the adapter
    // REALLY bound it (verified == true via CDP setBreakpointByUrl locations).
    let bps = session
        .set_breakpoints(
            &script_path,
            &[SourceBreakpoint {
                line: 2,
                column: None,
                condition: None,
            }],
        )
        .await
        .expect("set breakpoint");
    assert_eq!(bps.len(), 1, "one breakpoint");
    assert!(
        bps[0].verified,
        "breakpoint must bind to executable code (REAL CDP verified), got {bps:?}"
    );

    // Resume from entry; the process runs into add() and hits the breakpoint.
    let mut events = session.subscribe();
    session.continue_().await.expect("continue from entry");

    // Wait for the REAL Stopped(breakpoint) at line 2.
    let stop_line = wait_for_stop(&mut events).await;
    assert_eq!(
        stop_line,
        Some(2),
        "must stop at the breakpoint line 2 (real CDP location)"
    );

    // Read the REAL call stack; the top frame is `add` at line 2.
    let frames = session.stack_trace().await.expect("stack trace");
    assert!(!frames.is_empty(), "non-empty stack at pause");
    let top = &frames[0];
    assert_eq!(top.name, "add", "top frame is add()");
    assert_eq!(top.line, 2, "top frame at breakpoint line");

    // Read the REAL local scope variables: a == 2, b == 40.
    let scopes = session.scopes(&top.id).await.expect("scopes");
    let local = scopes
        .iter()
        .find(|s| s.name == "local")
        .expect("local scope present");
    let vars = session
        .variables(&local.variables_reference)
        .await
        .expect("variables");
    let a = vars.iter().find(|v| v.name == "a").expect("var a");
    let b = vars.iter().find(|v| v.name == "b").expect("var b");
    assert_eq!(a.value, "2", "REAL runtime value of a");
    assert_eq!(b.value, "40", "REAL runtime value of b");

    // Console eval at the paused frame: a + b == 42 (REAL evaluateOnCallFrame).
    let evaled = session
        .evaluate(&top.id, "a + b")
        .await
        .expect("evaluate a + b");
    assert_eq!(evaled, "42", "console eval computes REAL sum");

    // Step over advances to line 3 (`return sum;`).
    session.step(StepKind::Over).await.expect("step over");
    let frames_after = session.stack_trace().await.expect("stack after step");
    assert_eq!(
        frames_after[0].line, 3,
        "step over advances to the next line (real)"
    );

    // Continue to completion; the process prints result=42 and exits 0.
    session.continue_().await.expect("continue to end");
    let exit = wait_for_terminated(&mut events).await;
    assert_eq!(exit, Some(0), "real node process exits 0");

    // Explicit terminate is idempotent / returns the real exit code.
    let _ = session.terminate().await;
}

async fn wait_for_stop(
    events: &mut tokio::sync::broadcast::Receiver<handshake_core::debug_adapter::DebugEvent>,
) -> Option<u32> {
    use handshake_core::debug_adapter::DebugEvent;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            panic!("timed out waiting for Stopped event");
        }
        match tokio::time::timeout(remaining, events.recv()).await {
            Ok(Ok(DebugEvent::Stopped { top_frame_line, .. })) => return top_frame_line,
            Ok(Ok(_)) => continue,
            Ok(Err(_)) => continue,
            Err(_) => panic!("timed out waiting for Stopped event"),
        }
    }
}

async fn wait_for_terminated(
    events: &mut tokio::sync::broadcast::Receiver<handshake_core::debug_adapter::DebugEvent>,
) -> Option<i32> {
    use handshake_core::debug_adapter::DebugEvent;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            panic!("timed out waiting for Terminated event");
        }
        match tokio::time::timeout(remaining, events.recv()).await {
            Ok(Ok(DebugEvent::Terminated { exit_code })) => return exit_code,
            Ok(Ok(_)) => continue,
            Ok(Err(_)) => continue,
            Err(_) => panic!("timed out waiting for Terminated event"),
        }
    }
}
