//! WP-KERNEL-011 MT-028 — swarm concurrency-safety proof (over the real TCP transport).
//!
//! This is the concurrency-safety counterpart to MT-027's over-the-wire steering proof. It drives N
//! REAL MCP servers (one per agent token, all SHARING one lease registry + one attribution log) over
//! loopback TCP from N concurrent `tokio` tasks, each issuing a mix of `list_widgets` (shared-lease
//! reads) + `click_widget` + `set_value` (exclusive-lease mutations), and proves the MT-028 contract's
//! guarantees:
//!
//!   DETERMINISTIC + NO TORN STATE -> with N=5 agents hammering the SAME shared widgets concurrently,
//!     the exclusive per-widget leases SERIALIZE same-widget access (generous timeout, so they WAIT
//!     rather than time out), so every one of the N*M mutations enqueues exactly once — no race, no
//!     lost/duplicated action, no panic, no corrupted JSON response.
//!   ATTRIBUTED -> the shared action log holds exactly N*M entries, each carrying the agent_id of the
//!     task that issued it (= first 8 hex of SHA-256 of that task's distinct session token), and all N
//!     agent_ids are present and distinct.
//!   BOUNDED + QUIET -> the whole run completes well under the contract's 10s budget with no deadlock.
//!
//! Companion focused proofs:
//!   - `test_lease_exclusive_timeout`: two agents contend for the SAME widget's exclusive lease with a
//!     SHORT timeout; one wins, the other gets JSON-RPC -32004 "Lease timeout".
//!   - `test_layout_guard_rollback`: the restartable LayoutGuard restores the pre-op layout exactly.
//!
//! ## Test harness shape (mirrors MT-027's wire tests)
//!
//! egui/`live_snapshot` construct + drop a tokio current-thread runtime, which panics if dropped inside
//! an async context — so all egui construction happens on the test thread (sync) and only the socket I/O
//! runs inside a dedicated multi-thread `block_on`. The wire tests serialize on `WIRE_TEST_GUARD`
//! because each redirects the process-global app-data env var (the binding-file location) to its own
//! temp dir.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use handshake_native::accessibility::{collect_ui_tree_snapshot, UiTreeSnapshot};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::mcp::{
    agent_id_for_token, ActionChannel, ActionLog, LeaseKind, LeaseRegistry, ScreenshotError,
    ScreenshotResult, SessionToken, SwarmMcpServer, SwarmSafetyState, ERR_LEASE_TIMEOUT,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

const THEME_TOGGLE_AUTHOR_ID: &str = "shell.chrome.theme-toggle";
const RAIL_INPUT_AUTHOR_ID: &str = "bottom-rail.input";

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// One frame of the REAL shell on a plain AccessKit-enabled ctx -> the live UI-tree snapshot. The stable
/// widget NodeIds are fixed, so this snapshot addresses the same nodes the shell renders (the MT-026/027
/// path).
fn live_snapshot() -> UiTreeSnapshot {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = ok_app();
    let output = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
    let update = output
        .platform_output
        .accesskit_update
        .expect("AccessKit update produced (accesskit enabled + one frame run)");
    collect_ui_tree_snapshot(&update)
}

/// A dedicated multi-thread runtime so the concurrent tasks exercise TRUE parallelism (the contract's
/// `flavor = multi_thread, worker_threads = 4` requirement) and lock contention is real.
fn wire_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("build wire test runtime")
}

/// Serialize the wire tests (they redirect the process-global app-data env var for the binding file).
static WIRE_TEST_GUARD: Mutex<()> = Mutex::new(());

fn wire_guard() -> std::sync::MutexGuard<'static, ()> {
    WIRE_TEST_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

fn redirect_app_data(tmp: &std::path::Path) -> (&'static str, Option<std::ffi::OsString>) {
    #[cfg(target_os = "windows")]
    let var = "LOCALAPPDATA";
    #[cfg(not(target_os = "windows"))]
    let var = "XDG_DATA_HOME";
    let prev = std::env::var_os(var);
    std::env::set_var(var, tmp);
    (var, prev)
}

fn restore_app_data(var: &str, prev: Option<std::ffi::OsString>) {
    match prev {
        Some(v) => std::env::set_var(var, v),
        None => std::env::remove_var(var),
    }
}

/// A no-op screenshot capture (this test exercises leasing/attribution on the steering path, not the
/// GPU screenshot path).
fn stub_capture() -> Arc<dyn Fn() -> Result<ScreenshotResult, ScreenshotError> + Send + Sync> {
    Arc::new(|| Ok(handshake_native::mcp::screenshot::screenshot_from_png(b"x", 1, 1)))
}

/// Send one JSON-RPC request line and read one response line over a fresh TCP connection.
async fn rpc_once(addr: &str, request: serde_json::Value) -> serde_json::Value {
    let stream = TcpStream::connect(addr).await.expect("connect to mcp server");
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);

    let mut line = serde_json::to_string(&request).unwrap();
    line.push('\n');
    write_half.write_all(line.as_bytes()).await.expect("write request");
    write_half.flush().await.expect("flush");

    let mut resp = String::new();
    reader.read_line(&mut resp).await.expect("read response");
    serde_json::from_str(resp.trim()).expect("response is valid JSON")
}

/// AC / proof_target `test_swarm_concurrency_n5`: N=5 concurrent agents, each issuing 20 mutating
/// actions (+ interleaved reads) over the wire, share one lease registry + one attribution log. Proves:
/// 0 panics, exactly 100 attributed entries, 5 distinct agent_ids, < 10s, no torn state.
#[test]
fn test_swarm_concurrency_n5() {
    const N: usize = 5; // concurrent agents
    const M: usize = 20; // mutating actions per agent

    let tmp = std::env::temp_dir().join(format!("hsk_mcp_swarm_n5_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let _wire_guard = wire_guard();
    let (var, prev) = redirect_app_data(&tmp);

    // ONE shared registry + log across all N servers; the live UI-tree snapshot + a shared channel are
    // the same Arc<Mutex<_>> the running shell would own. Sync construction, outside block_on.
    let shared_leases = LeaseRegistry::new();
    let shared_log = ActionLog::new();
    let snapshot = Arc::new(Mutex::new(live_snapshot()));
    let channel = Arc::new(Mutex::new(ActionChannel::with_capacity(10_000))); // hold all N*M without back-pressure

    // One DISTINCT token per agent -> one DISTINCT agent_id per agent. Compute the expected agent_ids up
    // front so the assertion is independent of the server's derivation.
    let tokens: Vec<SessionToken> = (0..N).map(|_| SessionToken::generate()).collect();
    let token_hexes: Vec<String> = tokens.iter().map(|t| t.as_hex().to_owned()).collect();
    let expected_agent_ids: Vec<String> =
        token_hexes.iter().map(|h| agent_id_for_token(h)).collect();

    let rt = wire_runtime();
    let started = Instant::now();
    let addrs: Vec<String> = rt.block_on(async {
        let mut addrs = Vec::with_capacity(N);
        // Keep the servers alive for the whole run (collected so they are not dropped early).
        let mut servers: Vec<SwarmMcpServer> = Vec::with_capacity(N);
        for token in tokens {
            // Generous lease timeout: contending agents WAIT (serialize) rather than time out, so all
            // N*M mutations succeed deterministically. The timeout path is proven separately.
            let safety = SwarmSafetyState::with_shared(
                token,
                snapshot.clone(),
                channel.clone(),
                shared_leases.clone(),
                shared_log.clone(),
            )
            .with_lease_timeout(Duration::from_secs(5));
            let server = SwarmMcpServer::bind_with_safety(safety, stub_capture())
                .await
                .expect("bind agent server");
            addrs.push(server.tcp_addr().to_owned());
            servers.push(server);
        }

        // Spawn N concurrent agent tasks. Each issues M mutating actions (alternating click_widget on the
        // theme toggle + set_value on the rail input — both SHARED widgets, so the agents contend on the
        // SAME exclusive leases) interleaved with list_widgets reads (shared leases, not logged).
        let mut handles = Vec::with_capacity(N);
        for (agent_idx, addr) in addrs.iter().cloned().enumerate() {
            let token_hex = token_hexes[agent_idx].clone();
            handles.push(tokio::spawn(async move {
                let mut mutating_ok = 0usize;
                let mut read_ok = 0usize;
                for i in 0..M {
                    // A read between mutations (shared lease; exercises reader/writer coexistence).
                    let read = rpc_once(
                        &addr,
                        serde_json::json!({
                            "jsonrpc": "2.0", "id": i * 2, "method": "list_widgets",
                            "params": {}, "session_token": token_hex,
                        }),
                    )
                    .await;
                    if read["result"]["widget_count"].as_u64().unwrap_or(0) > 0 {
                        read_ok += 1;
                    }

                    // A mutating action (exclusive per-widget lease).
                    let (method, params) = if i % 2 == 0 {
                        ("click_widget", serde_json::json!({ "target": THEME_TOGGLE_AUTHOR_ID }))
                    } else {
                        (
                            "set_value",
                            serde_json::json!({ "target": RAIL_INPUT_AUTHOR_ID, "value": format!("a{agent_idx}-{i}") }),
                        )
                    };
                    let resp = rpc_once(
                        &addr,
                        serde_json::json!({
                            "jsonrpc": "2.0", "id": i * 2 + 1, "method": method,
                            "params": params, "session_token": token_hex,
                        }),
                    )
                    .await;
                    // Under a generous timeout the lease only WAITS; every mutation must enqueue.
                    assert_eq!(
                        resp["result"]["queued"], true,
                        "agent {agent_idx} action {i} ({method}) must enqueue (no torn state / no lost action); got {resp}"
                    );
                    assert!(resp.get("error").is_none(), "no error on the happy path; got {resp}");
                    // AC#2: each mutating success carries THIS agent's agent_id over the wire.
                    assert_eq!(
                        resp["result"]["agent_id"], agent_id_for_token(&token_hex),
                        "agent {agent_idx} action {i} result is stamped with its agent_id; got {resp}"
                    );
                    mutating_ok += 1;
                }
                (mutating_ok, read_ok)
            }));
        }

        // Join all agents; a panic in any task surfaces here (the "zero panics" assertion).
        let mut total_mutations = 0usize;
        let mut total_reads = 0usize;
        for h in handles {
            let (m, r) = h.await.expect("agent task completed without panicking");
            total_mutations += m;
            total_reads += r;
        }
        assert_eq!(total_mutations, N * M, "every agent completed all its mutations");
        assert_eq!(total_reads, N * M, "every read returned the live tree");

        for mut server in servers {
            server.shutdown();
        }
        addrs
    });
    let elapsed = started.elapsed();

    // The shared attribution log holds EXACTLY N*M entries (reads are not logged; only mutations are).
    let entries = shared_log.drain_log();
    assert_eq!(
        entries.len(),
        N * M,
        "exactly N*M={} attributed mutations recorded; got {}",
        N * M,
        entries.len()
    );

    // Every entry has a non-empty agent_id, and all N expected agent_ids are present + distinct.
    assert!(
        entries.iter().all(|e| !e.agent_id.is_empty()),
        "every attribution entry has a non-empty agent_id"
    );
    let mut seen: std::collections::BTreeSet<&str> =
        entries.iter().map(|e| e.agent_id.as_str()).collect();
    assert_eq!(seen.len(), N, "exactly N distinct agent_ids in the log; got {seen:?}");
    for id in &expected_agent_ids {
        assert!(
            seen.remove(id.as_str()),
            "expected agent_id {id} (= SHA-256(token)[..8]) present in the log"
        );
    }

    // Per-agent fairness: each agent contributed exactly M entries (no agent starved or double-counted).
    let mut per_agent: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for e in &entries {
        *per_agent.entry(e.agent_id.as_str()).or_default() += 1;
    }
    for (id, count) in &per_agent {
        assert_eq!(*count, M, "agent {id} recorded exactly M={M} actions (fairness); got {count}");
    }

    // Seqs are strictly monotonic (ordering is well-defined even under concurrency -> no torn log).
    for w in entries.windows(2) {
        assert!(w[1].seq > w[0].seq, "attribution seqs are strictly increasing");
    }

    // All leases released (no leaked exclusive holder after the run).
    assert_eq!(shared_leases.active_resource_count(), 0, "all leases released after the run");

    // Bounded: well under the 10s contract budget.
    assert!(elapsed < Duration::from_secs(10), "run completed in {elapsed:?} (< 10s budget)");

    println!(
        "PASS test_swarm_concurrency_n5: agents={N} actions/agent={M} -> log_entries={} (== N*M={}), \
         distinct_agent_ids={} servers={} elapsed={:?} panics=0 leases_leaked=0",
        entries.len(),
        N * M,
        per_agent.len(),
        addrs.len(),
        elapsed
    );

    restore_app_data(var, prev);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// AC / proof_target `test_lease_exclusive_timeout`: two agents contend for the SAME widget's exclusive
/// lease with a SHORT timeout. One holds the lease out-of-band; the other's mutating request must get
/// JSON-RPC -32004 "Lease timeout" within the timeout window (the contract's 100ms case).
#[test]
fn test_lease_exclusive_timeout() {
    let tmp = std::env::temp_dir().join(format!("hsk_mcp_lease_timeout_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let _wire_guard = wire_guard();
    let (var, prev) = redirect_app_data(&tmp);

    let shared_leases = LeaseRegistry::new();
    let shared_log = ActionLog::new();
    let snapshot = Arc::new(Mutex::new(live_snapshot()));
    let channel = Arc::new(Mutex::new(ActionChannel::new()));

    let token = SessionToken::generate();
    let token_hex = token.as_hex().to_owned();

    let rt = wire_runtime();
    rt.block_on(async {
        // Server with a SHORT lease timeout so a contended acquire fails fast (the contract's 100ms case).
        let safety = SwarmSafetyState::with_shared(
            token,
            snapshot.clone(),
            channel.clone(),
            shared_leases.clone(),
            shared_log.clone(),
        )
        .with_lease_timeout(Duration::from_millis(100));
        let mut server = SwarmMcpServer::bind_with_safety(safety, stub_capture())
            .await
            .expect("bind server");
        let addr = server.tcp_addr().to_owned();

        // Agent A: hold the theme-toggle's exclusive lease out-of-band (simulating an in-flight op).
        let held = shared_leases
            .try_acquire(THEME_TOGGLE_AUTHOR_ID, LeaseKind::Exclusive, Duration::from_millis(50))
            .expect("agent A holds the exclusive widget lease");

        // Agent B: click the SAME widget over the wire -> the server cannot acquire the lease within
        // 100ms -> JSON-RPC -32004 "Lease timeout".
        let started = Instant::now();
        let resp = rpc_once(
            &addr,
            serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "method": "click_widget",
                "params": { "target": THEME_TOGGLE_AUTHOR_ID }, "session_token": token_hex,
            }),
        )
        .await;
        let waited = started.elapsed();
        assert_eq!(
            resp["error"]["code"], ERR_LEASE_TIMEOUT,
            "the contended click is rejected with -32004 Lease timeout; got {resp}"
        );
        assert!(resp.get("result").is_none(), "no result leaked when the lease timed out");
        assert!(waited < Duration::from_secs(2), "the timeout fired promptly ({waited:?})");
        assert_eq!(channel.lock().unwrap().pending(), 0, "no action enqueued when the lease timed out");

        // Release agent A's lease; the SAME click now succeeds (the lease is acquirable again).
        drop(held);
        let resp2 = rpc_once(
            &addr,
            serde_json::json!({
                "jsonrpc": "2.0", "id": 2, "method": "click_widget",
                "params": { "target": THEME_TOGGLE_AUTHOR_ID }, "session_token": token_hex,
            }),
        )
        .await;
        assert_eq!(resp2["result"]["queued"], true, "click succeeds once the lease is free; got {resp2}");

        println!(
            "PASS test_lease_exclusive_timeout: contended click -> error code {} ({:?}) in {:?}; after release click queued={}",
            resp["error"]["code"],
            resp["error"]["message"].as_str().unwrap_or(""),
            waited,
            resp2["result"]["queued"]
        );
        server.shutdown();
    });

    restore_app_data(var, prev);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// MAJOR #1 proof (lease-first ordering through the REAL dispatch path): the per-widget LEASE — not the
/// global channel lock — is the contention gate. Hold ONE widget's exclusive lease out-of-band, then,
/// CONCURRENTLY over the wire:
///   - a click on the SAME widget must SERIALIZE on the lease -> -32004 Lease timeout, and
///   - a click/set_value on a DIFFERENT widget must NOT be blocked by that held lease -> it enqueues FAST.
///
/// ## Why the threshold is the different-widget request's OWN completion time, not a joined wall-clock
///
/// The regression this test must FALSIFY is the OLD wiring where the global channel `Mutex` is taken
/// BEFORE the lease and held across the whole (blocking) dispatch. Under that wiring, the same-widget
/// request would grab the channel lock and BLOCK on the lease wait for the full lease timeout while
/// HOLDING the channel lock — so the different-widget request (which also needs the channel lock) could
/// only proceed AFTER the same-widget request's whole lease wait finished. The different-widget request
/// would therefore complete in ~the full lease timeout, NOT promptly.
///
/// A loose JOINED threshold (e.g. "both tasks done within ~250ms" against a 150ms lease timeout) does
/// NOT falsify that regression: under the old wiring the different-widget request would still finish at
/// ~150ms < 250ms, so the assertion would pass even though the lease was bypassed. To make the test a
/// REAL falsifier we:
///   1. set the lease timeout LONG (800ms) and hold the lease LONGER (900ms), so the same-widget
///      contender waits ~800ms before timing out, and
///   2. measure the DIFFERENT-widget request's OWN t0..t_done independently and assert it is FAST
///      (< 150ms — far below the 800ms lease timeout) AND succeeded (queued + agent_id).
///
/// Only lease-first ordering can satisfy that: the different-widget request never contends on the held
/// lease and takes the channel lock only for the sub-millisecond enqueue, so it returns in well under
/// 150ms. Under channel-lock-first wiring it would be pinned behind the same-widget request's ~800ms
/// wait — an order of magnitude over the 150ms bound — and FAIL. The same-widget -> -32004 assertion is
/// kept as the serialization half of the proof.
#[test]
fn test_lease_serializes_same_widget_not_different_widgets() {
    // The same-widget lease timeout: LONG, so under the old channel-lock-first wiring the different-widget
    // request would be pinned behind ~this whole wait. The different-widget FAST bound is an order of
    // magnitude below it, so the two regimes (lease-first ~fast vs channel-lock-first ~LEASE_TIMEOUT) are
    // unambiguously separable.
    const LEASE_TIMEOUT: Duration = Duration::from_millis(800);
    // Hold the contended widget's lease LONGER than the timeout, so the same-widget click is guaranteed to
    // time out (it never acquires) — proving the serialization half over the wire.
    const HOLD: Duration = Duration::from_millis(900);
    // The different-widget request must complete in well under the lease timeout. Generous enough to
    // absorb TCP connect + scheduling jitter on a busy CI box, yet >5x below LEASE_TIMEOUT so a regression
    // (which would pin it at ~LEASE_TIMEOUT) cannot slip under it.
    const DIFFERENT_FAST_BOUND: Duration = Duration::from_millis(150);

    let tmp = std::env::temp_dir().join(format!("hsk_mcp_lease_distinct_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let _wire_guard = wire_guard();
    let (var, prev) = redirect_app_data(&tmp);

    let shared_leases = LeaseRegistry::new();
    let shared_log = ActionLog::new();
    let snapshot = Arc::new(Mutex::new(live_snapshot()));
    let channel = Arc::new(Mutex::new(ActionChannel::with_capacity(64)));

    let token = SessionToken::generate();
    let token_hex = token.as_hex().to_owned();

    let rt = wire_runtime();
    rt.block_on(async {
        let safety = SwarmSafetyState::with_shared(
            token,
            snapshot.clone(),
            channel.clone(),
            shared_leases.clone(),
            shared_log.clone(),
        )
        .with_lease_timeout(LEASE_TIMEOUT);
        let mut server = SwarmMcpServer::bind_with_safety(safety, stub_capture())
            .await
            .expect("bind server");
        let addr = server.tcp_addr().to_owned();

        // Hold the THEME_TOGGLE widget's exclusive lease out-of-band for HOLD (> LEASE_TIMEOUT), so the
        // same-widget click is guaranteed to wait the full LEASE_TIMEOUT and then time out while it is held.
        let leases_for_hold = shared_leases.clone();
        let holder = tokio::task::spawn_blocking(move || {
            let held = leases_for_hold
                .try_acquire(THEME_TOGGLE_AUTHOR_ID, LeaseKind::Exclusive, Duration::from_millis(50))
                .expect("hold theme-toggle lease");
            std::thread::sleep(HOLD);
            drop(held);
        });

        // Give the holder a moment to actually take the lease before the two wire requests fire.
        tokio::time::sleep(Duration::from_millis(30)).await;

        // CONCURRENTLY: same-widget click (must wait out the lease and time out) + different-widget
        // set_value (must succeed FAST). The different-widget task records its OWN t0..t_done so the FAST
        // assertion is independent of the same-widget request's long wait.
        let addr_same = addr.clone();
        let tok_same = token_hex.clone();
        let same = tokio::spawn(async move {
            rpc_once(
                &addr_same,
                serde_json::json!({
                    "jsonrpc": "2.0", "id": 1, "method": "click_widget",
                    "params": { "target": THEME_TOGGLE_AUTHOR_ID }, "session_token": tok_same,
                }),
            )
            .await
        });
        let addr_diff = addr.clone();
        let tok_diff = token_hex.clone();
        let different = tokio::spawn(async move {
            // Measure THIS request's own completion time (t0..t_done), independent of the same-widget wait.
            let t0 = Instant::now();
            let resp = rpc_once(
                &addr_diff,
                serde_json::json!({
                    "jsonrpc": "2.0", "id": 2, "method": "set_value",
                    "params": { "target": RAIL_INPUT_AUTHOR_ID, "value": "hello" },
                    "session_token": tok_diff,
                }),
            )
            .await;
            (resp, t0.elapsed())
        });

        let same_resp = same.await.expect("same-widget task ok");
        let (diff_resp, diff_elapsed) = different.await.expect("different-widget task ok");

        // Same widget: serialized on the LEASE -> typed -32004 timeout (NOT a successful enqueue).
        assert_eq!(
            same_resp["error"]["code"], ERR_LEASE_TIMEOUT,
            "the SAME-widget click serializes on the held lease -> -32004; got {same_resp}"
        );
        assert!(same_resp.get("result").is_none(), "no result leaked for the timed-out same-widget click");

        // Different widget: NOT blocked by the unrelated held lease -> it enqueues.
        assert_eq!(
            diff_resp["result"]["queued"], true,
            "the DIFFERENT-widget action is NOT blocked by the held theme-toggle lease; got {diff_resp}"
        );
        assert_eq!(
            diff_resp["result"]["agent_id"], token_hex_agent_id(&token_hex),
            "the different-widget success result is stamped with the acting agent_id (AC#2)"
        );

        // THE FALSIFIER: the different-widget request's OWN completion time must be FAST — far below the
        // 800ms lease timeout the same-widget contender is stuck waiting out. Under the old channel-lock-
        // first wiring this request would be pinned behind that ~800ms wait and blow past 150ms; only
        // lease-first ordering (lease acquired before the channel lock, channel lock held only for the
        // brief enqueue) lets it return this quickly.
        assert!(
            diff_elapsed < DIFFERENT_FAST_BOUND,
            "different-widget request completed FAST on its own clock (elapsed {diff_elapsed:?} < \
             {DIFFERENT_FAST_BOUND:?}); it did NOT serialize behind the same-widget {LEASE_TIMEOUT:?} \
             lease wait — proving lease-first ordering, not channel-lock-first"
        );

        holder.await.expect("holder released");

        println!(
            "PASS test_lease_serializes_same_widget_not_different_widgets: same-widget -> code {} (lease \
             timeout after ~{:?} wait), different-widget -> queued={} agent_id stamped in {:?} (< {:?}); \
             the lease is the gate, not the channel lock",
            same_resp["error"]["code"],
            LEASE_TIMEOUT,
            diff_resp["result"]["queued"],
            diff_elapsed,
            DIFFERENT_FAST_BOUND,
        );
        server.shutdown();
    });

    restore_app_data(var, prev);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// The expected agent_id for a token hex (mirrors the server's derivation) — used to assert the AC#2
/// agent_id stamp on a success result over the wire.
fn token_hex_agent_id(token_hex: &str) -> String {
    agent_id_for_token(token_hex)
}

/// AC / proof_target `test_layout_guard_rollback`: after an agent-driven layout op mutates the live
/// layout, `LayoutGuard::into_rollback()` returns the pre-op snapshot, which re-applies through the
/// shell's existing validated `apply_layout_snapshot` path to restore the EXACT prior layout (split
/// fractions match) without panicking.
#[test]
fn test_layout_guard_rollback() {
    use handshake_native::mcp::LayoutGuard;

    // A real shell whose live layout we can read, mutate, and restore.
    let mut app = ok_app();

    // BEFORE: capture the live layout (the real authority surface).
    let before = app.capture_layout_snapshot();
    let before_vertical = before.split_weights.vertical;
    let before_horizontal = before.split_weights.horizontal;

    // Checkpoint BEFORE the agent-driven op.
    let guard = LayoutGuard::checkpoint(before.clone());

    // Simulate an agent-driven layout op corrupting the split fractions to clearly-different values.
    {
        let weights = app.split_weights_mut();
        weights.vertical = 0.9;
        weights.horizontal = 0.1;
    }
    let mutated = app.capture_layout_snapshot();
    assert_ne!(
        mutated.split_weights.vertical, before_vertical,
        "the agent op actually changed the live layout"
    );

    // CONFLICT detected -> roll back: re-apply the checkpoint through the existing validated path.
    let extent = app.monitor_extent();
    let restored_snapshot = guard.into_rollback();
    assert!(restored_snapshot.validate().is_ok(), "the rolled-back snapshot is valid");
    app.apply_layout_snapshot(restored_snapshot, extent)
        .expect("apply_layout_snapshot restores the checkpoint without error");

    // AFTER: the live layout matches the pre-op state exactly (tile/split state restored).
    let after = app.capture_layout_snapshot();
    assert!(
        (after.split_weights.vertical - before_vertical).abs() < f32::EPSILON,
        "rollback restored the vertical split fraction ({} -> {})",
        before_vertical,
        after.split_weights.vertical
    );
    assert!(
        (after.split_weights.horizontal - before_horizontal).abs() < f32::EPSILON,
        "rollback restored the horizontal split fraction"
    );

    println!(
        "PASS test_layout_guard_rollback: layout before(v={before_vertical:.3},h={before_horizontal:.3}) \
         -> mutated(v={:.3}) -> rolled back to (v={:.3},h={:.3}) — match, no panic",
        mutated.split_weights.vertical,
        after.split_weights.vertical,
        after.split_weights.horizontal
    );
}
