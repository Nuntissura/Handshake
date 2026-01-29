use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
};

mod fonts;
mod session_chat_log;

use tauri::{Manager, WindowEvent};

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
                let state = window.app_handle().state::<OrchestratorState>();
                state.kill();
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
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
