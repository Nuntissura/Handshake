#![deny(clippy::disallowed_methods)]

use std::{
    net::Ipv4Addr,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

mod fonts;
mod foreground_exception_window;
mod foreground_warning;
mod inspector;
mod manual;
mod quiet_window;
mod session_chat_log;
mod swarm;
mod visual_debug;

mod commands {
    pub mod caa;
    pub mod cloud_lane;
    pub mod distillation;
    pub mod focus_audit;
    pub mod foreground_exception;
    pub mod kv_cache;
    pub mod lora;
    pub mod memory_calibration;
    pub mod memory_capsule;
    pub mod memory_pin;
    pub mod model_runtime;
    pub mod peft;
    pub mod refusal;
    pub mod sandbox;
    pub mod self_improve;
    pub mod session_distill;
    pub mod speculative;
    pub mod steering;
    pub mod subquadratic;
    pub mod swarm_runtime;
    #[cfg(test)]
    pub mod testing;
}

use quiet_window::QuietWindowBuilder;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WindowEvent};
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

macro_rules! handshake_invoke_handlers {
    () => {
        tauri::generate_handler![
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
            foreground_warning::foreground_warning_emit,
            inspector::kernel_inspector_port,
            inspector::kernel_inspector_list_sessions,
            inspector::kernel_inspector_session_state,
            inspector::kernel_inspector_event_ledger_tail,
            inspector::kernel_inspector_process_ledger_active,
            inspector::kernel_inspector_trace_projection,
            inspector::kernel_inspector_loaded_models,
            commands::model_runtime::kernel_model_runtime_capabilities,
            commands::model_runtime::kernel_model_runtime_list_loaded,
            commands::model_runtime::kernel_model_runtime_load,
            commands::model_runtime::kernel_model_runtime_unload,
            commands::swarm_runtime::kernel_swarm_spawn_session,
            commands::swarm_runtime::kernel_swarm_cancel_session,
            commands::swarm_runtime::kernel_swarm_list_active_sessions,
            commands::swarm_runtime::kernel_swarm_resource_snapshot,
            commands::swarm_runtime::kernel_swarm_board_snapshot,
            commands::swarm_runtime::kernel_swarm_chat_generate,
            commands::lora::kernel_model_runtime_lora_mount,
            commands::lora::kernel_model_runtime_lora_unmount,
            commands::lora::kernel_model_runtime_lora_swap,
            commands::lora::kernel_model_runtime_lora_list,
            commands::kv_cache::kernel_model_runtime_kv_set_quantization,
            commands::kv_cache::kernel_model_runtime_kv_prefix_commit,
            commands::kv_cache::kernel_model_runtime_kv_prefix_restore,
            commands::kv_cache::kernel_model_runtime_kv_evict_all,
            commands::kv_cache::kernel_model_runtime_kv_occupancy,
            commands::speculative::kernel_model_runtime_spec_set_mode,
            commands::speculative::kernel_model_runtime_spec_get_mode,
            commands::speculative::kernel_model_runtime_spec_validate,
            commands::subquadratic::kernel_model_runtime_subquad_state_commit,
            commands::subquadratic::kernel_model_runtime_subquad_state_restore,
            commands::subquadratic::kernel_model_runtime_subquad_state_list,
            commands::subquadratic::kernel_model_runtime_subquad_state_evict_all,
            commands::subquadratic::kernel_model_runtime_subquad_persist,
            commands::subquadratic::kernel_model_runtime_subquad_rehydrate,
            commands::memory_capsule::kernel_memory_capsule_list_recent,
            commands::memory_capsule::kernel_memory_capsule_get,
            commands::memory_capsule::kernel_memory_capsule_suppress_item,
            commands::memory_capsule::kernel_memory_capsule_suppress_capsule,
            commands::memory_calibration::kernel_memory_calibration_snapshot,
            commands::memory_pin::kernel_memory_pin_set,
            commands::memory_pin::kernel_memory_pin_unset,
            commands::memory_pin::kernel_memory_pin_list,
            commands::sandbox::kernel_sandbox_list_adapters,
            commands::sandbox::kernel_sandbox_capabilities,
            commands::self_improve::kernel_self_improve_status,
            commands::self_improve::kernel_self_improve_pause,
            commands::self_improve::kernel_self_improve_unpause,
            commands::self_improve::kernel_self_improve_review_pending,
            commands::self_improve::kernel_self_improve_approve_promotion,
            commands::self_improve::kernel_self_improve_reject_promotion,
            commands::steering::kernel_model_runtime_steering_capture,
            commands::steering::kernel_model_runtime_steering_register_vector,
            commands::steering::kernel_model_runtime_steering_set_active,
            commands::steering::kernel_model_runtime_steering_unregister,
            commands::steering::kernel_model_runtime_steering_list_vectors,
            commands::steering::kernel_model_runtime_steering_approve,
            commands::steering::kernel_model_runtime_steering_generate_ab,
            commands::focus_audit::kernel_operator_foreground_focus_audit_start,
            commands::focus_audit::kernel_operator_foreground_focus_audit_stop,
            commands::foreground_exception::foreground_exception_window_open,
            commands::refusal::kernel_model_runtime_refusal_extract,
            commands::caa::kernel_model_runtime_caa_extract,
            commands::session_distill::kernel_session_mark_for_distillation,
            commands::session_distill::kernel_session_get_distill_flag,
            commands::distillation::list_distill_sessions,
            commands::distillation::list_distill_candidates,
            commands::distillation::list_distill_jobs,
            commands::distillation::extract_distill_corpus,
            commands::distillation::promote_distill_candidate,
            commands::distillation::reject_distill_candidate,
            commands::peft::start_peft_training_job,
            commands::cloud_lane::list_cloud_lanes,
            commands::cloud_lane::register_cloud_lane,
            commands::cloud_lane::remove_cloud_lane,
            commands::cloud_lane::toggle_cloud_lane,
            commands::cloud_lane::store_api_key,
            commands::cloud_lane::rotate_api_key,
            commands::cloud_lane::delete_api_key,
            commands::cloud_lane::list_stored_keys,
            commands::cloud_lane::grant_consent,
            commands::cloud_lane::deny_consent,
            manual::model_manual_get,
            manual::model_manual_list_commands,
            manual::model_manual_search,
            session_chat_log::session_chat_get_session_id,
            session_chat_log::session_chat_append,
            session_chat_log::session_chat_read,
            visual_debug::kernel_visual_debug_launch_config,
            visual_debug::kernel_visual_debug_port,
            visual_debug::kernel_visual_debug_screenshot,
            visual_debug::kernel_visual_debug_dom_snapshot,
            visual_debug::kernel_visual_debug_console_stream_start,
            visual_debug::kernel_visual_debug_console_stream_stop,
        ]
    };
    ($($extra:path),+ $(,)?) => {
        tauri::generate_handler![
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
            foreground_warning::foreground_warning_emit,
            inspector::kernel_inspector_port,
            inspector::kernel_inspector_list_sessions,
            inspector::kernel_inspector_session_state,
            inspector::kernel_inspector_event_ledger_tail,
            inspector::kernel_inspector_process_ledger_active,
            inspector::kernel_inspector_trace_projection,
            inspector::kernel_inspector_loaded_models,
            commands::model_runtime::kernel_model_runtime_capabilities,
            commands::model_runtime::kernel_model_runtime_list_loaded,
            commands::model_runtime::kernel_model_runtime_load,
            commands::model_runtime::kernel_model_runtime_unload,
            commands::swarm_runtime::kernel_swarm_spawn_session,
            commands::swarm_runtime::kernel_swarm_cancel_session,
            commands::swarm_runtime::kernel_swarm_list_active_sessions,
            commands::swarm_runtime::kernel_swarm_resource_snapshot,
            commands::swarm_runtime::kernel_swarm_board_snapshot,
            commands::swarm_runtime::kernel_swarm_chat_generate,
            commands::lora::kernel_model_runtime_lora_mount,
            commands::lora::kernel_model_runtime_lora_unmount,
            commands::lora::kernel_model_runtime_lora_swap,
            commands::lora::kernel_model_runtime_lora_list,
            commands::kv_cache::kernel_model_runtime_kv_set_quantization,
            commands::kv_cache::kernel_model_runtime_kv_prefix_commit,
            commands::kv_cache::kernel_model_runtime_kv_prefix_restore,
            commands::kv_cache::kernel_model_runtime_kv_evict_all,
            commands::kv_cache::kernel_model_runtime_kv_occupancy,
            commands::speculative::kernel_model_runtime_spec_set_mode,
            commands::speculative::kernel_model_runtime_spec_get_mode,
            commands::speculative::kernel_model_runtime_spec_validate,
            commands::subquadratic::kernel_model_runtime_subquad_state_commit,
            commands::subquadratic::kernel_model_runtime_subquad_state_restore,
            commands::subquadratic::kernel_model_runtime_subquad_state_list,
            commands::subquadratic::kernel_model_runtime_subquad_state_evict_all,
            commands::subquadratic::kernel_model_runtime_subquad_persist,
            commands::subquadratic::kernel_model_runtime_subquad_rehydrate,
            commands::memory_capsule::kernel_memory_capsule_list_recent,
            commands::memory_capsule::kernel_memory_capsule_get,
            commands::memory_capsule::kernel_memory_capsule_suppress_item,
            commands::memory_capsule::kernel_memory_capsule_suppress_capsule,
            commands::memory_calibration::kernel_memory_calibration_snapshot,
            commands::memory_pin::kernel_memory_pin_set,
            commands::memory_pin::kernel_memory_pin_unset,
            commands::memory_pin::kernel_memory_pin_list,
            commands::sandbox::kernel_sandbox_list_adapters,
            commands::sandbox::kernel_sandbox_capabilities,
            commands::self_improve::kernel_self_improve_status,
            commands::self_improve::kernel_self_improve_pause,
            commands::self_improve::kernel_self_improve_unpause,
            commands::self_improve::kernel_self_improve_review_pending,
            commands::self_improve::kernel_self_improve_approve_promotion,
            commands::self_improve::kernel_self_improve_reject_promotion,
            commands::steering::kernel_model_runtime_steering_capture,
            commands::steering::kernel_model_runtime_steering_register_vector,
            commands::steering::kernel_model_runtime_steering_set_active,
            commands::steering::kernel_model_runtime_steering_unregister,
            commands::steering::kernel_model_runtime_steering_list_vectors,
            commands::steering::kernel_model_runtime_steering_approve,
            commands::steering::kernel_model_runtime_steering_generate_ab,
            commands::focus_audit::kernel_operator_foreground_focus_audit_start,
            commands::focus_audit::kernel_operator_foreground_focus_audit_stop,
            commands::foreground_exception::foreground_exception_window_open,
            commands::refusal::kernel_model_runtime_refusal_extract,
            commands::caa::kernel_model_runtime_caa_extract,
            commands::session_distill::kernel_session_mark_for_distillation,
            commands::session_distill::kernel_session_get_distill_flag,
            commands::distillation::list_distill_sessions,
            commands::distillation::list_distill_candidates,
            commands::distillation::list_distill_jobs,
            commands::distillation::extract_distill_corpus,
            commands::distillation::promote_distill_candidate,
            commands::distillation::reject_distill_candidate,
            commands::peft::start_peft_training_job,
            commands::cloud_lane::list_cloud_lanes,
            commands::cloud_lane::register_cloud_lane,
            commands::cloud_lane::remove_cloud_lane,
            commands::cloud_lane::toggle_cloud_lane,
            commands::cloud_lane::store_api_key,
            commands::cloud_lane::rotate_api_key,
            commands::cloud_lane::delete_api_key,
            commands::cloud_lane::list_stored_keys,
            commands::cloud_lane::grant_consent,
            commands::cloud_lane::deny_consent,
            manual::model_manual_get,
            manual::model_manual_list_commands,
            manual::model_manual_search,
            session_chat_log::session_chat_get_session_id,
            session_chat_log::session_chat_append,
            session_chat_log::session_chat_read,
            visual_debug::kernel_visual_debug_launch_config,
            visual_debug::kernel_visual_debug_port,
            visual_debug::kernel_visual_debug_screenshot,
            visual_debug::kernel_visual_debug_dom_snapshot,
            visual_debug::kernel_visual_debug_console_stream_start,
            visual_debug::kernel_visual_debug_console_stream_stop,
            $($extra),+
        ]
    };
}

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
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:65432/handshake_test".to_string()
        });
        cmd.args([
            "run",
            "--features",
            "app-runtime",
            "--bin",
            "handshake_core",
        ])
        .current_dir(workdir)
        .env("HANDSHAKE_WORKSPACE_ROOT", workspace_root())
        .env("DATABASE_URL", database_url)
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

fn peft_job_spawner_state() -> commands::peft::PeftJobSpawnerState {
    // MT-122: discovery uses the workspace root + the bundled
    // `scripts/distill/train_lora.py`. If the Python interpreter isn't on
    // PATH or the script isn't bundled (rare; only happens in detached
    // build trees), construct a state pointing at non-existent paths so
    // `start_peft_training_job` returns a real TrainerUnavailable error.
    // The error is the real subprocess error, not a placeholder.
    let repo_root = workspace_root();
    match commands::peft::PeftJobSpawnerState::from_repo_discovery(&repo_root) {
        Ok(state) => state,
        Err(error) => {
            eprintln!(
                "MT-122 peft job spawner state init failed: {error}; \
                 commands will surface real TrainerUnavailable errors until repaired"
            );
            commands::peft::PeftJobSpawnerState::new(
                PathBuf::from("python"),
                repo_root
                    .join("scripts")
                    .join("distill")
                    .join("train_lora.py"),
            )
        }
    }
}

fn distillation_jobs_state_from_app_data_root(
    app_data_root: &Path,
) -> Result<commands::distillation::DistillationJobsState, String> {
    let recorder_root = app_data_root.join("flight_recorder");
    std::fs::create_dir_all(&recorder_root).map_err(|error| {
        format!(
            "create Tauri distillation flight recorder root {}: {error}",
            recorder_root.display()
        )
    })?;
    let recorder_path = recorder_root.join("distillation_jobs.duckdb");
    let recorder = handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder::new_on_path(
        &recorder_path,
        7,
    )
    .map_err(|error| {
        format!(
            "open Tauri distillation flight recorder {}: {error}",
            recorder_path.display()
        )
    })?;
    let recorder: Arc<dyn handshake_core::flight_recorder::FlightRecorder> = Arc::new(recorder);
    Ok(commands::distillation::DistillationJobsState::new(recorder))
}

fn steering_vector_store_state() -> Result<commands::steering::SteeringVectorStoreState, String> {
    let workspace = workspace_root();
    let root = workspace.join(".handshake").join("steering_vector_store");
    std::fs::create_dir_all(&root)
        .map_err(|error| format!("create steering vector store root {root:?}: {error}"))?;
    let store = Arc::new(
        handshake_core::model_runtime::techniques::steering_vector_store::SteeringVectorStore::new(
            root,
        ),
    );
    Ok(commands::steering::SteeringVectorStoreState::new(store))
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
        .replace(['\r', '\n'], "")
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

    let session_id = Uuid::now_v7().to_string();
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
async fn md_session_open(
    app: AppHandle,
    session_id: String,
    start_url: String,
) -> Result<(), String> {
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
        record.last_used_at = Some(now_rfc3339()?);
        let _ = save_sessions_registry(&root, &registry);
        return Ok(());
    }

    let data_dir = stage_session_data_dir(&app, &record.session_id)?;
    let allow_private_network = record.allow_private_network;

    QuietWindowBuilder::new(&app, label, WebviewUrl::External(url))
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
    let visual_debug_state = visual_debug::VisualDebugState::initialize()
        .expect("WebView2 CDP visual debug launch config");
    let inspector_reader: Arc<dyn handshake_core::inspector_read::InspectorReadV1> =
        Arc::new(handshake_core::inspector_read::InspectorReadSnapshot::default());
    let sandbox_registry = Arc::new(
        handshake_core::sandbox::build_default_registry()
            .expect("sandbox adapter registry bootstrap failed"),
    );

    // MT-129: Cloud Lane IPC state wires the frontend control panel to
    // the production `OsKeychainSecretsVault` (Windows Credential
    // Manager via the `keyring` crate) + a fresh `ConsentGate`. The
    // in-memory lane registration registry lives inside
    // `CloudLaneIpcState` and is rebuilt on app start (the OS
    // keychain has no enumeration API; lane discovery is via the
    // registry).
    let cloud_lane_vault: Arc<dyn handshake_core::model_runtime::cloud::SecretsVault> = Arc::new(
        handshake_core::model_runtime::cloud::secrets_vault::OsKeychainSecretsVault::new(
            handshake_core::model_runtime::cloud::secrets_vault::HANDSHAKE_KEYCHAIN_SERVICE,
        ),
    );
    let cloud_lane_consent_gate =
        Arc::new(handshake_core::model_runtime::cloud::ConsentGate::default());
    let cloud_lane_state =
        commands::cloud_lane::CloudLaneIpcState::new(cloud_lane_vault, cloud_lane_consent_gate);

    // MT-068 + MT-096: the steering vector store backs the
    // `kernel_model_runtime_steering_register_vector` command with the
    // production `SteeringVectorStore` (MT-097). The store root lives under
    // the workspace `Handshake_Output/steering-vectors` directory so it is
    // visible to operators and follows the disk-agnostic portability policy.
    // When the workspace path cannot be resolved we fall back to a detached
    // store state; commands then dispatch through the live adapter without
    // persistence.
    let steering_store_state = match steering_vector_store_state() {
        Ok(state) => state,
        Err(error) => {
            eprintln!("steering vector store init failed: {error}; falling back to detached store");
            commands::steering::SteeringVectorStoreState::detached()
        }
    };

    let builder = tauri::Builder::default()
        .manage(visual_debug_state)
        .manage(inspector::InspectorPortState::new(None))
        .manage(inspector_reader)
        .manage(commands::model_runtime::ModelRuntimeState::default())
        .manage(commands::speculative::SpeculativeModeOverrides::default())
        .manage(steering_store_state)
        .manage(commands::memory_capsule::MemoryCapsuleIpcState::default())
        .manage(commands::memory_calibration::MemoryCalibrationIpcState::from_env_or_unavailable())
        .manage(commands::memory_pin::MemoryPinIpcState::from_env_or_unavailable())
        .manage(commands::session_distill::SessionDistillState::default())
        // MT-124: Distillation Queue UI (DistillationQueue.tsx) reads
        // through the production CandidateRegistry (MT-123). The
        // app-data-root FlightRecorder is managed in setup after Tauri
        // resolves the platform data directory.
        .manage(commands::distillation::DistillationCandidateState::default())
        // MT-122: PEFT training job spawner state. The Python interpreter +
        // trainer script are resolved at construction time from the workspace
        // root. If resolution fails the runtime falls back to a detached
        // state whose `start_peft_training_job` returns a real error rather
        // than a placeholder result.
        .manage(peft_job_spawner_state())
        .manage(cloud_lane_state)
        .manage(sandbox_registry)
        // MT-155: self-improvement loop IPC. Default state holds a fresh
        // LoopIpcState (24h iteration budget = 25 per HBR-SWARM-002). The
        // production PromotionGateSubmitter wiring lives in a follow-on MT
        // alongside the scheduler binding (MT-156); until then the approve
        // command returns a typed GateUnavailable error so the operator
        // gets a real failure instead of a placeholder success.
        .manage(commands::self_improve::SelfImproveIpcState::default())
        // MT-027: focus-audit IPC state holds the live `FocusAuditHandle`
        // map across the start -> (visual run) -> stop round-trip so the a2
        // smoke can drive the real Win32 foreground audit instead of asserting
        // a hardcoded empty `handshake_owned_events` vector.
        .manage(commands::focus_audit::FocusAuditIpcState::new())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let _ = fonts::fonts_bootstrap_pack(app.handle().clone(), None);
            let _ = fonts::fonts_list(app.handle().clone());

            let app_data_root = app
                .path()
                .app_data_dir()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            // rank-3: a DuckDB Flight Recorder for the swarm, so deployed
            // swarm/VM lifecycle events persist + are queryable (board drill-down
            // + audit). Best-effort: on init failure the swarm falls back to the
            // structured-stderr FR sink rather than failing app startup.
            let swarm_recorder: Option<Arc<dyn handshake_core::flight_recorder::FlightRecorder>> = {
                let _ = std::fs::create_dir_all(&app_data_root);
                let path = app_data_root.join("swarm_flight_recorder.duckdb");
                match handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder::new_on_path(
                    &path, 7,
                ) {
                    Ok(rec) => Some(Arc::new(rec)),
                    Err(error) => {
                        eprintln!("swarm flight recorder init failed ({error}); using stderr sink");
                        None
                    }
                }
            };
            let distillation_jobs_state =
                distillation_jobs_state_from_app_data_root(&app_data_root).map_err(|error| {
                    std::io::Error::other(format!(
                        "MT-124 distillation jobs FlightRecorder init failed: {error}"
                    ))
                })?;
            app.manage(session_chat_log::SessionChatLogState::new(app_data_root));
            app.manage(distillation_jobs_state);

            // MT-204: the production multi-model SWARM coordinator. Built inside
            // `setup` so the background ledger writer + lease reaper spawn into
            // the live Tauri async runtime. Later swarm commands (MT-205) and the
            // cloud routing policy (MT-206) drive this managed coordinator.
            let swarm_state = match swarm_recorder {
                Some(recorder) => {
                    commands::swarm_runtime::SwarmRuntimeState::production_with_fr_recorder(recorder)
                }
                None => commands::swarm_runtime::SwarmRuntimeState::production(),
            };
            // rank-4: start the single board forwarder (broadcast -> typed
            // swarm://event deltas) before managing the state moves it.
            let board_events = swarm_state.board_events();
            app.manage(swarm_state);
            commands::swarm_runtime::spawn_swarm_board_forwarder(
                app.handle().clone(),
                board_events,
            );

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
        });

    #[cfg(all(debug_assertions, feature = "swarm_ipc"))]
    let builder = builder.invoke_handler(handshake_invoke_handlers!(swarm::kernel_swarm_run));

    #[cfg(not(all(debug_assertions, feature = "swarm_ipc")))]
    let builder = builder.invoke_handler(handshake_invoke_handlers!());

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::flight_recorder::{
        EventFilter, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    };
    use serde_json::json;

    #[tokio::test]
    async fn distillation_jobs_state_from_app_data_root_attaches_writable_duckdb_recorder() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = distillation_jobs_state_from_app_data_root(tmp.path())
            .expect("app data root should create a writable distillation jobs recorder");
        let recorder = state
            .recorder()
            .expect("recorder should be attached")
            .clone();

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::DistillPiiDetected,
            FlightRecorderActor::System,
            Uuid::now_v7(),
            json!({
                "type": "distill.pii_detected",
                "fr_event_id": "FR-EVT-DISTILL-PII-DETECT",
                "turn_id": "turn-app-root-recorder",
                "pii_kinds": ["email"],
                "severity": "High"
            }),
        )
        .with_job_id("job-app-root-recorder");

        recorder
            .record_event(event)
            .await
            .expect("app recorder should accept valid distill event");
        let events = recorder
            .list_events(EventFilter {
                job_id: Some("job-app-root-recorder".to_string()),
                ..EventFilter::default()
            })
            .await
            .expect("app recorder should list recorded distill events");

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].event_type,
            FlightRecorderEventType::DistillPiiDetected
        );
    }
}
