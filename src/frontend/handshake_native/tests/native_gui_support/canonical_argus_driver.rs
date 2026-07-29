//! Reusable production-boundary Argus driver for mounted native GUI integration tests.

#![allow(dead_code)]

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::screenshot_harness::screenshot_marker;
use egui_kittest::kittest::NodeT;
use handshake_native::app::HandshakeApp;
use handshake_native::mcp::{
    ScreenshotError, SessionToken, SwarmMcpServer, ARGUS_CLICK_METHOD, ARGUS_INSPECT_METHOD,
    ARGUS_SET_VALUE_METHOD,
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct ArgusObservation {
    pub before: serde_json::Value,
    pub after: serde_json::Value,
    pub receipt_id: u64,
    pub receipt_status: String,
    pub agent_id: String,
    pub terminal_observed_sequence: u64,
    pub target_selected_before: Option<bool>,
    pub target_selected_after: Option<bool>,
    pub terminal_refreshed: bool,
    pub terminal_predicates: Vec<TerminalPredicateResult>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct TerminalPredicateResult {
    pub predicate_id: String,
    pub passed: bool,
    pub evidence: serde_json::Value,
}

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
                .expect("resolve canonical Argus current directory")
                .join(root)
        };
        std::fs::create_dir_all(&root).expect("create isolated Argus binding root");
        let root = std::fs::canonicalize(&root).expect("canonicalize isolated Argus binding root");
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
                    "remove isolated Argus binding root {}: {error}",
                    self.root.display()
                );
            }
        }
    }
}

pub fn json_has_author_id(value: &serde_json::Value, expected: &str) -> bool {
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

pub fn json_node_by_author_id<'a>(
    value: &'a serde_json::Value,
    expected: &str,
) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => {
            if object.get("author_id").and_then(serde_json::Value::as_str) == Some(expected) {
                return Some(value);
            }
            object
                .values()
                .find_map(|value| json_node_by_author_id(value, expected))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| json_node_by_author_id(value, expected)),
        _ => None,
    }
}

pub fn live_author_id_selected(
    harness: &egui_kittest::Harness<'_, HandshakeApp>,
    author_id: &str,
) -> Option<bool> {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .map(|node| node.accesskit_node().is_selected().unwrap_or(false))
}

fn bind_screenshot_to_matrix_receipt(receipt_id: u64) {
    if std::env::var("HANDSHAKE_ARGUS_MATRIX_RUN_ID")
        .ok()
        .is_some_and(|run_id| !run_id.trim().is_empty())
    {
        std::env::set_var("HANDSHAKE_PROOF_ACTION_RECEIPT_ID", receipt_id.to_string());
    }
}

/// Real localhost JSON-RPC Argus transport bound to the exact snapshot and action channel owned by a
/// mounted `HandshakeApp`.
pub struct CanonicalArgusDriver {
    runtime: tokio::runtime::Runtime,
    server: SwarmMcpServer,
    _app_data: Option<ScopedArgusAppData>,
    token: String,
    client_session_id: String,
    next_id: u64,
    action_targets: Vec<(String, String, Option<String>)>,
    observations: Vec<ArgusObservation>,
}

impl CanonicalArgusDriver {
    pub fn bind(app: &HandshakeApp, proof_id: &str) -> Self {
        let unique = uuid::Uuid::new_v4().simple().to_string();
        let binding_root = std::env::var_os("HANDSHAKE_ARGUS_BINDING_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new("../../../../Handshake_Artifacts/handshake-test")
                    .join(format!("{}-argus-binding", sanitize(proof_id)))
            })
            .join(format!("run-{unique}"));
        let app_data = ScopedArgusAppData::install(binding_root);
        let session_token = SessionToken::from_hex(format!("{}-{unique}", sanitize(proof_id)));
        Self::bind_inner(app, proof_id, session_token, Some(app_data))
    }

    /// Bind in the caller's already-isolated platform app-data root. This is used by integration tests
    /// where another process (for example the backend) must discover the same genuine server binding.
    pub fn bind_in_current_app_data(
        app: &HandshakeApp,
        proof_id: &str,
        session_token: SessionToken,
    ) -> Self {
        Self::bind_inner(app, proof_id, session_token, None)
    }

    fn bind_inner(
        app: &HandshakeApp,
        proof_id: &str,
        session_token: SessionToken,
        app_data: Option<ScopedArgusAppData>,
    ) -> Self {
        let token = session_token.as_hex().to_owned();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("canonical Argus proof runtime");
        let server = runtime
            .block_on(SwarmMcpServer::bind(
                session_token,
                app.mcp_snapshot_slot(),
                app.mcp_action_channel(),
                Arc::new(|| {
                    Err(ScreenshotError(
                        "this inspect/click proof does not request a screenshot".to_owned(),
                    ))
                }),
            ))
            .expect("bind the production Argus localhost server");
        Self {
            runtime,
            server,
            _app_data: app_data,
            token,
            client_session_id: format!("{}-agent", sanitize(proof_id)),
            next_id: 1,
            action_targets: Vec::new(),
            observations: Vec::new(),
        }
    }

    fn rpc_unchecked(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
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
        response
    }

    fn rpc(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let response = self.rpc_unchecked(method, params);
        assert!(
            response.get("error").is_none(),
            "canonical Argus request failed: {response}"
        );
        response
    }

    pub fn inspect(
        &mut self,
        harness: &mut egui_kittest::Harness<'_, HandshakeApp>,
    ) -> serde_json::Value {
        harness.state_mut().capture_mcp_snapshot_for_navigation();
        self.rpc(ARGUS_INSPECT_METHOD, serde_json::json!({}))["result"].clone()
    }

    pub fn click_and_reinspect(
        &mut self,
        harness: &mut egui_kittest::Harness<'_, HandshakeApp>,
        author_id: &str,
    ) -> ArgusObservation {
        let before = self.inspect(harness);
        self.click_from_snapshot_and_reinspect(harness, author_id, before)
    }

    pub fn click_expect_rejected(
        &mut self,
        harness: &mut egui_kittest::Harness<'_, HandshakeApp>,
        author_id: &str,
        expected_message: &str,
    ) -> serde_json::Value {
        let before = self.inspect(harness);
        assert!(json_has_author_id(&before, author_id));
        let response = self.rpc_unchecked(
            ARGUS_CLICK_METHOD,
            serde_json::json!({ "target": author_id }),
        );
        assert_eq!(response["error"]["code"], -32000, "{response}");
        assert!(
            response["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected_message)),
            "{response}"
        );
        response
    }

    /// Click against an already-inspected canonical snapshot. This models the real inspect -> external
    /// focus/state change -> click race without silently refreshing away the exact target the client saw.
    pub fn click_from_snapshot_and_reinspect(
        &mut self,
        harness: &mut egui_kittest::Harness<'_, HandshakeApp>,
        author_id: &str,
        before: serde_json::Value,
    ) -> ArgusObservation {
        let target_selected_before = live_author_id_selected(harness, author_id);
        assert!(
            json_has_author_id(&before, author_id),
            "canonical argus.inspect sees mounted target {author_id}"
        );
        let click = self.rpc(
            ARGUS_CLICK_METHOD,
            serde_json::json!({ "target": author_id }),
        );
        assert_eq!(click["result"]["queued"], true);
        let agent_id = click["result"]["agent_id"]
            .as_str()
            .expect("Argus click returns caller attribution")
            .to_owned();
        assert!(
            agent_id.ends_with(&format!(":client:{}", self.client_session_id)),
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
        let target_selected_after = live_author_id_selected(harness, author_id);
        let receipt = after["action_receipts"]
            .as_array()
            .and_then(|receipts| {
                receipts
                    .iter()
                    .find(|receipt| receipt["receipt_id"].as_u64() == Some(receipt_id))
            })
            .expect("fresh argus.inspect returns the click receipt");
        let receipt_status = receipt["status"]
            .as_str()
            .expect("Argus receipt has a typed status")
            .to_owned();
        assert!(
            matches!(receipt_status.as_str(), "applied" | "indeterminate"),
            "Argus receipt is terminal and non-rejected: {receipt}"
        );
        let terminal_observed_sequence = screenshot_marker::next_proof_event_sequence();
        bind_screenshot_to_matrix_receipt(receipt_id);
        self.action_targets
            .push((ARGUS_CLICK_METHOD.to_owned(), author_id.to_owned(), None));
        let observation = ArgusObservation {
            before,
            after,
            receipt_id,
            receipt_status,
            agent_id,
            terminal_observed_sequence,
            target_selected_before,
            target_selected_after,
            terminal_refreshed: false,
            terminal_predicates: Vec::new(),
        };
        self.observations.push(observation.clone());
        observation
    }

    /// Click a target carrying a parameterized JSON payload (`argus.click { target, payload }` ->
    /// `ClickWithPayload` -> AccessKit `ActionData::Value`). This is the swarm's parameterized-action path
    /// (e.g. `graph.select-node {block_id}`, `canvas.add-edge {source_id,target_id,edge_mode}`) — the same
    /// canonical localhost transport as [`Self::click_and_reinspect`], only carrying data.
    pub fn click_with_payload_and_reinspect(
        &mut self,
        harness: &mut egui_kittest::Harness<'_, HandshakeApp>,
        author_id: &str,
        payload: serde_json::Value,
    ) -> ArgusObservation {
        let before = self.inspect(harness);
        let target_selected_before = live_author_id_selected(harness, author_id);
        assert!(
            json_has_author_id(&before, author_id),
            "canonical argus.inspect sees parameterized target {author_id}"
        );
        let action_value = serde_json::to_string(&payload).expect("serialize Argus click payload");
        let click = self.rpc(
            ARGUS_CLICK_METHOD,
            serde_json::json!({ "target": author_id, "payload": payload }),
        );
        assert_eq!(click["result"]["queued"], true);
        let agent_id = click["result"]["agent_id"]
            .as_str()
            .expect("Argus parameterized click returns caller attribution")
            .to_owned();
        assert!(
            agent_id.ends_with(&format!(":client:{}", self.client_session_id)),
            "Argus parameterized click must retain caller attribution: {click}"
        );
        let receipt_id = click["result"]["receipt_id"]
            .as_u64()
            .expect("Argus parameterized click returns a receipt id");
        let mut raw_input = egui::RawInput::default();
        <HandshakeApp as eframe::App>::raw_input_hook(
            harness.state_mut(),
            &egui::Context::default(),
            &mut raw_input,
        );
        assert_eq!(
            raw_input.events.len(),
            1,
            "one canonical parameterized Argus click drains as one production egui event"
        );
        for event in raw_input.events {
            harness.event(event);
        }
        harness.run_steps(3);

        let after = self.inspect(harness);
        let target_selected_after = live_author_id_selected(harness, author_id);
        let receipt = after["action_receipts"]
            .as_array()
            .and_then(|receipts| {
                receipts
                    .iter()
                    .find(|receipt| receipt["receipt_id"].as_u64() == Some(receipt_id))
            })
            .expect("fresh argus.inspect returns the parameterized click receipt");
        let receipt_status = receipt["status"]
            .as_str()
            .expect("Argus receipt has a typed status")
            .to_owned();
        assert!(
            matches!(receipt_status.as_str(), "applied" | "indeterminate"),
            "Argus parameterized receipt is terminal and non-rejected: {receipt}"
        );
        let terminal_observed_sequence = screenshot_marker::next_proof_event_sequence();
        bind_screenshot_to_matrix_receipt(receipt_id);
        self.action_targets.push((
            ARGUS_CLICK_METHOD.to_owned(),
            author_id.to_owned(),
            Some(action_value),
        ));
        let observation = ArgusObservation {
            before,
            after,
            receipt_id,
            receipt_status,
            agent_id,
            terminal_observed_sequence,
            target_selected_before,
            target_selected_after,
            terminal_refreshed: false,
            terminal_predicates: Vec::new(),
        };
        self.observations.push(observation.clone());
        observation
    }

    pub fn set_value_and_reinspect(
        &mut self,
        harness: &mut egui_kittest::Harness<'_, HandshakeApp>,
        author_id: &str,
        value: &str,
    ) -> ArgusObservation {
        let before = self.inspect(harness);
        let target_selected_before = live_author_id_selected(harness, author_id);
        assert!(
            json_has_author_id(&before, author_id),
            "canonical argus.inspect sees value target {author_id}"
        );
        let set_value = self.rpc(
            ARGUS_SET_VALUE_METHOD,
            serde_json::json!({ "target": author_id, "value": value }),
        );
        assert_eq!(set_value["result"]["queued"], true);
        let agent_id = set_value["result"]["agent_id"]
            .as_str()
            .expect("Argus set-value returns caller attribution")
            .to_owned();
        assert!(
            agent_id.ends_with(&format!(":client:{}", self.client_session_id)),
            "Argus set-value must retain caller attribution: {set_value}"
        );
        let receipt_id = set_value["result"]["receipt_id"]
            .as_u64()
            .expect("Argus set-value returns a receipt id");
        let mut raw_input = egui::RawInput::default();
        <HandshakeApp as eframe::App>::raw_input_hook(
            harness.state_mut(),
            &egui::Context::default(),
            &mut raw_input,
        );
        assert_eq!(
            raw_input.events.len(),
            1,
            "one canonical Argus set-value drains as one production egui event"
        );
        for event in raw_input.events {
            harness.event(event);
        }
        harness.run_steps(3);

        let after = self.inspect(harness);
        let after_value = json_node_by_author_id(&after, author_id)
            .and_then(|node| node.get("value"))
            .and_then(serde_json::Value::as_str);
        assert_eq!(
            after_value,
            Some(value),
            "canonical SetValue must be visible in the immediate post-action snapshot for {author_id}"
        );
        let target_selected_after = live_author_id_selected(harness, author_id);
        let receipt = after["action_receipts"]
            .as_array()
            .and_then(|receipts| {
                receipts
                    .iter()
                    .find(|receipt| receipt["receipt_id"].as_u64() == Some(receipt_id))
            })
            .expect("fresh argus.inspect returns the set-value receipt");
        let receipt_status = receipt["status"]
            .as_str()
            .expect("Argus receipt has a typed status")
            .to_owned();
        assert!(
            matches!(receipt_status.as_str(), "applied" | "indeterminate"),
            "Argus set-value receipt is terminal and non-rejected: {receipt}"
        );
        let terminal_observed_sequence = screenshot_marker::next_proof_event_sequence();
        bind_screenshot_to_matrix_receipt(receipt_id);
        self.action_targets.push((
            ARGUS_SET_VALUE_METHOD.to_owned(),
            author_id.to_owned(),
            Some(value.to_owned()),
        ));
        let observation = ArgusObservation {
            before,
            after,
            receipt_id,
            receipt_status,
            agent_id,
            terminal_observed_sequence,
            target_selected_before,
            target_selected_after,
            terminal_refreshed: false,
            terminal_predicates: Vec::new(),
        };
        self.observations.push(observation.clone());
        observation
    }

    /// Replace the latest action's provisional three-frame observation with
    /// a fresh snapshot captured after the caller has awaited the product's
    /// authoritative terminal state.
    pub fn reinspect_latest_terminal(
        &mut self,
        harness: &mut egui_kittest::Harness<'_, HandshakeApp>,
    ) -> serde_json::Value {
        assert!(
            !self.observations.is_empty(),
            "terminal reinspection requires a preceding canonical action"
        );
        let after = self.inspect(harness);
        let observation = self
            .observations
            .last_mut()
            .expect("preceding canonical observation");
        let receipt_present = after["action_receipts"].as_array().is_some_and(|receipts| {
            receipts.iter().any(|receipt| {
                receipt["receipt_id"].as_u64() == Some(observation.receipt_id)
                    && matches!(
                        receipt["status"].as_str(),
                        Some("applied" | "indeterminate")
                    )
            })
        });
        assert!(
            receipt_present,
            "terminal reinspection must retain the exact action receipt"
        );
        observation.after = after.clone();
        observation.terminal_observed_sequence = screenshot_marker::next_proof_event_sequence();
        observation.terminal_refreshed = true;
        after
    }

    /// Capture the authoritative terminal tree for the latest action and bind
    /// an action-specific product-state predicate to that exact trace row.
    pub fn assert_latest_terminal_predicate(
        &mut self,
        harness: &mut egui_kittest::Harness<'_, HandshakeApp>,
        predicate_id: &str,
        predicate: impl FnOnce(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        self.assert_latest_terminal_predicate_with_evidence(
            harness,
            predicate_id,
            serde_json::Value::Null,
            predicate,
        )
    }

    /// The evidence value carries runtime-generated expected identities that
    /// an external verifier cannot otherwise derive (for example randomly
    /// allocated journal block ids). The external runner must still recompute
    /// the predicate against `after`; this is not a replacement pass flag.
    pub fn assert_latest_terminal_predicate_with_evidence(
        &mut self,
        harness: &mut egui_kittest::Harness<'_, HandshakeApp>,
        predicate_id: &str,
        evidence: serde_json::Value,
        predicate: impl FnOnce(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        assert!(
            !predicate_id.trim().is_empty(),
            "terminal predicate id must be non-empty"
        );
        let after = self.reinspect_latest_terminal(harness);
        let passed = predicate(&after);
        self.observations
            .last_mut()
            .expect("preceding canonical observation")
            .terminal_predicates
            .push(TerminalPredicateResult {
                predicate_id: predicate_id.to_owned(),
                passed,
                evidence,
            });
        assert!(
            passed,
            "canonical terminal predicate {predicate_id:?} failed against {after}"
        );
        after
    }

    pub fn finish(mut self) {
        let entries = self.server.action_log().drain_log();
        assert_eq!(entries.len(), self.action_targets.len());
        assert_eq!(self.observations.len(), self.action_targets.len());
        for observation in &self.observations {
            assert!(
                observation.terminal_refreshed,
                "every canonical action must be rebound to an authoritative terminal snapshot"
            );
            assert!(
                !observation.terminal_predicates.is_empty(),
                "every canonical action must carry at least one action-specific terminal predicate"
            );
            assert!(
                observation
                    .terminal_predicates
                    .iter()
                    .all(|predicate| predicate.passed),
                "every persisted canonical terminal predicate must pass"
            );
        }
        for (entry, (method, target, _)) in entries.iter().zip(&self.action_targets) {
            assert_eq!(&entry.op_name, method);
            assert_eq!(&entry.target_key, target);
            assert!(entry
                .agent_id
                .ends_with(&format!(":client:{}", self.client_session_id)));
            assert_ne!(entry.node_id, 0);
        }
        write_matrix_trace(
            &self.client_session_id,
            &self.action_targets,
            &self.observations,
        );
        assert_eq!(self.server.leases().active_resource_count(), 0);
        self.server.shutdown();
        drop(self.runtime);
    }
}

fn write_matrix_trace(
    client_session_id: &str,
    action_targets: &[(String, String, Option<String>)],
    observations: &[ArgusObservation],
) {
    let Ok(run_id) = std::env::var("HANDSHAKE_ARGUS_MATRIX_RUN_ID") else {
        return;
    };
    let scenario_id = required_matrix_env("HANDSHAKE_ARGUS_MATRIX_SCENARIO_ID");
    let surface = required_matrix_env("HANDSHAKE_ARGUS_MATRIX_SURFACE");
    let edge_state_tag = required_matrix_env("HANDSHAKE_ARGUS_MATRIX_EDGE_STATE");
    let source_sha = required_matrix_env("HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA");
    let process_correlation_id = required_matrix_env("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID");
    let artifact_root = PathBuf::from(required_matrix_env("HANDSHAKE_PROOF_ARTIFACT_DIR"));
    let run_dir = artifact_root.join(&run_id);
    std::fs::create_dir_all(&run_dir).expect("create canonical Argus matrix run directory");
    let trace_path = run_dir.join("canonical-argus-matrix.jsonl");

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&trace_path)
        .unwrap_or_else(|error| {
            panic!(
                "open canonical Argus matrix trace {}: {error}",
                trace_path.display()
            )
        });
    for ((method, target, action_value), observation) in action_targets.iter().zip(observations) {
        let row = serde_json::json!({
            "schema_id": "hsk.native_gui.canonical_argus_matrix_trace@1",
            "run_id": &run_id,
            "scenario_id": &scenario_id,
            "surface": &surface,
            "edge_state_tag": &edge_state_tag,
            "source_sha": &source_sha,
            "process_correlation_id": &process_correlation_id,
            "process_id": std::process::id(),
            "client_session_id": client_session_id,
            "method": method,
            "target": target,
            "action_value": action_value,
            "target_selected_before": observation.target_selected_before,
            "target_selected_after": observation.target_selected_after,
            "receipt_id": observation.receipt_id,
            "receipt_status": &observation.receipt_status,
            "agent_id": &observation.agent_id,
            "terminal_observed_sequence": observation.terminal_observed_sequence,
            "terminal_refreshed": observation.terminal_refreshed,
            "terminal_predicates": &observation.terminal_predicates,
            "before": &observation.before,
            "after": &observation.after,
        });
        writeln!(
            file,
            "{}",
            serde_json::to_string(&row).expect("serialize canonical Argus matrix trace")
        )
        .expect("append canonical Argus matrix trace row");
    }
    file.sync_all()
        .expect("flush canonical Argus matrix trace durably");
}

fn required_matrix_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is required for the MT-108 matrix run"))
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}
