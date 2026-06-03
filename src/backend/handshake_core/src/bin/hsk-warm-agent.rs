//! Resident Cloud Hypervisor warm-model guest agent.
//!
//! Research basis, checked 2026-06-02 against ggml-org/llama.cpp
//! `tools/server/README.md`: `llama-server` keeps a GGUF model resident behind
//! an HTTP server, accepts `--host` and `--port`, exposes `GET /health`, and
//! streams native `/completion` responses with `content`, `tokens`, and `stop`.
//! This binary translates that resident server into Handshake's
//! `hsk.warm_agent` JSONL protocol on stdin/stdout. It intentionally does not
//! shell out to `llama-cli` per request.

use std::{
    collections::BTreeMap,
    env,
    net::TcpListener,
    path::Path,
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use handshake_core::model_runtime::{
    decode_warm_agent_frame, encode_warm_agent_frame, WarmAgentGenerateRequest,
    WarmAgentGuestFrame, WarmAgentHostFrame, WARM_AGENT_PROTOCOL_ID, WARM_AGENT_PROTOCOL_VERSION,
};
use serde_json::{json, Value};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader, Stdout},
    process::{Child, Command},
    sync::{mpsc, Mutex},
    time,
};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_STARTUP_TIMEOUT_MS: u64 = 120_000;
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 600_000;
const HEALTH_POLL_MS: u64 = 250;
const MODEL_ALIAS: &str = "hsk-warm-model";
const CANCEL_POLL_MS: u64 = 50;

#[derive(Debug, thiserror::Error)]
enum AgentError {
    #[error("warm-agent config error: {0}")]
    Config(String),
    #[error("warm-agent protocol error: {0}")]
    Protocol(String),
    #[error("warm-agent I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("llama-server HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("llama-server error: {0}")]
    Server(String),
}

#[derive(Clone, Debug)]
struct AgentConfig {
    server_bin: String,
    host: String,
    port: u16,
    ctx_size: Option<u32>,
    startup_timeout: Duration,
    request_timeout: Duration,
    extra_args: Vec<String>,
    persistent_serial_input: bool,
}

impl AgentConfig {
    fn from_env() -> Result<Self, AgentError> {
        let persistent_serial_input =
            env::var("HSK_WARM_AGENT_TRANSPORT").ok().as_deref() == Some("serial");
        let host = env::var("HSK_WARM_AGENT_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
        let port = match env_u16("HSK_WARM_AGENT_PORT")? {
            Some(port) => port,
            None => allocate_loopback_port(&host)?,
        };
        Ok(Self {
            server_bin: env::var("HSK_WARM_AGENT_LLAMA_SERVER")
                .unwrap_or_else(|_| default_llama_server_bin()),
            host,
            port,
            ctx_size: env_u32("HSK_WARM_AGENT_CTX")?,
            startup_timeout: Duration::from_millis(
                env_u64("HSK_WARM_AGENT_STARTUP_TIMEOUT_MS")?.unwrap_or(DEFAULT_STARTUP_TIMEOUT_MS),
            ),
            request_timeout: Duration::from_millis(
                env_u64("HSK_WARM_AGENT_REQUEST_TIMEOUT_MS")?.unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS),
            ),
            extra_args: env::var("HSK_WARM_AGENT_LLAMA_SERVER_ARGS")
                .ok()
                .map(|raw| raw.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
            persistent_serial_input,
        })
    }

    fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    fn completion_url(&self) -> String {
        format!("{}/completion", self.base_url())
    }

    fn health_url(&self) -> String {
        format!("{}/health", self.base_url())
    }
}

fn default_llama_server_bin() -> String {
    env::current_exe()
        .ok()
        .and_then(|path| sibling_llama_server_path(&path))
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| "llama-server".to_string())
}

fn sibling_llama_server_path(agent_exe_path: &Path) -> Option<std::path::PathBuf> {
    let parent = agent_exe_path.parent()?;
    let candidate = parent.join(if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    });
    candidate.is_file().then_some(candidate)
}

fn allocate_loopback_port(host: &str) -> Result<u16, AgentError> {
    let listener = TcpListener::bind((host, 0)).map_err(|error| {
        AgentError::Config(format!(
            "failed to allocate ephemeral warm-agent port on {host}: {error}"
        ))
    })?;
    listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|error| {
            AgentError::Config(format!(
                "failed to read allocated ephemeral warm-agent port on {host}: {error}"
            ))
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LoadedModel {
    guest_path: String,
    artifact_sha256: String,
    ready_nonce: String,
}

#[derive(Clone, Default)]
struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

struct ActiveGeneration {
    cancel: CancelFlag,
}

enum AgentEvent {
    HostFrame(Result<WarmAgentHostFrame, AgentError>),
    HostInputClosed,
    GenerationDone {
        request_id: String,
        result: Result<(), AgentError>,
    },
}

struct WarmAgent {
    cfg: AgentConfig,
    client: reqwest::Client,
    server: Option<Child>,
    loaded: Option<LoadedModel>,
    active: BTreeMap<String, ActiveGeneration>,
    output: Arc<Mutex<Stdout>>,
}

impl WarmAgent {
    fn new(cfg: AgentConfig) -> Self {
        Self {
            cfg,
            client: reqwest::Client::new(),
            server: None,
            loaded: None,
            active: BTreeMap::new(),
            output: Arc::new(Mutex::new(io::stdout())),
        }
    }

    async fn run(mut self) -> Result<(), AgentError> {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        tokio::spawn(read_stdin_frames(
            sender.clone(),
            self.cfg.persistent_serial_input,
        ));

        self.run_event_loop(&mut receiver, sender, self.cfg.persistent_serial_input)
            .await
    }

    async fn run_event_loop(
        &mut self,
        receiver: &mut mpsc::UnboundedReceiver<AgentEvent>,
        events: mpsc::UnboundedSender<AgentEvent>,
        exit_on_input_close: bool,
    ) -> Result<(), AgentError> {
        let mut input_closed = false;
        while let Some(event) = receiver.recv().await {
            match event {
                AgentEvent::HostFrame(Ok(frame)) => {
                    self.handle_frame(frame, &events).await?;
                }
                AgentEvent::HostFrame(Err(error)) => {
                    self.emit_error(None, "decode_error", &error.to_string())
                        .await?;
                }
                AgentEvent::HostInputClosed => {
                    if exit_on_input_close {
                        for active in self.active.values() {
                            active.cancel.cancel();
                        }
                        self.active.clear();
                        break;
                    }
                    input_closed = true;
                    if self.active.is_empty() {
                        break;
                    }
                }
                AgentEvent::GenerationDone { request_id, result } => {
                    self.active.remove(&request_id);
                    if let Err(error) = result {
                        self.emit_error(Some(&request_id), "generate_error", &error.to_string())
                            .await?;
                    }
                    if input_closed && self.active.is_empty() {
                        break;
                    }
                }
            }
        }
        self.shutdown().await
    }

    async fn handle_frame(
        &mut self,
        frame: WarmAgentHostFrame,
        events: &mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<(), AgentError> {
        match frame {
            WarmAgentHostFrame::Load {
                request_id,
                model_guest_path,
                model_artifact_sha256,
            } => {
                if !self.active.is_empty() {
                    return self
                        .emit_error(
                            Some(&request_id),
                            "active_generation",
                            "load refused while a generation is active",
                        )
                        .await;
                }
                match self
                    .load_model(model_guest_path, model_artifact_sha256)
                    .await
                {
                    Ok(ready) => emit_frame(&self.output, &ready).await,
                    Err(error) => {
                        self.emit_error(Some(&request_id), "load_error", &error.to_string())
                            .await
                    }
                }
            }
            WarmAgentHostFrame::Generate { request } => self.start_generate(request, events).await,
            WarmAgentHostFrame::Cancel { request_id } => {
                if let Some(active) = self.active.get(&request_id) {
                    active.cancel.cancel();
                }
                Ok(())
            }
            WarmAgentHostFrame::Ping { request_id } => {
                emit_frame(
                    &self.output,
                    &WarmAgentGuestFrame::Heartbeat {
                        request_id: Some(request_id),
                    },
                )
                .await
            }
        }
    }

    async fn load_model(
        &mut self,
        model_guest_path: String,
        model_artifact_sha256: String,
    ) -> Result<WarmAgentGuestFrame, AgentError> {
        validate_model_path(&model_guest_path)?;
        if let Some(loaded) = self.loaded.as_ref() {
            if loaded.guest_path == model_guest_path
                && loaded.artifact_sha256 == model_artifact_sha256
                && self.health_ok().await
            {
                return Ok(ready_frame(loaded));
            }
        }

        self.stop_server().await?;
        let mut command = Command::new(&self.cfg.server_bin);
        command
            .arg("-m")
            .arg(&model_guest_path)
            .arg("--host")
            .arg(&self.cfg.host)
            .arg("--port")
            .arg(self.cfg.port.to_string())
            .arg("--alias")
            .arg(MODEL_ALIAS)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .stdin(Stdio::null())
            .kill_on_drop(true);
        if let Some(ctx_size) = self.cfg.ctx_size {
            command.arg("-c").arg(ctx_size.to_string());
        }
        for arg in &self.cfg.extra_args {
            command.arg(arg);
        }

        let child = command.spawn().map_err(|error| {
            AgentError::Server(format!(
                "failed to spawn `{}` for resident warm model: {error}",
                self.cfg.server_bin
            ))
        })?;
        self.server = Some(child);
        if let Err(error) = self.wait_until_ready().await {
            let _ = self.stop_server().await;
            return Err(error);
        }

        let loaded = LoadedModel {
            guest_path: model_guest_path,
            artifact_sha256: model_artifact_sha256,
            ready_nonce: uuid::Uuid::now_v7().simple().to_string(),
        };
        let ready = ready_frame(&loaded);
        self.loaded = Some(loaded);
        Ok(ready)
    }

    async fn start_generate(
        &mut self,
        request: WarmAgentGenerateRequest,
        events: &mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<(), AgentError> {
        let Some(loaded) = self.loaded.clone() else {
            self.emit_error(
                Some(&request.request_id),
                "model_not_loaded",
                "generate received before a matching load frame",
            )
            .await?;
            return Ok(());
        };
        if loaded.guest_path != request.model_guest_path
            || loaded.artifact_sha256 != request.model_artifact_sha256
        {
            self.emit_error(
                Some(&request.request_id),
                "model_identity_mismatch",
                "generate model path or sha does not match resident loaded model",
            )
            .await?;
            return Ok(());
        }
        if !self.active.is_empty() {
            self.emit_error(
                Some(&request.request_id),
                "active_generation",
                "warm-agent handles one active generation on the serial channel",
            )
            .await?;
            return Ok(());
        }

        let cancel = CancelFlag::default();
        let request_id = request.request_id.clone();
        self.active.insert(
            request_id.clone(),
            ActiveGeneration {
                cancel: cancel.clone(),
            },
        );
        emit_frame(
            &self.output,
            &WarmAgentGuestFrame::Heartbeat {
                request_id: Some(request_id.clone()),
            },
        )
        .await?;
        let output = Arc::clone(&self.output);
        let client = self.client.clone();
        let cfg = self.cfg.clone();
        let events = events.clone();
        tokio::spawn(async move {
            let result = stream_completion(client, cfg, request, cancel, output).await;
            let _ = events.send(AgentEvent::GenerationDone { request_id, result });
        });
        Ok(())
    }

    async fn emit_error(
        &self,
        request_id: Option<&str>,
        code: &str,
        message: &str,
    ) -> Result<(), AgentError> {
        emit_frame(
            &self.output,
            &WarmAgentGuestFrame::Error {
                request_id: request_id.map(str::to_string),
                code: code.to_string(),
                message: message.to_string(),
            },
        )
        .await
    }

    async fn wait_until_ready(&mut self) -> Result<(), AgentError> {
        let deadline = time::Instant::now() + self.cfg.startup_timeout;
        loop {
            if self.health_ok().await {
                return Ok(());
            }
            if let Some(child) = self.server.as_mut() {
                if let Some(status) = child.try_wait()? {
                    return Err(AgentError::Server(format!(
                        "llama-server exited before /health became ready: {status}"
                    )));
                }
            }
            if time::Instant::now() >= deadline {
                return Err(AgentError::Server(format!(
                    "llama-server did not become ready at {} within {:?}",
                    self.cfg.health_url(),
                    self.cfg.startup_timeout
                )));
            }
            time::sleep(Duration::from_millis(HEALTH_POLL_MS)).await;
        }
    }

    async fn health_ok(&self) -> bool {
        match self.client.get(self.cfg.health_url()).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    async fn shutdown(&mut self) -> Result<(), AgentError> {
        for active in self.active.values() {
            active.cancel.cancel();
        }
        self.active.clear();
        self.stop_server().await
    }

    async fn stop_server(&mut self) -> Result<(), AgentError> {
        let Some(mut child) = self.server.take() else {
            return Ok(());
        };
        let _ = child.kill().await;
        let _ = child.wait().await;
        self.loaded = None;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let result = async {
        let cfg = AgentConfig::from_env()?;
        WarmAgent::new(cfg).run().await
    }
    .await;

    if let Err(error) = result {
        let output = Arc::new(Mutex::new(io::stdout()));
        let _ = emit_frame(
            &output,
            &WarmAgentGuestFrame::Error {
                request_id: None,
                code: "agent_exit".to_string(),
                message: error.to_string(),
            },
        )
        .await;
        std::process::exit(1);
    }
}

async fn read_stdin_frames(sender: mpsc::UnboundedSender<AgentEvent>, persistent_serial: bool) {
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let decoded = decode_warm_agent_frame::<WarmAgentHostFrame>(&line)
                    .map_err(|error| AgentError::Protocol(error.to_string()));
                let _ = sender.send(AgentEvent::HostFrame(decoded));
            }
            Ok(None) => {
                if persistent_serial {
                    eprintln!("warm-agent serial stdin closed; exiting for init-wrapper restart");
                }
                let _ = sender.send(AgentEvent::HostInputClosed);
                return;
            }
            Err(error) => {
                if persistent_serial {
                    eprintln!(
                        "warm-agent serial stdin read failed; exiting for init-wrapper restart"
                    );
                }
                let _ = sender.send(AgentEvent::HostFrame(Err(AgentError::Io(error))));
                let _ = sender.send(AgentEvent::HostInputClosed);
                return;
            }
        }
    }
}

async fn stream_completion(
    client: reqwest::Client,
    cfg: AgentConfig,
    request: WarmAgentGenerateRequest,
    cancel: CancelFlag,
    output: Arc<Mutex<Stdout>>,
) -> Result<(), AgentError> {
    let body = json!({
        "prompt": request.prompt,
        "n_predict": request.max_tokens,
        "stream": true,
        "return_tokens": true,
        "cache_prompt": true
    });
    let send_request = client
        .post(cfg.completion_url())
        .timeout(cfg.request_timeout)
        .json(&body)
        .send();
    tokio::pin!(send_request);
    let response = tokio::select! {
        response = &mut send_request => response?,
        () = wait_for_cancel(cancel.clone()) => {
            emit_cancelled(&output, &request.request_id).await?;
            return Ok(());
        }
    };
    if !response.status().is_success() {
        return Err(AgentError::Server(format!(
            "/completion returned HTTP {}",
            response.status()
        )));
    }

    let mut token_index = 0_u32;
    let mut events = response.bytes_stream().eventsource();
    loop {
        if cancel.is_cancelled() {
            emit_cancelled(&output, &request.request_id).await?;
            return Ok(());
        }

        let next_event =
            match time::timeout(Duration::from_millis(CANCEL_POLL_MS), events.next()).await {
                Ok(item) => item,
                Err(_) => continue,
            };
        let Some(event) = next_event else {
            emit_frame(
                &output,
                &WarmAgentGuestFrame::Error {
                    request_id: Some(request.request_id.clone()),
                    code: "stream_closed".to_string(),
                    message: "llama-server stream closed before stop frame".to_string(),
                },
            )
            .await?;
            return Ok(());
        };
        let event = event.map_err(|error| AgentError::Server(error.to_string()))?;
        let data = event.data.trim();
        if data.is_empty() {
            continue;
        }
        if data == "[DONE]" {
            emit_frame(
                &output,
                &WarmAgentGuestFrame::Complete {
                    request_id: request.request_id.clone(),
                    finish_reason: "stop".to_string(),
                },
            )
            .await?;
            return Ok(());
        }
        let frames = completion_event_to_guest_frames(data, &request.request_id, token_index)?;
        let mut terminal = false;
        for frame in frames {
            if matches!(frame, WarmAgentGuestFrame::Token { .. }) {
                token_index = token_index.saturating_add(1);
            }
            terminal = matches!(frame, WarmAgentGuestFrame::Complete { .. })
                || matches!(frame, WarmAgentGuestFrame::Error { .. });
            emit_frame(&output, &frame).await?;
            if terminal {
                break;
            }
        }
        if terminal {
            return Ok(());
        }
    }
}

async fn wait_for_cancel(cancel: CancelFlag) {
    while !cancel.is_cancelled() {
        time::sleep(Duration::from_millis(CANCEL_POLL_MS)).await;
    }
}

async fn emit_cancelled(output: &Arc<Mutex<Stdout>>, request_id: &str) -> Result<(), AgentError> {
    emit_frame(
        output,
        &WarmAgentGuestFrame::Complete {
            request_id: request_id.to_string(),
            finish_reason: "cancelled".to_string(),
        },
    )
    .await
}

fn completion_event_to_guest_frames(
    raw_json: &str,
    request_id: &str,
    token_index: u32,
) -> Result<Vec<WarmAgentGuestFrame>, AgentError> {
    let value: Value = serde_json::from_str(raw_json)
        .map_err(|error| AgentError::Protocol(format!("invalid llama-server SSE JSON: {error}")))?;
    if let Some(error) = value.get("error") {
        return Ok(vec![WarmAgentGuestFrame::Error {
            request_id: Some(request_id.to_string()),
            code: error
                .get("type")
                .or_else(|| error.get("code"))
                .and_then(Value::as_str)
                .unwrap_or("server_error")
                .to_string(),
            message: error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("llama-server returned an error")
                .to_string(),
        }]);
    }
    let mut frames = Vec::new();
    let content = value
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !content.is_empty() {
        frames.push(WarmAgentGuestFrame::Token {
            request_id: request_id.to_string(),
            token_id: token_id_from_value(&value).unwrap_or(token_index),
            token_index: Some(token_index),
            text: content.to_string(),
        });
    }
    if value.get("stop").and_then(Value::as_bool).unwrap_or(false) {
        frames.push(WarmAgentGuestFrame::Complete {
            request_id: request_id.to_string(),
            finish_reason: finish_reason_from_stop_type(&value),
        });
    }
    Ok(frames)
}

fn token_id_from_value(value: &Value) -> Option<u32> {
    value
        .get("tokens")?
        .as_array()?
        .iter()
        .rev()
        .find_map(|token| token.as_u64())
        .and_then(|token| u32::try_from(token).ok())
}

fn finish_reason_from_stop_type(value: &Value) -> String {
    match value.get("stop_type").and_then(Value::as_str) {
        Some("limit") | Some("length") => "length".to_string(),
        Some("cancelled") | Some("canceled") => "cancelled".to_string(),
        Some("error") => "error".to_string(),
        _ => "stop".to_string(),
    }
}

async fn emit_frame(
    output: &Arc<Mutex<Stdout>>,
    frame: &WarmAgentGuestFrame,
) -> Result<(), AgentError> {
    let encoded =
        encode_warm_agent_frame(frame).map_err(|error| AgentError::Protocol(error.to_string()))?;
    let mut guard = output.lock().await;
    guard.write_all(encoded.as_bytes()).await?;
    guard.flush().await?;
    Ok(())
}

fn ready_frame(loaded: &LoadedModel) -> WarmAgentGuestFrame {
    WarmAgentGuestFrame::Ready {
        protocol_id: WARM_AGENT_PROTOCOL_ID.to_string(),
        protocol_version: WARM_AGENT_PROTOCOL_VERSION,
        agent_id: "hsk-warm-agent-llama-server".to_string(),
        ready_nonce: loaded.ready_nonce.clone(),
        loaded_model_sha256: Some(loaded.artifact_sha256.clone()),
        loaded_model_guest_path: Some(loaded.guest_path.clone()),
    }
}

fn validate_model_path(path: &str) -> Result<(), AgentError> {
    if path.trim() != path || !path.starts_with('/') || path.contains(char::is_whitespace) {
        return Err(AgentError::Protocol(
            "load model_guest_path must be an absolute guest path without whitespace".to_string(),
        ));
    }
    if path.starts_with("/tmp/") {
        return Err(AgentError::Protocol(
            "load model_guest_path must not live under /tmp".to_string(),
        ));
    }
    if !Path::new(path).is_file() {
        return Err(AgentError::Protocol(format!(
            "load model_guest_path does not exist as a file: {path}"
        )));
    }
    Ok(())
}

fn env_u16(name: &str) -> Result<Option<u16>, AgentError> {
    env::var(name)
        .ok()
        .map(|raw| {
            raw.parse::<u16>()
                .map_err(|error| AgentError::Config(format!("{name}={raw}: {error}")))
        })
        .transpose()
}

fn env_u32(name: &str) -> Result<Option<u32>, AgentError> {
    env::var(name)
        .ok()
        .map(|raw| {
            raw.parse::<u32>()
                .map_err(|error| AgentError::Config(format!("{name}={raw}: {error}")))
        })
        .transpose()
}

fn env_u64(name: &str) -> Result<Option<u64>, AgentError> {
    env::var(name)
        .ok()
        .map(|raw| {
            raw.parse::<u64>()
                .map_err(|error| AgentError::Config(format!("{name}={raw}: {error}")))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_sse_token_frame_maps_to_warm_token() {
        let frames = completion_event_to_guest_frames(
            r#"{"content":"hel","tokens":[313],"stop":false}"#,
            "req-1",
            0,
        )
        .expect("parse");

        assert_eq!(
            frames,
            vec![WarmAgentGuestFrame::Token {
                request_id: "req-1".to_string(),
                token_id: 313,
                token_index: Some(0),
                text: "hel".to_string(),
            }]
        );
    }

    #[test]
    fn completion_sse_stop_frame_maps_to_terminal_complete() {
        let frames = completion_event_to_guest_frames(
            r#"{"content":"","tokens":[],"stop":true,"stop_type":"limit"}"#,
            "req-2",
            3,
        )
        .expect("parse");

        assert_eq!(
            frames,
            vec![WarmAgentGuestFrame::Complete {
                request_id: "req-2".to_string(),
                finish_reason: "length".to_string(),
            }]
        );
    }

    #[test]
    fn completion_sse_final_content_and_stop_preserves_last_token() {
        let frames = completion_event_to_guest_frames(
            r#"{"content":"bye","tokens":[77],"stop":true,"stop_type":"eos"}"#,
            "req-final",
            9,
        )
        .expect("parse");

        assert_eq!(
            frames,
            vec![
                WarmAgentGuestFrame::Token {
                    request_id: "req-final".to_string(),
                    token_id: 77,
                    token_index: Some(9),
                    text: "bye".to_string(),
                },
                WarmAgentGuestFrame::Complete {
                    request_id: "req-final".to_string(),
                    finish_reason: "stop".to_string(),
                }
            ]
        );
    }

    #[test]
    fn completion_sse_server_error_maps_to_protocol_error_frame() {
        let frames = completion_event_to_guest_frames(
            r#"{"error":{"type":"invalid_request_error","message":"bad prompt"}}"#,
            "req-3",
            0,
        )
        .expect("parse");

        assert_eq!(
            frames,
            vec![WarmAgentGuestFrame::Error {
                request_id: Some("req-3".to_string()),
                code: "invalid_request_error".to_string(),
                message: "bad prompt".to_string(),
            }]
        );
    }

    #[test]
    fn ready_frame_carries_loaded_model_identity() {
        let loaded = LoadedModel {
            guest_path: "/models/model.gguf".to_string(),
            artifact_sha256: "sha".to_string(),
            ready_nonce: "nonce".to_string(),
        };

        assert_eq!(
            ready_frame(&loaded),
            WarmAgentGuestFrame::Ready {
                protocol_id: WARM_AGENT_PROTOCOL_ID.to_string(),
                protocol_version: WARM_AGENT_PROTOCOL_VERSION,
                agent_id: "hsk-warm-agent-llama-server".to_string(),
                ready_nonce: "nonce".to_string(),
                loaded_model_sha256: Some("sha".to_string()),
                loaded_model_guest_path: Some("/models/model.gguf".to_string()),
            }
        );
    }

    #[test]
    fn model_path_validation_rejects_tmp_and_relative_paths() {
        assert!(validate_model_path("models/model.gguf").is_err());
        assert!(validate_model_path("/tmp/model.gguf").is_err());
        assert!(validate_model_path("/models/with space.gguf").is_err());
    }

    #[test]
    fn default_llama_server_prefers_sibling_binary() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let agent = dir.join(if cfg!(windows) {
            "hsk-warm-agent.exe"
        } else {
            "hsk-warm-agent"
        });
        let server = dir.join(if cfg!(windows) {
            "llama-server.exe"
        } else {
            "llama-server"
        });
        std::fs::write(&agent, b"agent").expect("write agent");
        std::fs::write(&server, b"server").expect("write server");
        assert_eq!(sibling_llama_server_path(&agent), Some(server));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_port_allocation_uses_available_loopback_port() {
        let port = allocate_loopback_port(DEFAULT_HOST).expect("allocate port");
        assert!(port > 0);
    }

    #[test]
    fn explicit_warm_agent_port_env_is_honored() {
        let prior_port = env::var("HSK_WARM_AGENT_PORT").ok();
        let prior_transport = env::var("HSK_WARM_AGENT_TRANSPORT").ok();
        env::set_var("HSK_WARM_AGENT_PORT", "18444");
        env::remove_var("HSK_WARM_AGENT_TRANSPORT");
        let cfg = AgentConfig::from_env().expect("config from env");
        match prior_port {
            Some(value) => env::set_var("HSK_WARM_AGENT_PORT", value),
            None => env::remove_var("HSK_WARM_AGENT_PORT"),
        }
        match prior_transport {
            Some(value) => env::set_var("HSK_WARM_AGENT_TRANSPORT", value),
            None => env::remove_var("HSK_WARM_AGENT_TRANSPORT"),
        }
        assert_eq!(cfg.port, 18_444);
    }

    #[test]
    fn serial_transport_env_enables_persistent_input() {
        let prior = env::var("HSK_WARM_AGENT_TRANSPORT").ok();
        env::set_var("HSK_WARM_AGENT_TRANSPORT", "serial");
        let cfg = AgentConfig::from_env().expect("config from env");
        match prior {
            Some(value) => env::set_var("HSK_WARM_AGENT_TRANSPORT", value),
            None => env::remove_var("HSK_WARM_AGENT_TRANSPORT"),
        }
        assert!(cfg.persistent_serial_input);
    }

    #[tokio::test]
    async fn input_close_waits_for_active_generation_before_shutdown() {
        let mut agent = WarmAgent::new(AgentConfig {
            server_bin: "unused".to_string(),
            host: "127.0.0.1".to_string(),
            port: 9,
            ctx_size: None,
            startup_timeout: Duration::from_millis(50),
            request_timeout: Duration::from_millis(50),
            extra_args: Vec::new(),
            persistent_serial_input: false,
        });
        agent.active.insert(
            "gen-before-eof".to_string(),
            ActiveGeneration {
                cancel: CancelFlag::default(),
            },
        );
        let (sender, mut receiver) = mpsc::unbounded_channel();
        sender
            .send(AgentEvent::HostInputClosed)
            .expect("send input close");
        let loop_events = sender.clone();
        let loop_task = tokio::spawn(async move {
            agent
                .run_event_loop(&mut receiver, loop_events, false)
                .await
        });

        time::sleep(Duration::from_millis(25)).await;
        assert!(
            !loop_task.is_finished(),
            "input close must not shut down while generation is active"
        );
        sender
            .send(AgentEvent::GenerationDone {
                request_id: "gen-before-eof".to_string(),
                result: Ok(()),
            })
            .expect("send generation done");
        let result = time::timeout(Duration::from_secs(1), loop_task)
            .await
            .expect("loop exits after generation completes")
            .expect("loop task joins");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn serial_input_close_cancels_active_generation_and_exits_promptly() {
        let cancel = CancelFlag::default();
        let mut agent = WarmAgent::new(AgentConfig {
            server_bin: "unused".to_string(),
            host: "127.0.0.1".to_string(),
            port: 9,
            ctx_size: None,
            startup_timeout: Duration::from_millis(50),
            request_timeout: Duration::from_millis(50),
            extra_args: Vec::new(),
            persistent_serial_input: true,
        });
        agent.active.insert(
            "serial-gen-before-eof".to_string(),
            ActiveGeneration {
                cancel: cancel.clone(),
            },
        );
        let (sender, mut receiver) = mpsc::unbounded_channel();
        sender
            .send(AgentEvent::HostInputClosed)
            .expect("send serial input close");
        let loop_events = sender.clone();
        let loop_task =
            tokio::spawn(
                async move { agent.run_event_loop(&mut receiver, loop_events, true).await },
            );

        let result = time::timeout(Duration::from_secs(1), loop_task)
            .await
            .expect("serial loop exits promptly after EOF")
            .expect("loop task joins");
        assert!(result.is_ok());
        assert!(cancel.is_cancelled());
    }

    #[tokio::test]
    async fn cancellation_returns_while_http_headers_are_pending() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test listener");
        let addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let Ok((_socket, _peer)) = listener.accept().await else {
                return;
            };
            time::sleep(Duration::from_secs(5)).await;
        });
        let cfg = AgentConfig {
            server_bin: "unused".to_string(),
            host: "127.0.0.1".to_string(),
            port: addr.port(),
            ctx_size: None,
            startup_timeout: Duration::from_millis(50),
            request_timeout: Duration::from_secs(30),
            extra_args: Vec::new(),
            persistent_serial_input: false,
        };
        let cancel = CancelFlag::default();
        let request = WarmAgentGenerateRequest {
            request_id: "cancel-pending-headers".to_string(),
            model_id: "model".to_string(),
            model_guest_path: "/models/model.gguf".to_string(),
            model_artifact_sha256: "sha".to_string(),
            prompt: "hello".to_string(),
            max_tokens: 8,
        };
        let cancel_task = {
            let cancel = cancel.clone();
            tokio::spawn(async move {
                time::sleep(Duration::from_millis(25)).await;
                cancel.cancel();
            })
        };

        let result = time::timeout(
            Duration::from_secs(1),
            stream_completion(
                reqwest::Client::new(),
                cfg,
                request,
                cancel,
                Arc::new(Mutex::new(io::stdout())),
            ),
        )
        .await;

        server.abort();
        cancel_task.abort();
        assert!(matches!(result, Ok(Ok(()))));
    }
}
