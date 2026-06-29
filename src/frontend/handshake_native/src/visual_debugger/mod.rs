//! WP-KERNEL-012 MT-102 — worksurface/window structure inspector.
//!
//! This module is a diagnostic serializer, not a worksurface feature pane. It composes the live pane
//! registry, the existing layout snapshot, and the existing AccessKit widget tree into one JSON
//! artifact a no-context model can inspect without guessing which panes, widgets, or layout state are
//! mounted.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::accessibility::{UiNodeBounds, UiTreeNode, UiTreeSnapshot};
use crate::layout_persistence::LayoutSnapshot;
use crate::pane_registry::{DirtyState, LockState, PaneAuthority, PaneType};

pub const WORKSURFACE_SNAPSHOT_SCHEMA_ID: &str = "hsk.native_worksurface_inspector@1";
pub const WORKSURFACE_SNAPSHOT_VERSION: u32 = 1;

/// The MT-102 diagnostic marker intentionally uses `DiagEventCode::Other` because the diag-ring crate
/// is a reuse-only boundary for this MT. The numeric payload is pinned here: counter_a = panes,
/// counter_b = widgets.
pub const WORKSURFACE_DIAG_EVENT_CODE: &str = "Other";
pub const WORKSURFACE_DIAG_PHASE: &str = "End";
pub const WORKSURFACE_DIAG_SEVERITY: &str = "Info";

/// Stable AccessKit author_id for the Settings -> Diagnostics dump button.
pub const WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID: &str =
    "settings.diagnostics.worksurface-inspector.dump";

/// Stable AccessKit author_id for the last dump status row.
pub const WORKSURFACE_INSPECTOR_STATUS_AUTHOR_ID: &str =
    "settings.diagnostics.worksurface-inspector.status";

static CAPTURE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// A single structured dump of the live worksurface/window state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorksurfaceSnapshot {
    pub schema_id: String,
    pub version: u32,
    pub capture_id: String,
    pub captured_at_utc: String,
    pub pane_tree: Vec<PaneInspection>,
    pub widget_inventory: WidgetInventory,
    pub layout_tree: LayoutTreeInspection,
    pub screenshot: ScreenshotEvidence,
    pub internal_diagnostics: InternalDiagnosticsEvent,
}

/// One mounted pane from the registry, annotated with the current location and AccessKit node id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneInspection {
    pub pane_id: String,
    pub pane_type: String,
    pub pane_type_label: String,
    pub content_id: Option<String>,
    pub lock_state: String,
    pub dirty: String,
    pub authority: String,
    pub authority_agent_id: Option<String>,
    pub accesskit_node_id: Option<u64>,
    pub location: PaneLocation,
}

/// Where the pane is currently mounted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PaneLocation {
    MainSplit,
    PoppedOut { open: bool },
}

/// Compact flattened inventory plus the full nested AccessKit tree, copied into MT-102-owned DTOs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetInventory {
    pub widget_count: usize,
    pub author_id_count: usize,
    pub nodes: Vec<WidgetNodeInventory>,
    pub tree: WidgetTreeNode,
}

/// One AccessKit node entry flattened for fast scanning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetNodeInventory {
    pub id: String,
    pub author_id: Option<String>,
    pub node_id: u64,
    pub role: String,
    pub label: Option<String>,
    pub actions: Vec<String>,
    pub child_count: usize,
}

/// MT-102-owned nested UI tree node. This pins the inspector schema instead of embedding
/// `accessibility::UiTreeNode` directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetTreeNode {
    pub id: String,
    pub author_id: Option<String>,
    pub node_id: u64,
    pub role: String,
    pub label: Option<String>,
    pub value: Option<String>,
    pub disabled: bool,
    pub actions: Vec<String>,
    pub bounds: Option<WidgetNodeBounds>,
    pub children: Vec<WidgetTreeNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WidgetNodeBounds {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Layout state from the existing native layout snapshot, copied into MT-102-owned DTOs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutTreeInspection {
    pub schema_id: String,
    pub version: u32,
    pub split_weights: SplitWeightsInspection,
    pub active_pane: Option<String>,
    pub active_module: String,
    pub pane_count: usize,
    pub tab_bar_count: usize,
    pub pop_out_count: usize,
    pub panes: Vec<LayoutPaneSummary>,
    pub tab_bars: Vec<TabBarInspection>,
    pub pop_outs: Vec<PopOutInspection>,
    pub drawers: DrawersInspection,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SplitWeightsInspection {
    pub vertical: f32,
    pub horizontal: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutPaneSummary {
    pub pane_id: String,
    pub pane_type: String,
    pub content_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabBarInspection {
    pub pane_id: String,
    pub active_index: usize,
    pub tabs: Vec<TabInspection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabInspection {
    pub index: usize,
    pub pane_type: String,
    pub label: String,
    pub content_id: Option<String>,
    pub pinned: bool,
    pub dirty: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PopOutInspection {
    pub pane_id: String,
    pub open: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawersInspection {
    pub project: bool,
    pub bottom: bool,
}

/// Screenshot evidence for the dump. Headless test runs must be honest rather than emitting a fake PNG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ScreenshotEvidence {
    Captured {
        path: PathBuf,
        width: u32,
        height: u32,
    },
    Deferred {
        marker: String,
        reason: String,
    },
}

impl ScreenshotEvidence {
    pub fn deferred_headless_gpu() -> Self {
        Self::Deferred {
            marker: "screenshot_deferred_headless_gpu".to_owned(),
            reason: "the existing MCP screenshot path was unavailable on this headless/no-window host; run the same dump from a real Handshake window/GPU host to capture worksurface-screenshot.png".to_owned(),
        }
    }

    pub fn deferred_from_mcp_error(error: impl std::fmt::Display) -> Self {
        Self::Deferred {
            marker: "screenshot_deferred_headless_gpu".to_owned(),
            reason: format!(
                "the existing MCP screenshot path reported '{error}' on this headless/no-window host; run the same dump from a real Handshake window/GPU host to capture worksurface-screenshot.png"
            ),
        }
    }
}

/// Numeric marker mirrored to `internal_diagnostics`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalDiagnosticsEvent {
    pub event_code: String,
    pub phase: String,
    pub severity: String,
    pub counter_a_name: String,
    pub counter_a_value: u64,
    pub counter_b_name: String,
    pub counter_b_value: u64,
    pub timestamp_nanos: u64,
}

impl InternalDiagnosticsEvent {
    pub fn worksurface_inspected(pane_count: usize, widget_count: usize) -> Self {
        Self {
            event_code: WORKSURFACE_DIAG_EVENT_CODE.to_owned(),
            phase: WORKSURFACE_DIAG_PHASE.to_owned(),
            severity: WORKSURFACE_DIAG_SEVERITY.to_owned(),
            counter_a_name: "pane_count".to_owned(),
            counter_a_value: pane_count as u64,
            counter_b_name: "widget_count".to_owned(),
            counter_b_value: widget_count as u64,
            timestamp_nanos: monotonicish_timestamp_nanos(),
        }
    }
}

/// Receipt for a JSON snapshot written to an external artifact directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotWriteReceipt {
    pub path: PathBuf,
    pub bytes: u64,
}

#[derive(Debug, Default)]
pub struct WorksurfaceInspector;

impl WorksurfaceInspector {
    pub fn capture(
        capture_id: String,
        layout: LayoutSnapshot,
        pane_accesskit_ids: BTreeMap<String, Option<u64>>,
        widget_tree: UiTreeSnapshot,
        screenshot: ScreenshotEvidence,
    ) -> WorksurfaceSnapshot {
        let pane_tree = layout
            .panes
            .iter()
            .map(|(pane_id, pane)| {
                let pane_id_string = pane_id.as_ref().to_owned();
                let location = layout
                    .pop_outs
                    .get(pane_id)
                    .map(|pop| PaneLocation::PoppedOut { open: pop.open })
                    .unwrap_or(PaneLocation::MainSplit);
                let (authority, authority_agent_id) = authority_parts(&pane.authority);
                PaneInspection {
                    pane_id: pane_id_string.clone(),
                    pane_type: pane_type_key(&pane.pane_type),
                    pane_type_label: pane.pane_type.label(),
                    content_id: pane.content_id.clone(),
                    lock_state: lock_state_key(pane.lock_state),
                    dirty: dirty_state_key(pane.dirty),
                    authority,
                    authority_agent_id,
                    accesskit_node_id: pane_accesskit_ids.get(&pane_id_string).copied().flatten(),
                    location,
                }
            })
            .collect::<Vec<_>>();

        let widget_inventory = WidgetInventory::from_tree(widget_tree);
        let layout_tree = LayoutTreeInspection::from_snapshot(layout);
        let internal_diagnostics = InternalDiagnosticsEvent::worksurface_inspected(
            pane_tree.len(),
            widget_inventory.widget_count,
        );

        WorksurfaceSnapshot {
            schema_id: WORKSURFACE_SNAPSHOT_SCHEMA_ID.to_owned(),
            version: WORKSURFACE_SNAPSHOT_VERSION,
            capture_id,
            captured_at_utc: timestamp_utc_string(),
            pane_tree,
            widget_inventory,
            layout_tree,
            screenshot,
            internal_diagnostics,
        }
    }

    pub fn write_json(
        snapshot: &WorksurfaceSnapshot,
        artifact_root: impl AsRef<Path>,
    ) -> io::Result<SnapshotWriteReceipt> {
        let root = validate_external_artifact_root(artifact_root.as_ref())?;
        fs::create_dir_all(&root)?;
        let path = root.join(format!(
            "worksurface-snapshot-{}.json",
            safe_capture_id(&snapshot.capture_id)
        ));
        let bytes = serde_json::to_vec_pretty(snapshot)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(&bytes)?;
        Ok(SnapshotWriteReceipt {
            path,
            bytes: bytes.len() as u64,
        })
    }

    pub fn new_capture_id() -> String {
        format!(
            "{}-{}-{}",
            std::process::id(),
            safe_capture_id(&timestamp_utc_string()),
            CAPTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        )
    }
}

impl WidgetInventory {
    fn from_tree(tree: UiTreeSnapshot) -> Self {
        let nodes = tree
            .iter_nodes()
            .map(WidgetNodeInventory::from_node)
            .collect::<Vec<_>>();
        let author_id_count = nodes.iter().filter(|node| node.author_id.is_some()).count();
        Self {
            widget_count: tree.widget_count,
            author_id_count,
            nodes,
            tree: WidgetTreeNode::from_node(&tree.root),
        }
    }
}

impl WidgetNodeInventory {
    fn from_node(node: &UiTreeNode) -> Self {
        Self {
            id: node.id.clone(),
            author_id: node.author_id.clone(),
            node_id: node.node_id,
            role: node.role.clone(),
            label: node.label.clone(),
            actions: node.actions.clone(),
            child_count: node.children.len(),
        }
    }
}

impl WidgetTreeNode {
    fn from_node(node: &UiTreeNode) -> Self {
        Self {
            id: node.id.clone(),
            author_id: node.author_id.clone(),
            node_id: node.node_id,
            role: node.role.clone(),
            label: node.label.clone(),
            value: node.value.clone(),
            disabled: node.disabled,
            actions: node.actions.clone(),
            bounds: node.bounds.map(WidgetNodeBounds::from),
            children: node.children.iter().map(Self::from_node).collect(),
        }
    }
}

impl From<UiNodeBounds> for WidgetNodeBounds {
    fn from(bounds: UiNodeBounds) -> Self {
        Self {
            x: bounds.x,
            y: bounds.y,
            w: bounds.w,
            h: bounds.h,
        }
    }
}

impl LayoutTreeInspection {
    fn from_snapshot(snapshot: LayoutSnapshot) -> Self {
        let panes = snapshot
            .panes
            .iter()
            .map(|(pane_id, pane)| LayoutPaneSummary {
                pane_id: pane_id.as_ref().to_owned(),
                pane_type: pane_type_key(&pane.pane_type),
                content_id: pane.content_id.clone(),
            })
            .collect::<Vec<_>>();

        let tab_bars = snapshot
            .tab_bars
            .iter()
            .map(|(pane_id, tab_bar)| TabBarInspection {
                pane_id: pane_id.as_ref().to_owned(),
                active_index: tab_bar.active_index,
                tabs: tab_bar
                    .tabs
                    .iter()
                    .enumerate()
                    .map(|(index, tab)| TabInspection {
                        index,
                        pane_type: pane_type_key(&tab.pane_type),
                        label: tab.label(),
                        content_id: tab.content_id.clone(),
                        pinned: tab.pinned,
                        dirty: tab.dirty,
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();

        let pop_outs = snapshot
            .pop_outs
            .iter()
            .map(|(pane_id, pop)| PopOutInspection {
                pane_id: pane_id.as_ref().to_owned(),
                open: pop.open,
                x: pop.geometry.pos.x,
                y: pop.geometry.pos.y,
                width: pop.geometry.size.x,
                height: pop.geometry.size.y,
            })
            .collect::<Vec<_>>();

        Self {
            schema_id: snapshot.schema_id,
            version: snapshot.version,
            split_weights: SplitWeightsInspection {
                vertical: snapshot.split_weights.vertical,
                horizontal: snapshot.split_weights.horizontal,
            },
            active_pane: snapshot
                .active_pane
                .as_ref()
                .map(|id| id.as_ref().to_owned()),
            active_module: format!("{:?}", snapshot.active_module),
            pane_count: snapshot.panes.len(),
            tab_bar_count: snapshot.tab_bars.len(),
            pop_out_count: snapshot.pop_outs.len(),
            panes,
            tab_bars,
            pop_outs,
            drawers: DrawersInspection {
                project: snapshot.drawers.project,
                bottom: snapshot.drawers.bottom,
            },
        }
    }
}

/// Resolve the default external artifact root for an operator-triggered Settings dump.
///
/// Tests and governed proof commands should pass an explicit root to
/// `HandshakeApp::capture_worksurface_snapshot_to`. This fallback is for a manual Settings click.
pub fn default_artifact_root() -> PathBuf {
    if let Some(root) = std::env::var_os("HANDSHAKE_VISUAL_DEBUGGER_ARTIFACT_DIR") {
        let path = PathBuf::from(root);
        if validate_external_artifact_root(&path).is_ok() {
            return path;
        }
    }
    if let Some(root) = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT") {
        let path = PathBuf::from(root)
            .join("handshake-test")
            .join("wp-kernel-012-mt-102");
        if validate_external_artifact_root(&path).is_ok() {
            return path;
        }
    }
    discover_handshake_artifacts_root()
        .unwrap_or_else(|| std::env::temp_dir().join("Handshake_Artifacts"))
        .join("handshake-test")
        .join("wp-kernel-012-mt-102")
}

pub fn validate_external_artifact_root(root: &Path) -> io::Result<PathBuf> {
    let repo_root = repo_root();
    let root_abs = absolutize(root)?;
    let repo_cmp = repo_root
        .canonicalize()
        .unwrap_or_else(|_| normalize_lexical(&repo_root));

    if let Ok(root_canon) = root_abs.canonicalize() {
        reject_repo_local_root(root, &root_canon, &repo_cmp)?;
        return Ok(root_abs);
    }

    let parent_cmp = deepest_existing_ancestor(&root_abs)
        .and_then(|ancestor| ancestor.canonicalize().ok())
        .unwrap_or_else(|| normalize_lexical(&root_abs));
    reject_repo_local_root(root, &parent_cmp, &repo_cmp)?;

    if lexical_starts_with(
        &normalize_lexical(&root_abs),
        &normalize_lexical(&repo_root),
    ) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "visual debugger artifacts must be outside the product repo; rejected {}",
                root.display()
            ),
        ));
    }

    Ok(root_abs)
}

fn reject_repo_local_root(original: &Path, candidate: &Path, repo: &Path) -> io::Result<()> {
    if candidate.starts_with(repo) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "visual debugger artifacts must be outside the product repo; rejected {}",
                original.display()
            ),
        ));
    }
    Ok(())
}

fn absolutize(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn deepest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut cursor = path.to_path_buf();
    loop {
        if cursor.exists() {
            return Some(cursor);
        }
        if !cursor.pop() {
            return None;
        }
    }
}

fn discover_handshake_artifacts_root() -> Option<PathBuf> {
    let mut starts = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    starts.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    for start in starts {
        for ancestor in start.ancestors() {
            let candidate = ancestor.join("Handshake_Artifacts");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn lexical_starts_with(path: &Path, root: &Path) -> bool {
    let path = path.components().collect::<Vec<_>>();
    let root = root.components().collect::<Vec<_>>();
    path.len() >= root.len() && path.iter().zip(root.iter()).all(|(a, b)| a == b)
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn timestamp_utc_string() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:09}Z", now.as_secs(), now.subsec_nanos())
}

fn monotonicish_timestamp_nanos() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs()
        .saturating_mul(1_000_000_000)
        .saturating_add(now.subsec_nanos() as u64)
}

pub fn safe_capture_id(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn pane_type_key(pane_type: &PaneType) -> String {
    match pane_type {
        PaneType::Workspace => "Workspace",
        PaneType::LoomDailyJournal => "LoomDailyJournal",
        PaneType::LoomBlock => "LoomBlock",
        PaneType::LoomWikiPage => "LoomWikiPage",
        PaneType::AtelierEditor => "AtelierEditor",
        PaneType::KernelDcc => "KernelDcc",
        PaneType::InferenceLab => "InferenceLab",
        PaneType::ModelRuntime => "ModelRuntime",
        PaneType::Swarm => "Swarm",
        PaneType::Problems => "Problems",
        PaneType::Jobs => "Jobs",
        PaneType::Timeline => "Timeline",
        PaneType::UserManual => "UserManual",
        PaneType::CodeSymbol => "CodeSymbol",
        PaneType::SourceControl => "SourceControl",
        PaneType::MediaDownloader => "MediaDownloader",
        PaneType::FontManager => "FontManager",
        PaneType::FlightRecorder => "FlightRecorder",
        PaneType::VisualDebugger => "VisualDebugger",
        PaneType::LoomSearchV2 => "LoomSearchV2",
        PaneType::FindInFiles => "FindInFiles",
        PaneType::RuntimeChat => "RuntimeChat",
        PaneType::Placeholder(name) => return format!("Placeholder:{name}"),
    }
    .to_owned()
}

fn lock_state_key(lock_state: LockState) -> String {
    match lock_state {
        LockState::Unlocked => "Unlocked",
        LockState::Locked => "Locked",
    }
    .to_owned()
}

fn dirty_state_key(dirty: DirtyState) -> String {
    match dirty {
        DirtyState::Clean => "Clean",
        DirtyState::Dirty => "Dirty",
    }
    .to_owned()
}

fn authority_parts(authority: &PaneAuthority) -> (String, Option<String>) {
    match authority {
        PaneAuthority::Human => ("Human".to_owned(), None),
        PaneAuthority::Agent(id) => ("Agent".to_owned(), Some(id.clone())),
        PaneAuthority::System => ("System".to_owned(), None),
    }
}
