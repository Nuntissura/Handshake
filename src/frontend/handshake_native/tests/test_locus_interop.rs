//! Editors <-> Locus (Pillar 6, structured work tracking) interop proofs — WP-KERNEL-012 MT-068 (cluster E10).
//!
//! Maps each MT-068 acceptance criterion + proof target to real runtime proof through managed PostgreSQL,
//! the bound backend, a counted MT-034-search mock, an in-process negative-path server, and egui_kittest.
//!
//! ## VERIFIED BACKEND REALITY
//!
//! handshake_core exposes read-only Locus routes backed by the canonical `work_packets` and `micro_tasks`
//! PostgreSQL tables. `resolve_locus_ref` distinguishes a live missing record from an unavailable route. The
//! live proof seeds canonical rows, resolves them through those routes, and independently proves persisted
//! document attributes and reverse lookup.
//!
//! Proof map:
//! - AC-001 / PT-001: `parse_locus_ref` recognizes `locus://wp/WP-KERNEL-012` + `locus://mt/MT-034`
//!   (kind/id/normalized), an invalid scheme returns None.
//! - AC-002 / PT-002: `resolve_locus_ref` resolves canonical live WP/MT rows with non-empty titles. Mock 200 and
//!   unavailable-route responses independently prove projection and typed failure behavior.
//! - AC-003 / PT-003: the canonical localhost Argus loop inspects, clicks, and freshly re-inspects each
//!   `locus-ref-chip-{kind}-{id}`, then proves the matching ref reached the MT-030/MT-031 nav seam.
//! - AC-004 / PT-004: reverse lookup — seed a doc containing `locus://mt/MT-034`, `find_documents_referencing`
//!   lists it (keyed on the normalized ref, de-duplicated on (document_id, block_id)).
//! - AC-005 / PT-005: an unavailable Locus READ endpoint raises `LocusReadApiUnavailable` naming the endpoint,
//!   while a canonical live-route 404 is `NotFound`; the chip renders greyed without panic.
//! - AC-006 / PT-006: the `locus_ref` hsLink node survives save+reload with {kind,id,normalized-derivable}; a
//!   live-404 renders a greyed `unresolved` chip without panic.
//! - AC-007 / PT-007: grep proof — MT-032 normalizer reused, MT-034 CrossRef node/chip + search reused,
//!   `open-locus-ref` via the existing command/nav seam, AccessKit ids via the WP-011 registry; no duplicates.
//! - AC-008: kittest AccessKit dump — `locus-ref-chip-{kind}-{id}` (Link/Button) + `locus-refby-{document_id}`
//!   (ListItem) present with correct roles + no duplicate author_id.
//! - AC-009: diff/dependency gate — frontend resolution is GET-only, backend routes are read-only PostgreSQL,
//!   and no SQLite authority is introduced.
//! - AC-010: `cargo test -p handshake-native test_locus_interop` passes with no panics (this file).

use std::io::{BufRead, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;
use sha2::{Digest, Sha256};

use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::{
    HealthInfo, LoomSearchBlock, LoomSearchV2Body, LoomSearchV2Hit, LoomSearchV2Response,
};
use handshake_native::interop::{
    dispatch_locus_ref_open, normalize_locus_id, parse_locus_ref, CrossRefError, DocumentRef,
    FindNotesHttp, FindNotesSearch, InteractionBus, LocusInteropError, LocusInteropService,
    LocusRefKind, CMD_OPEN_LOCUS_REF, LOCUS_REF_KIND,
};
use handshake_native::mcp::{
    ScreenshotError, SessionToken, SwarmMcpServer, ARGUS_CLICK_METHOD, ARGUS_INSPECT_METHOD,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::document_model::doc_json::{
    from_json_string, to_content_json_value, to_json_string,
};
use handshake_native::rich_editor::document_model::node::{
    BlockNode, Child, HsLinkNode, NodeKind, TextLeaf,
};
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::rich_editor::wikilinks::inline_view::{
    chip_colors, chip_label, is_locus_ref, locus_ref_chip_author_id,
    locus_ref_chip_occurrence_author_id, EditorEvent,
};
use handshake_native::rich_editor::wikilinks::parser::parse_wikilink;
use handshake_native::tab_bar::TabState;
use handshake_native::theme::HsTheme;

// Shared managed-PostgreSQL fixture. The default live proofs attach to a healthy root-managed backend or
// start an already-built product executable, create an isolated workspace, and never invoke Cargo.
mod pg_proof_support;
use pg_proof_support::{require_live_backend, LiveBackend};

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Artifact hygiene (CX-212E / SCREENSHOT RULE): all artifacts go to the EXTERNAL root ONLY.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    let root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(4)
                .expect("native crate must live below a worktree root")
                .join("Handshake_Artifacts")
        });
    assert!(
        root.is_absolute(),
        "HANDSHAKE_ARTIFACTS_ROOT must resolve to an absolute path"
    );
    root.join("handshake-test").join(subdir)
}

const MT068_RELEVANT_SOURCE_PATHS: &[&str] = &[
    "src/backend/handshake_core/src/api/locus.rs",
    "src/frontend/handshake_native/src/accessibility/registry.rs",
    "src/frontend/handshake_native/src/app.rs",
    "src/frontend/handshake_native/src/interop/cross_ref.rs",
    "src/frontend/handshake_native/src/interop/interaction_bus.rs",
    "src/frontend/handshake_native/src/interop/locus_interop.rs",
    "src/frontend/handshake_native/src/manual_content_editors.rs",
    "src/frontend/handshake_native/src/rich_editor/wikilinks/inline_view.rs",
    "src/frontend/handshake_native/tests/native_gui_support/screenshot_harness.rs",
    "src/frontend/handshake_native/tests/pg_proof_support/mod.rs",
    "src/frontend/handshake_native/tests/test_locus_interop.rs",
    "src/frontend/handshake_native/tests/test_manual_content.rs",
];

fn product_repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("native crate must live at repo/src/frontend/handshake_native")
}

fn current_source_sha() -> String {
    let clean = std::process::Command::new("git")
        .args(["diff", "--quiet", "HEAD", "--"])
        .args(MT068_RELEVANT_SOURCE_PATHS)
        .current_dir(product_repo_root())
        .status()
        .expect("check MT-068 relevant source cleanliness");
    assert!(
        clean.success(),
        "MT-068 canonical proof refuses dirty relevant source; commit implementation and proof first"
    );
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(product_repo_root())
        .output()
        .expect("resolve current source hash");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("source hash UTF-8")
        .trim()
        .to_owned()
}

fn current_runtime_source_tree() -> String {
    let status = std::process::Command::new("git")
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .current_dir(product_repo_root())
        .output()
        .expect("inspect complete MT-068 runtime source cleanliness");
    assert!(status.status.success());
    let unexpected = String::from_utf8(status.stdout)
        .expect("git status UTF-8")
        .lines()
        .filter(|line| {
            let path = line.get(3..).unwrap_or_default();
            !matches!(path, "AGENTS.md" | "CLAUDE.md")
        })
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert!(
        unexpected.is_empty(),
        "MT-068 canonical proof refuses dirty/untracked transitive runtime source outside the known authority files: {unexpected:?}"
    );
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD^{tree}"])
        .current_dir(product_repo_root())
        .output()
        .expect("resolve complete committed runtime source tree");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("runtime source tree UTF-8")
        .trim()
        .to_owned()
}

fn current_proof_source_blobs() -> serde_json::Map<String, serde_json::Value> {
    MT068_RELEVANT_SOURCE_PATHS
        .iter()
        .map(|path| {
            let spec = format!("HEAD:{path}");
            let output = std::process::Command::new("git")
                .args(["rev-parse", &spec])
                .current_dir(product_repo_root())
                .output()
                .unwrap_or_else(|error| {
                    panic!("resolve committed MT-068 source blob {path}: {error}")
                });
            assert!(
                output.status.success(),
                "resolve committed MT-068 source blob {path}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            (
                path.to_string(),
                serde_json::Value::String(
                    String::from_utf8(output.stdout)
                        .expect("source blob UTF-8")
                        .trim()
                        .to_owned(),
                ),
            )
        })
        .collect()
}

/// Assert NO repo-local artifact directory exists under the crate (the SCREENSHOT/TEST-ARTIFACT RULE,
/// CX-212E). Artifacts go to the external `Handshake_Artifacts/handshake-test` root ONLY; a stray
/// `test_output/` OR `tests/screenshots/` is a hygiene FAILURE.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

/// Isolate the production Argus discovery binding under the external proof root. This uses the same
/// platform app-data indirection as the shipped server while preventing a test from overwriting a live
/// Handshake process's binding.
struct ScopedArgusAppData {
    variable: &'static str,
    previous: Option<std::ffi::OsString>,
    root: PathBuf,
}

impl ScopedArgusAppData {
    fn install(root: PathBuf) -> Self {
        let root = if root.is_absolute() {
            root
        } else {
            std::env::current_dir()
                .expect("resolve MT-068 Argus current directory")
                .join(root)
        };
        std::fs::create_dir_all(&root).expect("create isolated MT-068 Argus binding root");
        let root =
            std::fs::canonicalize(&root).expect("canonicalize isolated MT-068 Argus binding root");
        #[cfg(target_os = "windows")]
        let variable = "LOCALAPPDATA";
        #[cfg(not(target_os = "windows"))]
        let variable = "XDG_DATA_HOME";
        let previous = std::env::var_os(variable);
        std::env::set_var(variable, &root);
        Self {
            variable,
            previous,
            root,
        }
    }
}

impl Drop for ScopedArgusAppData {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(self.variable, value),
            None => std::env::remove_var(self.variable),
        }
        if let Err(error) = std::fs::remove_dir_all(&self.root) {
            if error.kind() != std::io::ErrorKind::NotFound {
                panic!(
                    "remove isolated MT-068 Argus binding root {}: {error}",
                    self.root.display()
                );
            }
        }
    }
}

fn json_has_author_id(value: &serde_json::Value, expected: &str) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            object.get("author_id").and_then(|value| value.as_str()) == Some(expected)
                || object
                    .values()
                    .any(|value| json_has_author_id(value, expected))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| json_has_author_id(value, expected)),
        _ => false,
    }
}

/// A real localhost JSON-RPC client/server loop over the exact snapshot and action channel owned by the
/// mounted `HandshakeApp`. This is the canonical Argus boundary, not direct kittest activation.
struct LocusArgusDriver {
    runtime: tokio::runtime::Runtime,
    server: SwarmMcpServer,
    _app_data: ScopedArgusAppData,
    token: String,
    client_session_id: String,
    next_id: u64,
    clicked_targets: Vec<String>,
}

impl LocusArgusDriver {
    fn bind(app: &HandshakeApp) -> Self {
        let unique = uuid::Uuid::new_v4().simple().to_string();
        let app_data = ScopedArgusAppData::install(
            external_artifact_dir("mt068-argus-binding").join(format!("run-{unique}")),
        );
        let session_token = SessionToken::from_hex(&format!("mt068-locus-{unique}"));
        let token = session_token.as_hex().to_owned();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("MT-068 Argus runtime");
        let server = runtime
            .block_on(SwarmMcpServer::bind(
                session_token,
                app.mcp_snapshot_slot(),
                app.mcp_action_channel(),
                Arc::new(|| {
                    Err(ScreenshotError(
                        "MT-068 inspect/click proof does not request a screenshot".to_owned(),
                    ))
                }),
            ))
            .expect("bind the production Argus localhost server");
        Self {
            runtime,
            server,
            _app_data: app_data,
            token,
            client_session_id: "mt068-locus-agent".to_owned(),
            next_id: 1,
            clicked_targets: Vec::new(),
        }
    }

    fn rpc(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
            "params": params,
            "session_token": self.token,
            "client_session_id": self.client_session_id,
        });
        self.next_id += 1;
        let mut stream = std::net::TcpStream::connect(self.server.tcp_addr())
            .expect("connect to production Argus TCP listener");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .expect("bound Argus read timeout");
        writeln!(stream, "{request}").expect("write Argus JSON-RPC request");
        stream.flush().expect("flush Argus JSON-RPC request");
        let mut response_line = String::new();
        std::io::BufReader::new(stream)
            .read_line(&mut response_line)
            .expect("read Argus JSON-RPC response");
        let response: serde_json::Value =
            serde_json::from_str(response_line.trim()).expect("decode Argus JSON-RPC response");
        assert!(
            response.get("error").is_none(),
            "canonical Argus request failed: {response}"
        );
        response
    }

    fn inspect(&mut self, harness: &mut Harness<'_, HandshakeApp>) -> serde_json::Value {
        harness.state_mut().capture_mcp_snapshot_for_navigation();
        self.rpc(ARGUS_INSPECT_METHOD, serde_json::json!({}))["result"].clone()
    }

    fn click_and_reinspect(
        &mut self,
        harness: &mut Harness<'_, HandshakeApp>,
        author_id: &str,
    ) -> serde_json::Value {
        let before = self.inspect(harness);
        assert!(
            json_has_author_id(&before, author_id),
            "canonical argus.inspect sees mounted Locus target {author_id}"
        );
        let click = self.rpc(
            ARGUS_CLICK_METHOD,
            serde_json::json!({ "target": author_id }),
        );
        assert_eq!(click["result"]["queued"], true);
        assert!(
            click["result"]["agent_id"]
                .as_str()
                .is_some_and(|agent| agent.ends_with(":client:mt068-locus-agent")),
            "Argus click must retain caller attribution: {click}"
        );
        let receipt_id = click["result"]["receipt_id"]
            .as_u64()
            .expect("Argus click returns a receipt id");

        let mut raw_input = egui::RawInput::default();
        <HandshakeApp as eframe::App>::raw_input_hook(
            harness.state_mut(),
            &egui::Context::default(),
            &mut raw_input,
        );
        assert_eq!(
            raw_input.events.len(),
            1,
            "one canonical Argus click drains as one production egui event"
        );
        for event in raw_input.events {
            harness.event(event);
        }
        harness.run_steps(3);

        let after = self.inspect(harness);
        let receipt = after["action_receipts"]
            .as_array()
            .and_then(|receipts| {
                receipts
                    .iter()
                    .find(|receipt| receipt["receipt_id"].as_u64() == Some(receipt_id))
            })
            .expect("fresh argus.inspect returns the click receipt");
        assert!(
            receipt["status"]
                .as_str()
                .is_some_and(|status| matches!(status, "applied" | "indeterminate")),
            "Argus receipt is terminal and non-rejected: {receipt}"
        );
        let receipt_status = receipt["status"]
            .as_str()
            .expect("Argus receipt status")
            .to_owned();
        let agent_id = click["result"]["agent_id"]
            .as_str()
            .expect("Argus click agent id")
            .to_owned();
        self.clicked_targets.push(author_id.to_owned());
        serde_json::json!({
            "method": ARGUS_CLICK_METHOD,
            "target": author_id,
            "before": before,
            "action_result": click["result"].clone(),
            "receipt_id": receipt_id,
            "receipt_status": receipt_status,
            "agent_id": agent_id,
            "after": after,
        })
    }

    fn finish(mut self) -> serde_json::Value {
        let entries = self.server.action_log().drain_log();
        assert_eq!(entries.len(), self.clicked_targets.len());
        for (entry, target) in entries.iter().zip(&self.clicked_targets) {
            assert_eq!(entry.op_name, ARGUS_CLICK_METHOD);
            assert_eq!(&entry.target_key, target);
            assert!(entry.agent_id.ends_with(":client:mt068-locus-agent"));
            assert_ne!(entry.node_id, 0);
        }
        let evidence = entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "method": entry.op_name,
                    "target": entry.target_key,
                    "agent_id": entry.agent_id,
                    "node_id": entry.node_id,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(self.server.leases().active_resource_count(), 0);
        self.server.shutdown();
        drop(self.runtime);
        serde_json::Value::Array(evidence)
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Test helpers (the proven MT-066/MT-067 patterns).
// ════════════════════════════════════════════════════════════════════════════════════════════════

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

/// Collect every author_id present in the live AccessKit tree.
fn author_ids<S>(harness: &Harness<'_, S>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// The `{:?}` role string of the first node with `author_id`, if present.
fn role_of(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<String> {
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return Some(format!("{:?}", ak.role()));
        }
    }
    None
}

/// Build a one-paragraph doc with a `locus` cross-ref hsLink atom embedded (the authored shape:
/// ref_kind="locus", ref_value=`locus://{kind}/{id}`, label=display id, resolved flag).
fn doc_with_locus_ref(locus_uri: &str, label: &str, resolved: bool) -> BlockNode {
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::new("see ")));
    let mut link = HsLinkNode::new(LOCUS_REF_KIND, locus_uri, label);
    link.resolved = resolved;
    para.children.push(Child::HsLink(link));
    para.children.push(Child::Text(TextLeaf::new("")));
    BlockNode::doc(vec![para])
}

/// A counted in-memory MT-034-search mock (NO backend): returns the seeded hits per query so the reverse
/// lookup drives the REAL `find_notes_with` pipeline without a live PG, and counts calls.
struct CountingReverseLookup {
    hits: Vec<LoomSearchV2Hit>,
    contents: std::collections::HashMap<String, serde_json::Value>,
    last_query: std::sync::Mutex<Option<String>>,
    calls: AtomicUsize,
}

impl CountingReverseLookup {
    fn new(hits: Vec<LoomSearchV2Hit>) -> Self {
        Self {
            hits,
            contents: std::collections::HashMap::new(),
            last_query: std::sync::Mutex::new(None),
            calls: AtomicUsize::new(0),
        }
    }

    fn with_locus_contents(mut self, document_ids: &[&str], locus_uri: &str) -> Self {
        for document_id in document_ids {
            self.contents.insert(
                (*document_id).to_owned(),
                serde_json::json!({
                    "type": "doc",
                    "content": [{
                        "type": "paragraph",
                        "content": [{
                            "type": "hsLink",
                            "attrs": {
                                "refKind": "locus",
                                "refValue": locus_uri,
                                "label": "MT"
                            }
                        }]
                    }]
                }),
            );
        }
        self
    }

    fn with_content(mut self, document_id: &str, content: serde_json::Value) -> Self {
        self.contents.insert(document_id.to_owned(), content);
        self
    }
}

impl FindNotesSearch for CountingReverseLookup {
    fn search<'a>(
        &'a self,
        _workspace_id: &'a str,
        body: &'a LoomSearchV2Body,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<LoomSearchV2Response, CrossRefError>>
                + Send
                + 'a,
        >,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.last_query.lock().unwrap() = Some(body.query.clone());
        let content_type = body.content_type.clone();
        let offset = body.offset as usize;
        let limit = body.limit as usize;
        let matching = self
            .hits
            .iter()
            .filter(|hit| {
                content_type
                    .as_deref()
                    .is_none_or(|expected| hit.block.content_type == expected)
            })
            .cloned()
            .collect::<Vec<_>>();
        Box::pin(async move {
            Ok(LoomSearchV2Response {
                hits: matching.iter().skip(offset).take(limit).cloned().collect(),
                content_type_facets: Default::default(),
                semantic_available: false,
                total: matching.len() as i64,
            })
        })
    }

    fn load_document_content<'a>(
        &'a self,
        document_id: &'a str,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, CrossRefError>> + Send + 'a>,
    > {
        let content = self.contents.get(document_id).cloned();
        Box::pin(async move {
            content.ok_or_else(|| {
                CrossRefError::NotFound(format!("counted reverse-lookup document {document_id}"))
            })
        })
    }
}

fn hit(
    block_id: &str,
    title: Option<&str>,
    content_type: &str,
    highlight: &str,
) -> LoomSearchV2Hit {
    LoomSearchV2Hit {
        block: LoomSearchBlock {
            block_id: block_id.to_owned(),
            content_type: content_type.to_owned(),
            document_id: None,
            title: title.map(str::to_owned),
        },
        score: 1.0,
        fts_rank: 0.0,
        trgm_sim: 0.0,
        vector_sim: 0.0,
        edge_degree: 0,
        highlight: highlight.to_owned(),
    }
}

/// Spin up a one-shot mock server that replies with `status_line` + `body` to the FIRST request and
/// captures that request's line. Returns (base_url, join handle delivering the request line). The PROVEN
/// MT-066/MT-067 TcpListener pattern — no new dependency.
fn spawn_mock(
    status_line: &'static str,
    body: serde_json::Value,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request_line = read_request_line(&mut stream);
        let body_str = body.to_string();
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body_str}",
            body_str.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        request_line
    });
    (base_url, handle)
}

fn read_request_line(stream: &mut std::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if String::from_utf8_lossy(&buf).contains("\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    text.lines().next().unwrap_or("").to_string()
}

/// An empty reverse-lookup backend (the resolution tests do not exercise reverse lookup).
fn no_reverse_lookup() -> Arc<dyn FindNotesSearch> {
    Arc::new(CountingReverseLookup::new(vec![]))
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn managed_pg_url() -> String {
    std::env::var("HSK_PROOF_DATABASE_URL")
        .or_else(|_| std::env::var("POSTGRES_TEST_URL"))
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres@127.0.0.1:5544/handshake".to_owned())
}

fn psql_executable() -> PathBuf {
    if let Ok(explicit) = std::env::var("HSK_PSQL_BIN") {
        let path = PathBuf::from(explicit);
        assert!(
            path.is_file(),
            "HSK_PSQL_BIN does not name psql: {}",
            path.display()
        );
        return path;
    }
    if std::process::Command::new("psql")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return PathBuf::from("psql");
    }
    #[cfg(windows)]
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        let root = PathBuf::from(program_files).join("PostgreSQL");
        if let Ok(versions) = std::fs::read_dir(root) {
            let mut candidates = versions
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("bin").join("psql.exe"))
                .filter(|path| path.is_file())
                .collect::<Vec<_>>();
            candidates.sort();
            if let Some(path) = candidates.pop() {
                return path;
            }
        }
    }
    panic!("managed-PG fixture requires psql");
}

fn locus_sql_output(sql: &str) -> Result<std::process::Output, String> {
    let mut command = std::process::Command::new(psql_executable());
    command
        .args(["-X", "-v", "ON_ERROR_STOP=1", "-q", "--dbname"])
        .arg(managed_pg_url())
        .arg("-c")
        .arg(sql);
    let output = bounded_command_output(command, std::time::Duration::from_secs(15))
        .map_err(|error| format!("bounded psql execution failed: {error}"))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "psql exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn run_locus_sql(sql: &str) {
    locus_sql_output(sql).expect("MT-068 canonical Locus fixture SQL");
}

fn bounded_command_output(
    mut command: std::process::Command,
    timeout: std::time::Duration,
) -> std::io::Result<std::process::Output> {
    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn()?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(_) => return child.wait_with_output(),
            None if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            None => {
                child.kill()?;
                child.wait()?;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("child process exceeded {} seconds", timeout.as_secs()),
                ));
            }
        }
    }
}

fn seed_locus_records(wp_id: &str, mt_id: &str) {
    run_locus_sql(&format!(
        "BEGIN; \
         INSERT INTO work_packets \
           (wp_id, version, title, description, status, priority, phase, routing, task_packet_path, \
            task_board_status, assignee, reporter, created_at, updated_at, vector_clock, metadata) \
         VALUES ({wp}, 1, 'MT-068 live work packet', 'canonical persisted Locus resolution proof', \
                 'in_progress', 1, 'implementation', 'native-editors', '', 'in_progress', NULL, \
                 'mt068-proof', '2026-07-16T00:00:00Z', '2026-07-16T00:00:00Z', '{{}}', '{{}}'); \
         INSERT INTO micro_tasks (mt_id, wp_id, name, status, current_iteration, escalation_level, metadata) \
         VALUES ({mt}, {wp}, 'MT-068 live microtask', 'in_progress', 1, 0, '{{}}'); \
         COMMIT;",
        wp = sql_literal(wp_id),
        mt = sql_literal(mt_id),
    ));
}

fn cleanup_locus_records(wp_id: &str) {
    let sql = format!(
        "DELETE FROM work_packets WHERE wp_id = {};",
        sql_literal(wp_id)
    );
    run_locus_sql(&sql);
}

struct LocusRecordCleanup {
    wp_id: String,
    active: bool,
}

impl LocusRecordCleanup {
    fn assert_cleanup(&mut self) {
        cleanup_locus_records(&self.wp_id);
        self.active = false;
    }
}

impl Drop for LocusRecordCleanup {
    fn drop(&mut self) {
        if self.active {
            let sql = format!(
                "DELETE FROM work_packets WHERE wp_id = {};",
                sql_literal(&self.wp_id)
            );
            if let Err(error) = locus_sql_output(&sql) {
                eprintln!(
                    "MT-068 best-effort cleanup failed for {}: {error}",
                    self.wp_id
                );
            }
            self.active = false;
        }
    }
}

fn created_doc_id(created: &serde_json::Value) -> String {
    created
        .pointer("/document/rich_document_id")
        .or_else(|| created.get("rich_document_id"))
        .and_then(serde_json::Value::as_str)
        .expect("created rich document carries rich_document_id")
        .to_owned()
}

fn created_doc_version(created: &serde_json::Value) -> i64 {
    created
        .pointer("/document/doc_version")
        .or_else(|| created.get("doc_version"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1)
}

fn loaded_content_json(loaded: &serde_json::Value) -> serde_json::Value {
    loaded
        .pointer("/document/content_json")
        .or_else(|| loaded.get("content_json"))
        .cloned()
        .expect("loaded rich document carries content_json")
}

fn doc_with_wp_mt_and_missing_refs(wp_uri: &str, mt_uri: &str, missing_uri: &str) -> BlockNode {
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children
        .push(Child::Text(TextLeaf::new("work packet ")));
    para.children
        .push(Child::HsLink(HsLinkNode::new(LOCUS_REF_KIND, wp_uri, "WP")));
    para.children
        .push(Child::Text(TextLeaf::new(" microtask ")));
    para.children
        .push(Child::HsLink(HsLinkNode::new(LOCUS_REF_KIND, mt_uri, "MT")));
    para.children
        .push(Child::Text(TextLeaf::new(" missing record ")));
    let mut missing = HsLinkNode::new(LOCUS_REF_KIND, missing_uri, "UNRESOLVED");
    missing.resolved = false;
    para.children.push(Child::HsLink(missing));
    para.children.push(Child::Text(TextLeaf::new("")));
    BlockNode::doc(vec![para])
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-001 / PT-001 — parse_locus_ref recognizes the wp/mt URI forms; an invalid scheme returns None.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac001_parse_locus_ref_wp_and_mt() {
    let wp = parse_locus_ref("locus://wp/WP-KERNEL-012").expect("a valid wp ref");
    assert_eq!(
        wp.kind,
        LocusRefKind::WorkPacket,
        "AC-001: locus://wp -> WorkPacket"
    );
    assert_eq!(
        wp.id, "WP-KERNEL-012",
        "AC-001: the id is extracted (original case)"
    );
    assert_eq!(
        wp.normalized, "locus://wp/wp-kernel-012",
        "AC-001: normalized is the lower-cased key"
    );

    let mt = parse_locus_ref("locus://mt/MT-034").expect("a valid mt ref");
    assert_eq!(
        mt.kind,
        LocusRefKind::Microtask,
        "AC-001: locus://mt -> Microtask"
    );
    assert_eq!(mt.id, "MT-034");
    assert_eq!(mt.normalized, "locus://mt/mt-034");

    // The prefix-stripped `{kind}/{id}` form the WIKILINK parser emits (`[[locus:wp/WP-KERNEL-012]]` ->
    // ref_value="wp/WP-KERNEL-012") parses to the SAME canonical key as the URI form (the must-fix /
    // RISK-001 single-shared-key invariant for the form a user actually authors).
    let authored_wp =
        parse_locus_ref("wp/WP-KERNEL-012").expect("AC-001: the wikilink authoring form parses");
    assert_eq!(authored_wp.kind, LocusRefKind::WorkPacket);
    assert_eq!(authored_wp.id, "WP-KERNEL-012");
    assert_eq!(
        authored_wp.normalized, "locus://wp/wp-kernel-012",
        "AC-001/RISK-001: the authored wp/ form normalizes to the same single key as the URI form"
    );
    let authored_mt = parse_locus_ref("mt/MT-034").expect("AC-001: the mt authoring form parses");
    assert_eq!(authored_mt.kind, LocusRefKind::Microtask);
    assert_eq!(authored_mt.normalized, "locus://mt/mt-034");

    // An invalid scheme returns None (AC-001).
    assert!(
        parse_locus_ref("https://wp/WP-1").is_none(),
        "AC-001: an invalid scheme returns None"
    );
    assert!(
        parse_locus_ref("loom://ws/blk").is_none(),
        "AC-001: the loom scheme is not a locus ref"
    );
    assert!(
        parse_locus_ref("locus://zz/X").is_none(),
        "AC-001: an unknown kind returns None"
    );
    // A non-locus prefix-stripped value (a code/file ref, no wp/mt leading segment) does NOT match the
    // `{kind}/{id}` branch — only `wp/`/`mt/` leading segments are locus refs.
    assert!(
        parse_locus_ref("src/app.ts").is_none(),
        "AC-001: a file path is not a locus ref"
    );
    assert!(
        parse_locus_ref("zz/Q").is_none(),
        "AC-001: an unknown kind/id leading segment returns None"
    );

    // The normalized key is consistent with the MT-032/MT-015 normalizer (no second normalizer — AC-007).
    assert_eq!(
        wp.normalized,
        normalize_locus_id(LocusRefKind::WorkPacket, "WP-KERNEL-012")
    );
    println!("AC-001/PT-001 OK: locus://wp/WP-KERNEL-012 + locus://mt/MT-034 parse; invalid scheme -> None");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-002 / PT-002 — resolve_locus_ref against the bound READ route. A route-level unavailable response maps
// to the typed blocker; a mock 200 independently proves the resolved-record projection (non-empty title).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac002_resolve_locus_ref_route_absent_is_typed_blocker() {
    // Negative path: a route-level 404 without the canonical record-not-found code maps to the typed blocker
    // `LocusReadApiUnavailable` naming the endpoint (NOT a fabricated record).
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 404 Not Found",
        serde_json::json!({"error": "absent"}),
    );
    let svc = LocusInteropService::with_base_url(base_url, "WS-1", no_reverse_lookup());
    let wp = parse_locus_ref("locus://wp/WP-KERNEL-012").unwrap();
    let result = rt().block_on(async { svc.resolve_locus_ref(&wp).await });
    let req_line = server.join().unwrap();

    // The probe is a read-only GET at the documented work-packets route.
    assert!(
        req_line.starts_with("GET "),
        "AC-009: the resolution read must be a GET; got '{req_line}'"
    );
    assert!(
        req_line.contains("/workspaces/WS-1/locus/work-packets/WP-KERNEL-012"),
        "AC-002: probes the documented work-packets route; got '{req_line}'"
    );
    match result {
        Err(LocusInteropError::LocusReadApiUnavailable { endpoint }) => {
            assert!(
                endpoint.contains("/locus/work-packets/WP-KERNEL-012"),
                "AC-005: the typed blocker names the probed endpoint; got '{endpoint}'"
            );
        }
        other => panic!("AC-002/AC-005: an absent route (404) must map to LocusReadApiUnavailable, got {other:?}"),
    }
    println!("AC-002/AC-005 OK: absent /locus/work-packets route -> LocusReadApiUnavailable (typed blocker)");
}

#[test]
fn ac002_resolve_locus_ref_resolved_record_projection() {
    // PROVES the resolved-record projection (non-empty title) deterministically: a mock 200 returns the
    // documented record body shape; resolve_locus_ref projects it into a LocusRecord with a non-empty title
    // (the AC-002 success assertion). The kind + id come from the LocusRef (request authority).
    let body = serde_json::json!({
        "title": "Native Editors: Obsidian + VS Code parity",
        "summary": "Rebuild the editors as native Rust tools",
        "status": "Ready for Dev"
    });
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", body);
    let svc = LocusInteropService::with_base_url(base_url, "WS-9", no_reverse_lookup());
    let wp = parse_locus_ref("locus://wp/WP-KERNEL-012").unwrap();
    let record = rt()
        .block_on(async { svc.resolve_locus_ref(&wp).await })
        .expect("AC-002: a 200 body resolves to a record");
    let _ = server.join();

    assert_eq!(record.kind, LocusRefKind::WorkPacket);
    assert_eq!(record.id, "WP-KERNEL-012");
    assert!(
        !record.title.is_empty(),
        "AC-002: a resolved record has a non-empty title"
    );
    assert_eq!(record.title, "Native Editors: Obsidian + VS Code parity");
    assert_eq!(
        record.summary.as_deref(),
        Some("Rebuild the editors as native Rust tools")
    );
    assert_eq!(record.status.as_deref(), Some("Ready for Dev"));
    println!("AC-002 OK: a Locus record body resolves to LocusRecord{{title non-empty}} (projection proof)");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-002 / PT-002 (LIVE) — default managed-runtime integration against the real Locus routes.
//
// The contract's bound READ APIs (GET /workspaces/{ws}/locus/work-packets/{id} + /locus/microtasks/{id})
// resolve canonical PostgreSQL records. This test proves non-empty WP/MT records, saved rich-document attrs,
// and persisted reverse lookup against the live route. It never fabricates a record.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn resolve_locus_ref_against_real_pg_live() {
    let source_sha = current_source_sha();
    let runtime_source_tree = current_runtime_source_tree();
    let proof_source_blobs = current_proof_source_blobs();
    let artifact_dir = external_artifact_dir(&format!(
        "wp-kernel-012-mt-068/canonical-argus/run-{}-{}",
        &source_sha[..12],
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&artifact_dir)
        .expect("create MT-068 canonical Argus artifact directory");

    let mut be: LiveBackend = require_live_backend();
    let ws = be.workspace_id.clone();
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let wp_id = format!("WP-MT068-{suffix}");
    let mt_id = format!("MT-MT068-{suffix}");
    seed_locus_records(&wp_id, &mt_id);
    let mut records_cleanup = LocusRecordCleanup {
        wp_id: wp_id.clone(),
        active: true,
    };

    let wp_uri = format!("locus://wp/{wp_id}");
    let mt_uri = format!("locus://mt/{mt_id}");
    let missing_uri = format!("locus://wp/{wp_id}-MISSING");
    let authored = doc_with_wp_mt_and_missing_refs(&wp_uri, &mt_uri, &missing_uri);
    let content_json = to_content_json_value(&authored);
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": ws,
            "title": "MT-068 persisted WP and MT reference proof",
            "content_json": content_json,
        }),
    );
    let document_id = created_doc_id(&created);
    let version = created_doc_version(&created);
    let saved = be.put_json(
        &format!("/knowledge/documents/{document_id}/save"),
        &serde_json::json!({
            "expected_version": version,
            "content_json": to_content_json_value(&authored),
        }),
    );
    let save_receipt_event_id = saved["save_receipt_event_id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .expect("AC-006 LIVE: the rich-document save returns an authentic receipt")
        .to_owned();
    assert!(
        !save_receipt_event_id.is_empty(),
        "AC-006 LIVE: the rich-document save returns an authentic receipt"
    );
    let (old_backend_base, new_backend_base) = be.restart_owned();
    let loaded = be.get_json(&format!("/knowledge/documents/{document_id}"));
    let loaded_content = loaded_content_json(&loaded);
    let reparsed = from_json_string(&loaded_content.to_string())
        .expect("AC-006 LIVE: persisted content_json reloads into the rich document model");
    let reserialized = to_content_json_value(&reparsed).to_string();
    assert!(
        reserialized.contains(&wp_uri)
            && reserialized.contains(&mt_uri)
            && reserialized.contains(&missing_uri),
        "AC-006 LIVE: all exact Locus attrs survive backend restart and save/reload"
    );

    let svc = LocusInteropService::with_base_url(
        be.base.clone(),
        ws.clone(),
        Arc::new(FindNotesHttp::new(be.base.clone())),
    );
    let wp = parse_locus_ref(&wp_uri).unwrap();
    let mt = parse_locus_ref(&mt_uri).unwrap();
    let (wp_record, mt_record, wp_docs, mt_docs) = rt().block_on(async {
        (
            svc.resolve_locus_ref(&wp).await,
            svc.resolve_locus_ref(&mt).await,
            svc.find_documents_referencing(&wp).await,
            svc.find_documents_referencing(&mt).await,
        )
    });
    let wp_record = wp_record.expect("AC-002 LIVE: persisted WP resolves through the live route");
    let mt_record = mt_record.expect("AC-002 LIVE: persisted MT resolves through the live route");
    assert!(!wp_record.title.is_empty() && !mt_record.title.is_empty());
    let wp_docs = wp_docs.expect("AC-004 LIVE: WP reverse lookup");
    let mt_docs = mt_docs.expect("AC-004 LIVE: MT reverse lookup");
    let mut reverse_lookup_counts = serde_json::Map::new();
    for (label, docs) in [("WP", &wp_docs), ("MT", &mt_docs)] {
        let matching = docs
            .iter()
            .filter(|document| document.document_id == document_id)
            .count();
        assert_eq!(
            matching, 1,
            "AC-004 LIVE: {label} reverse lookup returns the persisted document exactly once: {docs:?}"
        );
        reverse_lookup_counts.insert(label.to_owned(), serde_json::json!(matching));
    }

    // Mount a fresh production rich editor for each shared-navigation chip. The click must route the
    // exact WP/MT identity through the existing shell navigator; no direct navigation function is called
    // by the proof.
    let mut argus_state_matrix = Vec::new();
    for (state_label, uri, expected_content_id) in [
        ("work-packet", wp_uri.as_str(), format!("WP:{wp_id}")),
        ("microtask", mt_uri.as_str(), format!("MT::{mt_id}")),
    ] {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("MT-068 mounted navigation runtime");
        let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
            status: "ok".to_owned(),
            db_status: "ok".to_owned(),
            migration_version: Some(1),
        }));
        app.set_backend_base_url_for_test(&be.base, runtime.handle().clone());
        app.bind_active_project_for_integration_test(ws.clone());
        // Reuse the default Notes pane. Replacing `pane-a` (the default code pane) with another
        // LoomWikiPage would mount the one shared rich-document state twice and intentionally expose
        // two accessibility occurrences for every chip, which is not this single-pane navigation proof.
        let pane_id = PaneId::from("pane-b");
        app.pane_registry().lock().unwrap().insert(PaneRecord::new(
            pane_id.clone(),
            PaneType::LoomWikiPage,
            ws.clone(),
            Some(document_id.clone()),
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
        let mut tab = TabState::new(PaneType::LoomWikiPage);
        tab.content_id = Some(document_id.clone());
        let bar = app.tab_bar_states_mut().get_mut(&pane_id).unwrap();
        bar.tabs = vec![tab];
        bar.active_index = 0;
        app.set_active_pane_for_test(Some(pane_id.clone()));
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1000.0, 700.0))
            .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
        let chip_id = locus_ref_chip_author_id(uri);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            // The mounted app legitimately keeps repainting while its Locus/Flight-Recorder async
            // side work settles, so advance one deterministic frame instead of requiring global idle.
            harness.run_steps(1);
            if harness
                .root()
                .children_recursive()
                .any(|node| node.accesskit_node().author_id() == Some(chip_id.as_str()))
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "mounted Locus chip {chip_id}"
            );
        }
        let matching_chip_count = harness
            .root()
            .children_recursive()
            .filter(|node| node.accesskit_node().author_id() == Some(chip_id.as_str()))
            .count();
        assert_eq!(
            matching_chip_count, 1,
            "AC-008 LIVE: the canonical single Notes pane exposes exactly one {chip_id} node"
        );
        let chip = harness.get_by(|node| node.author_id() == Some(chip_id.as_str()));
        let chip_rect = chip.rect();
        let chip_node_id = chip.accesskit_node().id();
        harness.run_steps(1);
        let chip = harness.get_by(|node| node.author_id() == Some(chip_id.as_str()));
        assert_eq!(
            chip.accesskit_node().id(),
            chip_node_id,
            "the mounted chip NodeId remains stable between inspection and action"
        );
        assert!(
            chip.accesskit_node()
                .data()
                .supports_action(egui::accesskit::Action::Click),
            "the mounted Locus chip advertises the AccessKit Click action"
        );
        let missing_chip_id = locus_ref_chip_author_id(&missing_uri);
        let missing_chip =
            harness.get_by(|node| node.author_id() == Some(missing_chip_id.as_str()));
        assert!(
            matches!(
                missing_chip.accesskit_node().role(),
                egui::accesskit::Role::Button | egui::accesskit::Role::Link
            ),
            "the persisted missing Locus ref remains an addressable grey/unresolved chip"
        );
        let before_screenshot_path =
            artifact_dir.join(format!("mt068-locus-{state_label}-before.png"));
        harness
            .render()
            .expect("MT-068 pre-navigation state requires a material harness render")
            .save(&before_screenshot_path)
            .expect("save MT-068 pre-navigation harness render");
        let mut argus = LocusArgusDriver::bind(harness.state());
        let argus_observation = argus.click_and_reinspect(&mut harness, &chip_id);
        let post_action_inspection = &argus_observation["after"];
        assert!(
            post_action_inspection["action_receipts"]
                .as_array()
                .is_some_and(|receipts| !receipts.is_empty()),
            "fresh canonical argus.inspect exposes the navigation receipt"
        );
        assert!(
            json_has_author_id(&argus_observation["before"], &missing_chip_id),
            "canonical pre-action inspection includes the persisted missing/stale Locus chip"
        );
        let active = harness.state().active_pane().cloned().expect("active pane");
        let active_tab = harness
            .state()
            .tab_bar_states()
            .get(&active)
            .and_then(|bar| bar.tabs.get(bar.active_index))
            .expect("Locus click produced active navigator tab");
        assert_eq!(
            active_tab.content_id.as_deref(),
            Some(expected_content_id.as_str()),
            "the active-pane navigator inserts and focuses the exact Locus target after canonical Argus steering; source_chip_rect={chip_rect:?}"
        );
        let after_screenshot_path =
            artifact_dir.join(format!("mt068-locus-{state_label}-after.png"));
        harness
            .render()
            .expect("MT-068 post-navigation state requires a material harness render")
            .save(&after_screenshot_path)
            .expect("save MT-068 post-navigation harness render");
        let before_png = std::fs::read(&before_screenshot_path).expect("read pre-navigation PNG");
        let after_png = std::fs::read(&after_screenshot_path).expect("read post-navigation PNG");
        let before_dimensions = image::GenericImageView::dimensions(
            &image::load_from_memory(&before_png).expect("decode pre-navigation PNG"),
        );
        let after_dimensions = image::GenericImageView::dimensions(
            &image::load_from_memory(&after_png).expect("decode post-navigation PNG"),
        );
        let action_log = argus.finish();
        argus_state_matrix.push(serde_json::json!({
            "state": state_label,
            "persisted_uri": uri,
            "expected_navigation_content_id": expected_content_id,
            "observation": argus_observation,
            "action_log": action_log,
            "screenshots": {
                "before": {
                    "path": before_screenshot_path.display().to_string(),
                    "sha256": format!("{:x}", Sha256::digest(&before_png)),
                    "width": before_dimensions.0,
                    "height": before_dimensions.1,
                },
                "after": {
                    "path": after_screenshot_path.display().to_string(),
                    "sha256": format!("{:x}", Sha256::digest(&after_png)),
                    "width": after_dimensions.0,
                    "height": after_dimensions.1,
                }
            }
        }));
    }

    let missing = parse_locus_ref(&missing_uri).unwrap();
    let missing_result = rt().block_on(svc.resolve_locus_ref(&missing));
    assert!(matches!(
        missing_result,
        Err(LocusInteropError::NotFound { .. })
    ));
    let missing_outcome = "NotFound";

    let document_cleanup = be.delete(&format!("/knowledge/documents/{document_id}"));
    assert!(matches!(document_cleanup, 200 | 202 | 204 | 404));
    records_cleanup.assert_cleanup();
    drop(records_cleanup);
    be.assert_cleanup();
    locus_sql_output(&format!(
        "DO $$ BEGIN \
         IF EXISTS (SELECT 1 FROM work_packets WHERE wp_id = {wp}) \
            OR EXISTS (SELECT 1 FROM micro_tasks WHERE wp_id = {wp}) \
         THEN RAISE EXCEPTION 'MT-068 fixture residue remains'; END IF; \
         END $$;",
        wp = sql_literal(&wp_id)
    ))
    .expect("MT-068 exact Locus rows are absent after cleanup");
    assert_no_local_artifact_dir();

    let evidence_path = artifact_dir.join("mt068-locus-canonical-argus.json");
    std::fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_id": "handshake.mt068-locus-canonical-argus-proof.v1",
            "test": "resolve_locus_ref_against_real_pg_live",
            "status": "PASS",
            "recorded_at": chrono::Utc::now().to_rfc3339(),
            "source": {
                "source_sha": source_sha,
                "runtime_source_tree": runtime_source_tree,
                "proof_source_blob": proof_source_blobs
                    .get("src/frontend/handshake_native/tests/test_locus_interop.rs")
                    .cloned(),
                "proof_source_blobs": proof_source_blobs,
                "relevant_source_clean": true,
                "transitive_runtime_source_clean": true,
                "global_worktree_clean": false,
                "known_unrelated_dirty_paths": ["AGENTS.md", "CLAUDE.md"],
            },
            "backend": {
                "postgresql_eventledger": true,
                "old_base": old_backend_base,
                "new_base": new_backend_base,
                "same_listener_after_restart": old_backend_base == new_backend_base,
                "post_restart_document_readback": true,
            },
            "persisted_fixture": {
                "workspace_id": ws,
                "work_packet_id": wp_id,
                "microtask_id": mt_id,
                "document_id": document_id,
                "work_packet_uri": wp_uri,
                "microtask_uri": mt_uri,
                "missing_uri": missing_uri,
                "save_receipt_event_id": save_receipt_event_id,
                "rich_document_attrs_survived_restart": true,
            },
            "forward_resolution": {
                "work_packet": {
                    "id": wp_record.id,
                    "title": wp_record.title,
                    "status": wp_record.status,
                },
                "microtask": {
                    "id": mt_record.id,
                    "title": mt_record.title,
                    "status": mt_record.status,
                }
            },
            "reverse_lookup_matching_document_counts": reverse_lookup_counts,
            "missing_and_stale": {
                "live_route_outcome": missing_outcome,
                "mounted_unresolved_chip": true,
                "unavailable_route_negative_test": "ac002_resolve_locus_ref_route_absent_is_typed_blocker",
                "fabricated_record": false,
            },
            "canonical_argus_state_matrix": argus_state_matrix,
            "cleanup": {
                "workspace_absent": true,
                "persisted_document_absent": true,
                "work_packet_rows_zero": true,
                "microtask_rows_zero": true,
                "runtime_quiescent": true,
            }
        }))
        .expect("serialize MT-068 canonical Argus evidence"),
    )
    .expect("write MT-068 canonical Argus evidence");
    assert!(evidence_path.is_file());
    println!(
        "AC-002/003/004/005/006 LIVE OK: canonical WP {wp_id} + MT {mt_id} resolved after restart, saved/reloaded attrs, persisted reverse lookup returned {document_id} once per ref, canonical Argus steered both targets, and cleanup preceded PASS evidence; evidence={}",
        evidence_path.display()
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-003 / PT-003 — clicking a locus-ref chip dispatches open-locus-ref through the MT-030/MT-031 seam.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac003_click_locus_ref_chip_dispatches_open_locus_ref() {
    // Render a rich editor over a doc carrying a locus-ref chip. The chip's stable author_id is
    // `locus-ref-chip-{kind}-{id}` — the kittest targets it by that id.
    let locus_uri = "locus://wp/WP-KERNEL-012";
    let state = std::sync::Arc::new(std::sync::Mutex::new(RichEditorState::new(
        doc_with_locus_ref(locus_uri, "WP-KERNEL-012", true),
    )));
    let state_ck = std::sync::Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(std::sync::Arc::clone(&state)).show(ui);
        });
    harness.run();

    // The chip is addressable by the contract author_id `locus-ref-chip-wp-WP-KERNEL-012`.
    let chip_id = locus_ref_chip_author_id(locus_uri);
    assert_eq!(
        chip_id, "locus-ref-chip-wp-WP-KERNEL-012",
        "AC-008: the contract author_id shape"
    );
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&chip_id),
        "AC-003/AC-008: the locus-ref chip is addressable by `{chip_id}`; present ids: {ids:?}"
    );

    // Click the chip; the editor enqueues a WikilinkActivated{ref_kind=locus,...} event the host drains.
    let chip = harness.get_by(|n| n.author_id() == Some(chip_id.as_str()));
    chip.click();
    harness.run();

    let event = {
        let st = state_ck.lock().unwrap();
        st.pending_events.iter().find_map(|e| match e {
            EditorEvent::WikilinkActivated {
                ref_kind,
                ref_value,
                ..
            } if ref_kind == "locus" => Some((ref_kind.clone(), ref_value.clone())),
            _ => None,
        })
    };
    let (ref_kind, ref_value) = event
        .expect("AC-003: clicking the locus-ref chip enqueues a locus WikilinkActivated event");
    assert_eq!(ref_kind, "locus");
    assert_eq!(
        ref_value, locus_uri,
        "AC-003: the event carries the locus ref"
    );

    // The bridge stages the canonical ORIGINAL-CASE ref on the bus and dispatches `open-locus-ref` (no
    // new channel). The normalized value remains the lookup/search key only.
    let ctx = egui::Context::default();
    let mut bus = InteractionBus::new();
    bus.register_open_locus_ref_command();
    let evt = EditorEvent::WikilinkActivated {
        ref_kind,
        ref_value: ref_value.clone(),
        resolved: true,
    };
    let staged = dispatch_locus_ref_open(&ctx, &mut bus, &evt);
    assert_eq!(
        staged.as_deref(),
        Some("locus://wp/WP-KERNEL-012"),
        "AC-003: the bridge stages the original-case canonical navigation identity"
    );
    assert_eq!(
        bus.take_pending_locus_ref().as_deref(),
        Some("locus://wp/WP-KERNEL-012"),
        "AC-003: `open-locus-ref` preserved the uppercase work-unit identity on the nav seam"
    );
    println!("AC-003/PT-003 OK: clicked {chip_id} -> open-locus-ref staged locus://wp/WP-KERNEL-012 ({CMD_OPEN_LOCUS_REF})");
}

#[test]
fn ac003_generic_app_bus_drain_preserves_uppercase_locus_identity() {
    let app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    let event = EditorEvent::WikilinkActivated {
        ref_kind: LOCUS_REF_KIND.to_owned(),
        ref_value: "wp/WP-KERNEL-CaseID".to_owned(),
        resolved: true,
    };
    let staged = {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        InteractionBus::with_try_lock(&bus, |bus| {
            bus.register_open_locus_ref_command();
            dispatch_locus_ref_open(&harness.ctx, bus, &event)
        })
        .flatten()
    };
    assert_eq!(
        staged.as_deref(),
        Some("locus://wp/WP-KERNEL-CaseID"),
        "the command bridge must not stage the lowercase lookup key"
    );

    // Drive the ordinary per-frame pending-bus drain, not the direct ShellNavigator test seam.
    harness.run_steps(2);
    let active = harness.state().active_pane().cloned().expect("active pane");
    let tab = harness
        .state()
        .tab_bar_states()
        .get(&active)
        .and_then(|bar| bar.tabs.get(bar.active_index))
        .expect("generic Locus drain opened an active tab");
    assert_eq!(
        tab.content_id.as_deref(),
        Some("WP:WP-KERNEL-CaseID"),
        "the generic app drain must navigate with the original-case record identity"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-004 / PT-004 — reverse lookup lists documents referencing a given WP/MT (keyed on the normalized
// ref, de-duplicated on (document_id, block_id)).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac004_reverse_lookup_lists_referencing_documents() {
    // Seed a doc/code block whose content carries `locus://mt/MT-034`. The reverse-lookup mock returns it
    // for the search; find_documents_referencing keys on the NORMALIZED ref and lists it, de-duplicated.
    // The same block returned under two content types (note + journal) is listed ONCE (dedup proof).
    let hits = vec![
        hit(
            "DOC-7",
            Some("Design notes"),
            "note",
            "tracks <mark>locus://mt/MT-034</mark> here",
        ),
        // The SAME block id under the journal content-type query (the find_notes search runs once per
        // content type) — must de-duplicate to a single DocumentRef.
        hit(
            "DOC-7",
            Some("Design notes"),
            "journal",
            "again locus://mt/MT-034",
        ),
        hit("DOC-9", Some("Plan"), "note", "also locus://mt/MT-034"),
        hit(
            "DOC-10",
            Some("Plain-text false positive"),
            "note",
            "mentions locus://mt/MT-034 without a structured link",
        ),
    ];
    let backend = Arc::new(
        CountingReverseLookup::new(hits)
            .with_locus_contents(&["DOC-7", "DOC-9"], "locus://mt/MT-034")
            .with_content(
                "DOC-10",
                serde_json::json!({
                    "type": "doc",
                    "content": [{
                        "type": "paragraph",
                        "content": [{
                            "type": "text",
                            "text": "plain locus://mt/MT-034 mention"
                        }]
                    }]
                }),
            ),
    );
    let backend_dyn: Arc<dyn FindNotesSearch> = backend.clone();
    let svc = LocusInteropService::with_base_url("http://unused", "WS-1", backend_dyn);

    let mt = parse_locus_ref("locus://mt/MT-034").unwrap();
    let docs = rt()
        .block_on(async { svc.find_documents_referencing(&mt).await })
        .expect("AC-004: reverse lookup returns the referencing docs");

    let ids: Vec<&str> = docs.iter().map(|d| d.document_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["DOC-7", "DOC-9"],
        "AC-004: lists referencing docs, de-duplicated on (doc,block)"
    );
    assert!(
        docs.iter().all(|d| d.block_id.is_some()),
        "AC-004: each DocumentRef carries its block id (mirrors NoteRef)"
    );

    // RISK-001: the reverse lookup keyed on the NORMALIZED `locus://` ref value (the SINGLE shared key the
    // resolution direction also uses) — proven by the recorded query.
    let recorded = backend.last_query.lock().unwrap().clone();
    assert_eq!(
        recorded.as_deref(),
        Some("locus://mt/mt-034"),
        "AC-004/RISK-001: the reverse lookup is keyed on the normalized locus:// ref (the single key)"
    );
    assert_eq!(
        mt.normalized, "locus://mt/mt-034",
        "the key matches the parsed normalized form"
    );
    println!("AC-004/PT-004 OK: find_documents_referencing(MT-034) -> [DOC-7, DOC-9] keyed on locus://mt/mt-034, deduped");
}

#[test]
fn ac004_reverse_lookup_unreadable_candidate_fails_closed() {
    let backend: Arc<dyn FindNotesSearch> = Arc::new(CountingReverseLookup::new(vec![hit(
        "DOC-UNREADABLE",
        Some("Unreadable candidate"),
        "note",
        "locus://mt/MT-034",
    )]));
    let svc = LocusInteropService::with_base_url("http://unused", "WS-1", backend);
    let mt = parse_locus_ref("locus://mt/MT-034").unwrap();
    let error = rt()
        .block_on(async { svc.find_documents_referencing(&mt).await })
        .expect_err("an unreadable candidate makes the exact reverse lookup unknowable");
    assert!(
        matches!(error, LocusInteropError::ReverseLookup(_)),
        "unreadable candidates fail closed through the typed reverse-lookup error: {error:?}"
    );
}

#[test]
fn ac004_reverse_lookup_empty_is_not_an_error() {
    // No referencing docs -> an honest empty list (the "no documents reference this" state), not an error.
    let backend: Arc<dyn FindNotesSearch> = Arc::new(CountingReverseLookup::new(vec![]));
    let svc = LocusInteropService::with_base_url("http://unused", "WS-1", backend);
    let mt = parse_locus_ref("locus://mt/MT-999").unwrap();
    let docs = rt()
        .block_on(async { svc.find_documents_referencing(&mt).await })
        .unwrap();
    assert!(
        docs.is_empty(),
        "AC-004: zero references is an empty list, not an error"
    );

    // An empty workspace is the NoWorkspace error (a reverse lookup needs a workspace).
    let svc2 = LocusInteropService::with_base_url(
        "http://unused",
        "",
        Arc::new(CountingReverseLookup::new(vec![])),
    );
    let err = rt()
        .block_on(async { svc2.find_documents_referencing(&mt).await })
        .unwrap_err();
    assert_eq!(
        err,
        LocusInteropError::NoWorkspace,
        "AC-004: no workspace -> NoWorkspace"
    );
    println!("AC-004 OK: empty reverse lookup is Ok([]); no workspace -> NoWorkspace");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-005 / PT-005 — the missing Locus READ endpoint raises the typed blocker; the chip renders greyed
// (no panic), DISTINCT from a live-404 record-not-found (the two failure modes are not conflated).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac005_typed_blocker_distinct_from_live_404_and_chip_greys() {
    // The typed blocker (endpoint absent) is DISTINCT from a live-404 (record not found) — RISK-003/MC-003.
    let blocker = LocusInteropError::LocusReadApiUnavailable {
        endpoint: "/workspaces/WS-1/locus/microtasks/MT-034".into(),
    };
    assert!(
        blocker.is_read_api_unavailable(),
        "AC-005: the absent-endpoint blocker is the typed blocker"
    );
    assert!(
        !blocker.is_record_not_found(),
        "AC-005: it is NOT a record-not-found 404"
    );
    let not_found = LocusInteropError::NotFound { id: "MT-9".into() };
    assert!(
        not_found.is_record_not_found(),
        "AC-005: a live 404 is record-not-found"
    );
    assert!(
        !not_found.is_read_api_unavailable(),
        "AC-005: a 404 is NOT the typed blocker (not conflated)"
    );
    assert!(
        blocker
            .unavailable_tooltip()
            .contains("/locus/microtasks/MT-034"),
        "AC-005: the greyed-chip tooltip names the missing endpoint"
    );

    // The chip renders GREYED (the error affordance) and does NOT panic when the record is unresolved
    // (the designed unavailable/unresolved state -> resolved=false on the hsLink atom).
    let unresolved = HsLinkNode {
        ref_kind: LOCUS_REF_KIND.into(),
        ref_value: "locus://mt/MT-034".into(),
        label: "MT-034".into(),
        resolved: false,
        provenance: None,
    };
    let label = chip_label(&unresolved);
    assert!(
        label.contains("unresolved"),
        "AC-005: an unavailable locus chip reads as unresolved"
    );
    let palette = HsTheme::Dark.palette();
    let (bg, fg) = chip_colors(&unresolved, &palette);
    assert_eq!(
        bg, palette.error_bg,
        "AC-005: a greyed chip uses the theme error background (no Color32 literal)"
    );
    assert_eq!(fg, palette.error_text);

    // It RENDERS in a live editor without panicking (the doc carries the unavailable locus ref).
    let doc = doc_with_locus_ref("locus://mt/MT-034", "MT-034", false);
    let state = std::sync::Arc::new(std::sync::Mutex::new(RichEditorState::new(doc)));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 400.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(std::sync::Arc::clone(&state)).show(ui);
        });
    harness.run(); // no panic == pass
    let ids = author_ids(&harness);
    assert!(
        ids.contains("locus-ref-chip-mt-MT-034"),
        "AC-005: the greyed (unavailable) chip is still addressable; got {ids:?}"
    );
    println!("AC-005/PT-005 OK: LocusReadApiUnavailable distinct from 404; greyed chip ('{label}') renders, no panic");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-006 / PT-006 — the locus_ref hsLink node survives save+reload with attrs intact; a live-404 greys
// the chip without panic.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac006_locus_ref_node_round_trips_content_json() {
    // The authored locus atom: ref_kind="locus", ref_value=locus://wp/WP-KERNEL-012. It is the SAME hsLink
    // node the backend persists (a declared allowed node type, NOT an invented node), so save->reload
    // preserves the ref (AC-006). The {kind,id,normalized} are derivable from the round-tripped ref_value.
    let doc = doc_with_locus_ref("locus://wp/WP-KERNEL-012", "WP-KERNEL-012", true);
    let json = to_json_string(&doc).expect("serialize");
    let back = from_json_string(&json).expect("reload");
    assert_eq!(
        doc, back,
        "AC-006: the locus-ref doc round-trips through DocJson unchanged"
    );

    // The hsLink node carries the locus ref, type=hsLink (NOT an invented `locus_ref` node — AC-007).
    let v = to_content_json_value(&doc);
    let link = &v["content"][0]["content"][1];
    assert_eq!(
        link["type"], "hsLink",
        "AC-006/AC-007: a locus ref is an hsLink atom, never a `locus_ref` node"
    );
    assert_eq!(link["attrs"]["refKind"], "locus");
    assert_eq!(
        link["attrs"]["refValue"], "locus://wp/WP-KERNEL-012",
        "AC-006: the locus ref is preserved"
    );
    assert_eq!(link["attrs"]["label"], "WP-KERNEL-012");

    // The {kind,id,normalized} the contract names survive because they are derivable from the ref_value
    // (the single source of truth) — re-parse the round-tripped ref_value and assert the triple.
    let ref_value = link["attrs"]["refValue"].as_str().unwrap();
    let reparsed = parse_locus_ref(ref_value).expect("the round-tripped ref re-parses");
    assert_eq!(
        reparsed.kind,
        LocusRefKind::WorkPacket,
        "AC-006: kind survives (derivable from the ref)"
    );
    assert_eq!(reparsed.id, "WP-KERNEL-012", "AC-006: id survives");
    assert_eq!(
        reparsed.normalized, "locus://wp/wp-kernel-012",
        "AC-006: normalized survives"
    );
    println!("AC-006/PT-006 OK: locus hsLink atom round-trips content_json; {{kind,id,normalized}} re-derive intact");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-007 / PT-007 — grep proof: MT-032 normalizer reused, MT-034 CrossRef node/chip + search reused, the
// open-locus-ref nav via the existing seam, AccessKit ids via the WP-011 registry, no duplicates.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac007_reuses_mt032_normalizer_and_mt034_machinery_no_duplicates() {
    let interop_src = include_str!("../src/interop/locus_interop.rs");

    // (1) REUSES the MT-032/MT-015 normalizer — no second normalizer is defined (RISK-001/MC-001).
    assert!(
        interop_src.contains("crate::rich_editor::wikilinks::resolver::normalize_target"),
        "AC-007: locus_interop must reuse the MT-015/MT-032 normalize_target (single key), not define a new one"
    );
    assert!(
        !interop_src.contains("fn normalize_target"),
        "AC-007: locus_interop must NOT define its own normalize_target (no second normalizer)"
    );

    // (2) REUSES the MT-034 exhaustive candidate search machinery for reverse lookup — not a forked scan.
    assert!(
        interop_src.contains("find_all_note_candidates_with")
            && interop_src.contains("FindNotesSearch"),
        "AC-007: reverse lookup must reuse MT-034 bounded candidate pagination / FindNotesSearch"
    );
    assert!(
        interop_src.contains("percent_encode_symbol"),
        "AC-007: the URL id encoding must reuse the MT-034 percent_encode_symbol (no second encoder)"
    );

    // The locus_ref node is the EXISTING hsLink atom (the `locus:` prefix in the shared wikilink table),
    // NOT an invented node — proven by the parser table carrying the prefix and NO `locus_ref` node type.
    let parser_src = include_str!("../src/rich_editor/wikilinks/parser.rs");
    assert!(
        parser_src.contains("(\"locus\", \"locus\")"),
        "AC-007: the `locus:` prefix is registered in the SHARED wikilink prefix table (the hsLink atom)"
    );
    let node_src = include_str!("../src/rich_editor/document_model/node.rs");
    assert!(
        !node_src.to_lowercase().contains("struct locusrefnode"),
        "AC-007: there is NO invented LocusRefNode — the locus ref is the existing HsLinkNode atom"
    );

    // (3) `open-locus-ref` routes through the EXISTING command/nav seam (the InteractionBus stage-then-
    // dispatch), NOT a new navigation channel (RISK-007/MC-007).
    let bus_src = include_str!("../src/interop/interaction_bus.rs");
    assert!(
        bus_src.contains("CMD_OPEN_LOCUS_REF") && bus_src.contains("pending_locus_ref"),
        "AC-007: open-locus-ref uses the existing InteractionBus stage-then-dispatch seam (no new channel)"
    );
    assert_eq!(
        CMD_OPEN_LOCUS_REF, "interop.open-locus-ref",
        "AC-007: the contract command id"
    );

    // (4) AccessKit base ids are derived via the chip helper (registered through the WP-011
    // accessibility surface like every other chip — the renderer's accesskit_node_builder path).
    // The first occurrence keeps this deterministic `(kind,id)` base; the occurrence helper appends
    // stable document paths to repeats.
    assert_eq!(
        locus_ref_chip_author_id("locus://wp/WP-KERNEL-012"),
        "locus-ref-chip-wp-WP-KERNEL-012"
    );
    assert_eq!(
        locus_ref_chip_author_id("locus://mt/MT-034"),
        "locus-ref-chip-mt-MT-034"
    );
    // The same (kind,id) yields the SAME base; distinct work units yield distinct bases.
    assert_eq!(
        locus_ref_chip_author_id("locus://wp/WP-KERNEL-012"),
        locus_ref_chip_author_id("locus://wp/WP-KERNEL-012"),
        "AC-008: the first-occurrence base id is deterministic by (kind,id)"
    );
    assert_eq!(
        locus_ref_chip_occurrence_author_id("locus://wp/WP-KERNEL-012", &[4, 2], 1),
        "locus-ref-chip-wp-WP-KERNEL-012--path-4-2",
        "AC-008: a repeated occurrence is uniquely addressable by stable document path"
    );
    assert_ne!(
        locus_ref_chip_author_id("locus://wp/WP-KERNEL-012"),
        locus_ref_chip_author_id("locus://mt/MT-034"),
        "AC-008: distinct work units -> distinct chip ids"
    );
    println!("AC-007/PT-007 OK: MT-032 normalizer + MT-034 node/chip/search reused; open-locus-ref via existing seam; no duplicates");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-008 — AccessKit dump: the locus chip (Link) + a reverse-lookup row (ListItem) present with roles,
// no duplicate author_id. (+ a best-effort screenshot to the external root.)
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac008_accesskit_ids_present_with_roles_no_duplicates() {
    // Render a doc with TWO distinct locus chips (wp + mt) + render a reverse-lookup row leaf so both
    // author_id shapes appear in the live tree. The chips come from the rich editor; the refby row is the
    // contract's `locus-refby-{document_id}` ListItem rendered via a small leaf the downstream Locus panel
    // (MT-073) reuses — here proven addressable + correctly-roled.
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::new("refs ")));
    let mut wp = HsLinkNode::new(LOCUS_REF_KIND, "locus://wp/WP-KERNEL-012", "WP-KERNEL-012");
    wp.resolved = true;
    para.children.push(Child::HsLink(wp));
    para.children.push(Child::Text(TextLeaf::new(" and ")));
    let mut mt = HsLinkNode::new(LOCUS_REF_KIND, "locus://mt/MT-034", "MT-034");
    mt.resolved = true;
    para.children.push(Child::HsLink(mt));
    para.children.push(Child::Text(TextLeaf::new("")));
    let doc = BlockNode::doc(vec![para]);

    let docref = DocumentRef {
        document_id: "DOC-7".to_owned(),
        document_title: "Design notes".to_owned(),
        block_id: Some("BLK-1".to_owned()),
        excerpt: "tracks locus://mt/MT-034".to_owned(),
    };

    let state = std::sync::Arc::new(std::sync::Mutex::new(RichEditorState::new(doc)));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(820.0, 420.0))
        .wgpu()
        .build_ui(move |ui| {
            RichEditorWidget::new(std::sync::Arc::clone(&state)).show(ui);
            // Render the reverse-lookup row leaf (the contract's `locus-refby-{document_id}` ListItem) so
            // the AccessKit dump covers it. A Button-like clickable row, Role::ListItem.
            ui.separator();
            let refby_id = format!("locus-refby-{}", docref.document_id);
            let resp = ui.button(format!("{} — {}", docref.document_title, docref.excerpt));
            ui.ctx().accesskit_node_builder(resp.id, move |node| {
                node.set_role(egui::accesskit::Role::ListItem);
                node.set_author_id(refby_id.clone());
                node.set_label("Referencing document".to_owned());
            });
        });
    harness.run();
    harness.run();

    let root = harness.root();
    // The two locus chips are present with the contract author_ids + the chip role (Link).
    assert_eq!(
        role_of(&root, "locus-ref-chip-wp-WP-KERNEL-012").as_deref(),
        Some("Link"),
        "AC-008: locus-ref-chip-wp-WP-KERNEL-012 is a Role::Link (the chip role)"
    );
    assert_eq!(
        role_of(&root, "locus-ref-chip-mt-MT-034").as_deref(),
        Some("Link"),
        "AC-008: locus-ref-chip-mt-MT-034 is a Role::Link"
    );
    // The reverse-lookup row is the contract `locus-refby-{document_id}` ListItem.
    assert_eq!(
        role_of(&root, "locus-refby-DOC-7").as_deref(),
        Some("ListItem"),
        "AC-008: locus-refby-DOC-7 is a Role::ListItem"
    );

    // No duplicate author_id in the whole live tree (RISK-008/MC-008): collect every author_id and assert
    // each appears exactly once.
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for node in root.children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            *counts.entry(a.to_owned()).or_default() += 1;
        }
    }
    for id in [
        "locus-ref-chip-wp-WP-KERNEL-012",
        "locus-ref-chip-mt-MT-034",
        "locus-refby-DOC-7",
    ] {
        assert_eq!(
            counts.get(id).copied(),
            Some(1),
            "AC-008: author_id '{id}' must appear exactly once (no collision)"
        );
    }

    println!(
        "AC-008 accesskit dump: {{\"locus-ref-chip-wp-WP-KERNEL-012\":\"{}\",\"locus-ref-chip-mt-MT-034\":\"{}\",\"locus-refby-DOC-7\":\"{}\"}}",
        role_of(&root, "locus-ref-chip-wp-WP-KERNEL-012").unwrap_or_default(),
        role_of(&root, "locus-ref-chip-mt-MT-034").unwrap_or_default(),
        role_of(&root, "locus-refby-DOC-7").unwrap_or_default()
    );

    // Screenshot to the EXTERNAL root ONLY (best-effort pixel readback).
    if let Ok(image) = harness.render() {
        let ext_dir = external_artifact_dir("wp-kernel-012-mt-068");
        let _ = std::fs::create_dir_all(&ext_dir);
        let ext_path = ext_dir.join("MT-068-locus-cross-ref-chips.png");
        let saved = image.save(&ext_path).is_ok();
        println!(
            "AC-008 screenshot: {}x{} saved_ext={saved} ({})",
            image.width(),
            image.height(),
            ext_path.display()
        );
    } else {
        println!(
            "AC-008 screenshot: GPU readback unavailable on this host (structural proof stands)"
        );
    }

    assert_no_local_artifact_dir();
}

#[test]
fn ac008_repeated_locus_refs_have_unique_path_stable_author_ids_after_reflow() {
    let locus_uri = "locus://mt/MT-RepeatCase";
    let make_link = || {
        let mut link = HsLinkNode::new(LOCUS_REF_KIND, locus_uri, "MT-RepeatCase");
        link.resolved = true;
        Child::HsLink(link)
    };

    let mut first = BlockNode::new(NodeKind::Paragraph);
    first.children.push(Child::Text(TextLeaf::new(
        "A long prefix that wraps at the constrained viewport before ",
    )));
    first.children.push(make_link()); // path [0, 1] — first occurrence keeps the base id.
    first.children.push(Child::Text(TextLeaf::new(
        " and another long segment before the repeated ref ",
    )));
    first.children.push(make_link()); // path [0, 3] — repeated occurrence gets a path suffix.
    first.children.push(Child::Text(TextLeaf::new(".")));

    let mut second = BlockNode::new(NodeKind::Paragraph);
    second
        .children
        .push(Child::Text(TextLeaf::new("A later block repeats ")));
    second.children.push(make_link()); // path [1, 1] — another stable path suffix.
    second.children.push(Child::Text(TextLeaf::new(".")));

    let state = Arc::new(std::sync::Mutex::new(RichEditorState::new(BlockNode::doc(
        vec![first, second],
    ))));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, 440.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state)).show(ui);
        });
    harness.run_steps(2);

    let expected = [
        locus_ref_chip_occurrence_author_id(locus_uri, &[0, 1], 0),
        locus_ref_chip_occurrence_author_id(locus_uri, &[0, 3], 1),
        locus_ref_chip_occurrence_author_id(locus_uri, &[1, 1], 2),
    ];
    assert_eq!(
        expected[0], "locus-ref-chip-mt-MT-RepeatCase",
        "the first occurrence preserves the unsuffixed MT-068 contract id"
    );
    assert_eq!(
        expected[1], "locus-ref-chip-mt-MT-RepeatCase--path-0-3",
        "a repeat is disambiguated by its stable document path"
    );
    assert_eq!(expected[2], "locus-ref-chip-mt-MT-RepeatCase--path-1-1");

    let collect = |harness: &Harness<'_, ()>| {
        let mut ids = harness
            .root()
            .children_recursive()
            .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
            .filter(|id| id.starts_with("locus-ref-chip-mt-MT-RepeatCase"))
            .collect::<Vec<_>>();
        ids.sort();
        ids
    };
    let mut expected_sorted = expected.to_vec();
    expected_sorted.sort();
    let wide_ids = collect(&harness);
    assert_eq!(
        wide_ids, expected_sorted,
        "all repeated refs must expose unique deterministic author_ids"
    );

    // Force different wrapping without changing the document. Identity must follow the atom path, not
    // the chip's painted coordinates or line number.
    harness.set_size(egui::vec2(330.0, 440.0));
    harness.run_steps(2);
    let narrow_ids = collect(&harness);
    assert_eq!(
        narrow_ids, wide_ids,
        "viewport reflow must not change repeated Locus AccessKit identities"
    );
}

#[test]
fn ac008_raw_path_suffix_id_cannot_collide_with_repeated_ref_after_reflow() {
    let repeated_uri = "locus://mt/MT-X";
    let adversarial_uri = "locus://mt/MT-X--path-0-3";
    let adversarial_view_uri = "locus://mt/MT-X--view-document-70616e652d62";
    let make_link = |uri: &str| {
        let mut link = HsLinkNode::new(LOCUS_REF_KIND, uri, uri);
        link.resolved = true;
        Child::HsLink(link)
    };

    let mut paragraph = BlockNode::new(NodeKind::Paragraph);
    paragraph.children.push(Child::Text(TextLeaf::new(
        "A long prefix that changes wrapping before ",
    )));
    paragraph.children.push(make_link(repeated_uri)); // [0, 1], canonical unsuffixed first occurrence.
    paragraph
        .children
        .push(Child::Text(TextLeaf::new(" and the same ref again ")));
    paragraph.children.push(make_link(repeated_uri)); // [0, 3], author suffix is `--path-0-3`.
    paragraph.children.push(Child::Text(TextLeaf::new(
        " beside an authored id containing that exact suffix ",
    )));
    paragraph.children.push(make_link(adversarial_uri)); // [0, 5], reserved token must be escaped.
    paragraph.children.push(Child::Text(TextLeaf::new(
        " and an authored id containing the secondary-pane suffix ",
    )));
    paragraph.children.push(make_link(adversarial_view_uri)); // [0, 7], view token must be escaped.
    paragraph.children.push(Child::Text(TextLeaf::new(".")));

    let state = Arc::new(std::sync::Mutex::new(RichEditorState::new(BlockNode::doc(
        vec![paragraph],
    ))));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, 420.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state)).show(ui);
        });
    harness.run_steps(2);

    let expected = [
        "locus-ref-chip-mt-MT-X".to_owned(),
        "locus-ref-chip-mt-MT-X--path-0-3".to_owned(),
        "locus-ref-chip-mt-MT-X%2D%2Dpath%2D0-3".to_owned(),
        "locus-ref-chip-mt-MT-X%2D%2Dview%2Ddocument-70616e652d62".to_owned(),
    ];
    assert_eq!(
        locus_ref_chip_author_id(repeated_uri),
        expected[0],
        "collision-safe canonical ids retain the existing unsuffixed contract form"
    );
    assert_eq!(
        locus_ref_chip_occurrence_author_id(repeated_uri, &[0, 3], 1),
        expected[1],
        "the repeated MT-X occurrence keeps the stable path suffix"
    );
    assert_eq!(
        locus_ref_chip_author_id(adversarial_uri),
        expected[2],
        "an authored id containing the reserved suffix token is injectively escaped"
    );
    assert_ne!(
        locus_ref_chip_author_id("locus://mt/MT-X%2D%2Dpath%2D0-3"),
        expected[2],
        "a literal escape spelling cannot alias the reserved-token encoding"
    );
    assert_eq!(
        locus_ref_chip_author_id(adversarial_view_uri),
        expected[3],
        "an authored id containing the secondary-pane suffix token is injectively escaped"
    );
    assert_ne!(
        locus_ref_chip_author_id("locus://mt/MT-X%2D%2Dview%2Ddocument-70616e652d62"),
        expected[3],
        "a literal pane-view escape spelling cannot alias the reserved-token encoding"
    );

    let collect = |harness: &Harness<'_, ()>| {
        let mut ids = harness
            .root()
            .children_recursive()
            .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
            .filter(|id| id.starts_with("locus-ref-chip-mt-MT-X"))
            .collect::<Vec<_>>();
        ids.sort();
        ids
    };
    let mut expected_sorted = expected.to_vec();
    expected_sorted.sort();
    let wide_ids = collect(&harness);
    assert_eq!(
        wide_ids, expected_sorted,
        "the formerly colliding authored and occurrence identities must both be present exactly once"
    );

    harness.set_size(egui::vec2(320.0, 420.0));
    harness.run_steps(2);
    assert_eq!(
        collect(&harness),
        wide_ids,
        "injective identities remain stable when viewport reflow changes chip coordinates"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-009 — editor interop remains READ-only: GET-only, no SQLite, no Locus mutation.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac009_read_only_no_sqlite_no_mutation() {
    // Strip line-comments so the gate checks ACTUAL CODE, not the doc comments that explain the rules.
    fn code_only(src: &str) -> String {
        src.lines()
            .map(|line| match line.find("//") {
                Some(idx) => &line[..idx],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    let interop_code = code_only(include_str!("../src/interop/locus_interop.rs"));

    // No DB-driver usage (PostgreSQL/EventLedger is the only durable authority — AC-009 / RISK-006).
    for store in ["sqlite", "rusqlite", "diesel", "Sqlite", "SQLite", "sqlx"] {
        assert!(
            !interop_code.contains(store),
            "AC-009: locus_interop code must not reference '{store}' (PostgreSQL/EventLedger only, no SQLite)"
        );
    }
    // The Locus reads are GET-only — no write verbs in code (READ + REFERENCE ONLY, no mutation/transition).
    for verb in [".post(", ".put(", ".delete(", ".patch("] {
        assert!(
            !interop_code.contains(verb),
            "AC-009: locus_interop reads must be GET-only — found write verb '{verb}' (no Locus mutation)"
        );
    }
    // It reuses the shared backend pool + base url (no second HTTP stack), and issues a GET.
    let interop_src = include_str!("../src/interop/locus_interop.rs");
    assert!(
        interop_src.contains("shared_http_client") && interop_src.contains("BACKEND_BASE_URL"),
        "AC-009: the Locus reads must reuse the shared backend_client pool + base url (no second stack)"
    );
    assert!(
        interop_src.contains(".get(&url)"),
        "AC-009: the Locus record read must issue a GET via the reqwest builder"
    );
    println!("AC-009 OK: GET-only editor interop, no sqlite/rusqlite/diesel, shared client reused, no Locus mutation");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Hygiene (CX-212E) + AC-010 — no repo-local artifact dir; this file is the AC-010 suite.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn no_local_artifact_dir_under_crate() {
    assert_no_local_artifact_dir();
    println!("CX-212E: no repo-local test_output/ or tests/screenshots/ dir under the crate");
}

#[test]
fn parses_locus_wikilink_to_locus_hs_link() {
    // The `[[locus:wp/WP-KERNEL-012]]` authoring form parses to a `locus` hsLink atom (the shared wikilink
    // machinery), so the SAME chip path renders it (AC-007). is_locus_ref recognizes the atom.
    let parsed = parse_wikilink("[[locus:wp/WP-KERNEL-012]]").expect("a valid locus wikilink");
    let link = parsed.to_hs_link();
    assert_eq!(
        link.ref_kind, "locus",
        "the locus: prefix is a `locus` ref kind"
    );
    assert_eq!(link.ref_value, "wp/WP-KERNEL-012");
    assert!(link.resolved, "the locus: prefix is a known resolved kind");
    assert!(is_locus_ref(&link), "is_locus_ref recognizes the atom");
    println!("AC-007 OK: [[locus:wp/WP-KERNEL-012]] -> hsLink(locus, wp/WP-KERNEL-012)");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// REAL-AUTHORING END-TO-END (closes the tautology the adversarial review found): every other chip /
// dispatch / author_id proof hand-builds the hsLink atom with the FULL URI `ref_value`
// (`locus://wp/WP-KERNEL-012`) — a value the wikilink parser NEVER emits. The authoring path produces
// the prefix-stripped `ref_value="wp/WP-KERNEL-012"`. This test drives that EXACT value (from the real
// `parse_wikilink(...).to_hs_link()`) through `locus_ref_chip_author_id`, `chip_label`, and
// `dispatch_locus_ref_open`, asserting the contract author_id (AC-008), the short label (the work-unit
// id, not the raw `wp/...` form), the normalized lookup key, and the original-case canonical navigation
// payload all hold for the form a user actually types.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn authored_locus_wikilink_drives_chip_helpers_with_real_ref_value() {
    // (1) Drive the REAL authoring path: parse the wikilink + materialize the hsLink atom exactly as the
    // editor stores it on autocomplete-confirm. The `ref_value` is the prefix-stripped `wp/WP-KERNEL-012`.
    let link = parse_wikilink("[[locus:wp/WP-KERNEL-012]]")
        .expect("a valid locus wikilink")
        .to_hs_link();
    assert_eq!(
        link.ref_value, "wp/WP-KERNEL-012",
        "the authoring path stores the prefix-stripped kind/id form (NOT the full locus:// URI)"
    );

    // (2) parse_locus_ref must accept the prefix-stripped authoring form and normalize it to the SAME
    // canonical key as the URI form (the single-shared-key invariant for the form a user actually types).
    let parsed = parse_locus_ref(&link.ref_value).expect(
        "parse_locus_ref accepts the wikilink authoring form `wp/WP-KERNEL-012` (the must-fix)",
    );
    assert_eq!(parsed.kind, LocusRefKind::WorkPacket, "wp/ -> WorkPacket");
    assert_eq!(parsed.id, "WP-KERNEL-012", "the id keeps its original case");
    assert_eq!(
        parsed.normalized, "locus://wp/wp-kernel-012",
        "the authored form normalizes to the SAME single key as locus://wp/WP-KERNEL-012 (RISK-001)"
    );
    assert_eq!(
        parsed.normalized,
        normalize_locus_id(LocusRefKind::WorkPacket, "WP-KERNEL-012"),
        "the authored form and the explicit normalizer agree on the single key"
    );

    // (3) AC-008: the chip author_id for the REAL authored ref_value is the contract id (NOT the
    // `locus-ref-chip-unknown-{hash}` fallback the parse-None defect produced).
    assert_eq!(
        locus_ref_chip_author_id(&link.ref_value),
        "locus-ref-chip-wp-WP-KERNEL-012",
        "AC-008: the authored chip is swarm-addressable by its work unit (no unknown-hash fallback)"
    );

    // (4) The chip label is the SHORT work-unit id with the work-unit glyph, NOT the raw `wp/WP-KERNEL-012`.
    let mut resolved_link = link.clone();
    resolved_link.resolved = true;
    assert_eq!(
        chip_label(&resolved_link),
        "⎘ WP-KERNEL-012",
        "the authored chip renders the short work-unit id, not the raw wp/... value"
    );

    // (5) dispatch_locus_ref_open stages the canonical ORIGINAL-CASE navigation URI. The parsed
    // `normalized` field asserted above remains the single resolution/reverse-lookup key; it must not
    // replace the case-significant work-unit identity on the navigation seam.
    let ctx = egui::Context::default();
    let mut bus = InteractionBus::new();
    bus.register_open_locus_ref_command();
    let evt = EditorEvent::WikilinkActivated {
        ref_kind: link.ref_kind.clone(),
        ref_value: link.ref_value.clone(),
        resolved: true,
    };
    let staged = dispatch_locus_ref_open(&ctx, &mut bus, &evt);
    assert_eq!(
        staged.as_deref(),
        Some("locus://wp/WP-KERNEL-012"),
        "the authored ref stages a canonical URI without losing its case-significant identity"
    );
    assert_eq!(
        bus.take_pending_locus_ref().as_deref(),
        Some("locus://wp/WP-KERNEL-012"),
        "the original-case canonical URI is staged on the nav seam for the authored form"
    );

    // (6) The MT authoring form `[[locus:mt/MT-034]]` proves the sibling kind through the same path.
    let mt_link = parse_wikilink("[[locus:mt/MT-034]]")
        .expect("a valid locus mt wikilink")
        .to_hs_link();
    assert_eq!(mt_link.ref_value, "mt/MT-034");
    assert_eq!(
        locus_ref_chip_author_id(&mt_link.ref_value),
        "locus-ref-chip-mt-MT-034",
        "AC-008: the authored MT chip is swarm-addressable too"
    );

    println!(
        "MUST-FIX OK: authored ref_value=wp/WP-KERNEL-012 -> author_id=locus-ref-chip-wp-WP-KERNEL-012, label=WP-KERNEL-012, staged=locus://wp/WP-KERNEL-012"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-004 / PT-004 (LIVE) — default managed-runtime reverse-lookup case-robustness proof.
//
// The unit `ac004_reverse_lookup_lists_referencing_documents` above proves the reverse lookup KEYS on the
// NORMALIZED `locus://mt/mt-034` and de-dups — but its `CountingReverseLookup` returns the seeded hits
// REGARDLESS of the query string, so it can NOT prove that a document STORING the authored-case
// `locus://mt/MT-034` actually matches when the lookup keys on the lowercased `locus://mt/mt-034`. That
// case-robustness is a LIVE-PG claim. This proof creates a real RichDocument whose compact chip label is
// only `MT` while its structured refValue carries mixed-case `locus://mt/MT-034`. The synchronous Loom
// projection must make that refValue searchable, and exact readback must verify the persisted hsLink before
// returning the document. This prevents a label-only or plain-text false-positive proof.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac004_reverse_lookup_case_robust_against_real_pg_live() {
    let mut be: LiveBackend = require_live_backend();
    let ws = be.workspace_id.clone();
    let authored = doc_with_locus_ref("locus://mt/MT-034", "MT", false);
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": ws,
            "title": format!("MT-068 exact reverse lookup {}", uuid::Uuid::new_v4()),
            "content_json": to_content_json_value(&authored),
        }),
    );
    let document_id = created_doc_id(&created);

    let svc = LocusInteropService::with_base_url(
        be.base.clone(),
        ws.clone(),
        Arc::new(FindNotesHttp::new(be.base.clone())),
    );
    let mt = parse_locus_ref("locus://mt/MT-034").expect("a valid mt ref");
    assert_eq!(
        mt.normalized, "locus://mt/mt-034",
        "AC-004 LIVE: the lookup keys on the LOWERCASED normalized ref (the case-robustness pivot)"
    );
    let docs = rt()
        .block_on(async { svc.find_documents_referencing(&mt).await })
        .expect("AC-004 LIVE: the real loom search-v2 reverse lookup returns Ok");

    let found = docs.iter().any(|d| d.document_id == document_id);
    let cleanup = be.delete(&format!("/knowledge/documents/{document_id}"));
    assert!(matches!(cleanup, 200 | 202 | 204 | 404));
    be.assert_cleanup();

    assert!(
        found,
        "AC-004 LIVE: a note storing authored-case locus://mt/MT-034 must be returned by the reverse \
         lookup keyed on the lowercased locus://mt/mt-034 (case-robust match); got docs={docs:?}"
    );
    println!(
        "AC-004 LIVE OK: RichDocument {document_id} with compact label MT and authored MT-034 was found via \
         normalized mt-034 and exact structured hsLink readback"
    );
}
