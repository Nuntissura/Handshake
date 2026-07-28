//! WP-KERNEL-012 MT-045 — E8 Large-Document Performance Proof, rich-editor scenarios (LR-01..LR-07).
//!
//! ## Managed PostgreSQL runtime
//!
//! LR-01..LR-04, LR-06, LR-07 and the LIVE 50-hop half of LR-05 BIND the handshake_core backend
//! (knowledge documents create/load/save/projection + the loom transclusion read-through). The shared
//! fixture attaches to a healthy managed product backend or starts an already-built product executable;
//! every test creates and deletes its own workspace and data through production HTTP. No scenario is
//! ignored and no operator-preseeded identifiers are accepted.
//!
//! ## EXCEPTION — LR-05 cycle-detection logic runs NOW (contract REALITY note, RISK-4 / CTRL-4)
//!
//! The TRANSCLUSION ENDPOINT is PG-gated, but the cycle-detection RESOLVER LOGIC (a recursive walk that
//! tracks visited block ids in a `HashSet<String>` and returns `Err("cycle_detected")` when a block id
//! repeats) is the NATIVE contribution the React reference lacks — and it is frontend-testable NOW,
//! independent of PG, because it is a pure algorithm over a "fetch one hop" function. Per CTRL-4 it is
//! proven as two logic checks inside the single LR-05 catalog test:
//!   - a LINEAR chain of 50 resolves correctly
//!     (returns the full path, no false cycle).
//!   - a CYCLIC chain of 5 returns
//!     `cycle_detected` WITHOUT panicking or
//!     looping forever, AND a cycle reported by the resolver is specifically a repeated id (not an error
//!     for ANY transclusion — RISK-4 guard).
//!
//! The LIVE 50-hop chain self-seeds native RichDocuments and traverses the real
//! `GET /loom/blocks/{id}/transclusion` endpoint.
//!
//! ## No mock smuggling (RISK-2 / CTRL-2)
//!
//! The LR-05 resolver under test is a real algorithm; its `fetch_hop` is in-memory for the two logic
//! tests (a deterministic chain/cycle map, NOT a backend mock — there is no PG route being faked), and
//! is the live transclusion route for the gated 50-hop proof. No sqlite, no in-memory backend stub.

mod perf_proof_support;
mod pg_proof_support;

use perf_proof_support::{measurement, Budget, ScenarioAttempt};
use pg_proof_support::LiveBackend;

use handshake_native::rich_editor::document_model::doc_json;
use handshake_native::rich_editor::find_replace::scanner::{self, FindQuery};
use handshake_native::rich_editor::renderer::block_author_id;
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};

// ── The native cycle-aware transclusion-chain resolver (LR-05) ────────────────────────────────────
//
// WAVE-2 REMEDIATION: the resolver used to be DEFINED in this test file (a test-only fork with no
// production caller). It now lives in PRODUCT code —
// `rich_editor::wikilinks::transclusion_resolver` — where the transclusion render path
// (`WikilinkRuntime::detect_transclusion_cycle` + `transclusion_view::render_transclusion`) guards
// cyclic chains with it. These LR-05 tests import the PRODUCT symbol, so the perf proof and the
// product guard are one algorithm.
use handshake_native::rich_editor::wikilinks::transclusion_resolver::{
    resolve_transclusion_chain, TransclusionResolveError,
};

// ── LR-05 (logic, runs NOW): a LINEAR chain of 50 resolves correctly ──────────────────────────────

fn run_lr05_logic_linear() {
    let budget = Budget::resolve("LR-05", "PERF_BUDGET_LR05_MS", 5_000);

    // A deterministic in-memory LINEAR chain of 50: block-0 -> block-1 -> ... -> block-49 -> (end).
    let chain: std::collections::HashMap<String, String> = (0..49)
        .map(|i| (format!("block-{i}"), format!("block-{}", i + 1)))
        .collect();

    let (result, elapsed_ms) = perf_proof_support::time_ms(|| {
        resolve_transclusion_chain("block-0", 100, |id| chain.get(id).cloned())
    });

    let order = result.expect("LR-05: a linear 50-chain must resolve, not report a false cycle");
    assert_eq!(
        order.len(),
        50,
        "LR-05: the linear chain must visit all 50 blocks in order (got {})",
        order.len()
    );
    assert_eq!(
        order.first().map(String::as_str),
        Some("block-0"),
        "LR-05: chain starts at block-0"
    );
    assert_eq!(
        order.last().map(String::as_str),
        Some("block-49"),
        "LR-05: chain ends at block-49"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LR-05: linear 50-hop resolve {elapsed_ms} ms must be <= {} ms (override {})",
        budget.ceiling,
        budget.env_var
    );

    // This logic test proves the resolver independently; the default live test below proves the same
    // algorithm against a self-seeded PostgreSQL chain and records the authoritative measurement.
    println!(
        "LR-05 (linear logic) measured={elapsed_ms}ms (<= {}ms) PASS — native resolver walks a 50-hop \
         linear chain without a false cycle",
        budget.ceiling
    );
}

// ── LR-05 (logic, runs NOW): a CYCLIC chain of 5 returns cycle_detected, no panic / infinite loop ─

fn run_lr05_logic_cycle_detected() {
    let budget = Budget::resolve("LR-05", "PERF_BUDGET_LR05_MS", 5_000);
    // A 5-block CYCLE: block-0 -> block-1 -> block-2 -> block-3 -> block-4 -> block-0 (back to start).
    let cycle: std::collections::HashMap<String, String> = (0..5)
        .map(|i| (format!("block-{i}"), format!("block-{}", (i + 1) % 5)))
        .collect();

    // The resolver MUST return Err(CycleDetected), NOT panic and NOT loop forever. A 100-depth bound is
    // far above the 5-cycle, so a DepthExceeded here would be a BUG (it must catch the cycle first).
    let started = std::time::Instant::now();
    let result = resolve_transclusion_chain("block-0", 100, |id| cycle.get(id).cloned());
    let elapsed_ms = started.elapsed().as_millis();
    match result {
        Err(TransclusionResolveError::CycleDetected { at }) => {
            // RISK-4 guard: the cycle is flagged at the FIRST repeated id (block-0, the start we loop
            // back to), proving it detected a CYCLE specifically — not an error for any transclusion.
            assert_eq!(
                at, "block-0",
                "LR-05: the cycle must be flagged at the repeated id block-0"
            );
            println!(
                "LR-05 (cycle logic) PASS — cyclic-5 returns cycle_detected at block {at} (no panic, no \
                 infinite loop)"
            );
        }
        Err(other) => panic!("LR-05: a 5-cycle must be CycleDetected, not {other:?}"),
        Ok(order) => panic!(
            "LR-05: a 5-cycle must NOT resolve as a clean chain (got order of {} ids)",
            order.len()
        ),
    }

    // RISK-4 guard #2: a NON-cyclic chain through the SAME resolver does NOT report a cycle — so the
    // resolver is not just returning an error for any transclusion. A short linear chain resolves clean.
    let linear: std::collections::HashMap<String, String> = [
        ("a".to_string(), "b".to_string()),
        ("b".to_string(), "c".to_string()),
    ]
    .into_iter()
    .collect();
    let ok = resolve_transclusion_chain("a", 100, |id| linear.get(id).cloned())
        .expect("LR-05: a clean linear chain must NOT be reported as a cycle");
    assert_eq!(
        ok,
        vec!["a", "b", "c"],
        "LR-05: the clean chain resolves a->b->c (no false cycle)"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LR-05: cyclic-5 detection {elapsed_ms} ms must be <= {} ms (override {})",
        budget.ceiling,
        budget.env_var
    );
}

// ── LR-01: load a 1000-block rich document — round-trip <= 2 s, native parse <= 100 ms (REQUIRES_PG) ─

#[test]
fn perf_proof_perf_lr01_load_large_doc() {
    let budget = Budget::resolve("LR-01", "PERF_BUDGET_LR01_MS", 2_000);
    let parse_budget = Budget::resolve("LR-01", "PERF_BUDGET_LR01_PARSE_MS", 100);
    let Some(attempt) = ScenarioAttempt::begin_or_skip(
        "LR-01",
        "primary",
        &[
            ("backend_round_trip", &budget, "ms"),
            ("native_parse", &parse_budget, "ms"),
        ],
    ) else {
        return;
    };
    let mut be = require_be();

    // FIXTURE (NOT timed): build a 1000-paragraph-block content doc and POST it.
    let content = big_paragraph_doc(1000, "para");
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": be.workspace_id, "title": "mt045-lr01", "content_json": content }),
    );
    let doc_id = created_doc_id(&created);
    let _guard = DocGuard {
        be: &be,
        doc_id: doc_id.clone(),
    };

    // MEASURED (round-trip): GET the 1000-block doc back through real PG.
    let (loaded, rt_ms) =
        perf_proof_support::time_ms(|| be.get_json(&format!("/knowledge/documents/{doc_id}")));
    attempt.stage(
        serde_json::json!([measurement("backend_round_trip", rt_ms as f64, "ms")]),
        serde_json::json!({"phase": "backend_round_trip", "response_received": true}),
    );
    // MEASURED (native parse): build the native block tree from the loaded JSON.
    let content_json = loaded
        .get("document")
        .and_then(|d| d.get("content_json"))
        .cloned()
        .unwrap_or(loaded.clone());
    let (native_doc, parse_ms) = perf_proof_support::time_ms(|| {
        doc_json::from_json_value(&content_json)
            .expect("LR-01: persisted content_json must parse through the native DocModel")
    });
    let block_count = native_doc.children.len();
    attempt.stage(
        serde_json::json!([
            measurement("backend_round_trip", rt_ms as f64, "ms"),
            measurement("native_parse", parse_ms as f64, "ms")
        ]),
        serde_json::json!({"block_count": block_count}),
    );

    assert!(
        block_count >= 1000,
        "LR-01: the reloaded doc must carry >= 1000 blocks (got {block_count})"
    );
    assert!(
        budget.passes(rt_ms),
        "LR-01: load round-trip {rt_ms} ms must be <= {} ms",
        budget.ceiling
    );
    assert!(
        parse_budget.passes(parse_ms),
        "LR-01: native parse {parse_ms} ms must be <= {} ms",
        parse_budget.ceiling
    );

    println!("LR-01 measured={rt_ms}ms round-trip (<= {}ms), parse {parse_ms}ms (<= {}ms) PASS — {block_count} blocks (live PG)", budget.ceiling, parse_budget.ceiling);
    drop(_guard);
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([
            measurement("backend_round_trip", rt_ms as f64, "ms"),
            measurement("native_parse", parse_ms as f64, "ms")
        ]),
        serde_json::json!({"block_count": block_count}),
    );
}

// ── LR-02: scroll through a 1000-block doc — 100 viewport steps <= 1000 ms (REQUIRES_PG) ──────────

#[test]
fn perf_proof_perf_lr02_scroll_large_doc() {
    let budget = Budget::resolve("LR-02", "PERF_BUDGET_LR02_MS", 1_000);
    let Some(attempt) =
        ScenarioAttempt::begin_or_skip("LR-02", "primary", &[("scroll_100_steps", &budget, "ms")])
    else {
        return;
    };
    let mut be = require_be();

    let content = big_paragraph_doc(1000, "scroll");
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": be.workspace_id, "title": "mt045-lr02", "content_json": content }),
    );
    let doc_id = created_doc_id(&created);
    let _guard = DocGuard {
        be: &be,
        doc_id: doc_id.clone(),
    };
    let loaded = be.get_json(&format!("/knowledge/documents/{doc_id}"));
    let content_json = loaded
        .get("document")
        .and_then(|d| d.get("content_json"))
        .cloned()
        .unwrap_or(loaded.clone());
    let native_doc = doc_json::from_json_value(&content_json)
        .expect("LR-02: persisted content_json must parse through the native DocModel");
    let blocks = native_doc.children.len();
    assert!(
        blocks >= 1000,
        "LR-02: 1000 blocks loaded for the scroll (got {})",
        blocks
    );

    let state = std::sync::Arc::new(std::sync::Mutex::new(RichEditorState::new(native_doc)));
    let frame_heartbeat = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let state_for_ui = std::sync::Arc::clone(&state);
    let heartbeat_for_ui = std::sync::Arc::clone(&frame_heartbeat);
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(900.0, 640.0))
        .build_ui(move |ui| {
            heartbeat_for_ui.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(std::sync::Arc::clone(&state_for_ui)).show(ui);
        });
    harness.step();
    let heartbeat_before = frame_heartbeat.load(std::sync::atomic::Ordering::Relaxed);

    // MEASURED: drive the shipped rich-editor renderer and its existing ScrollArea through 100 stable
    // block-addressed viewport requests from block 0 to block 999. Each `harness.step()` is a real egui
    // layout/paint frame; the heartbeat proves every requested frame returned rather than freezing.
    let mut all_scroll_targets_consumed = true;
    let (_, elapsed_ms) = perf_proof_support::time_ms(|| {
        for step in 0..100usize {
            let block_index = step * (blocks - 1) / 99;
            let mut widget = RichEditorWidget::new(std::sync::Arc::clone(&state));
            widget.scroll_to_block(&block_author_id(&[block_index]));
            harness.step();
            all_scroll_targets_consumed &= state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .pending_scroll_block
                .is_none();
        }
    });
    let heartbeat_after = frame_heartbeat.load(std::sync::atomic::Ordering::Relaxed);
    attempt.stage(
        serde_json::json!([measurement("scroll_100_steps", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "blocks": blocks,
            "frames": heartbeat_after - heartbeat_before,
            "all_scroll_targets_consumed": all_scroll_targets_consumed,
        }),
    );
    assert!(
        all_scroll_targets_consumed,
        "LR-02: the native renderer must consume every block-addressed scroll target"
    );
    assert!(
        heartbeat_after >= heartbeat_before + 100,
        "LR-02: the native render heartbeat must advance for all 100 scroll frames ({heartbeat_before}->{heartbeat_after})"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LR-02: 100 scroll steps {elapsed_ms} ms must be <= {} ms",
        budget.ceiling
    );

    println!("LR-02 measured={elapsed_ms}ms (<= {}ms) PASS — 100 viewport steps over 1000 blocks, no layout panic (live PG)", budget.ceiling);
    drop(_guard);
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([measurement("scroll_100_steps", elapsed_ms as f64, "ms")]),
        serde_json::json!({"blocks": blocks, "frames": heartbeat_after - heartbeat_before}),
    );
}

// ── LR-03: find in a rich doc — 500 matches <= 200 ms (REQUIRES_PG) ───────────────────────────────

#[test]
fn perf_proof_perf_lr03_find_in_doc() {
    let budget = Budget::resolve("LR-03", "PERF_BUDGET_LR03_MS", 200);
    let Some(attempt) =
        ScenarioAttempt::begin_or_skip("LR-03", "primary", &[("find", &budget, "ms")])
    else {
        return;
    };
    let mut be = require_be();

    // FIXTURE: 1000 blocks; "FINDME" in every other block (500 occurrences).
    let blocks: Vec<serde_json::Value> = (0..1000)
        .map(|i| {
            let text = if i % 2 == 0 { "FINDME here".to_string() } else { format!("plain {i}") };
            serde_json::json!({ "type": "paragraph", "content": [ { "type": "text", "text": text } ] })
        })
        .collect();
    let content = serde_json::json!({ "type": "doc", "content": blocks });
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": be.workspace_id, "title": "mt045-lr03", "content_json": content }),
    );
    let doc_id = created_doc_id(&created);
    let _guard = DocGuard {
        be: &be,
        doc_id: doc_id.clone(),
    };
    let loaded = be.get_json(&format!("/knowledge/documents/{doc_id}"));
    let content_json = loaded
        .get("document")
        .and_then(|d| d.get("content_json"))
        .cloned()
        .unwrap_or(loaded.clone());

    // MEASURED: collect all 500 "FINDME" spans from the loaded doc text.
    let native_doc = doc_json::from_json_value(&content_json)
        .expect("LR-03: persisted content_json must parse through the native DocModel");
    let (scan, elapsed_ms) =
        perf_proof_support::time_ms(|| scanner::scan(&native_doc, &FindQuery::literal("FINDME")));
    attempt.stage(
        serde_json::json!([measurement("find", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "matches": scan.matches.len(),
            "scan_error": format!("{:?}", scan.error),
            "truncated": scan.truncated,
        }),
    );
    assert!(
        scan.error.is_none(),
        "LR-03: literal native scan cannot fail"
    );
    assert!(
        !scan.truncated,
        "LR-03: 500 native matches fit below MAX_MATCHES"
    );
    let count = scan.matches.len();
    assert_eq!(
        count, 500,
        "LR-03: must collect all 500 FINDME spans (got {count})"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LR-03: find {elapsed_ms} ms must be <= {} ms",
        budget.ceiling
    );

    println!("LR-03 measured={elapsed_ms}ms (<= {}ms) PASS — 500 FINDME matches in a 1000-block doc (live PG)", budget.ceiling);
    drop(_guard);
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([measurement("find", elapsed_ms as f64, "ms")]),
        serde_json::json!({"matches": count}),
    );
}

// ── LR-04: save a 1000-block doc — round-trip <= 3 s, version advances (REQUIRES_PG) ──────────────

#[test]
fn perf_proof_perf_lr04_save_large_doc() {
    let budget = Budget::resolve("LR-04", "PERF_BUDGET_LR04_MS", 3_000);
    let Some(attempt) =
        ScenarioAttempt::begin_or_skip("LR-04", "primary", &[("save_round_trip", &budget, "ms")])
    else {
        return;
    };
    let mut be = require_be();

    let content = big_paragraph_doc(1000, "save");
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": be.workspace_id, "title": "mt045-lr04", "content_json": content }),
    );
    let doc_id = created_doc_id(&created);
    let _guard = DocGuard {
        be: &be,
        doc_id: doc_id.clone(),
    };
    let loaded = be.get_json(&format!("/knowledge/documents/{doc_id}"));
    // KnowledgeRichDocument serializes its version field as `doc_version` (i64), wrapped under
    // `document` on both create+load and save responses (storage/knowledge.rs:1816;
    // api/knowledge_documents.rs:730,1077). Reading top-level/`version` would silently default to 1.
    let base_version = loaded
        .get("document")
        .and_then(|d| d.get("doc_version"))
        .and_then(|v| v.as_i64())
        .unwrap_or(1);

    // MEASURED: save a mutated version through the real save route. The response receipt is then read
    // back from EventLedger authority below; a version increment alone is not receipt proof.
    let mutated = big_paragraph_doc(1000, "save-v2");
    let (resp, elapsed_ms) = perf_proof_support::time_ms(|| {
        be.put_json(
            &format!("/knowledge/documents/{doc_id}/save"),
            &serde_json::json!({ "expected_version": base_version, "content_json": mutated }),
        )
    });
    attempt.stage(
        serde_json::json!([measurement("save_round_trip", elapsed_ms as f64, "ms")]),
        serde_json::json!({"base_version": base_version, "response_received": true}),
    );
    let new_version = resp
        .get("document")
        .and_then(|d| d.get("doc_version"))
        .and_then(|v| v.as_i64())
        .unwrap_or(base_version);
    attempt.stage(
        serde_json::json!([measurement("save_round_trip", elapsed_ms as f64, "ms")]),
        serde_json::json!({"base_version": base_version, "new_version": new_version}),
    );
    assert!(
        new_version > base_version,
        "LR-04: save must advance the version ({base_version} -> {new_version})"
    );
    assert!(
        resp.get("receipt_error")
            .is_none_or(serde_json::Value::is_null),
        "LR-04: a committed perf-proof save must not report an EventLedger receipt failure: {resp}"
    );
    let receipt_id = resp
        .get("save_receipt_event_id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty())
        .expect("LR-04: save must return a nonblank EventLedger receipt id");
    let events = be.get_json(&format!(
        "/kernel/events/aggregates/knowledge_rich_document/{doc_id}"
    ));
    let receipt = events
        .as_array()
        .and_then(|rows| {
            rows.iter().find(|event| {
                event.get("event_id").and_then(serde_json::Value::as_str) == Some(receipt_id)
            })
        })
        .expect("LR-04: the exact response receipt must be readable from EventLedger authority");
    assert_eq!(
        receipt
            .get("aggregate_id")
            .and_then(serde_json::Value::as_str),
        Some(doc_id.as_str()),
        "LR-04: receipt aggregate is the saved rich document"
    );
    assert_eq!(
        receipt
            .get("event_type")
            .and_then(serde_json::Value::as_str),
        Some("KNOWLEDGE_RICH_DOCUMENT_SAVED"),
        "LR-04: receipt is the typed rich-document save event"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LR-04: save round-trip {elapsed_ms} ms must be <= {} ms",
        budget.ceiling
    );

    println!("LR-04 measured={elapsed_ms}ms (<= {}ms) PASS — 1000-block save, version {base_version}->{new_version} (live PG)", budget.ceiling);
    drop(_guard);
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([measurement("save_round_trip", elapsed_ms as f64, "ms")]),
        serde_json::json!({"base_version": base_version, "new_version": new_version, "event_id": receipt_id}),
    );
}

// ── LR-05 (LIVE 50-hop): resolve a 50-deep chain over the real transclusion endpoint (REQUIRES_PG) ─

#[test]
fn perf_proof_perf_lr05_transclusion_chain_live() {
    let budget = Budget::resolve("LR-05", "PERF_BUDGET_LR05_MS", 5_000);
    // Both contract proof attempts must invalidate prior results before any fallible logic assertion,
    // backend check, or fixture setup. LR-05 therefore retains exactly these two proof ids.
    let linear_attempt = ScenarioAttempt::begin(
        "LR-05",
        "linear-50",
        &[("transclusion_resolve", &budget, "ms")],
    );
    let cyclic_attempt =
        ScenarioAttempt::begin("LR-05", "cyclic-5", &[("cycle_detection", &budget, "ms")]);
    if perf_proof_support::skip_all() {
        linear_attempt.skipped("SKIP_PERF_TESTS=1");
        cyclic_attempt.skipped("SKIP_PERF_TESTS=1");
        return;
    }

    let mut be = require_be();

    // FIXTURE (NOT timed): create the tail first, then 49 native RichDocuments whose authority content
    // carries one loomTransclusion atom to the previously-created same-ID Loom projection.
    let setup_deadline = pg_proof_support::SetupDeadline::begin("LR-05-linear-50");
    let mut next: Option<String> = None;
    for index in (0..50).rev() {
        setup_deadline.check();
        let content_json = match next.as_deref() {
            Some(next_block_id) => serde_json::json!({
                "type": "doc",
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "loomTransclusion",
                        "attrs": { "refValue": next_block_id }
                    }]
                }]
            }),
            None => serde_json::json!({
                "type": "doc",
                "content": [{
                    "type": "paragraph",
                    "content": [{ "type": "text", "text": "transclusion-chain-tail" }]
                }]
            }),
        };
        let created = be.post_json(
            "/knowledge/documents",
            &serde_json::json!({
                "workspace_id": be.workspace_id,
                "title": format!("mt045-lr05-{index:02}"),
                "content_json": content_json,
            }),
        );
        next = Some(created_block_id(&created));
    }
    setup_deadline.check();
    let head = next.expect("LR-05 fixture creates a 50-document chain head");

    let mut every_projection_resolved = true;
    let (result, elapsed_ms) = perf_proof_support::time_ms(|| {
        resolve_transclusion_chain(&head, 60, |block_id| {
            let resp = be.get_json(&format!(
                "/workspaces/{}/loom/blocks/{}/transclusion",
                be.workspace_id, block_id
            ));
            every_projection_resolved &= resp["resolved"] == true;
            first_transclusion_ref(&resp["content_json"])
        })
    });
    linear_attempt.stage(
        serde_json::json!([measurement("transclusion_resolve", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "shape": "linear",
            "expected_hops": 50,
            "every_projection_resolved": every_projection_resolved,
        }),
    );
    assert!(
        every_projection_resolved,
        "LR-05: every native projection in the linear-50 chain resolves"
    );
    let order = result.expect("LR-05 live: the seeded 50-hop chain must resolve without a cycle");
    assert_eq!(
        order.len(),
        50,
        "LR-05 live: the chain must resolve exactly 50 hops"
    );
    assert!(
        budget.passes(elapsed_ms),
        "LR-05 live: 50-hop resolve {elapsed_ms} ms must be <= {} ms",
        budget.ceiling
    );

    println!("LR-05 measured={elapsed_ms}ms (<= {}ms) PASS — live 50-hop transclusion chain, cycle-safe (live PG)", budget.ceiling);
    be.assert_cleanup();
    // `LiveBackend` owns the cross-test fixture lock. Release the cleaned linear fixture before the
    // cyclic proof acquires its own workspace; retaining `be` here self-deadlocks for 60 seconds.
    drop(be);
    // The declared LR-05 proof target must prove BOTH persisted shapes in one scenario invocation.
    // Keep the cyclic path as a helper so `-- perf_proof` still reports exactly one test per catalog
    // scenario (20 total), while a failure in the live cycle path fails this LR-05 test before PASS.
    run_lr05_transclusion_chain_cycle_live(cyclic_attempt, &budget);
    // LR-05 remains one catalog scenario. Run its fast algorithmic guards only after the persisted
    // contract workloads have staged their measurements, so an assertion failure cannot erase metrics.
    run_lr05_logic_linear();
    run_lr05_logic_cycle_detected();
    linear_attempt.pass(
        serde_json::json!([measurement("transclusion_resolve", elapsed_ms as f64, "ms")]),
        serde_json::json!({"shape": "linear", "hops": order.len()}),
    );
}

// ── LR-05 (LIVE cyclic-5): persisted transclusions report a typed cycle, never spin ───────────────

fn run_lr05_transclusion_chain_cycle_live(attempt: ScenarioAttempt, budget: &Budget) {
    let mut be = require_be();

    // Create five persisted document projections first so every target block id exists, then save a
    // ring A->B->C->D->E->A through the public rich-document save route.
    let setup_deadline = pg_proof_support::SetupDeadline::begin("LR-05-cyclic-5");
    let mut docs: Vec<(String, String, i64)> = Vec::with_capacity(5);
    for index in 0..5usize {
        setup_deadline.check();
        let created = be.post_json(
            "/knowledge/documents",
            &serde_json::json!({
                "workspace_id": be.workspace_id,
                "title": format!("mt045-lr05-cycle-{index}"),
                "content_json": big_paragraph_doc(1, "cycle-seed"),
            }),
        );
        let doc_id = created_doc_id(&created);
        let block_id = created_block_id(&created);
        let version = created
            .pointer("/document/doc_version")
            .and_then(serde_json::Value::as_i64)
            .expect("LR-05 cycle fixture: create returns doc_version");
        docs.push((doc_id, block_id, version));
    }
    for index in 0..docs.len() {
        setup_deadline.check();
        let next_block_id = &docs[(index + 1) % docs.len()].1;
        let saved = be.put_json(
            &format!("/knowledge/documents/{}/save", docs[index].0),
            &serde_json::json!({
                "expected_version": docs[index].2,
                "content_json": {
                    "type": "doc",
                    "content": [{
                        "type": "paragraph",
                        "content": [{
                            "type": "loomTransclusion",
                            "attrs": { "refValue": next_block_id }
                        }]
                    }]
                }
            }),
        );
        assert!(
            saved
                .get("save_receipt_event_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| !id.is_empty()),
            "LR-05 cycle fixture: every persisted ring edge has an EventLedger receipt"
        );
    }
    setup_deadline.check();

    let head = docs[0].1.clone();
    let mut every_projection_resolved = true;
    let (result, elapsed_ms) = perf_proof_support::time_ms(|| {
        resolve_transclusion_chain(&head, 20, |block_id| {
            let response = be.get_json(&format!(
                "/workspaces/{}/loom/blocks/{block_id}/transclusion",
                be.workspace_id
            ));
            every_projection_resolved &= response
                .get("resolved")
                .and_then(serde_json::Value::as_bool)
                == Some(true);
            first_transclusion_ref(&response["content_json"])
        })
    });
    attempt.stage(
        serde_json::json!([measurement("cycle_detection", elapsed_ms as f64, "ms")]),
        serde_json::json!({
            "shape": "cycle",
            "nodes": 5,
            "every_projection_resolved": every_projection_resolved,
            "repeated_block_id": head,
        }),
    );
    assert!(
        every_projection_resolved,
        "LR-05 cycle fixture: every persisted projection hop resolves"
    );
    match result {
        Err(TransclusionResolveError::CycleDetected { at }) => assert_eq!(
            at, head,
            "LR-05: persisted cyclic-5 must detect the first repeated block"
        ),
        Err(other) => panic!("LR-05: persisted cyclic-5 must be CycleDetected, not {other:?}"),
        Ok(order) => panic!(
            "LR-05: persisted cyclic-5 must not resolve as a clean chain (visited {})",
            order.len()
        ),
    }
    assert!(
        budget.passes(elapsed_ms),
        "LR-05: persisted cyclic-5 resolve {elapsed_ms} ms must be <= {} ms",
        budget.ceiling
    );
    println!(
        "LR-05 cycle measured={elapsed_ms}ms (<= {}ms) PASS — persisted cyclic-5 returns typed cycle_detected",
        budget.ceiling
    );
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([measurement("cycle_detection", elapsed_ms as f64, "ms")]),
        serde_json::json!({"shape": "cycle", "nodes": 5, "repeated_block_id": head}),
    );
}

// ── LR-06: memory budget for a 1000-block doc — RSS delta <= 30 MB (REQUIRES_PG) ──────────────────

#[test]
fn perf_proof_perf_lr06_memory() {
    let budget = Budget::resolve("LR-06", "PERF_BUDGET_LR06_MB", 30);
    let Some(attempt) =
        ScenarioAttempt::begin_or_skip("LR-06", "primary", &[("rss_delta_worst", &budget, "MiB")])
    else {
        return;
    };
    let mut be = require_be();

    let content = big_paragraph_doc(1000, "mem");
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": be.workspace_id, "title": "mt045-lr06", "content_json": content }),
    );
    let doc_id = created_doc_id(&created);
    let _guard = DocGuard {
        be: &be,
        doc_id: doc_id.clone(),
    };

    // MEASURED (worst-of-3, adversarial review B3): each run loads + parses the 1000-block doc, holding it
    // alive across the "after" RSS read. The worst (max) delta (MB) is the honest cold-load cost (RISK-5 / CTRL-5).
    let worst_mb = perf_proof_support::measure_rss_delta_worst(|| {
        let loaded = be.get_json(&format!("/knowledge/documents/{doc_id}"));
        let content_json = loaded
            .get("document")
            .and_then(|d| d.get("content_json"))
            .cloned()
            .unwrap_or(loaded.clone());
        let native_doc = doc_json::from_json_value(&content_json)
            .expect("LR-06: loaded content must parse into the native DocModel");
        (loaded, native_doc)
    });
    attempt.stage(
        serde_json::json!([measurement("rss_delta_worst", worst_mb, "MiB")]),
        serde_json::json!({"sample_count": 3}),
    );
    assert!(
        worst_mb <= budget.ceiling as f64,
        "LR-06: RSS delta worst-of-3 {worst_mb:.2} MB must be <= {} MB",
        budget.ceiling
    );

    println!("LR-06 measured={worst_mb:.2}mb (<= {}mb) PASS — 1000-block doc load RSS delta worst of 3 (live PG)", budget.ceiling);
    drop(_guard);
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([measurement("rss_delta_worst", worst_mb, "MiB")]),
        serde_json::json!({"sample_count": 3}),
    );
}

// ── LR-07: HTML projection of a 1000-block doc — <= 2 s, length > 50000 (REQUIRES_PG) ─────────────

#[test]
fn perf_proof_perf_lr07_html_projection() {
    let budget = Budget::resolve("LR-07", "PERF_BUDGET_LR07_MS", 2_000);
    let Some(attempt) =
        ScenarioAttempt::begin_or_skip("LR-07", "primary", &[("html_projection", &budget, "ms")])
    else {
        return;
    };
    let mut be = require_be();

    // Keep the contract-sized 1000 blocks while ensuring their deterministic text payload really
    // projects beyond the contract's 50k-character floor.
    let content = big_paragraph_doc(1000, "projection-payload");
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": be.workspace_id, "title": "mt045-lr07", "content_json": content }),
    );
    let doc_id = created_doc_id(&created);
    let _guard = DocGuard {
        be: &be,
        doc_id: doc_id.clone(),
    };

    // MEASURED: the server HTML projection response time + length.
    let (html, elapsed_ms) = perf_proof_support::time_ms(|| {
        be.get_text(&format!(
            "/knowledge/documents/{doc_id}/projection?format=html"
        ))
    });
    attempt.stage(
        serde_json::json!([measurement("html_projection", elapsed_ms as f64, "ms")]),
        serde_json::json!({"html_chars": html.len()}),
    );
    assert!(
        html.len() > 50_000,
        "LR-07: the projected HTML must be > 50000 chars (got {})",
        html.len()
    );
    assert!(
        budget.passes(elapsed_ms),
        "LR-07: HTML projection {elapsed_ms} ms must be <= {} ms",
        budget.ceiling
    );

    println!("LR-07 measured={elapsed_ms}ms (<= {}ms) PASS — 1000-block HTML projection, {} chars (live PG)", budget.ceiling, html.len());
    drop(_guard);
    be.assert_cleanup();
    attempt.pass(
        serde_json::json!([measurement("html_projection", elapsed_ms as f64, "ms")]),
        serde_json::json!({"html_chars": html.len()}),
    );
}

// ── shared helpers ────────────────────────────────────────────────────────────────────────────────

fn require_be() -> LiveBackend {
    pg_proof_support::require_live_backend()
}

/// A `{ type:"doc", content:[ <count> paragraph blocks ] }` payload (~50 chars/block). Deterministic.
fn big_paragraph_doc(count: usize, tag: &str) -> serde_json::Value {
    let blocks: Vec<serde_json::Value> = (0..count)
        .map(|i| serde_json::json!({
            "type": "paragraph",
            "content": [ { "type": "text", "text": format!("{tag} block {i} lorem ipsum dolor sit amet") } ]
        }))
        .collect();
    serde_json::json!({ "type": "doc", "content": blocks })
}

/// The created rich document id (`document.rich_document_id`, the real create-response shape).
fn created_doc_id(created: &serde_json::Value) -> String {
    created
        .get("document")
        .and_then(|d| d.get("rich_document_id").or_else(|| d.get("id")))
        .and_then(|v| v.as_str())
        .or_else(|| created.get("rich_document_id").and_then(|v| v.as_str()))
        .or_else(|| created.get("id").and_then(|v| v.as_str()))
        .expect("LR: the create response must carry a rich_document_id")
        .to_owned()
}

/// The same-ID Loom projection id returned with every native RichDocument.
fn created_block_id(created: &serde_json::Value) -> String {
    created
        .pointer("/document/block_id")
        .and_then(serde_json::Value::as_str)
        .expect("LR-05: create response must carry document.block_id")
        .to_owned()
}

fn first_transclusion_ref(content_json: &serde_json::Value) -> Option<String> {
    fn walk(node: &serde_json::Value) -> Option<String> {
        if node.get("type").and_then(serde_json::Value::as_str) == Some("loomTransclusion") {
            return node
                .pointer("/attrs/refValue")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
        }
        node.get("content")
            .and_then(serde_json::Value::as_array)
            .and_then(|children| children.iter().find_map(walk))
    }
    walk(content_json)
}

/// Best-effort public-API cleanup for individual document fixtures. The workspace fixture additionally
/// deletes the complete owned workspace, so a panic cannot leak these rows into another proof run.
struct DocGuard<'a> {
    be: &'a LiveBackend,
    doc_id: String,
}
impl Drop for DocGuard<'_> {
    fn drop(&mut self) {
        let _ = self
            .be
            .delete(&format!("/knowledge/documents/{}", self.doc_id));
    }
}
