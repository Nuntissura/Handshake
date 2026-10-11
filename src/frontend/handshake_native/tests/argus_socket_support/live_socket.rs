//! Shared LIVE production-socket Argus harness (WP-1 MT-008 / MT-012 / MT-014 / MT-015).
//!
//! Every helper here drives the REAL production transport: the `handshake-native` binary is spawned,
//! it publishes its owner-only `swarm_mcp_binding.json`, and this module talks newline-delimited
//! JSON-RPC over the real `TcpStream` the `SwarmMcpServer` bound. There is no in-process harness, no
//! `egui_kittest` shortcut, and no transport mock anywhere in this file: if the production socket is
//! not up, every helper fails instead of degrading to an in-process path.
//!
//! It is `#[path]`-included by each live socket test binary (the same shared-test-module convention
//! `native_gui_support/proof_report.rs` already uses in this crate), so all live proofs share one
//! spawn/discovery/receipt/redaction implementation instead of drifting copies.

#![allow(dead_code)] // each live test binary consumes a subset of this shared surface.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use base64::Engine as _;
use sha2::{Digest, Sha256};

/// The caller-controlled display label every live proof attributes its actions with.
pub const AGENT_LABEL: &str = "production-socket-live";

/// How long a live proof waits for a surface (pane body, pop-out window) to appear.
pub const SURFACE_TIMEOUT: Duration = Duration::from_secs(20);

const PROXY_FORWARD: u8 = 0;
const PROXY_HOLD: u8 = 1;
const PROXY_FAIL: u8 = 2;
const PROXY_BACKEND_AUTHORITY: &str = "127.0.0.1:37501";

/// An owned loopback HTTP proxy for production-child failure-path proof.
///
/// It never mutates or stops the backend. The spawned native child is pointed at this proxy through
/// its process environment, allowing the test to hold, reject, and then forward the child's real
/// `/usermanual` socket traffic while the backend remains untouched.
pub struct LoopbackHttpFaultProxy {
    addr: String,
    mode: Arc<AtomicU8>,
    stop: Arc<AtomicBool>,
    held_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    forwarded_requests: Arc<AtomicU64>,
    accept_thread: Option<JoinHandle<()>>,
}

impl LoopbackHttpFaultProxy {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind owned HTTP fault proxy");
        listener
            .set_nonblocking(true)
            .expect("configure owned HTTP fault proxy");
        let addr = listener.local_addr().expect("read owned proxy address");
        let mode = Arc::new(AtomicU8::new(PROXY_FORWARD));
        let stop = Arc::new(AtomicBool::new(false));
        let held_requests = Arc::new(AtomicU64::new(0));
        let failed_requests = Arc::new(AtomicU64::new(0));
        let forwarded_requests = Arc::new(AtomicU64::new(0));
        let accept_mode = mode.clone();
        let accept_stop = stop.clone();
        let accept_held_requests = held_requests.clone();
        let accept_failed_requests = failed_requests.clone();
        let accept_forwarded_requests = forwarded_requests.clone();
        let accept_thread = std::thread::spawn(move || {
            while !accept_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let connection_mode = accept_mode.clone();
                        let connection_stop = accept_stop.clone();
                        let connection_held_requests = accept_held_requests.clone();
                        let connection_failed_requests = accept_failed_requests.clone();
                        let connection_forwarded_requests = accept_forwarded_requests.clone();
                        std::thread::spawn(move || {
                            proxy_connection(
                                stream,
                                connection_mode,
                                connection_stop,
                                connection_held_requests,
                                connection_failed_requests,
                                connection_forwarded_requests,
                            )
                        });
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            addr: addr.to_string(),
            mode,
            stop,
            held_requests,
            failed_requests,
            forwarded_requests,
            accept_thread: Some(accept_thread),
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn hold(&self) {
        self.mode.store(PROXY_HOLD, Ordering::Release);
    }

    pub fn fail(&self) {
        self.mode.store(PROXY_FAIL, Ordering::Release);
    }

    pub fn forward(&self) {
        self.mode.store(PROXY_FORWARD, Ordering::Release);
    }

    pub fn held_request_count(&self) -> u64 {
        self.held_requests.load(Ordering::Acquire)
    }

    pub fn failed_request_count(&self) -> u64 {
        self.failed_requests.load(Ordering::Acquire)
    }

    pub fn forwarded_request_count(&self) -> u64 {
        self.forwarded_requests.load(Ordering::Acquire)
    }
}

impl Drop for LoopbackHttpFaultProxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(&self.addr);
        if let Some(thread) = self.accept_thread.take() {
            let _ = thread.join();
        }
    }
}

fn proxy_connection(
    mut client: TcpStream,
    mode: Arc<AtomicU8>,
    stop: Arc<AtomicBool>,
    held_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    forwarded_requests: Arc<AtomicU64>,
) {
    let _ = client.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = client.set_write_timeout(Some(Duration::from_secs(5)));
    let request = match read_http_request(&mut client) {
        Ok(request) => request,
        Err(_) => return,
    };
    let Some((forwarded, is_user_manual)) = rewrite_proxy_request(&request) else {
        let _ = client.write_all(
            b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        return;
    };
    if is_user_manual {
        if mode.load(Ordering::Acquire) == PROXY_HOLD {
            held_requests.fetch_add(1, Ordering::AcqRel);
        }
        while mode.load(Ordering::Acquire) == PROXY_HOLD && !stop.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(10));
        }
        if stop.load(Ordering::Acquire) {
            return;
        }
        if mode.load(Ordering::Acquire) == PROXY_FAIL {
            failed_requests.fetch_add(1, Ordering::AcqRel);
            let _ = client.write_all(
                b"HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain\r\nContent-Length: 32\r\nConnection: close\r\n\r\nUserManual socket fault injected",
            );
            return;
        }
        forwarded_requests.fetch_add(1, Ordering::AcqRel);
    }
    let mut backend = match TcpStream::connect_timeout(
        &PROXY_BACKEND_AUTHORITY
            .parse()
            .expect("fixed backend authority"),
        Duration::from_secs(3),
    ) {
        Ok(stream) => stream,
        Err(_) => {
            let _ = client.write_all(
                b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            return;
        }
    };
    let _ = backend.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = backend.set_write_timeout(Some(Duration::from_secs(5)));
    if backend.write_all(&forwarded).is_ok() {
        let _ = std::io::copy(&mut backend, &mut client);
    }
}

fn read_http_request(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    const MAX_REQUEST_BYTES: usize = 1024 * 1024;
    let mut request = Vec::new();
    let mut buffer = [0u8; 4096];
    let mut expected_len = None;
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if request.len() > MAX_REQUEST_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "proxied request exceeded bounded size",
            ));
        }
        if expected_len.is_none() {
            if let Some(header_end) = find_header_end(&request) {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.split_once(':').and_then(|(name, value)| {
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                    })
                    .unwrap_or(0);
                expected_len = Some(header_end + 4 + content_length);
            }
        }
        if expected_len.is_some_and(|length| request.len() >= length) {
            break;
        }
    }
    Ok(request)
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn rewrite_proxy_request(request: &[u8]) -> Option<(Vec<u8>, bool)> {
    let header_end = find_header_end(request)?;
    let header = std::str::from_utf8(&request[..header_end]).ok()?;
    let mut lines = header.split("\r\n");
    let first = lines.next()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?;
    let target = parts.next()?;
    let version = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let path = if let Some(absolute) = target.strip_prefix("http://") {
        let (authority, path) = absolute.split_once('/').unwrap_or((absolute, ""));
        if !authority.eq_ignore_ascii_case(PROXY_BACKEND_AUTHORITY) {
            return None;
        }
        format!("/{path}")
    } else if target.starts_with('/') {
        target.to_owned()
    } else {
        return None;
    };
    let lines = lines.collect::<Vec<_>>();
    let host = lines.iter().find_map(|line| {
        line.split_once(':')
            .and_then(|(name, value)| name.eq_ignore_ascii_case("host").then(|| value.trim()))
    });
    if !host.is_some_and(|host| host.eq_ignore_ascii_case(PROXY_BACKEND_AUTHORITY)) {
        return None;
    }
    let is_user_manual = path == "/usermanual"
        || path.starts_with("/usermanual/")
        || path.starts_with("/usermanual?");
    let mut forwarded = format!("{method} {path} {version}\r\n").into_bytes();
    for line in lines {
        let header_name = line.split_once(':').map(|(name, _)| name.trim());
        if header_name.is_some_and(|name| {
            name.eq_ignore_ascii_case("connection") || name.eq_ignore_ascii_case("proxy-connection")
        }) {
            continue;
        }
        forwarded.extend_from_slice(line.as_bytes());
        forwarded.extend_from_slice(b"\r\n");
    }
    forwarded.extend_from_slice(b"Connection: close\r\n\r\n");
    forwarded.extend_from_slice(&request[header_end + 4..]);
    Some((forwarded, is_user_manual))
}

#[derive(serde::Deserialize)]
pub struct DiscoveredBinding {
    pub tcp_addr: String,
    pub token: String,
    pub pid: u32,
}

pub struct ChildGuard(pub Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub struct ArgusClient {
    pub addr: String,
    pub token: String,
    pub next_id: u64,
    pub agent_token: Option<String>,
    pub agent_id: Option<String>,
    pub transcript: Vec<serde_json::Value>,
}

impl ArgusClient {
    pub fn call(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let token = self.token.clone();
        self.call_with_credentials(method, params, &token, AGENT_LABEL)
    }

    pub fn call_with_credentials(
        &mut self,
        method: &str,
        params: serde_json::Value,
        token: &str,
        agent_label: &str,
    ) -> serde_json::Value {
        self.send(method, params, token, agent_label, true)
    }

    fn send(
        &mut self,
        method: &str,
        params: serde_json::Value,
        token: &str,
        agent_label: &str,
        record_in_transcript: bool,
    ) -> serde_json::Value {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
            "params": params,
            "session_token": token,
            "agent_token": self.agent_token.as_deref(),
            "agent_label": agent_label,
        });
        self.next_id += 1;
        let response = rpc(&self.addr, &request)
            .unwrap_or_else(|error| panic!("{method} transport failed: {error}"));
        if record_in_transcript {
            self.transcript.push(serde_json::json!({
                "request": redact_request_for_proof(&request),
                "response": redact_response_for_proof(&response),
            }));
        }
        response
    }

    /// Read-only poll over the SAME live socket that deliberately stays out of the proof
    /// transcript: a wait loop repeats one identical read many times, which would bloat the
    /// artifact with megabytes of duplicate trees without adding any evidence. Every decisive
    /// read a proof asserts on still goes through [`ArgusClient::inspect`].
    pub fn poll_inspect(&mut self, window_id: &str) -> serde_json::Value {
        let token = self.token.clone();
        let response = self.send(
            "argus.inspect",
            serde_json::json!({"window_id": window_id}),
            &token,
            AGENT_LABEL,
            false,
        );
        assert_success(&response, "argus.inspect");
        response["result"].clone()
    }

    pub fn authenticate_agent(&mut self) -> String {
        let session_token = self.token.clone();
        let response = self.call_with_credentials(
            "argus.authenticate_agent",
            serde_json::json!({}),
            &session_token,
            AGENT_LABEL,
        );
        assert_success(&response, "argus.authenticate_agent");
        let agent_id = response["result"]["agent_id"]
            .as_str()
            .expect("broker returned agent_id")
            .to_owned();
        self.agent_token = Some(
            response["result"]["agent_token"]
                .as_str()
                .expect("broker returned agent_token")
                .to_owned(),
        );
        self.agent_id = Some(agent_id.clone());
        agent_id
    }

    pub fn inspect(&mut self, window_id: &str) -> serde_json::Value {
        let response = self.call("argus.inspect", serde_json::json!({"window_id": window_id}));
        assert_success(&response, "argus.inspect");
        response["result"].clone()
    }

    /// Inspect through an explicit method name so a proof can exercise BOTH the canonical
    /// `argus.inspect` and its `list_widgets` compatibility alias over the same live socket.
    pub fn inspect_via(&mut self, method: &str, window_id: &str) -> serde_json::Value {
        let response = self.call(method, serde_json::json!({"window_id": window_id}));
        assert_success(&response, method);
        response
    }

    pub fn screenshot(&mut self, window_id: &str) -> serde_json::Value {
        let response = self.call(
            "argus.screenshot",
            serde_json::json!({"window_id": window_id}),
        );
        assert_success(&response, "argus.screenshot");
        response
    }

    pub fn mutation(
        &mut self,
        method: &str,
        window_id: &str,
        author_id: &str,
        extra: Option<(&str, serde_json::Value)>,
    ) -> serde_json::Value {
        let before = self.inspect(window_id);
        let revision = before["revision"]
            .as_u64()
            .expect("inspect revision is numeric");
        let mut params = serde_json::Map::from_iter([
            (
                "window_id".to_owned(),
                serde_json::Value::String(window_id.to_owned()),
            ),
            (
                "author_id".to_owned(),
                serde_json::Value::String(author_id.to_owned()),
            ),
            (
                "expected_snapshot_revision".to_owned(),
                serde_json::Value::from(revision),
            ),
        ]);
        if let Some((key, value)) = extra {
            params.insert(key.to_owned(), value);
        }
        let response = self.call(method, serde_json::Value::Object(params));
        assert_applied_durable(&response, revision, method);
        assert_eq!(
            response["result"]["agent_id"].as_str(),
            self.agent_id.as_deref(),
            "{method} receipt was not attributed to the broker-authenticated principal"
        );
        assert_eq!(
            response["result"]["agent_label"], AGENT_LABEL,
            "{method} receipt lost its distinct caller-controlled display label"
        );
        response
    }

    /// Issue a mutating action WITHOUT asserting the outcome, returning `(before_revision, response)`.
    ///
    /// A live pane can publish a newer snapshot between the read and the write (the ModelRuntime pane
    /// repaints while its async transport is in flight), so a lost compare-and-swap race is retried
    /// against a fresh revision. Only the fail-closed stale-revision error is retried; every other
    /// outcome — including a genuinely non-applied receipt — is returned to the caller untouched, so
    /// this helper can carry both positive and negative proofs.
    pub fn attempt_mutation(
        &mut self,
        method: &str,
        window_id: &str,
        author_id: &str,
        extra: Option<(&str, serde_json::Value)>,
    ) -> (u64, serde_json::Value) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let before = self.poll_inspect(window_id);
            let revision = before["revision"]
                .as_u64()
                .expect("inspect revision is numeric");
            let mut params = serde_json::Map::from_iter([
                (
                    "window_id".to_owned(),
                    serde_json::Value::String(window_id.to_owned()),
                ),
                (
                    "author_id".to_owned(),
                    serde_json::Value::String(author_id.to_owned()),
                ),
                (
                    "expected_snapshot_revision".to_owned(),
                    serde_json::Value::from(revision),
                ),
            ]);
            if let Some((key, value)) = extra.clone() {
                params.insert(key.to_owned(), value);
            }
            let response = self.call(method, serde_json::Value::Object(params));
            let lost_cas_race = response["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("stale Argus snapshot"));
            if !lost_cas_race || Instant::now() >= deadline {
                return (revision, response);
            }
            std::thread::sleep(Duration::from_millis(150));
        }
    }

    /// Mutating action on a live (repainting) surface: retries only a lost compare-and-swap race,
    /// then asserts the SAME applied/durable/attribution contract as [`ArgusClient::mutation`].
    pub fn mutation_on_live_surface(
        &mut self,
        method: &str,
        window_id: &str,
        author_id: &str,
        extra: Option<(&str, serde_json::Value)>,
    ) -> serde_json::Value {
        let (revision, response) = self.attempt_mutation(method, window_id, author_id, extra);
        assert_applied_durable(&response, revision, method);
        assert_eq!(
            response["result"]["agent_id"].as_str(),
            self.agent_id.as_deref(),
            "{method} receipt was not attributed to the broker-authenticated principal"
        );
        assert_eq!(
            response["result"]["agent_label"], AGENT_LABEL,
            "{method} receipt lost its distinct caller-controlled display label"
        );
        response
    }

    /// Assert the redacted transcript never retained a live credential or the given canaries.
    pub fn assert_transcript_is_secret_free(&self, canaries: &[&str]) -> Vec<u8> {
        let transcript =
            serde_json::to_vec_pretty(&self.transcript).expect("serialize redacted transcript");
        let text = String::from_utf8_lossy(&transcript).into_owned();
        assert!(
            !text.contains(self.token.as_str()),
            "proof transcript retained the live session token"
        );
        assert!(
            self.agent_token
                .as_deref()
                .is_none_or(|agent_token| !text.contains(agent_token)),
            "proof transcript retained the broker-minted agent token"
        );
        for canary in canaries {
            assert!(
                !text.contains(canary),
                "proof transcript retained the sensitive-value canary"
            );
        }
        transcript
    }
}

pub fn redact_request_for_proof(request: &serde_json::Value) -> serde_json::Value {
    let mut redacted = request.clone();
    if let Some(object) = redacted.as_object_mut() {
        if object.contains_key("session_token") {
            object.insert(
                "session_token".to_owned(),
                serde_json::Value::String("[REDACTED]".to_owned()),
            );
        }
        if object.contains_key("agent_token") {
            object.insert(
                "agent_token".to_owned(),
                serde_json::Value::String("[REDACTED]".to_owned()),
            );
        }
        if let Some(params) = object
            .get_mut("params")
            .and_then(serde_json::Value::as_object_mut)
        {
            let sensitive = params
                .get("author_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(handshake_native::accessibility::is_sensitive_author_id);
            if sensitive && params.contains_key("value") {
                params.insert(
                    "value".to_owned(),
                    serde_json::Value::String("[REDACTED]".to_owned()),
                );
            }
        }
    }
    redacted
}

pub fn redact_response_for_proof(response: &serde_json::Value) -> serde_json::Value {
    let mut redacted = response.clone();
    if let Some(result) = redacted
        .get_mut("result")
        .and_then(serde_json::Value::as_object_mut)
    {
        for key in ["session_token", "agent_token"] {
            if result.contains_key(key) {
                result.insert(
                    key.to_owned(),
                    serde_json::Value::String("[REDACTED]".to_owned()),
                );
            }
        }
    }
    redacted
}

pub fn assert_visual_png(png: &[u8], context: &str) {
    let image = image::load_from_memory(png)
        .unwrap_or_else(|error| panic!("{context} was not a decodable image: {error}"))
        .to_rgba8();
    let mut colors = std::collections::HashSet::new();
    let mut visible_nonblack = false;
    for pixel in image.pixels() {
        assert_eq!(
            pixel[3], 255,
            "{context} contains a non-opaque capture pixel"
        );
        colors.insert(pixel.0);
        visible_nonblack |= pixel[3] != 0 && (pixel[0] > 8 || pixel[1] > 8 || pixel[2] > 8);
        if colors.len() > 4 && visible_nonblack {
            return;
        }
    }
    panic!(
        "{context} was blank/uniform: {} distinct colors, visible_nonblack={visible_nonblack}",
        colors.len()
    );
}

pub fn rpc(addr: &str, request: &serde_json::Value) -> std::io::Result<serde_json::Value> {
    let stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(
        handshake_native::mcp::ACTION_RECEIPT_TRANSPORT_TIMEOUT,
    ))?;
    let mut writer = stream.try_clone()?;
    serde_json::to_writer(&mut writer, request)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    serde_json::from_str(line.trim())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

pub fn assert_success(response: &serde_json::Value, operation: &str) {
    assert!(
        response.get("error").is_none() && response.get("result").is_some(),
        "{operation} returned a JSON-RPC error: {response}"
    );
}

pub fn assert_applied_durable(response: &serde_json::Value, before: u64, operation: &str) {
    assert_success(response, operation);
    let receipt = &response["result"];
    assert_eq!(receipt["status"], "applied", "{operation}: {receipt}");
    assert_eq!(receipt["before_revision"], before, "{operation}: {receipt}");
    assert!(
        receipt["after_revision"]
            .as_u64()
            .is_some_and(|after| after > before),
        "{operation} did not publish a newer revision: {receipt}"
    );
    assert!(
        receipt["evidence_ref"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "{operation} lacked durable evidence: {receipt}"
    );
    assert!(
        receipt["durability_error"].is_null(),
        "{operation} was applied but not durable: {receipt}"
    );
}

/// A live action that must NOT have taken effect: the receipt exists but is not `applied`
/// (or the request was refused outright). Never satisfied by a missing response.
pub fn assert_not_applied(response: &serde_json::Value, operation: &str) {
    if response.get("error").is_some() {
        return;
    }
    let receipt = &response["result"];
    assert!(
        receipt.get("status").is_some(),
        "{operation} returned neither an error nor a receipt: {response}"
    );
    assert_ne!(
        receipt["status"], "applied",
        "{operation} was applied although the surface must fail closed: {receipt}"
    );
}

pub fn require_palmistry_ready_backend() -> PathBuf {
    assert_eq!(
        std::env::var("HANDSHAKE_ARGUS_LIVE_BACKEND_READY").as_deref(),
        Ok("1"),
        "set HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1 only after the injected embedded SurrealDB and \
         production backend are ready for the live proof"
    );
    let diagnostics_dir = PathBuf::from(
        std::env::var("HANDSHAKE_DIAGNOSTICS_DIR")
            .expect("HANDSHAKE_DIAGNOSTICS_DIR is required for the production Argus proof"),
    );
    assert!(
        diagnostics_dir.is_absolute() && diagnostics_dir.is_dir(),
        "HANDSHAKE_DIAGNOSTICS_DIR must be an existing absolute directory: {}",
        diagnostics_dir.display()
    );

    let mut stream = TcpStream::connect_timeout(
        &"127.0.0.1:37501".parse().expect("fixed backend address"),
        Duration::from_secs(3),
    )
    .expect("handshake_core is not accepting connections on 127.0.0.1:37501");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("set health timeout");
    stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1:37501\r\nConnection: close\r\n\r\n")
        .expect("write backend health probe");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read backend health probe");
    assert!(
        response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200"),
        "handshake_core /health was not ready: {}",
        response.lines().next().unwrap_or("<empty>")
    );
    diagnostics_dir
}

pub fn discover_binding(path: &Path, pid: u32, deadline: Instant) -> DiscoveredBinding {
    while Instant::now() < deadline {
        if let Ok(body) = std::fs::read_to_string(path) {
            if let Ok(binding) = serde_json::from_str::<DiscoveredBinding>(&body) {
                if binding.pid == pid && !binding.tcp_addr.is_empty() && !binding.token.is_empty() {
                    return binding;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!(
        "production child pid {pid} did not publish its owned binding at {}",
        path.display()
    );
}

pub fn contains_author_id(node: &serde_json::Value, author_id: &str) -> bool {
    node.get("author_id").and_then(|value| value.as_str()) == Some(author_id)
        || node
            .get("children")
            .and_then(|value| value.as_array())
            .is_some_and(|children| {
                children
                    .iter()
                    .any(|child| contains_author_id(child, author_id))
            })
}

/// The exact snapshot node addressed by `author_id`, so a proof can read its live
/// `label` / `value` / `disabled` / `actions` instead of only asserting presence.
pub fn node_by_author_id<'tree>(
    node: &'tree serde_json::Value,
    author_id: &str,
) -> Option<&'tree serde_json::Value> {
    if node.get("author_id").and_then(|value| value.as_str()) == Some(author_id) {
        return Some(node);
    }
    node.get("children")
        .and_then(|value| value.as_array())
        .and_then(|children| {
            children
                .iter()
                .find_map(|child| node_by_author_id(child, author_id))
        })
}

pub fn require_node<'tree>(
    root: &'tree serde_json::Value,
    author_id: &str,
) -> &'tree serde_json::Value {
    node_by_author_id(root, author_id)
        .unwrap_or_else(|| panic!("live snapshot has no node for author_id `{author_id}`"))
}

pub fn node_label(node: &serde_json::Value) -> String {
    node.get("label")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

pub fn node_value(node: &serde_json::Value) -> String {
    node.get("value")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The operator-readable text a node carries.
///
/// egui puts a plain `ui.label(...)`'s text in the AccessKit node's `value` (role `Label` maps
/// `WidgetInfo.label` onto `set_value`), while surfaces that own their node set an explicit
/// `label`. A live proof must read whichever one the production surface actually emitted, so it
/// cannot pass or fail on that internal distinction.
pub fn node_text(node: &serde_json::Value) -> String {
    let label = node_label(node);
    if label.trim().is_empty() {
        node_value(node)
    } else {
        label
    }
}

pub fn node_is_disabled(node: &serde_json::Value) -> bool {
    node.get("disabled").and_then(serde_json::Value::as_bool) == Some(true)
}

pub fn node_supports(node: &serde_json::Value, action: &str) -> bool {
    node.get("actions")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|actions| actions.iter().any(|value| value == action))
}

pub fn collect_author_ids(node: &serde_json::Value) -> Vec<String> {
    let mut ids = Vec::new();
    collect_author_ids_into(node, &mut ids);
    ids
}

fn collect_author_ids_into(node: &serde_json::Value, ids: &mut Vec<String>) {
    if let Some(author_id) = node.get("author_id").and_then(serde_json::Value::as_str) {
        ids.push(author_id.to_owned());
    }
    if let Some(children) = node.get("children").and_then(serde_json::Value::as_array) {
        for child in children {
            collect_author_ids_into(child, ids);
        }
    }
}

/// Resolve WHICH split pane currently hosts a surface, using only the product's own stable
/// conventions: every pane publishes a `pane-{pane_id}-header` control
/// ([`handshake_native::pane_header::pane_header_author_id`]) and a pane container node whose
/// `author_id` is the bare `pane_id` and whose label is `PaneType::label()`
/// (`PaneRegistry::build_accesskit_node`). No layout constant is assumed, so a proof keeps working
/// if the shell opens a surface on a different pane.
pub fn pane_id_hosting(root: &serde_json::Value, pane_type_label: &str) -> String {
    let mut candidates = collect_author_ids(root)
        .into_iter()
        .filter_map(|author_id| {
            author_id
                .strip_prefix("pane-")
                .and_then(|rest| rest.strip_suffix("-header"))
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    let hosting = candidates
        .iter()
        .filter(|pane_id| {
            node_by_author_id(root, pane_id).map(node_text).as_deref() == Some(pane_type_label)
        })
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        hosting.len(),
        1,
        "expected exactly one live pane labelled `{pane_type_label}`, found {hosting:?} among {candidates:?}"
    );
    hosting.into_iter().next().expect("checked exactly one")
}

pub fn list_has_window(list_response: &serde_json::Value, window_id: &str) -> bool {
    list_response["result"]["windows"]
        .as_array()
        .is_some_and(|windows| {
            windows.iter().any(|window| {
                window["window_id"] == window_id
                    && window["snapshot_available"].as_bool() == Some(true)
            })
        })
}

pub fn list_contains_window(list_response: &serde_json::Value, window_id: &str) -> bool {
    list_response["result"]["windows"]
        .as_array()
        .is_some_and(|windows| {
            windows
                .iter()
                .any(|window| window["window_id"] == window_id)
        })
}

pub fn wait_for_window(client: &mut ArgusClient, window_id: &str, present: bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let list = client.call("argus.list_windows", serde_json::json!({}));
        assert_success(&list, "argus.list_windows");
        let reached = if present {
            list_has_window(&list, window_id)
        } else {
            !list_contains_window(&list, window_id)
        };
        if reached {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("window {window_id} did not reach present={present}");
}

/// Poll the live window until `author_id` is present, returning the inspect result that
/// contained it. Fails (never degrades) when the surface never renders.
pub fn wait_for_author_id(
    client: &mut ArgusClient,
    window_id: &str,
    author_id: &str,
    timeout: Duration,
) -> serde_json::Value {
    let deadline = Instant::now() + timeout;
    let mut last = serde_json::Value::Null;
    while Instant::now() < deadline {
        last = client.poll_inspect(window_id);
        if contains_author_id(&last["snapshot"]["root"], author_id) {
            // Re-read through the recorded path so the decisive observation lands in the proof
            // transcript; keep the polled observation if the surface changed in between (a menu
            // can close), so recording can never make a real observation flaky.
            let recorded = client.inspect(window_id);
            if contains_author_id(&recorded["snapshot"]["root"], author_id) {
                return recorded;
            }
            return last;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!(
        "author_id `{author_id}` never appeared in live window `{window_id}`; last observed ids: {:?}",
        collect_author_ids(&last["snapshot"]["root"])
    );
}

/// Poll the live window until some author_id matching `prefix` + `suffix` exists, returning it.
/// Used to discover the runtime pane id the shell assigned to a freshly opened pane.
pub fn wait_for_author_id_between(
    client: &mut ArgusClient,
    window_id: &str,
    prefix: &str,
    suffix: &str,
    timeout: Duration,
) -> String {
    let deadline = Instant::now() + timeout;
    let mut last = serde_json::Value::Null;
    while Instant::now() < deadline {
        last = client.poll_inspect(window_id);
        if let Some(found) = collect_author_ids(&last["snapshot"]["root"])
            .into_iter()
            .find(|author_id| author_id.starts_with(prefix) && author_id.ends_with(suffix))
        {
            return found;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!(
        "no author_id matching `{prefix}*{suffix}` appeared in live window `{window_id}`; last observed ids: {:?}",
        collect_author_ids(&last["snapshot"]["root"])
    );
}

pub fn proof_dir() -> PathBuf {
    let artifact_root = PathBuf::from(
        std::env::var("HANDSHAKE_ARTIFACTS_DIR")
            .expect("HANDSHAKE_ARTIFACTS_DIR is required for live proof artifacts"),
    );
    let artifact_root = std::fs::canonicalize(&artifact_root).unwrap_or_else(|error| {
        panic!(
            "canonicalize configured artifact root {}: {error}",
            artifact_root.display()
        )
    });
    let proof_dir = PathBuf::from(
        std::env::var("HANDSHAKE_PROOF_ARTIFACT_DIR")
            .expect("HANDSHAKE_PROOF_ARTIFACT_DIR is required for live proof artifacts"),
    );
    assert!(
        proof_dir.is_absolute(),
        "HANDSHAKE_PROOF_ARTIFACT_DIR must be absolute"
    );
    let relative_proof_dir = proof_dir
        .strip_prefix(&artifact_root)
        .expect("proof directory must be lexically beneath HANDSHAKE_ARTIFACTS_DIR");
    assert!(
        relative_proof_dir
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_))),
        "proof directory must not contain traversal or root components"
    );
    std::fs::create_dir_all(&proof_dir).unwrap_or_else(|error| {
        panic!(
            "create configured proof directory {}: {error}",
            proof_dir.display()
        )
    });
    let proof_dir = std::fs::canonicalize(&proof_dir).unwrap_or_else(|error| {
        panic!(
            "canonicalize configured proof directory {}: {error}",
            proof_dir.display()
        )
    });
    assert!(
        proof_dir.starts_with(&artifact_root),
        "proof directory must stay beneath HANDSHAKE_ARTIFACTS_DIR"
    );
    proof_dir
}

pub fn request_child_close(pid: u32) {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE,
    };

    unsafe extern "system" fn close_owned_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let wanted_pid = lparam as u32;
        let mut window_pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut window_pid);
            if window_pid == wanted_pid {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
        }
        1
    }

    unsafe {
        EnumWindows(Some(close_owned_window), pid as LPARAM);
    }
}

/// Decode + verify one `argus.screenshot` response and return its PNG bytes.
///
/// Proves the capture really came from the addressed live window of the spawned production
/// process (window_id + pid + non-zero size + self-consistent sha256 + non-blank pixels).
pub fn decode_verified_capture(
    response: &serde_json::Value,
    window_id: &str,
    child_pid: u32,
    context: &str,
) -> Vec<u8> {
    assert_eq!(response["result"]["window_id"], window_id, "{context}");
    assert_eq!(response["result"]["pid"], child_pid, "{context}");
    assert!(
        response["result"]["width"]
            .as_u64()
            .is_some_and(|value| value > 0)
            && response["result"]["height"]
                .as_u64()
                .is_some_and(|value| value > 0),
        "{context} had zero dimensions"
    );
    let png = base64::engine::general_purpose::STANDARD
        .decode(
            response["result"]["png_base64"]
                .as_str()
                .unwrap_or_else(|| panic!("{context} carried no png_base64")),
        )
        .unwrap_or_else(|error| panic!("{context} PNG did not decode: {error}"));
    assert!(
        png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "{context} is not a PNG"
    );
    assert_eq!(
        response["result"]["sha256"],
        format!("{:x}", Sha256::digest(&png)),
        "{context} sha256 does not match its own bytes"
    );
    let decoded = image::load_from_memory(&png)
        .unwrap_or_else(|error| panic!("{context} PNG did not decode for dimensions: {error}"));
    assert_eq!(
        decoded.width() as u64,
        response["result"]["width"].as_u64().unwrap(),
        "{context} decoded width differs from capture metadata"
    );
    assert_eq!(
        decoded.height() as u64,
        response["result"]["height"].as_u64().unwrap(),
        "{context} decoded height differs from capture metadata"
    );
    assert_visual_png(&png, context);
    png
}

/// Assert a byte payload (e.g. a captured PNG) does not literally embed a canary string.
pub fn assert_bytes_exclude(bytes: &[u8], canary: &str, context: &str) {
    assert!(
        !bytes
            .windows(canary.len())
            .any(|window| window == canary.as_bytes()),
        "{context} disclosed the canary"
    );
}

/// One spawned production `handshake-native` process plus its authenticated Argus socket client.
///
/// The child gets an isolated `LOCALAPPDATA` so its owner-only binding file cannot collide with a
/// parallel proof, and the shared `HANDSHAKE_DIAGNOSTICS_DIR` the backend/Palmistry already use.
pub struct LiveApp {
    pub client: ArgusClient,
    pub child_pid: u32,
    pub authenticated_agent_id: String,
    pub binding_path: PathBuf,
    tmp: PathBuf,
    child: ChildGuard,
}

impl LiveApp {
    /// Spawn the production binary, discover its owned binding, and authenticate an agent.
    pub fn start(scope: &str) -> Self {
        Self::start_with_child_proxy(scope, None)
    }

    /// Spawn the production binary with an owned loopback HTTP proxy for real socket fault proof.
    pub fn start_with_http_proxy(scope: &str, proxy_url: &str) -> Self {
        assert!(
            proxy_url.starts_with("http://127.0.0.1:"),
            "live proof proxy must be an owned loopback listener"
        );
        Self::start_with_child_proxy(scope, Some(proxy_url))
    }

    fn start_with_child_proxy(scope: &str, proxy_url: Option<&str>) -> Self {
        let diagnostics_dir = require_palmistry_ready_backend();
        let tmp = std::env::temp_dir().join(format!(
            "hsk_argus_production_socket_{scope}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("create isolated LOCALAPPDATA");
        let binding_path = tmp.join("handshake").join("swarm_mcp_binding.json");

        let mut command = Command::new(env!("CARGO_BIN_EXE_handshake-native"));
        command
            .env("LOCALAPPDATA", &tmp)
            .env("HANDSHAKE_DIAGNOSTICS_DIR", &diagnostics_dir);
        if let Some(proxy_url) = proxy_url {
            command
                .env("HTTP_PROXY", proxy_url)
                .env("http_proxy", proxy_url)
                .env("NO_PROXY", "")
                .env("no_proxy", "");
        }
        let child = command
            .spawn()
            .expect("spawn production handshake-native binary");
        let child_pid = child.id();
        let child = ChildGuard(child);
        let binding = discover_binding(
            &binding_path,
            child_pid,
            Instant::now() + Duration::from_secs(30),
        );
        let mut client = ArgusClient {
            addr: binding.tcp_addr,
            token: binding.token,
            next_id: 1,
            agent_token: None,
            agent_id: None,
            transcript: Vec::new(),
        };
        let authenticated_agent_id = client.authenticate_agent();
        assert!(
            !authenticated_agent_id.is_empty(),
            "broker returned an empty agent id"
        );
        // The binding is published from the tokio runtime, so the first main-window snapshot can
        // still be a frame away. Wait for it (bounded, then fail) instead of racing the very first
        // publish; every later assertion needs a real rendered tree anyway.
        wait_for_window(&mut client, "main", true);
        Self {
            client,
            child_pid,
            authenticated_agent_id,
            binding_path,
            tmp,
            child,
        }
    }

    /// Open a MODELS-menu leaf through the real menu bar (`menu-models` -> `menu.models.*`).
    ///
    /// The WP navigation leaves live under the top-level MODELS menu, so every live proof reaches
    /// its surface the same way an operator does, not by mutating app state directly.
    pub fn open_models_menu_leaf(&mut self, leaf_author_id: &str) {
        self.client
            .mutation_on_live_surface("argus.click", "main", "menu-models", None);
        let menu = wait_for_author_id(
            &mut self.client,
            "main",
            leaf_author_id,
            Duration::from_secs(10),
        );
        assert!(
            contains_author_id(&menu["snapshot"]["root"], leaf_author_id),
            "MODELS menu did not expose {leaf_author_id}"
        );
        self.client
            .mutation_on_live_surface("argus.click", "main", leaf_author_id, None);
    }

    /// Detach `pane_id` into a real second OS window through the pane context menu, returning the
    /// detached window's Argus `window_id`.
    pub fn pop_out_pane(&mut self, pane_id: &str) -> String {
        let header = handshake_native::pane_header::pane_header_author_id(pane_id);
        self.client
            .mutation_on_live_surface("argus.show_context_menu", "main", &header, None);
        let menu = wait_for_author_id(
            &mut self.client,
            "main",
            "ctx-menu.pane.pop_out",
            Duration::from_secs(10),
        );
        assert!(
            contains_author_id(&menu["snapshot"]["root"], "ctx-menu.pane.pop_out"),
            "pane context menu did not expose its stable pop-out item"
        );
        self.client
            .mutation_on_live_surface("argus.click", "main", "ctx-menu.pane.pop_out", None);
        let window_id = handshake_native::popout_window::argus_window_id(pane_id);
        wait_for_window(&mut self.client, &window_id, true);
        window_id
    }

    /// Merge a detached pane back into the main window and prove the window is gone.
    pub fn merge_back_pane(&mut self, pane_id: &str) {
        let merge_back = handshake_native::popout_window::merge_back_author_id(pane_id);
        self.client
            .mutation_on_live_surface("argus.click", "main", &merge_back, None);
        let window_id = handshake_native::popout_window::argus_window_id(pane_id);
        wait_for_window(&mut self.client, &window_id, false);
    }

    /// Write one proof artifact under the external artifact root and return its path.
    pub fn write_proof_artifact(&self, file_name: &str, bytes: &[u8]) -> PathBuf {
        let dir = proof_dir();
        std::fs::create_dir_all(&dir).expect("create external proof directory");
        let path = dir.join(file_name);
        std::fs::write(&path, bytes).expect("write live socket proof artifact");
        path
    }

    /// Close the production child through a real WM_CLOSE and prove the owned binding is reclaimed.
    pub fn shutdown(mut self) {
        request_child_close(self.child_pid);
        // A real WM_CLOSE can legitimately wait for the product's bounded persistence/diagnostics
        // teardown. Keep the proof non-destructive (never terminate the child) while allowing the
        // slower loaded-machine path observed in production, which completed just after ten seconds.
        let exit_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < exit_deadline {
            if self
                .child
                .0
                .try_wait()
                .expect("poll production child")
                .is_some()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(
            self.child
                .0
                .try_wait()
                .expect("final production child poll")
                .is_some(),
            "production child did not exit after WM_CLOSE"
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.binding_path.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(
            !self.binding_path.exists(),
            "owned binding survived production child shutdown"
        );
        let _ = std::fs::remove_dir_all(&self.tmp);
    }
}
