//! MT-008 LSP client proofs (WP-KERNEL-012 E1 code editor). These run STANDALONE — no backend, no
//! real language server — so they are part of the default `cargo test` run.
//!
//! AC-004 / PT-004 (`cargo test -p handshake-native lsp_client_graceful`): with NO language server
//! configured, EVERY LSP method (`initialize`, `did_open`, `did_change`, `completion`, `hover`,
//! `goto_definition`, `references`) returns empty/None without panicking (graceful degradation).
//!
//! AC-008 / PT-007: an LSP `textDocument/publishDiagnostics` NOTIFICATION (no `id`) is received over
//! the stdio transport and ROUTED to the diagnostics channel, then mapped to a gutter marker. This
//! drives the SAME production reader loop (`LspClient::spawn_reader_for_test` runs the real
//! `transport::read_loop` + `route_message`) against an in-memory pipe carrying a real
//! `Content-Length`-framed publishDiagnostics frame — proving the production notification-routing path,
//! not a parallel reimplementation. A MOCK "language server" here is the in-memory pipe writer that
//! emits one error diagnostic frame (the MT impl-note minimal stdio mock, without spawning a real OS
//! process so the test is deterministic + fast + focus-safe).

use handshake_native::code_editor::lsp_client::{
    published_diagnostics_from_lsp, LspClient, LspServerConfig,
};

/// AC-004: a client built with NO server config is not configured + not running.
#[test]
fn lsp_client_graceful_unconfigured_is_not_running() {
    let client = LspClient::disabled();
    assert!(
        !client.is_configured(),
        "AC-004: disabled client reports not configured"
    );
    assert!(
        !client.is_running(),
        "AC-004: disabled client has no spawned process"
    );

    // A config with a non-empty command IS configured (but still not spawned until did_open).
    let configured = LspClient::new(LspServerConfig::command("rust-analyzer"));
    assert!(configured.is_configured());
    assert!(
        !configured.is_running(),
        "configured but not spawned until did_open"
    );
}

/// AC-004 / PT-004: with no server, every method degrades gracefully (empty/None, no panic). Runs the
/// REAL async methods on a current-thread runtime — the same path the editor calls.
#[test]
fn lsp_client_graceful_all_methods_return_empty_without_server() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let client = LspClient::disabled();
        // initialize returns false (no server), no panic.
        assert!(
            !client.initialize(None).await,
            "AC-004: initialize false without server"
        );
        // did_open / did_change are graceful no-ops (no panic, no spawn).
        client
            .did_open("file:///x.rs", "rust", "fn main() {}")
            .await;
        client.did_change("file:///x.rs", 2, "fn main() {}").await;
        assert!(
            !client.is_running(),
            "AC-004: no process spawned for a disabled client"
        );

        let pos = lsp_types::Position {
            line: 0,
            character: 0,
        };
        assert!(
            client.completion("file:///x.rs", pos).await.is_empty(),
            "AC-004: completion empty without server"
        );
        assert!(
            client.hover("file:///x.rs", pos).await.is_none(),
            "AC-004: hover None without server"
        );
        assert!(
            client.goto_definition("file:///x.rs", pos).await.is_none(),
            "AC-004: goto_definition None without server"
        );
        assert!(
            client.references("file:///x.rs", pos).await.is_empty(),
            "AC-004: references empty without server"
        );
        println!("PT-004 lsp_client_graceful: all methods returned empty/None without a server");
    });
}

/// AC-008 / PT-007: a `publishDiagnostics` notification framed exactly as a real LSP server sends it is
/// received over the stdio transport and routed to the diagnostics channel, then mapped to a 0-based
/// gutter line + severity. The MOCK server is the in-memory pipe writer emitting one error diagnostic.
#[test]
fn lsp_publish_diagnostics_notification_is_routed_to_channel() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let client = LspClient::disabled();
        let mut diagnostics_rx = client
            .take_diagnostics_receiver()
            .expect("diagnostics receiver available before reader starts");

        // An in-memory duplex pipe stands in for the server's stdout: the test (the "mock server")
        // writes a publishDiagnostics frame; the client's REAL reader loop reads it.
        let (client_read, mut mock_write) = tokio::io::duplex(8192);
        client.spawn_reader_for_test(client_read);

        // One ERROR diagnostic on line 5 (0-based 4 in LSP coordinates), as a real server would send.
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///mock.rs",
                "diagnostics": [{
                    "range": {
                        "start": { "line": 4, "character": 0 },
                        "end": { "line": 4, "character": 7 }
                    },
                    "severity": 1,
                    "message": "expected `;`, found `}`"
                }]
            }
        });
        let frame = LspClient::frame_message_for_test(&notification);
        use tokio::io::AsyncWriteExt;
        mock_write.write_all(&frame).await.expect("write frame");
        mock_write.flush().await.expect("flush");

        // The reader routes it to the diagnostics channel (bounded wait so a failure does not hang).
        let published =
            tokio::time::timeout(std::time::Duration::from_secs(3), diagnostics_rx.recv())
                .await
                .expect("AC-008: publishDiagnostics routed within the timeout")
                .expect("AC-008: diagnostics channel delivered a notification");

        assert_eq!(published.uri, "file:///mock.rs");
        assert_eq!(
            published.diagnostics.len(),
            1,
            "AC-008: one diagnostic received"
        );
        assert_eq!(
            published.diagnostics[0].line, 4,
            "AC-008: LSP range.start.line (0-based) maps to the gutter line"
        );
        assert_eq!(
            published.diagnostics[0].severity, 1,
            "AC-008: error severity preserved"
        );
        assert!(published.diagnostics[0].message.contains("expected"));
        println!(
            "PT-007 lsp publishDiagnostics routed: uri={} line={} sev={} msg={:?}",
            published.uri,
            published.diagnostics[0].line,
            published.diagnostics[0].severity,
            published.diagnostics[0].message
        );
    });
}

/// AC-008: a malformed (non-JSON) stdout line BEFORE a valid frame is SKIPPED, never panicked on
/// (RISK-003), and the following valid publishDiagnostics frame is still routed.
#[test]
fn lsp_reader_skips_malformed_lines_then_routes_valid_frame() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let client = LspClient::disabled();
        let mut diagnostics_rx = client.take_diagnostics_receiver().expect("receiver");
        let (client_read, mut mock_write) = tokio::io::duplex(8192);
        client.spawn_reader_for_test(client_read);

        use tokio::io::AsyncWriteExt;
        // A stray non-header debug print (no Content-Length) — RISK-003: must be skipped, not panic.
        mock_write
            .write_all(b"this is a stray server debug line with no header\r\n\r\n")
            .await
            .expect("write garbage");
        // Then a malformed framed body (declares a length but the body is not JSON).
        mock_write
            .write_all(b"Content-Length: 11\r\n\r\nNOT-JSON!!!")
            .await
            .expect("write malformed body");
        // Then a VALID publishDiagnostics frame.
        let valid = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///ok.rs",
                "diagnostics": [{
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } },
                    "severity": 2,
                    "message": "unused"
                }]
            }
        });
        mock_write
            .write_all(&LspClient::frame_message_for_test(&valid))
            .await
            .expect("write valid");
        mock_write.flush().await.expect("flush");

        let published = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            diagnostics_rx.recv(),
        )
        .await
        .expect("RISK-003: reader survived malformed input and routed the valid frame")
        .expect("valid frame delivered");
        assert_eq!(published.uri, "file:///ok.rs");
        assert_eq!(published.diagnostics[0].severity, 2);
        println!("RISK-003: malformed lines skipped; valid frame still routed");
    });
}

/// Sanity: the LSP->editor diagnostic mapping (`published_diagnostics_from_lsp`) is the same function
/// the channel feeds, so a direct call mirrors what the gutter receives (AC-008 mapping).
#[test]
fn lsp_diagnostics_map_to_zero_based_lines() {
    use lsp_types::{
        Diagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Url,
    };
    let params = PublishDiagnosticsParams {
        uri: Url::parse("file:///z.rs").unwrap(),
        version: None,
        diagnostics: vec![Diagnostic {
            range: Range {
                start: Position {
                    line: 7,
                    character: 1,
                },
                end: Position {
                    line: 7,
                    character: 4,
                },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            message: "w".to_owned(),
            ..Default::default()
        }],
    };
    let mapped = published_diagnostics_from_lsp(params);
    assert_eq!(mapped.diagnostics[0].line, 7);
    assert_eq!(mapped.diagnostics[0].severity, 2);
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-008 REMEDIATION: server DISCOVERY (typed absent-state, un-gated) + the GATED
// real-process spawn/initialize/Drop-no-zombie proof against a canned stdio LSP subprocess.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use handshake_native::code_editor::lsp_client::{discover_lsp_server_in, LspServerDiscovery};

/// MT-008 REMEDIATION (un-gated): the HONEST typed absent-state. A host with no server on PATH, a
/// missing PATH variable, and a language with no known canonical server all resolve to the typed
/// [`LspServerDiscovery::Absent`] carrying WHAT was probed — never a fake `Found` and never a panic.
/// This is the exact state the live shell surfaces (`LspAttachState::Absent`) when rust-analyzer is
/// absent on the host.
#[test]
fn lsp_discovery_typed_absent_state() {
    // Empty PATH: the canonical rust server is probed but not found.
    let got = discover_lsp_server_in("rust", Some(std::ffi::OsString::new()));
    assert_eq!(
        got,
        LspServerDiscovery::Absent {
            language_id: "rust".to_owned(),
            probed_command: "rust-analyzer".to_owned(),
        },
        "empty PATH -> typed Absent naming the probed canonical command"
    );
    assert!(!got.is_found());

    // No PATH variable at all: same typed absent-state (no panic, no fabricated config).
    let got = discover_lsp_server_in("rust", None);
    assert_eq!(
        got,
        LspServerDiscovery::Absent {
            language_id: "rust".to_owned(),
            probed_command: "rust-analyzer".to_owned(),
        },
    );

    // A language this build knows no canonical server for: Absent with an EMPTY probed_command (the
    // honest "nothing was even probed" disclosure).
    let got = discover_lsp_server_in("cobol", Some(std::env::var_os("PATH").unwrap_or_default()));
    assert_eq!(
        got,
        LspServerDiscovery::Absent {
            language_id: "cobol".to_owned(),
            probed_command: String::new(),
        },
    );
}

/// MT-008 REMEDIATION (un-gated): the Found branch, proven deterministically against a temp dir
/// placed on the probe PATH containing a `rust-analyzer` executable file. The discovery must resolve
/// the ABSOLUTE launch path (so a later PATH change cannot redirect the lazy first spawn).
#[test]
fn lsp_discovery_finds_server_in_path_dir() {
    let dir = std::env::temp_dir().join(format!("hs-lsp-discovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create probe dir");
    let exe_name = if cfg!(windows) {
        "rust-analyzer.exe"
    } else {
        "rust-analyzer"
    };
    let exe = dir.join(exe_name);
    std::fs::write(&exe, b"stub-not-executed").expect("write probe stub");

    let got = discover_lsp_server_in("rust", Some(dir.clone().into_os_string()));
    match got {
        LspServerDiscovery::Found(config) => {
            let resolved = std::path::Path::new(&config.command);
            assert!(
                resolved.is_absolute(),
                "discovery resolves the ABSOLUTE executable path, got {resolved:?}"
            );
            assert!(
                config.command.ends_with(exe_name),
                "resolved command names the probed executable: {}",
                config.command
            );
            assert!(config.args.is_empty(), "no default args for rust-analyzer");
        }
        other => panic!("expected Found for an on-PATH executable, got {other:?}"),
    }

    let _ = std::fs::remove_file(&exe);
    let _ = std::fs::remove_dir(&dir);
}

/// Whether an OS process with `pid` currently exists (the no-zombie probe for the gated test).
fn process_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&format!("\"{pid}\"")))
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// Write one `Content-Length`-framed LSP message to REAL stdout (the canned server's transport half).
/// A LEADING CRLF terminates any partial line libtest left on stdout (it prints `test <name> ... `
/// WITHOUT a newline before the test runs, which would otherwise glue onto the `Content-Length:`
/// header and defeat the client's header parse); the client's resilient reader (RISK-003) skips the
/// resulting non-header fragment line and resyncs on the real header.
fn canned_server_write_frame(msg: &serde_json::Value) {
    use std::io::Write;
    let body = serde_json::to_vec(msg).expect("serialize frame");
    let mut out = std::io::stdout().lock();
    let _ = write!(out, "\r\nContent-Length: {}\r\n\r\n", body.len());
    let _ = out.write_all(&body);
    let _ = out.flush();
}

/// The CANNED stdio LSP server the gated real-process test spawns. In a normal test run (the env var
/// is unset) this is an immediate no-op. When THIS test binary is re-executed with
/// `HANDSHAKE_CANNED_LSP_SERVER=1` (the gated test spawns `current_exe()` filtered to exactly this
/// test with `--nocapture`, so stdout is the REAL pipe), it becomes a real OS subprocess speaking
/// canned LSP frames over real stdin/stdout: it answers `initialize` (and `shutdown`), exits on the
/// `exit` notification or stdin EOF, and writes its OS pid to `HANDSHAKE_CANNED_LSP_PIDFILE` so the
/// parent can prove process liveness + reaping. libtest's own progress lines on stdout are tolerated
/// by the client's resilient reader (RISK-003 skips non-frame lines).
#[test]
fn canned_lsp_server_main() {
    if std::env::var("HANDSHAKE_CANNED_LSP_SERVER").as_deref() != Ok("1") {
        return; // Parent run: no-op. Only the re-exec'd child (env set) runs the server loop.
    }
    if let Ok(pidfile) = std::env::var("HANDSHAKE_CANNED_LSP_PIDFILE") {
        let _ = std::fs::write(&pidfile, std::process::id().to_string());
    }
    use std::io::Read;
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    loop {
        // ── Read one Content-Length header block (byte-wise; ends at the blank line). ──
        let mut content_length: Option<usize> = None;
        let mut line: Vec<u8> = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            if stdin.read_exact(&mut byte).is_err() {
                return; // stdin EOF/closed: the client is gone — exit (never a zombie loop).
            }
            if byte[0] == b'\n' {
                let text = String::from_utf8_lossy(&line);
                let trimmed = text.trim_end_matches('\r');
                if trimmed.is_empty() {
                    line.clear();
                    break; // end of headers
                }
                if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                    content_length = rest.trim().parse().ok();
                }
                line.clear();
            } else {
                line.push(byte[0]);
            }
        }
        let Some(len) = content_length else { continue };
        let mut body = vec![0u8; len];
        if stdin.read_exact(&mut body).is_err() {
            return;
        }
        let Ok(msg) = serde_json::from_slice::<serde_json::Value>(&body) else {
            continue;
        };
        let method = msg
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or_default()
            .to_owned();
        let id = msg.get("id").cloned();
        match (method.as_str(), id) {
            ("initialize", Some(id)) => canned_server_write_frame(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "capabilities": {},
                    "serverInfo": { "name": "handshake-canned-lsp" }
                }
            })),
            ("shutdown", Some(id)) => canned_server_write_frame(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": null
            })),
            ("exit", None) => std::process::exit(0),
            // Notifications (initialized / didOpen / didChange) need no response.
            _ => {}
        }
    }
}

/// MT-008 REMEDIATION (GATED — set `HANDSHAKE_LSP_PROCESS_PROOF=1`): the REAL-PROCESS proof. Spawns a
/// genuine OS subprocess (this test binary re-executed as [`canned_lsp_server_main`], a canned stdio
/// LSP server), drives the PRODUCTION `LspClient::initialize` spawn + handshake over real pipes, then
/// drops the client and proves the child process is REAPED (RISK-001 / MC-001 — no zombie). Skips
/// with a disclosed reason when un-gated so the default run stays deterministic and process-free.
#[test]
fn gated_real_process_lsp_spawn_initialize_drop_no_zombie() {
    if std::env::var("HANDSHAKE_LSP_PROCESS_PROOF").as_deref() != Ok("1") {
        eprintln!(
            "SKIP (gated): set HANDSHAKE_LSP_PROCESS_PROOF=1 to run the real-process LSP \
             spawn/initialize/Drop proof"
        );
        return;
    }
    let exe = std::env::current_exe().expect("current test binary path");
    let pidfile = std::env::temp_dir().join(format!(
        "handshake-canned-lsp-pid-{}.txt",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&pidfile);
    // The child inherits this process's env: activate the canned-server loop + name the pidfile.
    std::env::set_var("HANDSHAKE_CANNED_LSP_SERVER", "1");
    std::env::set_var("HANDSHAKE_CANNED_LSP_PIDFILE", &pidfile);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let client = LspClient::new(LspServerConfig {
        command: exe.to_string_lossy().into_owned(),
        args: vec![
            "canned_lsp_server_main".to_owned(),
            "--exact".to_owned(),
            "--nocapture".to_owned(),
            "--test-threads=1".to_owned(),
        ],
    });
    let initialized = rt.block_on(async { client.initialize(None).await });
    // The activation env vars are scoped to the spawn; clear them immediately.
    std::env::remove_var("HANDSHAKE_CANNED_LSP_SERVER");
    std::env::remove_var("HANDSHAKE_CANNED_LSP_PIDFILE");
    assert!(
        initialized,
        "real-process proof: LspClient spawned the canned stdio server and completed the \
         initialize handshake over real pipes"
    );
    assert!(
        client.is_running(),
        "real-process proof: transport live after initialize"
    );

    // The canned server wrote its OS pid on startup; poll it (bounded).
    let mut pid: Option<u32> = None;
    for _ in 0..50 {
        if let Ok(text) = std::fs::read_to_string(&pidfile) {
            if let Ok(parsed) = text.trim().parse::<u32>() {
                pid = Some(parsed);
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let pid = pid.expect("canned server wrote its pid file");
    assert!(
        process_alive(pid),
        "real-process proof: the spawned canned server (pid {pid}) is ALIVE while the client \
         holds the transport"
    );

    // Drop the client: the Drop path sends shutdown/exit (bounded) and kills the child (RISK-001).
    // The runtime must outlive the drop (shutdown_now drives async work on the stored handle).
    drop(client);
    let mut reaped = false;
    for _ in 0..50 {
        if !process_alive(pid) {
            reaped = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let _ = std::fs::remove_file(&pidfile);
    assert!(
        reaped,
        "RISK-001 / MC-001: the spawned canned LSP server (pid {pid}) must be REAPED on client \
         Drop — no zombie process"
    );
    drop(rt);
    println!(
        "MT-008 real-process proof: spawn + initialize + Drop-reap complete (canned server pid {pid})"
    );
}
