use std::{
    collections::BTreeMap,
    net::TcpListener,
    path::{Path, PathBuf},
    time::Duration,
};

use base64::{engine::general_purpose, Engine as _};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::process_ledger::{ProcessEngineKind, ProcessStart};

pub const WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: &str = "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS";
pub const WEBVIEW2_USER_DATA_FOLDER: &str = "WEBVIEW2_USER_DATA_FOLDER";
pub const WEBVIEW2_CDP_SANDBOX_ADAPTER_ID: &str = "webview2-cdp";

#[derive(Debug, Error)]
pub enum CdpClientError {
    #[error("failed to allocate WebView2 CDP port: {0}")]
    PortAllocation(std::io::Error),
    #[error("failed to create WebView2 user data folder {path}: {source}")]
    UserDataFolder {
        path: String,
        source: std::io::Error,
    },
    #[error("CDP HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("CDP websocket failed: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("CDP JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CDP base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("CDP discovery returned no page targets")]
    NoPageTargets,
    #[error("CDP response for request {id} missing result object")]
    MissingResult { id: u64 },
    #[error("CDP response for request {id} returned error: {message}")]
    CdpError { id: u64, message: String },
    #[error("CDP screenshot response did not contain a PNG payload")]
    MissingScreenshotData,
    #[error("CDP screenshot payload was not PNG")]
    InvalidPng,
    #[error("element screenshot scope requires runtime evaluation before Page.captureScreenshot")]
    ElementScopeRequiresRuntimeEvaluation,
    #[error("Runtime.evaluate did not return element bounds")]
    MissingElementBounds,
    #[error("CDP DOM response did not contain a node")]
    MissingDomNode,
    #[error("CDP DOM node contained invalid attributes")]
    InvalidDomAttributes,
    #[error("CDP DOM.querySelector response did not contain nodeId")]
    MissingQuerySelectorNodeId,
    #[error("CDP DOM.querySelector found no element")]
    DomSelectorNotFound,
    #[error("CDP websocket closed before request {id} completed")]
    Closed { id: u64 },
    #[error("CDP console stream event missing required field: {field}")]
    InvalidConsoleEvent { field: &'static str },
}

#[derive(Debug, Clone)]
pub struct VisualDebugLaunchConfig {
    pub remote_debugging_port: u16,
    pub user_data_folder: PathBuf,
    pub env: BTreeMap<String, String>,
    pub ledger_start: ProcessStart,
}

impl VisualDebugLaunchConfig {
    pub fn new(
        user_data_root: impl AsRef<Path>,
        owner_role: impl Into<String>,
        owner_wp: Option<String>,
    ) -> Result<Self, CdpClientError> {
        let remote_debugging_port = allocate_localhost_port()?;
        Self::with_port(
            user_data_root,
            remote_debugging_port,
            std::process::id(),
            "WEBVIEW2-CDP",
            owner_role,
            owner_wp,
        )
    }

    pub fn with_port(
        user_data_root: impl AsRef<Path>,
        remote_debugging_port: u16,
        os_pid: u32,
        parent_session_id: impl Into<String>,
        owner_role: impl Into<String>,
        owner_wp: Option<String>,
    ) -> Result<Self, CdpClientError> {
        let user_data_folder = user_data_root
            .as_ref()
            .join(format!("handshake-webview2-cdp-{remote_debugging_port}"));
        std::fs::create_dir_all(&user_data_folder).map_err(|source| {
            CdpClientError::UserDataFolder {
                path: user_data_folder.to_string_lossy().to_string(),
                source,
            }
        })?;

        let mut env = BTreeMap::new();
        env.insert(
            WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS.to_string(),
            format!("--remote-debugging-port={remote_debugging_port}"),
        );
        env.insert(
            WEBVIEW2_USER_DATA_FOLDER.to_string(),
            user_data_folder.to_string_lossy().to_string(),
        );

        Ok(Self {
            remote_debugging_port,
            user_data_folder,
            env,
            ledger_start: build_webview2_cdp_process_start(
                Some(os_pid),
                parent_session_id,
                owner_role,
                owner_wp,
            ),
        })
    }

    pub fn apply_to_current_process(&self) {
        for (key, value) in &self.env {
            std::env::set_var(key, value);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenshotClip {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScreenshotScope {
    Full,
    Element { selector: String },
    Region(ScreenshotClip),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenshotOptions {
    pub capture_beyond_viewport: bool,
    pub from_surface: bool,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            capture_beyond_viewport: true,
            from_surface: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DomScope {
    Full,
    Selector { selector: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomTree {
    pub root: DomNode,
}

impl DomTree {
    pub fn contains_stable_element_id(&self, stable_element_id: &str) -> bool {
        self.root.contains_stable_element_id(stable_element_id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomNode {
    pub node_id: i64,
    pub node_name: String,
    pub node_type: u8,
    pub attributes: BTreeMap<String, String>,
    pub children: Vec<DomNode>,
    pub stable_element_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConsoleScope {
    All,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConsoleEvent {
    Log {
        level: String,
        message: String,
        timestamp: f64,
    },
    Exception {
        message: String,
        stack: Option<String>,
        timestamp: f64,
    },
    PageError {
        message: String,
        timestamp: f64,
    },
}

pub trait ConsoleSink {
    fn on_event(&mut self, event: ConsoleEvent);
}

impl<F> ConsoleSink for F
where
    F: FnMut(ConsoleEvent),
{
    fn on_event(&mut self, event: ConsoleEvent) {
        self(event);
    }
}

impl DomNode {
    fn contains_stable_element_id(&self, stable_element_id: &str) -> bool {
        self.stable_element_id.as_deref() == Some(stable_element_id)
            || self
                .children
                .iter()
                .any(|child| child.contains_stable_element_id(stable_element_id))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CdpTarget {
    pub id: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub url: Option<String>,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub web_socket_debugger_url: String,
}

#[derive(Debug, Clone)]
pub struct CdpClient {
    http: reqwest::Client,
    base_url: String,
}

impl CdpClient {
    pub fn new(remote_debugging_port: u16) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: format!("http://127.0.0.1:{remote_debugging_port}"),
        }
    }

    pub async fn page_targets(&self) -> Result<Vec<CdpTarget>, CdpClientError> {
        let url = format!("{}/json", self.base_url);
        let targets = self
            .http
            .get(url)
            .send()
            .await?
            .json::<Vec<CdpTarget>>()
            .await?;
        Ok(targets
            .into_iter()
            .filter(|target| target.target_type == "page")
            .collect())
    }

    pub async fn first_page_session(&self) -> Result<CdpPageSession, CdpClientError> {
        let target = self
            .page_targets()
            .await?
            .into_iter()
            .next()
            .ok_or(CdpClientError::NoPageTargets)?;
        Ok(CdpPageSession::new(target.web_socket_debugger_url))
    }
}

#[derive(Debug, Clone)]
pub struct CdpPageSession {
    web_socket_debugger_url: String,
}

impl CdpPageSession {
    pub fn new(web_socket_debugger_url: impl Into<String>) -> Self {
        Self {
            web_socket_debugger_url: web_socket_debugger_url.into(),
        }
    }

    pub async fn capture_screenshot(
        &self,
        scope: ScreenshotScope,
        options: ScreenshotOptions,
    ) -> Result<Vec<u8>, CdpClientError> {
        let scope = match scope {
            ScreenshotScope::Element { selector } => {
                ScreenshotScope::Region(self.element_clip(&selector).await?)
            }
            other => other,
        };
        let result = self
            .send_method(page_capture_request(1, scope, options)?)
            .await?;
        decode_capture_screenshot_result(&result)
    }

    pub async fn runtime_evaluate(&self, expression: &str) -> Result<Value, CdpClientError> {
        self.send_method(json!({
            "id": 1,
            "method": "Runtime.evaluate",
            "params": {
                "expression": expression,
                "awaitPromise": true,
                "returnByValue": true,
            }
        }))
        .await
    }

    pub async fn dom_snapshot(&self, scope: DomScope) -> Result<DomTree, CdpClientError> {
        match scope {
            DomScope::Full => {
                let result = self.send_method(dom_get_document_request(1)).await?;
                decode_dom_snapshot_result(&result)
            }
            DomScope::Selector { selector } => {
                let document = self.send_method(dom_get_document_request(1)).await?;
                let tree = decode_dom_snapshot_result(&document)?;
                let query = self
                    .send_method(dom_query_selector_request(2, tree.root.node_id, &selector))
                    .await?;
                let node_id = decode_query_selector_result(&query)?;
                let described = self
                    .send_method(dom_describe_node_request(3, node_id))
                    .await?;
                decode_dom_snapshot_result(&described)
            }
        }
    }

    pub async fn console_stream<S>(
        &self,
        _scope: ConsoleScope,
        mut sink: S,
    ) -> Result<(), CdpClientError>
    where
        S: ConsoleSink,
    {
        let (mut socket, _) = connect_async(self.web_socket_debugger_url.as_str()).await?;
        socket
            .send(Message::Text(
                serde_json::to_string(&runtime_enable_request(1))?.into(),
            ))
            .await?;

        while let Some(message) = socket.next().await {
            let message = message?;
            let Message::Text(text) = message else {
                continue;
            };
            let raw: Value = serde_json::from_str(&text)?;
            if raw.get("id").and_then(Value::as_u64) == Some(1) {
                continue;
            }
            if let Some(event) = decode_console_stream_event(&raw)? {
                sink.on_event(event);
            }
        }

        Err(CdpClientError::Closed { id: 1 })
    }

    async fn element_clip(&self, selector: &str) -> Result<ScreenshotClip, CdpClientError> {
        let selector_json = serde_json::to_string(selector)?;
        let expression = format!(
            r#"(function() {{
  const element = document.querySelector({selector_json});
  if (!element) {{
    throw new Error("element not found for CDP screenshot");
  }}
  const rect = element.getBoundingClientRect();
  return {{
    x: rect.x,
    y: rect.y,
    width: rect.width,
    height: rect.height,
    scale: window.devicePixelRatio || 1
  }};
}})()"#
        );
        let result = self.runtime_evaluate(&expression).await?;
        let value = result
            .get("result")
            .and_then(|entry| entry.get("value"))
            .cloned()
            .ok_or(CdpClientError::MissingElementBounds)?;
        serde_json::from_value(value).map_err(CdpClientError::from)
    }

    async fn send_method(&self, request: Value) -> Result<Value, CdpClientError> {
        let request_id = request.get("id").and_then(Value::as_u64).unwrap_or(1);
        let (mut socket, _) = connect_async(self.web_socket_debugger_url.as_str()).await?;
        socket
            .send(Message::Text(serde_json::to_string(&request)?.into()))
            .await?;

        while let Some(message) = socket.next().await {
            let message = message?;
            let Message::Text(text) = message else {
                continue;
            };
            let response: Value = serde_json::from_str(&text)?;
            if response.get("id").and_then(Value::as_u64) != Some(request_id) {
                continue;
            }
            return extract_cdp_result(&response, request_id);
        }

        Err(CdpClientError::Closed { id: request_id })
    }
}

pub fn build_webview2_cdp_process_start(
    os_pid: Option<u32>,
    parent_session_id: impl Into<String>,
    owner_role: impl Into<String>,
    owner_wp: Option<String>,
) -> ProcessStart {
    let mut start = ProcessStart::new(ProcessEngineKind::Webview2Cdp, owner_role, owner_wp)
        .with_parent_session_id(parent_session_id)
        .with_sandbox_adapter_id(WEBVIEW2_CDP_SANDBOX_ADAPTER_ID);
    if let Some(os_pid) = os_pid {
        start = start.with_os_pid(os_pid);
    }
    start
}

pub fn page_capture_request(
    id: u64,
    scope: ScreenshotScope,
    options: ScreenshotOptions,
) -> Result<Value, CdpClientError> {
    let mut params = json!({
        "format": "png",
        "captureBeyondViewport": options.capture_beyond_viewport,
        "fromSurface": options.from_surface,
    });

    match scope {
        ScreenshotScope::Full => {}
        ScreenshotScope::Region(clip) => {
            params["clip"] = json!({
                "x": clip.x,
                "y": clip.y,
                "width": clip.width,
                "height": clip.height,
                "scale": clip.scale,
            });
        }
        ScreenshotScope::Element { .. } => {
            return Err(CdpClientError::ElementScopeRequiresRuntimeEvaluation);
        }
    }

    Ok(json!({
        "id": id,
        "method": "Page.captureScreenshot",
        "params": params,
    }))
}

pub fn dom_get_document_request(id: u64) -> Value {
    json!({
        "id": id,
        "method": "DOM.getDocument",
        "params": {
            "depth": -1,
            "pierce": true,
        },
    })
}

pub fn dom_query_selector_request(id: u64, node_id: i64, selector: &str) -> Value {
    json!({
        "id": id,
        "method": "DOM.querySelector",
        "params": {
            "nodeId": node_id,
            "selector": selector,
        },
    })
}

pub fn dom_describe_node_request(id: u64, node_id: i64) -> Value {
    json!({
        "id": id,
        "method": "DOM.describeNode",
        "params": {
            "nodeId": node_id,
            "depth": -1,
            "pierce": true,
        },
    })
}

pub fn runtime_enable_request(id: u64) -> Value {
    json!({
        "id": id,
        "method": "Runtime.enable",
    })
}

pub fn decode_console_stream_event(
    message: &Value,
) -> Result<Option<ConsoleEvent>, CdpClientError> {
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return Ok(None);
    };
    let params = message.get("params").unwrap_or(&Value::Null);
    match method {
        "Runtime.consoleAPICalled" => decode_console_api_called(params).map(Some),
        "Runtime.exceptionThrown" => decode_exception_thrown(params).map(Some),
        _ => Ok(None),
    }
}

pub fn decode_dom_snapshot_response(response: &Value) -> Result<DomTree, CdpClientError> {
    let result = extract_cdp_result(
        response,
        response.get("id").and_then(Value::as_u64).unwrap_or(0),
    )?;
    decode_dom_snapshot_result(&result)
}

pub fn decode_query_selector_node_id(response: &Value) -> Result<i64, CdpClientError> {
    let result = extract_cdp_result(
        response,
        response.get("id").and_then(Value::as_u64).unwrap_or(0),
    )?;
    decode_query_selector_result(&result)
}

pub fn decode_capture_screenshot_response(response: &Value) -> Result<Vec<u8>, CdpClientError> {
    let result = extract_cdp_result(
        response,
        response.get("id").and_then(Value::as_u64).unwrap_or(0),
    )?;
    decode_capture_screenshot_result(&result)
}

fn decode_capture_screenshot_result(result: &Value) -> Result<Vec<u8>, CdpClientError> {
    let data = result
        .get("data")
        .and_then(Value::as_str)
        .ok_or(CdpClientError::MissingScreenshotData)?;
    let bytes = general_purpose::STANDARD.decode(data)?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(CdpClientError::InvalidPng);
    }
    Ok(bytes)
}

fn decode_dom_snapshot_result(result: &Value) -> Result<DomTree, CdpClientError> {
    let node = result
        .get("root")
        .or_else(|| result.get("node"))
        .ok_or(CdpClientError::MissingDomNode)?;
    Ok(DomTree {
        root: decode_dom_node(node)?,
    })
}

fn decode_dom_node(value: &Value) -> Result<DomNode, CdpClientError> {
    let node_id = value
        .get("nodeId")
        .and_then(value_as_i64)
        .ok_or(CdpClientError::MissingDomNode)?;
    let node_name = value
        .get("nodeName")
        .and_then(Value::as_str)
        .ok_or(CdpClientError::MissingDomNode)?
        .to_string();
    let node_type = value
        .get("nodeType")
        .and_then(Value::as_u64)
        .filter(|node_type| *node_type <= u8::MAX as u64)
        .ok_or(CdpClientError::MissingDomNode)? as u8;
    let attributes = decode_dom_attributes(value.get("attributes"))?;
    let mut children = decode_dom_node_array(value.get("children"))?;
    children.extend(decode_dom_node_array(value.get("shadowRoots"))?);
    let stable_element_id = attributes
        .get("data-testid")
        .filter(|value| !value.trim().is_empty())
        .cloned();

    Ok(DomNode {
        node_id,
        node_name,
        node_type,
        attributes,
        children,
        stable_element_id,
    })
}

fn decode_dom_attributes(
    value: Option<&Value>,
) -> Result<BTreeMap<String, String>, CdpClientError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let entries = value
        .as_array()
        .ok_or(CdpClientError::InvalidDomAttributes)?;
    if entries.len() % 2 != 0 {
        return Err(CdpClientError::InvalidDomAttributes);
    }

    let mut attributes = BTreeMap::new();
    for pair in entries.chunks_exact(2) {
        let name = pair[0]
            .as_str()
            .ok_or(CdpClientError::InvalidDomAttributes)?;
        let value = pair[1]
            .as_str()
            .ok_or(CdpClientError::InvalidDomAttributes)?;
        attributes.insert(name.to_string(), value.to_string());
    }
    Ok(attributes)
}

fn decode_dom_node_array(value: Option<&Value>) -> Result<Vec<DomNode>, CdpClientError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let nodes = value.as_array().ok_or(CdpClientError::MissingDomNode)?;
    nodes.iter().map(decode_dom_node).collect()
}

fn decode_query_selector_result(result: &Value) -> Result<i64, CdpClientError> {
    let node_id = result
        .get("nodeId")
        .and_then(value_as_i64)
        .ok_or(CdpClientError::MissingQuerySelectorNodeId)?;
    if node_id == 0 {
        return Err(CdpClientError::DomSelectorNotFound);
    }
    Ok(node_id)
}

fn decode_console_api_called(params: &Value) -> Result<ConsoleEvent, CdpClientError> {
    let level = params
        .get("type")
        .and_then(Value::as_str)
        .ok_or(CdpClientError::InvalidConsoleEvent { field: "type" })?
        .to_string();
    let timestamp = params
        .get("timestamp")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let message = params
        .get("args")
        .and_then(Value::as_array)
        .map(|args| {
            args.iter()
                .filter_map(remote_object_message_part)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| format!("console.{level}"));

    Ok(ConsoleEvent::Log {
        level,
        message,
        timestamp,
    })
}

fn decode_exception_thrown(params: &Value) -> Result<ConsoleEvent, CdpClientError> {
    let details = params
        .get("exceptionDetails")
        .ok_or(CdpClientError::InvalidConsoleEvent {
            field: "exceptionDetails",
        })?;
    let timestamp = params
        .get("timestamp")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let message = details
        .get("text")
        .and_then(Value::as_str)
        .or_else(|| {
            details
                .get("exception")
                .and_then(|exception| exception.get("description"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            details
                .get("exception")
                .and_then(|exception| exception.get("value"))
                .and_then(Value::as_str)
        })
        .unwrap_or("Runtime.exceptionThrown")
        .to_string();
    let stack = details.get("stackTrace").map(Value::to_string).or_else(|| {
        details
            .get("exception")
            .and_then(|exception| exception.get("description"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });

    Ok(ConsoleEvent::Exception {
        message,
        stack,
        timestamp,
    })
}

fn remote_object_message_part(value: &Value) -> Option<String> {
    value
        .get("value")
        .and_then(value_to_string)
        .or_else(|| {
            value
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            value
                .get("unserializableValue")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => Some("null".to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.clone()),
        Value::Array(_) | Value::Object(_) => Some(value.to_string()),
    }
}

fn value_as_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
}

fn extract_cdp_result(response: &Value, id: u64) -> Result<Value, CdpClientError> {
    if let Some(error) = response.get("error") {
        return Err(CdpClientError::CdpError {
            id,
            message: error.to_string(),
        });
    }
    response
        .get("result")
        .cloned()
        .ok_or(CdpClientError::MissingResult { id })
}

fn allocate_localhost_port() -> Result<u16, CdpClientError> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(CdpClientError::PortAllocation)?;
    let port = listener
        .local_addr()
        .map_err(CdpClientError::PortAllocation)?
        .port();
    drop(listener);
    Ok(port)
}
