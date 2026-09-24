//! MT-008 source fixtures use the real account-scoped backend and assigned artifact owner.

use std::path::{Component, Path, PathBuf};

// Recorded live indexing reached ingestion projection at 14.6s, before reconciliation
// and symbol indexing. Keep setup bounded without raising unrelated request budgets.
#[cfg(feature = "integration")]
const INDEX_SETUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

pub fn external_artifact_dir(subdir: &str) -> PathBuf {
    let root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .expect("runner must supply the verified artifact root");
    assert!(root.is_absolute() && root.is_dir());
    let root = root.canonicalize().expect("resolve verified artifact root");
    let owner = std::env::var_os("HANDSHAKE_TEST_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .expect("runner must supply the assigned artifact owner");
    assert!(owner.is_absolute() && owner.is_dir());
    let owner = owner
        .canonicalize()
        .expect("resolve assigned artifact owner");
    let relative = owner
        .strip_prefix(&root)
        .expect("owner must be inside verified artifact root");
    assert!(
        relative.components().count() >= 3,
        "owner must include WP/MT/owner isolation"
    );
    assert!(Path::new(subdir)
        .components()
        .all(|part| matches!(part, Component::Normal(_))));
    owner.join(subdir)
}

#[cfg(feature = "integration")]
#[path = "../backend_proof_support/mod.rs"]
mod backend_proof_support;

#[cfg(feature = "integration")]
pub struct CodeFixture {
    pub backend: backend_proof_support::LiveBackend,
    pub runtime: tokio::runtime::Runtime,
    pub symbol_entity_id: String,
    root_id: String,
    // Retained evidence: this helper never removes or moves artifact directories/files.
    source_dir: PathBuf,
}

#[cfg(feature = "integration")]
impl CodeFixture {
    pub fn new() -> Self {
        let backend = backend_proof_support::require_live_backend();
        assert!(
            !backend.workspace_id.is_empty(),
            "managed fixture creates its owner workspace"
        );
        let path = external_artifact_dir(&format!(
            "mt008-source-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir(&path).expect("create unique owned source directory");
        std::fs::write(path.join("lib.rs"),
            "/// Adds two numbers.\npub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn caller() -> i32 { add(1, 2) }\n")
            .expect("write owned Rust source");
        let indexed = backend.post_json_with_timeout(
            &format!("/workspaces/{}/code-nav/index", backend.workspace_id),
            &serde_json::json!({"root_path": path.to_string_lossy()}),
            INDEX_SETUP_TIMEOUT,
        );
        assert!(
            indexed["symbol_count"].as_u64().unwrap_or(0) >= 2,
            "real index must contain add and caller: {indexed}"
        );
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("MT008 fixture runtime");
        let client = handshake_native::code_editor::code_nav::CodeNavClient::new(&backend.base)
            .with_authenticated_context(backend.account());
        let symbol = runtime
            .block_on(client.lookup_symbols(&backend.workspace_id, "add", 5))
            .expect("real account-scoped lookup")
            .into_iter()
            .find(|symbol| symbol.display_name == "add")
            .expect("indexed add symbol");
        Self {
            backend,
            runtime,
            symbol_entity_id: symbol.symbol_entity_id,
            root_id: indexed["root_id"]
                .as_str()
                .expect("index root id")
                .to_owned(),
            source_dir: path,
        }
    }

    pub fn client(&self) -> handshake_native::code_editor::code_nav::CodeNavClient {
        handshake_native::code_editor::code_nav::CodeNavClient::new(&self.backend.base)
            .with_authenticated_context(self.backend.account())
    }

    pub fn mark_stale(&self) {
        // The same workspace/root identity observes an empty directory on this pass. Product
        // reconciliation marks A's retained rows stale; neither source files nor artifacts move.
        let empty = self.source_dir.join("empty");
        std::fs::create_dir(&empty).expect("create empty owned reindex directory");
        let indexed = self.backend.post_json_with_timeout(
            &format!("/workspaces/{}/code-nav/index", self.backend.workspace_id),
            &serde_json::json!({"root_path": empty.to_string_lossy()}),
            INDEX_SETUP_TIMEOUT,
        );
        assert_eq!(indexed["root_id"].as_str(), Some(self.root_id.as_str()));
        assert_eq!(indexed["files_ingested"].as_u64(), Some(0));
        assert_eq!(indexed["files_indexed"].as_u64(), Some(0));
        assert_eq!(indexed["files_failed"].as_u64(), Some(0));
        let symbol = self
            .runtime
            .block_on(self.client().get_symbol(&self.symbol_entity_id))
            .expect("retained symbol remains readable after empty-root reconciliation")
            .symbol;
        assert_eq!(symbol.symbol_entity_id, self.symbol_entity_id);
        let staleness = symbol.staleness.expect("persisted staleness is served");
        assert_eq!(staleness.state.as_deref(), Some("marked_stale"));
        assert!(!staleness.fresh);
        assert!(
            self.source_dir.join("lib.rs").is_file(),
            "original source is retained"
        );
    }

    pub fn cleanup(&mut self) {
        self.backend.assert_cleanup();
    }
}
