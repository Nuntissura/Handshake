use std::{
    collections::{BTreeMap, HashMap},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::Mutex,
    time::Duration,
};

use base64::{engine::general_purpose, Engine as _};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{Emitter, State, Window};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

pub const IPC_PORT_COMMAND_REF: &str = "kernel.visual_debug.port";
pub const IPC_SCREENSHOT_COMMAND_REF: &str = "kernel.visual_debug.screenshot";
pub const IPC_DOM_SNAPSHOT_COMMAND_REF: &str = "kernel.visual_debug.dom_snapshot";
pub const IPC_CONSOLE_STREAM_START_COMMAND_REF: &str = "kernel.visual_debug.console_stream.start";
pub const IPC_CONSOLE_STREAM_STOP_COMMAND_REF: &str = "kernel.visual_debug.console_stream.stop";
pub const CONSOLE_EVENT_CHANNEL: &str = "console_event";
const WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: &str = "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS";
const WEBVIEW2_USER_DATA_FOLDER: &str = "WEBVIEW2_USER_DATA_FOLDER";

#[derive(Debug)]
pub struct VisualDebugState {
    remote_debugging_port: u16,
    user_data_folder: PathBuf,
    env: BTreeMap<String, String>,
    console_streams: Mutex<HashMap<String, tauri::async_runtime::JoinHandle<()>>>,
}

impl VisualDebugState {
    pub fn initialize() -> Result<Self, String> {
        let root = user_data_root();
        let remote_debugging_port = allocate_localhost_port()?;
        let user_data_folder = root.join(format!("handshake-webview2-cdp-{remote_debugging_port}"));
        std::fs::create_dir_all(&user_data_folder).map_err(|error| error.to_string())?;

        let mut env = BTreeMap::new();
        env.insert(
            WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS.to_string(),
            format!("--remote-debugging-port={remote_debugging_port}"),
        );
        env.insert(
            WEBVIEW2_USER_DATA_FOLDER.to_string(),
            user_data_folder.to_string_lossy().to_string(),
        );
        for (key, value) in &env {
            std::env::set_var(key, value);
        }

        Ok(Self {
            remote_debugging_port,
            user_data_folder,
            env,
            console_streams: Mutex::new(HashMap::new()),
        })
    }

    pub fn remote_debugging_port(&self) -> u16 {
        self.remote_debugging_port
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualDebugScreenshotScope {
    Full,
    Element {
        selector: String,
    },
    Region {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        #[serde(default = "default_scale")]
        scale: f64,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct VisualDebugScreenshotOpts {
    #[serde(default = "default_true")]
    pub capture_beyond_viewport: bool,
    #[serde(default = "default_true")]
    pub from_surface: bool,
}

impl Default for VisualDebugScreenshotOpts {
    fn default() -> Self {
        Self {
            capture_beyond_viewport: true,
            from_surface: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualDebugDomScope {
    Full,
    Selector { selector: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualDebugConsoleScope {
    All,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisualDebugConsoleStreamStartResponse {
    stream_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisualDebugConsoleStreamStopResponse {
    stopped: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisualDebugDomTree {
    root: VisualDebugDomNode,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisualDebugDomNode {
    node_id: i64,
    node_name: String,
    node_type: u8,
    attributes: BTreeMap<String, String>,
    children: Vec<VisualDebugDomNode>,
    stable_element_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisualDebugLaunchConfigSnapshot {
    remote_debugging_port: u16,
    user_data_folder: String,
    env: BTreeMap<String, String>,
    port_command_ref: &'static str,
    screenshot_command_ref: &'static str,
    dom_snapshot_command_ref: &'static str,
    console_stream_start_command_ref: &'static str,
    console_stream_stop_command_ref: &'static str,
    console_event_channel: &'static str,
}

#[tauri::command]
pub fn kernel_visual_debug_launch_config(
    state: State<'_, VisualDebugState>,
) -> VisualDebugLaunchConfigSnapshot {
    VisualDebugLaunchConfigSnapshot {
        remote_debugging_port: state.remote_debugging_port,
        user_data_folder: state.user_data_folder.to_string_lossy().to_string(),
        env: state.env.clone(),
        port_command_ref: IPC_PORT_COMMAND_REF,
        screenshot_command_ref: IPC_SCREENSHOT_COMMAND_REF,
        dom_snapshot_command_ref: IPC_DOM_SNAPSHOT_COMMAND_REF,
        console_stream_start_command_ref: IPC_CONSOLE_STREAM_START_COMMAND_REF,
        console_stream_stop_command_ref: IPC_CONSOLE_STREAM_STOP_COMMAND_REF,
        console_event_channel: CONSOLE_EVENT_CHANNEL,
    }
}

#[tauri::command]
pub fn kernel_visual_debug_port(state: State<'_, VisualDebugState>) -> u16 {
    state.remote_debugging_port()
}

#[tauri::command]
pub async fn kernel_visual_debug_screenshot(
    state: State<'_, VisualDebugState>,
    scope: VisualDebugScreenshotScope,
    opts: Option<VisualDebugScreenshotOpts>,
) -> Result<Vec<u8>, String> {
    let session = CdpSession::connect_first_page(state.remote_debugging_port())
        .await
        .map_err(|error| error.to_string())?;
    session
        .capture_screenshot(scope, opts.unwrap_or_default())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn kernel_visual_debug_dom_snapshot(
    state: State<'_, VisualDebugState>,
    scope: VisualDebugDomScope,
) -> Result<VisualDebugDomTree, String> {
    let session = CdpSession::connect_first_page(state.remote_debugging_port())
        .await
        .map_err(|error| error.to_string())?;
    session.dom_snapshot(scope).await
}

#[tauri::command]
pub async fn kernel_visual_debug_console_stream_start(
    window: Window,
    state: State<'_, VisualDebugState>,
    scope: VisualDebugConsoleScope,
) -> Result<VisualDebugConsoleStreamStartResponse, String> {
    let stream_id = Uuid::now_v7().to_string();
    let session = CdpSession::connect_first_page(state.remote_debugging_port())
        .await
        .map_err(|error| error.to_string())?;
    let task_stream_id = stream_id.clone();
    let task = tauri::async_runtime::spawn(async move {
        let event_sink = WindowConsoleSink {
            window,
            stream_id: task_stream_id,
        };
        let _ = session.console_stream(scope, event_sink).await;
    });

    state
        .console_streams
        .lock()
        .map_err(|_| "console stream registry mutex poisoned".to_string())?
        .insert(stream_id.clone(), task);
    Ok(VisualDebugConsoleStreamStartResponse { stream_id })
}

#[tauri::command]
pub fn kernel_visual_debug_console_stream_stop(
    state: State<'_, VisualDebugState>,
    stream_id: String,
) -> Result<VisualDebugConsoleStreamStopResponse, String> {
    let Some(task) = state
        .console_streams
        .lock()
        .map_err(|_| "console stream registry mutex poisoned".to_string())?
        .remove(&stream_id)
    else {
        return Ok(VisualDebugConsoleStreamStopResponse { stopped: false });
    };
    task.abort();
    Ok(VisualDebugConsoleStreamStopResponse { stopped: true })
}

fn user_data_root() -> PathBuf {
    if let Ok(value) = std::env::var("HANDSHAKE_WEBVIEW2_USER_DATA_ROOT") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    std::env::temp_dir().join("handshake-webview2-user-data")
}

fn default_scale() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
struct CdpTarget {
    #[serde(rename = "type")]
    target_type: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    web_socket_debugger_url: String,
}

struct CdpSession {
    web_socket_debugger_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualDebugConsoleEvent {
    Log {
        stream_id: String,
        level: String,
        message: String,
        timestamp: f64,
    },
    Exception {
        stream_id: String,
        message: String,
        stack: Option<String>,
        timestamp: f64,
    },
    #[allow(dead_code)]
    PageError {
        stream_id: String,
        message: String,
        timestamp: f64,
    },
}

trait ConsoleSink {
    fn on_event(&mut self, event: VisualDebugConsoleEvent);
}

struct WindowConsoleSink {
    window: Window,
    stream_id: String,
}

impl ConsoleSink for WindowConsoleSink {
    fn on_event(&mut self, mut event: VisualDebugConsoleEvent) {
        event.set_stream_id(&self.stream_id);
        let _ = self.window.emit(CONSOLE_EVENT_CHANNEL, event);
    }
}

impl VisualDebugConsoleEvent {
    fn set_stream_id(&mut self, stream_id: &str) {
        match self {
            VisualDebugConsoleEvent::Log {
                stream_id: slot, ..
            }
            | VisualDebugConsoleEvent::Exception {
                stream_id: slot, ..
            }
            | VisualDebugConsoleEvent::PageError {
                stream_id: slot, ..
            } => {
                *slot = stream_id.to_string();
            }
        }
    }
}

impl CdpSession {
    async fn connect_first_page(remote_debugging_port: u16) -> Result<Self, String> {
        let targets =
            tauri::async_runtime::spawn_blocking(move || discover_targets(remote_debugging_port))
                .await
                .map_err(|error| error.to_string())?
                .map_err(|error| error.to_string())?;
        let target = targets
            .into_iter()
            .find(|target| target.target_type == "page")
            .ok_or_else(|| "CDP discovery returned no page targets".to_string())?;
        Ok(Self {
            web_socket_debugger_url: target.web_socket_debugger_url,
        })
    }

    async fn capture_screenshot(
        &self,
        scope: VisualDebugScreenshotScope,
        opts: VisualDebugScreenshotOpts,
    ) -> Result<Vec<u8>, String> {
        let scope = match scope {
            VisualDebugScreenshotScope::Element { selector } => {
                let clip = self.element_clip(&selector).await?;
                VisualDebugScreenshotScope::Region {
                    x: clip.x,
                    y: clip.y,
                    width: clip.width,
                    height: clip.height,
                    scale: clip.scale,
                }
            }
            other => other,
        };
        let result = self.send(page_capture_request(1, scope, opts)?).await?;
        decode_screenshot_result(&result)
    }

    async fn dom_snapshot(&self, scope: VisualDebugDomScope) -> Result<VisualDebugDomTree, String> {
        match scope {
            VisualDebugDomScope::Full => {
                let result = self.send(dom_get_document_request(1)).await?;
                decode_dom_snapshot_result(&result)
            }
            VisualDebugDomScope::Selector { selector } => {
                let document = self.send(dom_get_document_request(1)).await?;
                let tree = decode_dom_snapshot_result(&document)?;
                let query = self
                    .send(dom_query_selector_request(2, tree.root.node_id, &selector))
                    .await?;
                let node_id = decode_query_selector_result(&query)?;
                let described = self.send(dom_describe_node_request(3, node_id)).await?;
                decode_dom_snapshot_result(&described)
            }
        }
    }

    async fn console_stream<S>(
        &self,
        _scope: VisualDebugConsoleScope,
        mut sink: S,
    ) -> Result<(), String>
    where
        S: ConsoleSink,
    {
        let (mut socket, _) = connect_async(self.web_socket_debugger_url.as_str())
            .await
            .map_err(|error| error.to_string())?;
        socket
            .send(Message::Text(
                serde_json::to_string(&runtime_enable_request(1))
                    .map_err(|error| error.to_string())?
                    .into(),
            ))
            .await
            .map_err(|error| error.to_string())?;

        while let Some(message) = socket.next().await {
            let message = message.map_err(|error| error.to_string())?;
            let Message::Text(text) = message else {
                continue;
            };
            let raw: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
            if raw.get("id").and_then(Value::as_u64) == Some(1) {
                continue;
            }
            if let Some(event) = decode_console_stream_event(&raw)? {
                sink.on_event(event);
            }
        }

        Err("CDP websocket closed during console stream".to_string())
    }

    async fn element_clip(&self, selector: &str) -> Result<ScreenshotClip, String> {
        let selector_json = serde_json::to_string(selector).map_err(|error| error.to_string())?;
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
        let result = self
            .send(json!({
                "id": 1,
                "method": "Runtime.evaluate",
                "params": {
                    "expression": expression,
                    "awaitPromise": true,
                    "returnByValue": true,
                }
            }))
            .await?;
        let value = result
            .get("result")
            .and_then(|entry| entry.get("value"))
            .cloned()
            .ok_or_else(|| "Runtime.evaluate did not return element bounds".to_string())?;
        serde_json::from_value(value).map_err(|error| error.to_string())
    }

    async fn send(&self, request: Value) -> Result<Value, String> {
        let request_id = request.get("id").and_then(Value::as_u64).unwrap_or(1);
        let (mut socket, _) = connect_async(self.web_socket_debugger_url.as_str())
            .await
            .map_err(|error| error.to_string())?;
        socket
            .send(Message::Text(
                serde_json::to_string(&request)
                    .map_err(|error| error.to_string())?
                    .into(),
            ))
            .await
            .map_err(|error| error.to_string())?;

        while let Some(message) = socket.next().await {
            let message = message.map_err(|error| error.to_string())?;
            let Message::Text(text) = message else {
                continue;
            };
            let response: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
            if response.get("id").and_then(Value::as_u64) != Some(request_id) {
                continue;
            }
            if let Some(error) = response.get("error") {
                return Err(error.to_string());
            }
            return response
                .get("result")
                .cloned()
                .ok_or_else(|| "CDP response missing result object".to_string());
        }

        Err("CDP websocket closed before request completed".to_string())
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct ScreenshotClip {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale: f64,
}

fn page_capture_request(
    id: u64,
    scope: VisualDebugScreenshotScope,
    opts: VisualDebugScreenshotOpts,
) -> Result<Value, String> {
    let mut params = json!({
        "format": "png",
        "captureBeyondViewport": opts.capture_beyond_viewport,
        "fromSurface": opts.from_surface,
    });

    match scope {
        VisualDebugScreenshotScope::Full => {}
        VisualDebugScreenshotScope::Region {
            x,
            y,
            width,
            height,
            scale,
        } => {
            params["clip"] = json!({
                "x": x,
                "y": y,
                "width": width,
                "height": height,
                "scale": scale,
            });
        }
        VisualDebugScreenshotScope::Element { .. } => {
            return Err("element screenshot scope requires Runtime.evaluate".to_string());
        }
    }

    Ok(json!({
        "id": id,
        "method": "Page.captureScreenshot",
        "params": params,
    }))
}

fn dom_get_document_request(id: u64) -> Value {
    json!({
        "id": id,
        "method": "DOM.getDocument",
        "params": {
            "depth": -1,
            "pierce": true,
        },
    })
}

fn dom_query_selector_request(id: u64, node_id: i64, selector: &str) -> Value {
    json!({
        "id": id,
        "method": "DOM.querySelector",
        "params": {
            "nodeId": node_id,
            "selector": selector,
        },
    })
}

fn dom_describe_node_request(id: u64, node_id: i64) -> Value {
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

fn runtime_enable_request(id: u64) -> Value {
    json!({
        "id": id,
        "method": "Runtime.enable",
    })
}

fn decode_screenshot_result(result: &Value) -> Result<Vec<u8>, String> {
    let data = result
        .get("data")
        .and_then(Value::as_str)
        .ok_or_else(|| "CDP screenshot response missing data".to_string())?;
    let bytes = general_purpose::STANDARD
        .decode(data)
        .map_err(|error| error.to_string())?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("CDP screenshot response was not PNG".to_string());
    }
    Ok(bytes)
}

fn decode_dom_snapshot_result(result: &Value) -> Result<VisualDebugDomTree, String> {
    let node = result
        .get("root")
        .or_else(|| result.get("node"))
        .ok_or_else(|| "CDP DOM response did not contain a node".to_string())?;
    Ok(VisualDebugDomTree {
        root: decode_dom_node(node)?,
    })
}

fn decode_dom_node(value: &Value) -> Result<VisualDebugDomNode, String> {
    let node_id = value
        .get("nodeId")
        .and_then(value_as_i64)
        .ok_or_else(|| "CDP DOM node missing nodeId".to_string())?;
    let node_name = value
        .get("nodeName")
        .and_then(Value::as_str)
        .ok_or_else(|| "CDP DOM node missing nodeName".to_string())?
        .to_string();
    let node_type = value
        .get("nodeType")
        .and_then(Value::as_u64)
        .filter(|node_type| *node_type <= u8::MAX as u64)
        .ok_or_else(|| "CDP DOM node missing nodeType".to_string())? as u8;
    let attributes = decode_dom_attributes(value.get("attributes"))?;
    let mut children = decode_dom_node_array(value.get("children"))?;
    children.extend(decode_dom_node_array(value.get("shadowRoots"))?);
    let stable_element_id = attributes
        .get("data-testid")
        .filter(|value| !value.trim().is_empty())
        .cloned();

    Ok(VisualDebugDomNode {
        node_id,
        node_name,
        node_type,
        attributes,
        children,
        stable_element_id,
    })
}

fn decode_dom_attributes(value: Option<&Value>) -> Result<BTreeMap<String, String>, String> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let entries = value
        .as_array()
        .ok_or_else(|| "CDP DOM node attributes were not an array".to_string())?;
    if entries.len() % 2 != 0 {
        return Err("CDP DOM node attributes must be name/value pairs".to_string());
    }

    let mut attributes = BTreeMap::new();
    for pair in entries.chunks_exact(2) {
        let name = pair[0]
            .as_str()
            .ok_or_else(|| "CDP DOM attribute name was not a string".to_string())?;
        let value = pair[1]
            .as_str()
            .ok_or_else(|| "CDP DOM attribute value was not a string".to_string())?;
        attributes.insert(name.to_string(), value.to_string());
    }
    Ok(attributes)
}

fn decode_dom_node_array(value: Option<&Value>) -> Result<Vec<VisualDebugDomNode>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let nodes = value
        .as_array()
        .ok_or_else(|| "CDP DOM node children were not an array".to_string())?;
    nodes.iter().map(decode_dom_node).collect()
}

fn decode_query_selector_result(result: &Value) -> Result<i64, String> {
    let node_id = result
        .get("nodeId")
        .and_then(value_as_i64)
        .ok_or_else(|| "DOM.querySelector response missing nodeId".to_string())?;
    if node_id == 0 {
        return Err("DOM.querySelector found no element".to_string());
    }
    Ok(node_id)
}

fn decode_console_stream_event(message: &Value) -> Result<Option<VisualDebugConsoleEvent>, String> {
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

fn decode_console_api_called(params: &Value) -> Result<VisualDebugConsoleEvent, String> {
    let level = params
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "Runtime.consoleAPICalled missing type".to_string())?
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

    Ok(VisualDebugConsoleEvent::Log {
        stream_id: String::new(),
        level,
        message,
        timestamp,
    })
}

fn decode_exception_thrown(params: &Value) -> Result<VisualDebugConsoleEvent, String> {
    let details = params
        .get("exceptionDetails")
        .ok_or_else(|| "Runtime.exceptionThrown missing exceptionDetails".to_string())?;
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

    Ok(VisualDebugConsoleEvent::Exception {
        stream_id: String::new(),
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

fn allocate_localhost_port() -> Result<u16, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|error| error.to_string())?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    drop(listener);
    Ok(port)
}

fn discover_targets(remote_debugging_port: u16) -> Result<Vec<CdpTarget>, String> {
    let mut stream = TcpStream::connect(("127.0.0.1", remote_debugging_port)).map_err(|error| {
        format!("failed to connect to WebView2 CDP port {remote_debugging_port}: {error}")
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(b"GET /json HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .map_err(|error| error.to_string())?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| error.to_string())?;
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .ok_or_else(|| "CDP /json response missing HTTP body".to_string())?;
    serde_json::from_str(body).map_err(|error| error.to_string())
}
