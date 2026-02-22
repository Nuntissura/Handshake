use std::{
    net::Ipv4Addr,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
};

mod fonts;
mod session_chat_log;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Default)]
struct OrchestratorState {
    child: Mutex<Option<Child>>,
}

impl OrchestratorState {
    fn spawn(&self, workdir: PathBuf) -> std::io::Result<()> {
        let mut guard = self.child.lock().expect("orchestrator mutex poisoned");
        if guard.is_some() {
            return Ok(());
        }

        // DEV-ONLY: spawns handshake_core via cargo run; later we'll replace this with a packaged binary path.
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "--bin", "handshake_core"])
            .current_dir(workdir)
            .env("HANDSHAKE_WORKSPACE_ROOT", workspace_root())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let child = cmd.spawn()?;
        println!("spawned handshake_core via cargo run (pid {})", child.id());
        *guard = Some(child);
        Ok(())
    }

    fn kill(&self) {
        let mut guard = self.child.lock().expect("orchestrator mutex poisoned");
        if let Some(mut child) = guard.take() {
            #[cfg(windows)]
            {
                let _ = Command::new("taskkill")
                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for OrchestratorState {
    fn drop(&mut self) {
        self.kill();
    }
}

fn orchestrator_workdir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("src")
        .join("backend")
        .join("handshake_core")
}

const MD_OUTPUT_ROOT_DIR_SCHEMA_V0: &str = "hsk.output_root_dir@v0";
const MD_OUTPUT_ROOT_DIR_FILENAME: &str = "output_root_dir.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdOutputRootDirConfigV0 {
    schema_version: String,
    output_root_dir: String,
}

const MD_SESSIONS_REGISTRY_SCHEMA_V0: &str = "hsk.media_downloader.sessions@v0";
const MD_SESSIONS_REGISTRY_FILENAME: &str = "media_downloader_sessions.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdSessionsRegistryV0 {
    schema_version: String,
    #[serde(default)]
    sessions: Vec<MdSessionRecordV0>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MdSessionRecordV0 {
    session_id: String,
    kind: String,
    label: String,
    created_at: String,
    #[serde(default)]
    last_used_at: Option<String>,
    allow_private_network: bool,
    #[serde(default)]
    cookie_jar_artifact_ref: Option<serde_json::Value>,
}

fn workspace_root() -> PathBuf {
    if let Ok(value) = std::env::var("HANDSHAKE_WORKSPACE_ROOT") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let tmp = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

fn now_rfc3339() -> Result<String, String> {
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|e| e.to_string())
}

fn default_output_root_dir() -> PathBuf {
    let home = if cfg!(windows) {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    } else {
        std::env::var_os("HOME").map(PathBuf::from)
    };

    let mut base = home
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let docs = base.join("Documents");
    if docs.is_dir() {
        base = docs;
    }

    base.join("Handshake_Output")
}

fn output_root_dir_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join(".handshake")
        .join("gov")
        .join(MD_OUTPUT_ROOT_DIR_FILENAME)
}

fn sessions_registry_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join(".handshake")
        .join("gov")
        .join(MD_SESSIONS_REGISTRY_FILENAME)
}

fn load_or_init_output_root_dir(workspace_root: &Path) -> Result<PathBuf, String> {
    let path = output_root_dir_config_path(workspace_root);
    if path.exists() {
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let config: MdOutputRootDirConfigV0 =
            serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if config.schema_version != MD_OUTPUT_ROOT_DIR_SCHEMA_V0 {
            return Err("output_root_dir.json schema_version mismatch".to_string());
        }
        let trimmed = config.output_root_dir.trim();
        if trimmed.is_empty() {
            return Err("output_root_dir is empty".to_string());
        }
        let dir = PathBuf::from(trimmed);
        if !dir.is_absolute() {
            return Err(format!("output_root_dir is not absolute: {trimmed}"));
        }
        ensure_dir(&dir)?;
        return Ok(dir);
    }

    let default_dir = default_output_root_dir();
    if !default_dir.is_absolute() {
        return Err(format!(
            "default output root dir is not absolute: {}",
            default_dir.display()
        ));
    }
    ensure_dir(&default_dir)?;

    let config = MdOutputRootDirConfigV0 {
        schema_version: MD_OUTPUT_ROOT_DIR_SCHEMA_V0.to_string(),
        output_root_dir: default_dir.to_string_lossy().to_string(),
    };
    write_json_atomic(&path, &config)?;
    Ok(default_dir)
}

fn load_or_init_sessions_registry(workspace_root: &Path) -> Result<MdSessionsRegistryV0, String> {
    let path = sessions_registry_path(workspace_root);
    if !path.exists() {
        let registry = MdSessionsRegistryV0 {
            schema_version: MD_SESSIONS_REGISTRY_SCHEMA_V0.to_string(),
            sessions: Vec::new(),
        };
        write_json_atomic(&path, &registry)?;
        return Ok(registry);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let registry: MdSessionsRegistryV0 = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if registry.schema_version != MD_SESSIONS_REGISTRY_SCHEMA_V0 {
        return Err("media_downloader_sessions.json schema_version mismatch".to_string());
    }
    Ok(registry)
}

fn save_sessions_registry(
    workspace_root: &Path,
    registry: &MdSessionsRegistryV0,
) -> Result<(), String> {
    let path = sessions_registry_path(workspace_root);
    write_json_atomic(&path, registry)
}

fn stage_sessions_root(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let root = app_data.join("stage_sessions");
    ensure_dir(&root)?;
    Ok(root)
}

fn stage_session_data_dir(app: &AppHandle, session_id: &str) -> Result<PathBuf, String> {
    let root = stage_sessions_root(app)?;
    let dir = root.join(session_id);
    ensure_dir(&dir)?;
    Ok(dir)
}

fn is_private_host(host: &str) -> bool {
    let host = host.trim().trim_end_matches('.');
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if host.eq_ignore_ascii_case("0.0.0.0") {
        return true;
    }
    if host.eq_ignore_ascii_case("[::1]") || host.eq_ignore_ascii_case("::1") {
        return true;
    }
    if let Ok(addr) = host.parse::<Ipv4Addr>() {
        let octets = addr.octets();
        if octets[0] == 0 {
            return true;
        }
        if octets[0] == 10 {
            return true;
        }
        if octets[0] == 127 {
            return true;
        }
        if octets[0] == 192 && octets[1] == 168 {
            return true;
        }
        if octets[0] == 172 && (16..=31).contains(&octets[1]) {
            return true;
        }
        if octets[0] == 169 && octets[1] == 254 {
            return true;
        }
        if octets[0] == 100 && (64..=127).contains(&octets[1]) {
            return true;
        }
    }
    false
}

fn navigation_allowed(url: &Url, allow_private_network: bool) -> bool {
    match url.scheme() {
        "http" | "https" => {}
        _ => return false,
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    if allow_private_network {
        return true;
    }
    !is_private_host(host)
}

fn cookie_field_sanitize(input: &str) -> String {
    input
        .replace('\t', " ")
        .replace('\r', "")
        .replace('\n', "")
        .trim()
        .to_string()
}

fn cookie_to_netscape_line(cookie: &tauri::webview::Cookie<'_>) -> Option<String> {
    let domain = cookie.domain()?.trim().trim_end_matches('.');
    if domain.is_empty() {
        return None;
    }
    let mut domain_field = cookie_field_sanitize(domain);
    if !domain_field.starts_with('.') {
        domain_field = format!(".{domain_field}");
    }
    if cookie.http_only().unwrap_or(false) {
        domain_field = format!("#HttpOnly_{domain_field}");
    }

    let include_subdomains = "TRUE";
    let path = cookie_field_sanitize(cookie.path().unwrap_or("/"));
    let secure = if cookie.secure().unwrap_or(false) {
        "TRUE"
    } else {
        "FALSE"
    };

    let expires = cookie
        .expires_datetime()
        .map(|t| t.unix_timestamp())
        .unwrap_or(0);

    let name = cookie_field_sanitize(cookie.name());
    let value = cookie_field_sanitize(cookie.value());

    Some(format!(
        "{domain_field}\t{include_subdomains}\t{path}\t{secure}\t{expires}\t{name}\t{value}"
    ))
}

#[tauri::command]
fn md_output_root_dir_get() -> Result<String, String> {
    let root = workspace_root();
    let dir = load_or_init_output_root_dir(&root)?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
fn md_output_root_dir_set(output_root_dir: String) -> Result<(), String> {
    let root = workspace_root();
    let trimmed = output_root_dir.trim();
    if trimmed.is_empty() {
        return Err("output_root_dir is empty".to_string());
    }
    let dir = PathBuf::from(trimmed);
    if !dir.is_absolute() {
        return Err(format!("output_root_dir is not absolute: {trimmed}"));
    }
    ensure_dir(&dir)?;
    let config = MdOutputRootDirConfigV0 {
        schema_version: MD_OUTPUT_ROOT_DIR_SCHEMA_V0.to_string(),
        output_root_dir: dir.to_string_lossy().to_string(),
    };
    let path = output_root_dir_config_path(&root);
    write_json_atomic(&path, &config)
}

#[tauri::command]
fn md_sessions_list() -> Result<Vec<MdSessionRecordV0>, String> {
    let root = workspace_root();
    let registry = load_or_init_sessions_registry(&root)?;
    Ok(registry.sessions)
}

#[tauri::command]
fn md_session_create(app: AppHandle, label: String) -> Result<MdSessionRecordV0, String> {
    let root = workspace_root();
    let mut registry = load_or_init_sessions_registry(&root)?;

    let label = label.trim().to_string();
    let label = if label.is_empty() {
        format!("Session {}", registry.sessions.len().saturating_add(1))
    } else {
        label
    };

    let session_id = Uuid::new_v4().to_string();
    let created_at = now_rfc3339()?;

    let record = MdSessionRecordV0 {
        session_id: session_id.clone(),
        kind: "stage_session".to_string(),
        label,
        created_at,
        last_used_at: None,
        allow_private_network: false,
        cookie_jar_artifact_ref: None,
    };
    registry.sessions.push(record.clone());
    save_sessions_registry(&root, &registry)?;

    let _ = stage_session_data_dir(&app, &session_id)?;

    Ok(record)
}

#[tauri::command]
fn md_session_open(app: AppHandle, session_id: String, start_url: String) -> Result<(), String> {
    let root = workspace_root();
    let mut registry = load_or_init_sessions_registry(&root)?;
    let record = registry
        .sessions
        .iter_mut()
        .find(|s| s.session_id == session_id && s.kind == "stage_session")
        .ok_or_else(|| "unknown stage_session_id".to_string())?;

    let url = Url::parse(start_url.trim()).map_err(|e| e.to_string())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("start_url must be http(s)".to_string());
    }

    let label = format!("md-stage-session-{}", record.session_id);
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.navigate(url);
        let _ = existing.set_focus();
        record.last_used_at = Some(now_rfc3339()?);
        let _ = save_sessions_registry(&root, &registry);
        return Ok(());
    }

    let data_dir = stage_session_data_dir(&app, &record.session_id)?;
    let allow_private_network = record.allow_private_network;

    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(url))
        .title(format!("Stage Session: {}", record.label))
        .data_directory(data_dir)
        .on_navigation(move |url| navigation_allowed(url, allow_private_network))
        .build()
        .map_err(|e| e.to_string())?;

    record.last_used_at = Some(now_rfc3339()?);
    let _ = save_sessions_registry(&root, &registry);

    Ok(())
}

#[tauri::command]
async fn md_session_export_cookies(app: AppHandle, session_id: String) -> Result<String, String> {
    let root = workspace_root();
    let mut registry = load_or_init_sessions_registry(&root)?;
    let record = registry
        .sessions
        .iter_mut()
        .find(|s| s.session_id == session_id && s.kind == "stage_session")
        .ok_or_else(|| "unknown stage_session_id".to_string())?;

    let label = format!("md-stage-session-{}", record.session_id);
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| "stage session window is not open".to_string())?;

    let cookies = tauri::async_runtime::spawn_blocking(move || window.cookies())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let mut out = String::new();
    out.push_str("# Netscape HTTP Cookie File\n");
    out.push_str("# Exported from Handshake Stage Session\n\n");

    for cookie in &cookies {
        if let Some(line) = cookie_to_netscape_line(cookie) {
            out.push_str(&line);
            out.push('\n');
        }
    }

    // Write to a workspace tmp path so the backend cookie-import job can clean it up.
    let tmp_root = root
        .join(".handshake")
        .join("tmp")
        .join("media_downloader")
        .join("stage_sessions")
        .join(&record.session_id);
    ensure_dir(&tmp_root)?;

    let ts = time::OffsetDateTime::now_utc().unix_timestamp();
    let dest = tmp_root.join(format!("cookies-{ts}.txt"));
    std::fs::write(&dest, out.as_bytes()).map_err(|e| e.to_string())?;

    record.last_used_at = Some(now_rfc3339()?);
    save_sessions_registry(&root, &registry)?;

    Ok(dest.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let _ = fonts::fonts_bootstrap_pack(app.handle().clone(), None);
            let _ = fonts::fonts_list(app.handle().clone());

            let app_data_root = app
                .path()
                .app_data_dir()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            app.manage(session_chat_log::SessionChatLogState::new(app_data_root));

            let state = OrchestratorState::default();
            state.spawn(orchestrator_workdir())?;
            app.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                // Only kill the backend when the main app window is closing; Stage Session windows
                // should not terminate the orchestrator.
                if window.label() == "main" {
                    let state = window.app_handle().state::<OrchestratorState>();
                    state.kill();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            md_output_root_dir_get,
            md_output_root_dir_set,
            md_sessions_list,
            md_session_create,
            md_session_open,
            md_session_export_cookies,
            fonts::fonts_bootstrap_pack,
            fonts::fonts_rebuild_manifest,
            fonts::fonts_list,
            fonts::fonts_import,
            fonts::fonts_remove,
            session_chat_log::session_chat_get_session_id,
            session_chat_log::session_chat_append,
            session_chat_log::session_chat_read,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
