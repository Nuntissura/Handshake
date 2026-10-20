use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::ace::ArtifactHandle;

use super::EntityRef;

const HANDSHAKE_DIR: &str = ".handshake";
const ARTIFACTS_DIR: &str = "artifacts";
pub(crate) const ARTIFACT_MANIFEST_FILENAME: &str = "artifact.json";

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("workspace root resolve failed: {0}")]
    WorkspaceRootResolve(String),
    #[error("invalid relative path: {path}")]
    InvalidRelPath { path: String },
    #[error("path traversal blocked: {path}")]
    PathTraversalBlocked { path: String },
    #[error("write blocked: target escapes root: {path}")]
    RootEscape { path: String },
    #[error(
        "missing retention_ttl_days for high-sensitivity artifact: artifact_id={artifact_id} kind={kind:?}"
    )]
    MissingRetentionTtlDays {
        artifact_id: Uuid,
        kind: ArtifactPayloadKind,
    },
    #[error("invalid sha256 hex: {field}")]
    InvalidSha256Hex { field: String },
    #[error("content hash mismatch")]
    ContentHashMismatch,
    #[error("persisted artifact manifest binding mismatch")]
    ManifestBindingMismatch,
    #[error("artifact publication custody unavailable: {0}")]
    PublicationCustody(String),
    #[error("payload exceeds the ingest size limit: limit={limit_bytes} observed>={observed_bytes}")]
    SizeLimitExceeded { limit_bytes: u64, observed_bytes: u64 },
    #[error("payload stream failed: {0}")]
    Stream(String),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactLayer {
    #[serde(rename = "L1")]
    L1,
    #[serde(rename = "L2")]
    L2,
    #[serde(rename = "L3")]
    L3,
    #[serde(rename = "L4")]
    L4,
}

impl ArtifactLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactLayer::L1 => "L1",
            ArtifactLayer::L2 => "L2",
            ArtifactLayer::L3 => "L3",
            ArtifactLayer::L4 => "L4",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactPayloadKind {
    File,
    ToolOutput,
    Transcript,
    DatasetSlice,
    PromptPayload,
    Report,
    Bundle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactClassification {
    Low,
    Medium,
    High,
}

/// Spec 2.3.10.6 minimum schema (+ optional hash_basis fields for deterministic validation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactManifest {
    pub artifact_id: Uuid,
    pub layer: ArtifactLayer,
    pub kind: ArtifactPayloadKind,
    pub mime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename_hint: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_job_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_entity_refs: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_artifact_refs: Vec<ArtifactHandle>,
    pub content_hash: String,
    pub size_bytes: u64,
    pub classification: ArtifactClassification,
    pub exportable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_ttl_days: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    /// Optional but recommended for directory artifacts to enable deterministic content_hash
    /// validation (hash basis and explicit excludes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_basis: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hash_exclude_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleIndexEntry {
    pub path: String,
    pub content_hash: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ArtifactWriteEntry {
    pub rel_path: String,
    pub bytes: Vec<u8>,
}

/// Storage integrity only. The producer must establish the durable protected-resource
/// and workspace binding before publishing a resolvable reference.
pub(crate) struct PersistedFileArtifact {
    manifest: ArtifactManifest,
}

pub(crate) struct ArtifactPublicationResources {
    operation_id: Uuid,
    workspace_root: PathBuf,
    inventory: Mutex<ArtifactPublicationFilesystemInventory>,
}

struct ArtifactPublicationFilesystemInventory {
    admission_frozen: bool,
    active_io: usize,
    intended: Vec<PathBuf>,
    closed: Option<ArtifactPublicationPhysicalClosure>,
    #[cfg(windows)]
    native: native_artifact_custody::Custody,
}

struct ArtifactPublicationPhysicalClosure {
    committed: bool,
}

pub(crate) struct ArtifactPublicationOwnedClosure {
    resources: Arc<ArtifactPublicationResources>,
    committed: bool,
}

impl ArtifactPublicationOwnedClosure {
    pub(crate) fn matches_terminal(
        &self,
        terminal: &crate::storage::surreal::ArtifactPublicationCanonicalTerminalForOwner,
    ) -> bool {
        let committed = matches!(terminal.decision(),
            crate::storage::surreal::ArtifactPublicationCanonicalDecision::Commit);
        terminal.operation_id() == self.resources.operation_id
            && terminal.matches_resources(&self.resources)
            && self.committed == committed
            && self.resources.inventory.lock().is_ok_and(|inventory| {
                inventory.closed.as_ref().is_some_and(|closed| {
                    closed.committed == committed && inventory.active_io == 0
                        && inventory.admission_frozen
                })
            })
    }
}

impl ArtifactPublicationResources {
    /// Attaching the original operation owner performs no filesystem effects.
    pub(crate) fn new(operation_id: Uuid, workspace_root: PathBuf) -> Self {
        Self {
            operation_id,
            workspace_root,
            inventory: Mutex::new(ArtifactPublicationFilesystemInventory {
                admission_frozen: false,
                active_io: 0,
                intended: Vec::new(),
                closed: None,
                #[cfg(windows)]
                native: native_artifact_custody::Custody::new(),
            }),
        }
    }

    pub(crate) fn close_after_canonical_terminal(
        self: &Arc<Self>,
        terminal: &crate::storage::surreal::ArtifactPublicationCanonicalTerminalForOwner,
    ) -> Result<ArtifactPublicationOwnedClosure, ArtifactError> {
        if terminal.operation_id() != self.operation_id || !terminal.matches_resources(self) {
            return Err(ArtifactError::PublicationCustody("foreign canonical terminal".into()));
        }
        self.freeze_admission()?;
        let committed = matches!(terminal.decision(),
            crate::storage::surreal::ArtifactPublicationCanonicalDecision::Commit);
        let mut inventory = self.inventory.lock().map_err(|_| {
            ArtifactError::PublicationCustody("owner inventory poisoned".into())
        })?;
        if let Some(closed) = inventory.closed.as_ref() {
            if closed.committed != committed {
                return Err(ArtifactError::PublicationCustody("conflicting canonical terminal decision".into()));
            }
            return Ok(ArtifactPublicationOwnedClosure {
                resources: Arc::clone(self),
                committed,
            });
        }
        #[cfg(windows)]
        {
            inventory.native.close(committed)?;
            inventory.closed = Some(ArtifactPublicationPhysicalClosure { committed });
            return Ok(ArtifactPublicationOwnedClosure { resources: Arc::clone(self), committed });
        }
        #[cfg(not(windows))]
        Err(ArtifactError::PublicationCustody(
            "native physical closure has not been established".into(),
        ))
    }

    pub(crate) fn persist_file_artifact_owned(
        &self,
        manifest: &ArtifactManifest,
        payload: &[u8],
    ) -> Result<PersistedFileArtifact, ArtifactError> {
        self.with_owned_io(artifact_root_dir(&self.workspace_root, manifest.layer, manifest.artifact_id),
            |inventory| {
                #[cfg(windows)]
                { inventory.native.persist(&self.workspace_root, manifest, payload) }
                #[cfg(not(windows))]
                { let _ = (inventory, payload); Err(ArtifactError::PublicationCustody("native custody unsupported on this target".into())) }
            })
    }

    pub(crate) fn read_persisted_file_artifact_owned(
        &self,
        layer: ArtifactLayer,
        id: Uuid,
    ) -> Result<PersistedFileArtifact, ArtifactError> {
        self.with_owned_io(artifact_root_dir(&self.workspace_root, layer, id), |inventory| {
            #[cfg(windows)]
            { inventory.native.read(&self.workspace_root, layer, id) }
            #[cfg(not(windows))]
            { let _ = inventory; Err(ArtifactError::PublicationCustody("native custody unsupported on this target".into())) }
        })
    }

    fn with_owned_io<T>(&self, target: PathBuf, operation: impl FnOnce(&mut ArtifactPublicationFilesystemInventory) -> Result<T, ArtifactError>) -> Result<T, ArtifactError> {
        let mut inventory = self.inventory.lock().map_err(|_| {
            ArtifactError::PublicationCustody("owner inventory poisoned".into())
        })?;
        if inventory.admission_frozen {
            return Err(ArtifactError::PublicationCustody("owner admission frozen".into()));
        }
        let active_io = inventory.active_io.checked_add(1).ok_or_else(|| {
            ArtifactError::PublicationCustody("owner I/O count exhausted".into())
        })?;
        inventory.intended.push(target);
        inventory.active_io = active_io;
        let result = operation(&mut inventory);
        inventory.active_io -= 1;
        if result.is_err() { inventory.admission_frozen = true; }
        result
    }

    fn freeze_admission(&self) -> Result<(), ArtifactError> {
        let mut inventory = self.inventory.lock().map_err(|_| {
            ArtifactError::PublicationCustody("owner inventory poisoned".into())
        })?;
        inventory.admission_frozen = true;
        if inventory.active_io != 0 {
            return Err(ArtifactError::PublicationCustody("owner I/O remains in flight".into()));
        }
        Ok(())
    }
}

#[cfg(windows)]
mod native_artifact_custody {
    use super::*;
    use std::io::{Read, Seek, SeekFrom};
    use std::mem::{size_of, ManuallyDrop};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
    use windows_sys::Wdk::Storage::FileSystem::{
        NtCreateFile, FILE_CREATE, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE,
        FILE_OPEN, FILE_OPEN_IF, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
    };
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, HANDLE, OBJ_CASE_INSENSITIVE, UNICODE_STRING,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, GetFileInformationByHandleEx, SetFileInformationByHandle,
        FileAttributeTagInfo, FileDispositionInfo, FileIdInfo, FileRenameInfo, DELETE,
        FILE_ADD_FILE, FILE_ADD_SUBDIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_ATTRIBUTE_TAG_INFO, FILE_DISPOSITION_INFO, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_READ_ATTRIBUTES,
        FILE_RENAME_INFO, FILE_RENAME_INFO_0,
        FILE_READ_DATA, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_WRITE_DATA,
        OPEN_EXISTING, SYNCHRONIZE,
    };
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;
    use windows_sys::Win32::System::WindowsProgramming::FILE_CREATED;

    struct Held {
        raw: usize,
        relative: PathBuf,
        identity: Option<(u64, [u8; 16])>,
        created: bool,
        delete_pending: bool,
    }

    // Raw handles intentionally have no Drop closer. The operation inventory survives
    // errors in its Parent-owned Arc until explicit canonical terminal closure.
    pub(super) struct Custody {
        held: Vec<Held>,
        intended: Vec<PathBuf>,
        root: Option<PathBuf>,
        root_index: Option<usize>,
        close_failed: bool,
        acquisition_uncertain: bool,
        legacy_uncertain: bool,
        legacy_owned: bool,
    }

    fn failure(message: impl Into<String>) -> ArtifactError {
        ArtifactError::PublicationCustody(message.into())
    }

    fn win_error(operation: &str) -> ArtifactError {
        failure(format!("{operation}: win32={}", unsafe { GetLastError() }))
    }

    impl Custody {
        pub(super) fn new() -> Self {
            Self { held: Vec::new(), intended: Vec::new(), root: None,
                root_index: None, close_failed: false, acquisition_uncertain: false, legacy_uncertain: false,
                legacy_owned: false }
        }

        pub(super) fn new_legacy() -> Self {
            let mut custody = Self::new();
            custody.legacy_owned = true;
            custody
        }

        fn identity(raw: usize) -> Result<(u64, [u8; 16]), ArtifactError> {
            let mut tag = FILE_ATTRIBUTE_TAG_INFO::default();
            if unsafe { GetFileInformationByHandleEx(raw as HANDLE, FileAttributeTagInfo,
                (&mut tag as *mut FILE_ATTRIBUTE_TAG_INFO).cast(), size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32) } == 0 {
                return Err(win_error("query reparse identity"));
            }
            if tag.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err(failure("reparse object is not admitted for artifact custody"));
            }
            let mut id = FILE_ID_INFO::default();
            if unsafe { GetFileInformationByHandleEx(raw as HANDLE, FileIdInfo,
                (&mut id as *mut FILE_ID_INFO).cast(), size_of::<FILE_ID_INFO>() as u32) } == 0 {
                return Err(win_error("query file identity"));
            }
            Ok((id.VolumeSerialNumber, id.FileId.Identifier))
        }

        fn verify(&self, index: usize) -> Result<(), ArtifactError> {
            let entry = &self.held[index];
            let expected = entry.identity.ok_or_else(|| failure("acquired identity unresolved"))?;
            if Self::identity(entry.raw)? != expected {
                return Err(failure("held filesystem identity changed"));
            }
            Ok(())
        }

        fn retain(&mut self, raw: usize, relative: PathBuf, created: bool) -> Result<usize, ArtifactError> {
            let index = self.held.len();
            self.held.push(Held { raw, relative, identity: None, created, delete_pending: false });
            let identity = Self::identity(raw)?;
            self.held[index].identity = Some(identity);
            Ok(index)
        }

        pub(super) fn acquire(&mut self, root: &Path) -> Result<(), ArtifactError> {
            if self.legacy_uncertain { return Err(failure("legacy physical custody is frozen for reconciliation")); }
            if self.close_failed { return Err(failure("native close outcome requires reconciliation")); }
            if let Some(held_root) = &self.root {
                if held_root != root { return Err(failure("operation root changed")); }
                return self.verify(self.root_index.ok_or_else(|| failure("root acquisition incomplete"))?);
            }
            if !root.is_absolute() || root.components().any(|part| matches!(part, Component::ParentDir | Component::CurDir)) {
                return Err(failure("managed root must be absolute without traversal"));
            }
            self.root = Some(root.to_path_buf());
            let mut ancestors: Vec<_> = root.ancestors().collect();
            ancestors.reverse();
            for path in ancestors {
                self.intended.push(path.to_path_buf());
                let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
                // All ancestors exclude DELETE; the managed root is also the common
                // exclusive gate. An opener cannot replace this gate while it is held.
                let share = if path == root { 0 } else { FILE_SHARE_READ | FILE_SHARE_WRITE };
                let access = FILE_READ_ATTRIBUTES | SYNCHRONIZE
                    | if path == root { FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY } else { 0 };
                self.acquisition_uncertain = true;
                let handle = unsafe { CreateFileW(wide.as_ptr(), access, share, std::ptr::null(),
                    OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                    std::ptr::null_mut()) };
                if handle.is_null() || handle as isize == -1 {
                    self.acquisition_uncertain = false;
                    return Err(win_error("open anchored artifact root"));
                }
                let index = self.retain(handle as usize, path.to_path_buf(), false)?;
                self.acquisition_uncertain = false;
                if path == root { self.root_index = Some(index); }
            }
            Ok(())
        }

        fn open_relative(&mut self, relative: &Path, directory: bool, disposition: u32) -> Result<usize, ArtifactError> {
            if relative.as_os_str().is_empty() {
                return self.root_index.ok_or_else(|| failure("root not acquired"));
            }
            if let Some(index) = self.held.iter().position(|item| item.relative == relative) {
                self.verify(index)?;
                if disposition == FILE_CREATE {
                    return Err(failure("write-once artifact target already held"));
                }
                return Ok(index);
            }
            let parent = relative.parent().unwrap_or_else(|| Path::new(""));
            let parent_index = self.open_relative(parent, true, if disposition == FILE_OPEN { FILE_OPEN } else { FILE_OPEN_IF })?;
            self.verify(parent_index)?;
            let name = relative.file_name().ok_or_else(|| failure("missing relative component"))?;
            if relative.components().any(|part| !matches!(part, Component::Normal(_))) {
                return Err(failure("non-relative artifact component"));
            }
            self.intended.push(relative.to_path_buf());
            let mut wide: Vec<u16> = name.encode_wide().collect();
            let length = wide.len().checked_mul(2).and_then(|n| u16::try_from(n).ok())
                .ok_or_else(|| failure("native artifact name too long"))?;
            let mut name = UNICODE_STRING { Length: length, MaximumLength: length, Buffer: wide.as_mut_ptr() };
            let attributes = OBJECT_ATTRIBUTES {
                Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
                RootDirectory: self.held[parent_index].raw as HANDLE,
                ObjectName: &mut name,
                Attributes: OBJ_CASE_INSENSITIVE,
                SecurityDescriptor: std::ptr::null(),
                SecurityQualityOfService: std::ptr::null(),
            };
            let mut status = IO_STATUS_BLOCK::default();
            let mut handle: HANDLE = std::ptr::null_mut();
            let options = FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT
                | if directory { FILE_DIRECTORY_FILE } else { FILE_NON_DIRECTORY_FILE };
            let access = FILE_READ_ATTRIBUTES | SYNCHRONIZE | DELETE
                | if directory { FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY } else { FILE_READ_DATA | FILE_WRITE_DATA };
            self.acquisition_uncertain = true;
            let result = unsafe { NtCreateFile(&mut handle, access, &attributes, &mut status,
                std::ptr::null(), 0, if directory { FILE_SHARE_READ | FILE_SHARE_WRITE } else { 0 },
                disposition, options, std::ptr::null(), 0) };
            // Keep every returned handle, including a failed/incomplete acquisition.
            let acquired = if !handle.is_null() && handle as isize != -1 {
                Some(self.retain(handle as usize, relative.to_path_buf(),
                    result >= 0 && status.Information == FILE_CREATED as usize)?)
            } else { None };
            if result < 0 {
                self.acquisition_uncertain = false;
                return Err(failure(format!("anchored native open status={result}")));
            }
            let index = acquired.ok_or_else(|| failure("native open succeeded without a handle"))?;
            self.acquisition_uncertain = false;
            Ok(index)
        }

        fn write_created(&mut self, relative: &Path, bytes: &[u8]) -> Result<(), ArtifactError> {
            let index = self.open_relative(relative, false, FILE_CREATE)?;
            self.verify(index)?;
            if !self.held[index].created { return Err(failure("artifact create attribution missing")); }
            let mut file = ManuallyDrop::new(unsafe { fs::File::from_raw_handle(self.held[index].raw as _) });
            file.write_all(bytes)?;
            file.sync_all()?;
            self.verify(index)
        }

        fn read_bytes(&mut self, relative: &Path) -> Result<Vec<u8>, ArtifactError> {
            let index = self.open_relative(relative, false, FILE_OPEN)?;
            self.verify(index)?;
            let mut file = ManuallyDrop::new(unsafe { fs::File::from_raw_handle(self.held[index].raw as _) });
            file.seek(SeekFrom::Start(0))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            self.verify(index)?;
            Ok(bytes)
        }

        pub(super) fn legacy_create_stream_file(&mut self, relative: &Path) -> Result<(), ArtifactError> {
            if !self.legacy_owned { return Err(failure("Parent inventory is not a legacy streaming flight")); }
            self.open_relative(relative, false, FILE_CREATE)?;
            Ok(())
        }

        pub(super) fn legacy_write_stream_manifest(&mut self, relative: &Path, bytes: &[u8]) -> Result<(), ArtifactError> {
            if !self.legacy_owned { return Err(failure("Parent inventory is not a legacy streaming flight")); }
            self.write_created(relative, bytes)
        }

        pub(super) fn legacy_write_stream_chunk(&mut self, relative: &Path, bytes: &[u8], sync: bool)
            -> Result<(), ArtifactError> {
            if !self.legacy_owned { return Err(failure("Parent inventory is not a legacy streaming flight")); }
            let index = self.held.iter().position(|entry| entry.relative == relative)
                .ok_or_else(|| failure("streaming file handle not retained"))?;
            self.verify(index)?;
            let mut file = ManuallyDrop::new(unsafe { fs::File::from_raw_handle(self.held[index].raw as _) });
            file.write_all(bytes)?;
            if sync { file.sync_all()?; }
            self.verify(index)
        }

        pub(super) fn legacy_publish_stream_file(&mut self, relative: &Path, target: &Path) -> Result<(), ArtifactError> {
            if !self.legacy_owned { return Err(failure("Parent inventory is not a legacy streaming flight")); }
            if relative.parent() != target.parent()
                || target.components().any(|part| !matches!(part, Component::Normal(_))) {
                return Err(failure("stream publication must remain in its held artifact directory"));
            }
            let index = self.held.iter().position(|entry| entry.relative == relative)
                .ok_or_else(|| failure("streaming file handle not retained"))?;
            if !self.held[index].created { return Err(failure("streaming source creation is not owned")); }
            let parent = self.open_relative(target.parent().ok_or_else(|| failure("missing publication parent"))?, true, FILE_OPEN)?;
            self.verify(index)?;
            self.verify(parent)?;
            let wide: Vec<u16> = target.file_name().ok_or_else(|| failure("missing publication filename"))?
                .encode_wide().collect();
            if wide.contains(&0) { return Err(failure("NUL publication filename")); }
            let name_bytes = wide.len().checked_mul(2).and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| failure("publication filename too long"))?;
            let buffer_bytes = size_of::<FILE_RENAME_INFO>().checked_add(name_bytes as usize)
                .ok_or_else(|| failure("publication buffer too large"))?;
            let mut buffer = vec![0u64; buffer_bytes.div_ceil(size_of::<u64>())];
            let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
            self.intended.push(target.to_path_buf());
            // Aligned storage includes the variable UTF-16 filename tail. Rename uses
            // the retained source and parent handles, never a reopened source path.
            unsafe {
                std::ptr::write(info, FILE_RENAME_INFO {
                    Anonymous: FILE_RENAME_INFO_0 { ReplaceIfExists: false },
                    RootDirectory: self.held[parent].raw as HANDLE,
                    FileNameLength: name_bytes,
                    FileName: [0],
                });
                std::ptr::copy_nonoverlapping(wide.as_ptr(),
                    std::ptr::addr_of_mut!((*info).FileName).cast::<u16>(), wide.len());
                if SetFileInformationByHandle(self.held[index].raw as HANDLE, FileRenameInfo,
                    info.cast(), u32::try_from(buffer_bytes).map_err(|_| failure("publication buffer too large"))?) == 0 {
                    return Err(win_error("publish exact owned streaming handle"));
                }
            }
            self.held[index].relative = target.to_path_buf();
            self.verify(index)
        }

        fn artifact_relative(layer: ArtifactLayer, id: Uuid) -> PathBuf {
            PathBuf::from(HANDSHAKE_DIR).join(ARTIFACTS_DIR).join(layer.as_str()).join(id.to_string())
        }

        pub(super) fn persist(&mut self, root: &Path, manifest: &ArtifactManifest, payload: &[u8])
            -> Result<PersistedFileArtifact, ArtifactError> {
            self.acquire(root)?;
            if manifest.content_hash != sha256_hex(payload) || manifest.size_bytes != payload.len() as u64 {
                return Err(ArtifactError::ContentHashMismatch);
            }
            if (matches!(manifest.kind, ArtifactPayloadKind::PromptPayload)
                || matches!(manifest.classification, ArtifactClassification::High))
                && manifest.retention_ttl_days.is_none() {
                return Err(ArtifactError::MissingRetentionTtlDays { artifact_id: manifest.artifact_id, kind: manifest.kind });
            }
            let relative = Self::artifact_relative(manifest.layer, manifest.artifact_id);
            self.write_created(&relative.join("payload"), payload)?;
            self.write_created(&relative.join(ARTIFACT_MANIFEST_FILENAME), &serde_json::to_vec_pretty(manifest)?)?;
            let observed = self.read(root, manifest.layer, manifest.artifact_id)?;
            if serde_json::to_vec(observed.manifest())? != serde_json::to_vec(manifest)? {
                return Err(ArtifactError::ManifestBindingMismatch);
            }
            Ok(observed)
        }

        pub(super) fn read(&mut self, root: &Path, layer: ArtifactLayer, id: Uuid)
            -> Result<PersistedFileArtifact, ArtifactError> {
            self.acquire(root)?;
            let relative = Self::artifact_relative(layer, id);
            let manifest: ArtifactManifest = serde_json::from_slice(&self.read_bytes(&relative.join(ARTIFACT_MANIFEST_FILENAME))?)?;
            if manifest.artifact_id != id || manifest.layer != layer {
                return Err(ArtifactError::ManifestBindingMismatch);
            }
            let payload = self.read_bytes(&relative.join("payload"))?;
            if sha256_hex(&payload) != manifest.content_hash || payload.len() as u64 != manifest.size_bytes {
                return Err(ArtifactError::ContentHashMismatch);
            }
            Ok(PersistedFileArtifact { manifest })
        }

        pub(super) fn close(&mut self, committed: bool) -> Result<(), ArtifactError> {
            if self.legacy_uncertain { return Err(failure("legacy physical custody is frozen for reconciliation")); }
            self.close_resources(committed)
        }

        pub(super) fn close_legacy_preserving(&mut self) -> Result<(), ArtifactError> {
            if !self.legacy_owned { return Err(failure("Parent inventory cannot use legacy physical release")); }
            self.close_resources(true)
        }

        fn close_resources(&mut self, committed: bool) -> Result<(), ArtifactError> {
            if self.close_failed { return Err(failure("native close outcome requires reconciliation")); }
            if self.acquisition_uncertain { return Err(failure("native acquisition outcome requires reconciliation")); }
            while let Some(index) = self.held.len().checked_sub(1) {
                if let Err(error) = self.verify(index) {
                    self.close_failed = true;
                    return Err(error);
                }
                if !committed && self.held[index].created && !self.held[index].delete_pending {
                    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
                    if unsafe { SetFileInformationByHandle(self.held[index].raw as HANDLE, FileDispositionInfo,
                        (&disposition as *const FILE_DISPOSITION_INFO).cast(), size_of::<FILE_DISPOSITION_INFO>() as u32) } == 0 {
                        self.close_failed = true;
                        return Err(win_error("delete exact operation-created object"));
                    }
                    self.held[index].delete_pending = true;
                }
                if unsafe { CloseHandle(self.held[index].raw as HANDLE) } == 0 {
                    self.close_failed = true;
                    return Err(win_error("close owned artifact handle"));
                }
                self.held.pop();
            }
            self.root_index = None;
            Ok(())
        }

        pub(super) fn freeze_uncertainty(&mut self) {
            self.legacy_uncertain = true;
        }
    }
}

impl PersistedFileArtifact {
    pub(crate) fn manifest(&self) -> &ArtifactManifest {
        &self.manifest
    }
}

pub(crate) fn read_persisted_file_artifact(
    workspace_root: &Path,
    layer: ArtifactLayer,
    artifact_id: Uuid,
) -> Result<PersistedFileArtifact, ArtifactError> {
    let manifest = read_artifact_manifest(workspace_root, layer, artifact_id)?;
    if manifest.artifact_id != artifact_id || manifest.layer != layer {
        return Err(ArtifactError::ManifestBindingMismatch);
    }
    read_file_artifact_with_manifest(workspace_root, &manifest)?;
    Ok(PersistedFileArtifact { manifest })
}

/// Leaves partial writes intact on failure for canonical read reconciliation.
/// Neither this write nor its returned witness grants resource access.
pub(crate) fn persist_file_artifact(
    workspace_root: &Path,
    manifest: &ArtifactManifest,
    payload_bytes: &[u8],
) -> Result<PersistedFileArtifact, ArtifactError> {
    write_file_artifact(workspace_root, manifest, payload_bytes)?;
    let persisted = read_persisted_file_artifact(
        workspace_root,
        manifest.layer,
        manifest.artifact_id,
    )?;
    if serde_json::to_vec(persisted.manifest())? != serde_json::to_vec(manifest)? {
        return Err(ArtifactError::ManifestBindingMismatch);
    }
    Ok(persisted)
}

/// Environment override for the workspace root the ArtifactStore lives under.
pub const HANDSHAKE_WORKSPACE_ROOT_ENV: &str = "HANDSHAKE_WORKSPACE_ROOT";

/// Resolve the workspace root at RUNTIME, never from a path baked into the binary.
///
/// Order: (1) `HANDSHAKE_WORKSPACE_ROOT`; (2) the nearest ancestor of the current working
/// directory that already contains a `.handshake/` workspace directory; (3) the nearest
/// ancestor that is a git checkout root (`.git` file or directory), which is what development
/// and test processes run inside; (4) a hard error naming the two ways to fix it.
///
/// The previous fallback embedded `CARGO_MANIFEST_DIR` at compile time, so a binary carried the
/// build host's drive and worktree with it: run from another worktree it opened THAT worktree's
/// artifact store, and moved to another disk it pointed at a path that did not exist. Both
/// break the disk-agnostic rule [CX-109B]; a runtime-discovered root cannot.
pub fn resolve_workspace_root() -> Result<PathBuf, ArtifactError> {
    if let Ok(value) = std::env::var(HANDSHAKE_WORKSPACE_ROOT_ENV) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    let cwd = std::env::current_dir().map_err(|err| {
        ArtifactError::WorkspaceRootResolve(format!(
            "{HANDSHAKE_WORKSPACE_ROOT_ENV} is unset and the current directory is unreadable: {err}"
        ))
    })?;
    if let Some(root) = discover_workspace_root_from(&cwd) {
        return Ok(root);
    }
    Err(ArtifactError::WorkspaceRootResolve(format!(
        "{HANDSHAKE_WORKSPACE_ROOT_ENV} is unset and no ancestor of {} contains a `.handshake/` \
         workspace directory or a git checkout root; set {HANDSHAKE_WORKSPACE_ROOT_ENV} to the \
         workspace that owns `.handshake/artifacts`",
        cwd.display()
    )))
}

/// Runtime workspace-root discovery used by [`resolve_workspace_root`]: the nearest ancestor of
/// `start` (inclusive) holding a `.handshake/` directory wins; failing that, the nearest ancestor
/// that is a git checkout root. Pure function over the filesystem, no environment reads.
pub fn discover_workspace_root_from(start: &Path) -> Option<PathBuf> {
    let mut cursor: Option<&Path> = Some(start);
    while let Some(dir) = cursor {
        if dir.join(HANDSHAKE_DIR).is_dir() {
            return Some(dir.to_path_buf());
        }
        cursor = dir.parent();
    }
    let mut cursor: Option<&Path> = Some(start);
    while let Some(dir) = cursor {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        cursor = dir.parent();
    }
    None
}

pub fn artifact_store_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join(HANDSHAKE_DIR).join(ARTIFACTS_DIR)
}

pub fn artifact_root_dir(
    workspace_root: &Path,
    layer: ArtifactLayer,
    artifact_id: Uuid,
) -> PathBuf {
    artifact_store_root(workspace_root)
        .join(layer.as_str())
        .join(artifact_id.to_string())
}

pub fn artifact_root_rel(layer: ArtifactLayer, artifact_id: Uuid) -> String {
    format!(
        "{}/{}/{}/{}",
        HANDSHAKE_DIR,
        ARTIFACTS_DIR,
        layer.as_str(),
        artifact_id
    )
}

/// Canonical sha256 -> bare lowercase hex. This is THE crate-level helper; the ~40 private
/// copies across the crate are being folded onto it (see `normalize_sha256_hex_ref` for the
/// three ref spellings in use).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// The three content-hash spellings the tree has accumulated. `Bare` is canonical for storage
/// columns and manifests; the two prefixed forms are accepted on input and emitted only where a
/// consumer contract demands them (`artifact://sha256/<hex>` is the Flight Recorder / memory
/// artifact-ref contract; `sha256-<hex>` appears in ETags and legacy aggregate ids;
/// `sha256:<hex>` is the Atelier row spelling).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sha256RefFormat {
    Bare,
    ColonPrefixed,
    DashPrefixed,
    ArtifactUrl,
}

/// Accept any of the known spellings and return the bare lowercase 64-hex digest plus the
/// format it arrived in; `None` when the digest is not 64 hex characters.
pub fn normalize_sha256_hex_ref(value: &str) -> Option<(String, Sha256RefFormat)> {
    let trimmed = value.trim();
    let (hex_part, format) = if let Some(rest) = trimmed.strip_prefix("artifact://sha256/") {
        (rest, Sha256RefFormat::ArtifactUrl)
    } else if let Some(rest) = trimmed.strip_prefix("sha256:") {
        (rest, Sha256RefFormat::ColonPrefixed)
    } else if let Some(rest) = trimmed.strip_prefix("sha256-") {
        (rest, Sha256RefFormat::DashPrefixed)
    } else {
        (trimmed, Sha256RefFormat::Bare)
    };
    if hex_part.len() != 64 || !hex_part.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some((hex_part.to_ascii_lowercase(), format))
}

/// Render a bare digest in one of the known spellings.
pub fn format_sha256_ref(bare_hex: &str, format: Sha256RefFormat) -> String {
    match format {
        Sha256RefFormat::Bare => bare_hex.to_ascii_lowercase(),
        Sha256RefFormat::ColonPrefixed => format!("sha256:{}", bare_hex.to_ascii_lowercase()),
        Sha256RefFormat::DashPrefixed => format!("sha256-{}", bare_hex.to_ascii_lowercase()),
        Sha256RefFormat::ArtifactUrl => {
            format!("artifact://sha256/{}", bare_hex.to_ascii_lowercase())
        }
    }
}

/// True when two content hashes in any known spelling name the same digest.
pub fn sha256_refs_match(left: &str, right: &str) -> bool {
    match (normalize_sha256_hex_ref(left), normalize_sha256_hex_ref(right)) {
        (Some((l, _)), Some((r, _))) => l == r,
        _ => false,
    }
}

fn normalize_rel_path(input: &str) -> String {
    input
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn ensure_safe_rel_path(rel_path: &str) -> Result<(), ArtifactError> {
    let path = Path::new(rel_path);
    if path.is_absolute() {
        return Err(ArtifactError::PathTraversalBlocked {
            path: rel_path.to_string(),
        });
    }
    for component in path.components() {
        match component {
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(ArtifactError::PathTraversalBlocked {
                    path: rel_path.to_string(),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

fn ensure_root_escape_blocked(root: &Path, target_path: &Path) -> Result<(), ArtifactError> {
    let root = fs::canonicalize(root)?;
    let parent = target_path
        .parent()
        .ok_or_else(|| ArtifactError::InvalidRelPath {
            path: target_path.to_string_lossy().to_string(),
        })?;
    fs::create_dir_all(parent)?;
    let parent_canon = fs::canonicalize(parent)?;
    if !parent_canon.starts_with(&root) {
        return Err(ArtifactError::RootEscape {
            path: target_path.to_string_lossy().to_string(),
        });
    }
    Ok(())
}

/// Atomic file write (temp + fsync + publication) with best-effort parent dir fsync.
/// Write-once publication uses a hard link so a concurrent winner is never replaced.
fn write_file_atomic_inner(
    root: &Path,
    target_path: &Path,
    bytes: &[u8],
    overwrite: bool,
) -> Result<(), ArtifactError> {
    ensure_root_escape_blocked(root, target_path)?;

    let parent = target_path
        .parent()
        .ok_or_else(|| ArtifactError::InvalidRelPath {
            path: target_path.to_string_lossy().to_string(),
        })?;
    let parent_canon = fs::canonicalize(parent)?;

    let tmp_name = format!(".hsk_tmp_{}", Uuid::now_v7());
    let tmp_path = parent_canon.join(tmp_name);
    let mut tmp_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)?;
    tmp_file.write_all(bytes)?;
    tmp_file.sync_all()?;
    drop(tmp_file);

    if overwrite && target_path.exists() {
        if target_path.is_dir() {
            let _ = fs::remove_file(&tmp_path);
            return Err(ArtifactError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "target path exists and is a directory",
            )));
        }
        fs::remove_file(target_path)?;
    }

    let publication = if overwrite {
        fs::rename(&tmp_path, target_path)
    } else {
        fs::hard_link(&tmp_path, target_path)
    };
    if let Err(err) = publication {
        let _ = fs::remove_file(&tmp_path);
        return Err(ArtifactError::Io(err));
    }
    if !overwrite {
        let _ = fs::remove_file(&tmp_path);
    }

    if let Ok(dir_handle) = fs::File::open(&parent_canon) {
        let _ = dir_handle.sync_all();
    }
    Ok(())
}

/// Deterministic, canonical BundleIndex (sorted paths + per-item content_hash + size_bytes).
pub fn canonical_bundle_index(mut entries: Vec<BundleIndexEntry>) -> Vec<BundleIndexEntry> {
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}

pub fn bundle_index_json(entries: &[BundleIndexEntry]) -> Result<Vec<u8>, ArtifactError> {
    let mut normalized: Vec<BundleIndexEntry> = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.path.trim().is_empty() {
            return Err(ArtifactError::InvalidRelPath {
                path: entry.path.clone(),
            });
        }
        let rel = normalize_rel_path(&entry.path);
        ensure_safe_rel_path(&rel)?;
        if rel.contains(':') {
            return Err(ArtifactError::PathTraversalBlocked { path: rel });
        }
        normalized.push(BundleIndexEntry {
            path: rel,
            content_hash: entry.content_hash.clone(),
            size_bytes: entry.size_bytes,
        });
    }
    let normalized = canonical_bundle_index(normalized);
    Ok(serde_json::to_vec(&normalized)?)
}

pub fn bundle_index_content_hash(bundle_index_json: &[u8]) -> String {
    sha256_hex(bundle_index_json)
}

pub fn compute_entries_index(
    entries: &[ArtifactWriteEntry],
    exclude_paths: &[String],
) -> Result<(Vec<BundleIndexEntry>, u64), ArtifactError> {
    let mut index: Vec<BundleIndexEntry> = Vec::new();
    let mut total_size: u64 = 0;

    for entry in entries {
        let rel = normalize_rel_path(&entry.rel_path);
        if rel.trim().is_empty() {
            return Err(ArtifactError::InvalidRelPath { path: rel });
        }
        ensure_safe_rel_path(&rel)?;
        if rel.contains(':') {
            return Err(ArtifactError::PathTraversalBlocked { path: rel });
        }

        total_size = total_size.saturating_add(entry.bytes.len() as u64);
        if exclude_paths.iter().any(|p| normalize_rel_path(p) == rel) {
            continue;
        }
        let content_hash = sha256_hex(&entry.bytes);
        index.push(BundleIndexEntry {
            path: rel,
            content_hash,
            size_bytes: entry.bytes.len() as u64,
        });
    }

    Ok((canonical_bundle_index(index), total_size))
}

/// LocalFile materialize: writes a set of files into `export_root` (directory), enforcing
/// traversal-safe relative paths and atomic per-file writes.
fn materialize_local_dir_inner(
    export_root: &Path,
    entries: &[ArtifactWriteEntry],
    overwrite: bool,
) -> Result<Vec<String>, ArtifactError> {
    if !export_root.is_absolute() {
        return Err(ArtifactError::InvalidRelPath {
            path: export_root.to_string_lossy().to_string(),
        });
    }
    fs::create_dir_all(export_root)?;
    if !export_root.is_dir() {
        return Err(ArtifactError::Io(io::Error::new(
            io::ErrorKind::NotADirectory,
            "export_root is not a directory",
        )));
    }
    let export_root_canon = fs::canonicalize(export_root)?;

    let mut materialized_paths: Vec<String> = Vec::with_capacity(entries.len());
    for entry in entries {
        let rel = normalize_rel_path(&entry.rel_path);
        ensure_safe_rel_path(&rel)?;
        if rel.contains(':') {
            return Err(ArtifactError::PathTraversalBlocked { path: rel });
        }

        let target_path = export_root_canon.join(Path::new(&rel));
        write_file_atomic_inner(&export_root_canon, &target_path, &entry.bytes, overwrite)?;
        materialized_paths.push(rel);
    }

    materialized_paths.sort();
    Ok(materialized_paths)
}

fn write_artifact_manifest_atomic_inner(
    artifact_root: &Path,
    manifest: &ArtifactManifest,
) -> Result<(), ArtifactError> {
    let manifest_path = artifact_root.join(ARTIFACT_MANIFEST_FILENAME);
    let bytes = serde_json::to_vec_pretty(manifest)?;
    write_file_atomic_inner(artifact_root, &manifest_path, &bytes, false)
}

fn write_file_artifact_inner(
    workspace_root: &Path,
    manifest: &ArtifactManifest,
    payload_bytes: &[u8],
) -> Result<(), ArtifactError> {
    if matches!(manifest.kind, ArtifactPayloadKind::PromptPayload)
        || matches!(manifest.classification, ArtifactClassification::High)
    {
        if manifest.retention_ttl_days.is_none() {
            return Err(ArtifactError::MissingRetentionTtlDays {
                artifact_id: manifest.artifact_id,
                kind: manifest.kind,
            });
        }
    }

    let artifact_root = artifact_root_dir(workspace_root, manifest.layer, manifest.artifact_id);
    fs::create_dir_all(&artifact_root)?;

    // Validate hash fields match payload.
    if manifest.content_hash != sha256_hex(payload_bytes) {
        return Err(ArtifactError::ContentHashMismatch);
    }
    if manifest.size_bytes != payload_bytes.len() as u64 {
        return Err(ArtifactError::ContentHashMismatch);
    }

    let payload_path = artifact_root.join("payload");
    write_file_atomic_inner(&artifact_root, &payload_path, payload_bytes, false)?;
    write_artifact_manifest_atomic_inner(&artifact_root, manifest)?;
    Ok(())
}

/// Inputs for a streaming single-file artifact write whose content hash and size are NOT known
/// up front (an HTTP upload, a video file, a render being produced). Everything the manifest
/// needs except `content_hash` / `size_bytes` / `created_at`, plus the ingest ceiling.
#[derive(Debug, Clone)]
pub struct StreamingFileArtifactSpec {
    pub artifact_id: Uuid,
    pub layer: ArtifactLayer,
    pub mime: String,
    pub filename_hint: Option<String>,
    pub created_by_job_id: Option<Uuid>,
    pub source_entity_refs: Vec<EntityRef>,
    pub source_artifact_refs: Vec<ArtifactHandle>,
    pub classification: ArtifactClassification,
    pub exportable: bool,
    pub retention_ttl_days: Option<u32>,
    pub pinned: Option<bool>,
    /// Hard ceiling in bytes. The write aborts and attempts temporary-file cleanup when the
    /// stream would exceed it; there is no unbounded-ingest path on this store.
    pub max_bytes: u64,
}

/// Streaming, size-capped counterpart of [`write_file_artifact`]: bytes are hashed and counted
/// as they arrive and written to a temp file inside the artifact root, so peak memory is one
/// chunk, not the payload. The manifest is written only after the payload is fsynced and
/// published. Cancellation or a crash can leave temporary files or a payload without a
/// manifest; cleanup is best effort and L1 has no automatic reclamation. The same
/// write-once, hash-addressed contract as the buffered writer holds: an existing `payload` is
/// never overwritten, and the returned manifest's `content_hash` is the sha256 of exactly the
/// bytes on disk. This is the ingest primitive the Studio placed-asset binding needs for video
/// and other bulk binary ([STU-ASSET-008]).
///
/// On [`ArtifactError::SizeLimitExceeded`] this stops reading at the ceiling. An HTTP caller
/// should pass `&mut stream` (a `&mut S` is itself a `Stream` when `S: Stream + Unpin`) so it
/// keeps the body and can drain a bounded remainder before answering 413 — a server that stops
/// reading and responds immediately makes the peer observe a connection reset instead of the
/// status.
#[cfg(not(windows))]
async fn write_file_artifact_streaming_inner<S, B, E>(
    workspace_root: &Path,
    spec: StreamingFileArtifactSpec,
    mut stream: S,
) -> Result<ArtifactManifest, ArtifactError>
where
    S: futures_util::Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    E: std::fmt::Display,
{
    use tokio::io::AsyncWriteExt;

    if matches!(spec.classification, ArtifactClassification::High)
        && spec.retention_ttl_days.is_none()
    {
        return Err(ArtifactError::MissingRetentionTtlDays {
            artifact_id: spec.artifact_id,
            kind: ArtifactPayloadKind::File,
        });
    }
    if spec.max_bytes == 0 {
        return Err(ArtifactError::SizeLimitExceeded {
            limit_bytes: 0,
            observed_bytes: 0,
        });
    }

    let artifact_root = artifact_root_dir(workspace_root, spec.layer, spec.artifact_id);
    fs::create_dir_all(&artifact_root)?;
    let artifact_root_canon = fs::canonicalize(&artifact_root)?;
    let payload_path = artifact_root_canon.join("payload");
    if payload_path.exists() {
        return Err(ArtifactError::Io(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "artifact payload already exists (write-once store)",
        )));
    }
    ensure_root_escape_blocked(&artifact_root_canon, &payload_path)?;

    let tmp_path = artifact_root_canon.join(format!(".hsk_tmp_{}", Uuid::now_v7()));
    let mut tmp_file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)
        .await?;

    let mut hasher = Sha256::new();
    let mut observed: u64 = 0;
    let outcome: Result<(), ArtifactError> = async {
        while let Some(chunk) = futures_util::StreamExt::next(&mut stream).await {
            let chunk = chunk.map_err(|err| ArtifactError::Stream(err.to_string()))?;
            let bytes = chunk.as_ref();
            let next = observed.saturating_add(bytes.len() as u64);
            if next > spec.max_bytes {
                return Err(ArtifactError::SizeLimitExceeded {
                    limit_bytes: spec.max_bytes,
                    observed_bytes: next,
                });
            }
            hasher.update(bytes);
            tmp_file.write_all(bytes).await?;
            observed = next;
        }
        if observed == 0 {
            return Err(ArtifactError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "empty payload stream",
            )));
        }
        tmp_file.sync_all().await?;
        Ok(())
    }
    .await;
    drop(tmp_file);
    if let Err(err) = outcome {
        let _ = fs::remove_file(&tmp_path);
        let _ = fs::remove_dir(&artifact_root_canon);
        return Err(err);
    }

    // The preflight exists check can become stale while reading the stream.
    // Linking publishes only if the destination is still absent.
    if let Err(err) = fs::hard_link(&tmp_path, &payload_path) {
        let _ = fs::remove_file(&tmp_path);
        let _ = fs::remove_dir(&artifact_root_canon);
        return Err(ArtifactError::Io(err));
    }
    let _ = fs::remove_file(&tmp_path);
    if let Ok(dir_handle) = fs::File::open(&artifact_root_canon) {
        let _ = dir_handle.sync_all();
    }

    let manifest = ArtifactManifest {
        artifact_id: spec.artifact_id,
        layer: spec.layer,
        kind: ArtifactPayloadKind::File,
        mime: spec.mime,
        filename_hint: spec.filename_hint,
        created_at: Utc::now(),
        created_by_job_id: spec.created_by_job_id,
        source_entity_refs: spec.source_entity_refs,
        source_artifact_refs: spec.source_artifact_refs,
        content_hash: hex::encode(hasher.finalize()),
        size_bytes: observed,
        classification: spec.classification,
        exportable: spec.exportable,
        retention_ttl_days: spec.retention_ttl_days,
        pinned: spec.pinned,
        hash_basis: None,
        hash_exclude_paths: Vec::new(),
    };
    if let Err(err) = write_artifact_manifest_atomic_inner(&artifact_root_canon, &manifest) {
        // Publication succeeded for this writer; a losing writer returns above
        // and must never remove another writer's published payload.
        let _ = fs::remove_file(&payload_path);
        let _ = fs::remove_dir(&artifact_root_canon);
        return Err(err);
    }
    Ok(manifest)
}

/// Remove one artifact (payload + manifest + its directory). This is the compensating action a
/// caller takes when the catalog write that should have referenced a freshly written artifact
/// fails or dedups to an existing row. It attempts to remove the unreferenced payload;
/// cleanup errors propagate to the caller. Refuses anything outside the artifact store root.
fn remove_file_artifact_inner(
    workspace_root: &Path,
    layer: ArtifactLayer,
    artifact_id: Uuid,
) -> Result<(), ArtifactError> {
    let store_root = fs::canonicalize(artifact_store_root(workspace_root))?;
    let artifact_root = artifact_root_dir(workspace_root, layer, artifact_id);
    if !artifact_root.exists() {
        return Ok(());
    }
    let artifact_root_canon = fs::canonicalize(&artifact_root)?;
    if !artifact_root_canon.starts_with(&store_root) {
        return Err(ArtifactError::RootEscape {
            path: artifact_root_canon.to_string_lossy().to_string(),
        });
    }
    fs::remove_dir_all(&artifact_root_canon)?;
    Ok(())
}

fn write_dir_artifact_inner(
    workspace_root: &Path,
    manifest: &ArtifactManifest,
    entries: &[ArtifactWriteEntry],
) -> Result<(), ArtifactError> {
    if matches!(manifest.kind, ArtifactPayloadKind::PromptPayload)
        || matches!(manifest.classification, ArtifactClassification::High)
    {
        if manifest.retention_ttl_days.is_none() {
            return Err(ArtifactError::MissingRetentionTtlDays {
                artifact_id: manifest.artifact_id,
                kind: manifest.kind,
            });
        }
    }

    let artifact_root = artifact_root_dir(workspace_root, manifest.layer, manifest.artifact_id);
    let payload_root = artifact_root.join("payload");
    fs::create_dir_all(&payload_root)?;

    // Validate hash fields match entries (structural hashing).
    let (index, total_size) = compute_entries_index(entries, &manifest.hash_exclude_paths)?;
    let index_json = bundle_index_json(&index)?;
    let computed = bundle_index_content_hash(&index_json);
    if manifest.content_hash != computed || manifest.size_bytes != total_size {
        return Err(ArtifactError::ContentHashMismatch);
    }

    // Deterministic write order: lexicographic by normalized rel_path.
    let mut write_entries: Vec<ArtifactWriteEntry> = entries.to_vec();
    write_entries
        .sort_by(|a, b| normalize_rel_path(&a.rel_path).cmp(&normalize_rel_path(&b.rel_path)));

    for entry in &write_entries {
        let rel = normalize_rel_path(&entry.rel_path);
        ensure_safe_rel_path(&rel)?;
        let target_path = payload_root.join(Path::new(&rel));
        write_file_atomic_inner(&payload_root, &target_path, &entry.bytes, false)?;
    }

    write_artifact_manifest_atomic_inner(&artifact_root, manifest)?;
    Ok(())
}

fn common_artifact_gate_root(root: &Path) -> Result<PathBuf, ArtifactError> {
    let absolute = if root.is_absolute() { root.to_path_buf() } else { std::env::current_dir()?.join(root) };
    let root = absolute.as_path();
    let mut selected = root;
    for ancestor in root.ancestors() {
        if ancestor.file_name().is_some_and(|name| name == ARTIFACTS_DIR) {
            if let Some(handshake) = ancestor.parent() {
                if handshake.file_name().is_some_and(|name| name == HANDSHAKE_DIR) {
                    selected = handshake.parent().ok_or_else(|| ArtifactError::InvalidRelPath {
                        path: root.to_string_lossy().into_owned(),
                    })?;
                    break;
                }
            }
        }
    }
    while !selected.exists() {
        selected = selected.parent().ok_or_else(|| ArtifactError::InvalidRelPath {
            path: root.to_string_lossy().into_owned(),
        })?;
    }
    Ok(fs::canonicalize(selected)?)
}

#[cfg(windows)]
fn legacy_target_root(root: &Path) -> Result<PathBuf, ArtifactError> {
    let absolute = if root.is_absolute() { root.to_path_buf() } else { std::env::current_dir()?.join(root) };
    let mut existing = absolute.as_path();
    let mut missing = Vec::new();
    while !existing.exists() {
        missing.push(existing.file_name().ok_or_else(|| ArtifactError::InvalidRelPath {
            path: root.to_string_lossy().into_owned(),
        })?.to_os_string());
        existing = existing.parent().ok_or_else(|| ArtifactError::InvalidRelPath {
            path: root.to_string_lossy().into_owned(),
        })?;
    }
    let mut target = fs::canonicalize(existing)?;
    for name in missing.into_iter().rev() { target.push(name); }
    Ok(target)
}

#[cfg(windows)]
type LegacyPhysicalInventory = Arc<LegacyPhysicalResources>;

#[cfg(windows)]
struct LegacyPhysicalResources {
    native: Mutex<native_artifact_custody::Custody>,
    frames: Arc<tokio::sync::Semaphore>,
    workers: Mutex<Vec<tokio::task::JoinHandle<Result<(), ArtifactError>>>>,
    settled: Mutex<Vec<Result<Result<(), ArtifactError>, tokio::task::JoinError>>>,
    admission_frozen: std::sync::atomic::AtomicBool,
}

#[cfg(windows)]
fn retained_legacy_physical_inventories() -> &'static Mutex<Vec<LegacyPhysicalInventory>> {
    static INVENTORIES: std::sync::OnceLock<Mutex<Vec<LegacyPhysicalInventory>>> =
        std::sync::OnceLock::new();
    INVENTORIES.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(windows)]
struct LegacyArtifactPhysicalFlight {
    inventory: LegacyPhysicalInventory,
    _frame: tokio::sync::OwnedSemaphorePermit,
    physically_closed: bool,
    target_relative: PathBuf,
}

#[cfg(windows)]
impl LegacyArtifactPhysicalFlight {
    fn acquire(root: &Path) -> Result<Self, ArtifactError> {
        // Recovery is preserve-all physical release, never a Parent terminal.
        let _ = recover_retained_legacy_physical_inventories();
        let inventory = Arc::new(LegacyPhysicalResources {
            native: Mutex::new(native_artifact_custody::Custody::new_legacy()),
            frames: Arc::new(tokio::sync::Semaphore::new(1)),
            workers: Mutex::new(Vec::new()),
            settled: Mutex::new(Vec::new()),
            admission_frozen: std::sync::atomic::AtomicBool::new(false),
        });
        let frame = Arc::clone(&inventory.frames).try_acquire_owned().map_err(|_| {
            ArtifactError::PublicationCustody("legacy frame lease unavailable".into())
        })?;
        retained_legacy_physical_inventories().lock().map_err(|_| {
            ArtifactError::PublicationCustody("legacy physical inventory retention poisoned".into())
        })?.push(Arc::clone(&inventory));
        // Strong retention precedes resolution and the first native acquisition.
        let mut flight = Self { inventory, _frame: frame, physically_closed: false, target_relative: PathBuf::new() };
        let target = legacy_target_root(root)?;
        let root = common_artifact_gate_root(root)?;
        flight.target_relative = target.strip_prefix(&root).map_err(|_| {
            ArtifactError::PublicationCustody("legacy target escaped its physical anchor".into())
        })?.to_path_buf();
        flight.inventory.native.lock().map_err(|_| {
            ArtifactError::PublicationCustody("legacy physical inventory poisoned".into())
        })?.acquire(&root)?;
        Ok(flight)
    }

    fn finish<T>(&mut self, outcome: Result<T, ArtifactError>) -> Result<T, ArtifactError> {
        let value = outcome?;
        self.inventory.admission_frozen.store(true, std::sync::atomic::Ordering::SeqCst);
        if !observe_legacy_workers(&self.inventory)? {
            return Err(ArtifactError::PublicationCustody("legacy I/O has not settled".into()));
        }
        self.inventory.native.lock().map_err(|_| {
            ArtifactError::PublicationCustody("legacy physical inventory poisoned".into())
        })?.close_legacy_preserving()?;
        self.physically_closed = true;
        retained_legacy_physical_inventories().lock().map_err(|_| {
            ArtifactError::PublicationCustody("legacy physical inventory retention poisoned".into())
        })?.retain(|entry| !Arc::ptr_eq(entry, &self.inventory));
        Ok(value)
    }
}

#[cfg(windows)]
impl Drop for LegacyArtifactPhysicalFlight {
    fn drop(&mut self) {
        if !self.physically_closed {
            // Drop is not I/O settlement. Keep handles and inventory; do not close/delete.
            self.inventory.admission_frozen.store(true, std::sync::atomic::Ordering::SeqCst);
            if let Ok(mut inventory) = self.inventory.native.try_lock() {
                inventory.freeze_uncertainty();
            }
        }
    }
}

#[cfg(windows)]
fn observe_legacy_workers(inventory: &LegacyPhysicalInventory) -> Result<bool, ArtifactError> {
    use futures_util::FutureExt;
    let mut workers = inventory.workers.lock().map_err(|_| {
        ArtifactError::PublicationCustody("legacy worker inventory poisoned".into())
    })?;
    let mut settled = inventory.settled.lock().map_err(|_| {
        ArtifactError::PublicationCustody("legacy settled inventory poisoned".into())
    })?;
    while let Some(worker) = workers.last_mut() {
        // Poll a borrowed retained handle. Pending never consumes/detaches the owner.
        let Some(outcome) = (&mut *worker).now_or_never() else { return Ok(false); };
        settled.push(outcome);
        workers.pop();
    }
    Ok(true)
}

#[cfg(windows)]
fn recover_retained_legacy_physical_inventories() -> Result<usize, ArtifactError> {
    let inventories = retained_legacy_physical_inventories().lock().map_err(|_| {
        ArtifactError::PublicationCustody("legacy physical inventory retention poisoned".into())
    })?.clone();
    let mut released = 0;
    let mut first_error = None;
    for inventory in inventories {
        // A free owned frame lease establishes frame exclusion, not I/O settlement.
        let Ok(_frame) = Arc::clone(&inventory.frames).try_acquire_owned() else { continue; };
        inventory.admission_frozen.store(true, std::sync::atomic::Ordering::SeqCst);
        match observe_legacy_workers(&inventory) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(error) => { first_error.get_or_insert(error); continue; }
        }
        let Ok(mut native) = inventory.native.try_lock() else { continue; };
        native.freeze_uncertainty();
        if let Err(error) = native.close_legacy_preserving() {
            first_error.get_or_insert(error);
            continue;
        }
        drop(native);
        retained_legacy_physical_inventories().lock().map_err(|_| {
            ArtifactError::PublicationCustody("legacy physical inventory retention poisoned".into())
        })?.retain(|entry| !Arc::ptr_eq(entry, &inventory));
        released += 1;
    }
    match first_error { Some(error) => Err(error), None => Ok(released) }
}

#[cfg(windows)]
async fn run_legacy_blocking_io(
    flight: &LegacyArtifactPhysicalFlight,
    operation: impl FnOnce(&mut native_artifact_custody::Custody) -> Result<(), ArtifactError> + Send + 'static,
) -> Result<(), ArtifactError> {
    use std::future::Future;
    let inventory = Arc::clone(&flight.inventory);
    {
        let mut workers = inventory.workers.lock().map_err(|_| {
            ArtifactError::PublicationCustody("legacy worker inventory poisoned".into())
        })?;
        if inventory.admission_frozen.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(ArtifactError::PublicationCustody("legacy worker admission frozen".into()));
        }
        let (start, ready) = std::sync::mpsc::channel::<()>();
        let worker_inventory = Arc::clone(&inventory);
        let worker = tokio::task::spawn_blocking(move || {
            ready.recv().map_err(|_| ArtifactError::PublicationCustody("legacy worker was not admitted".into()))?;
            let mut native = worker_inventory.native.lock().map_err(|_| {
                ArtifactError::PublicationCustody("legacy physical inventory poisoned".into())
            })?;
            operation(&mut native)
        });
        workers.push(worker);
        // The worker cannot touch the filesystem before its actual join handle is retained.
        start.send(()).map_err(|_| ArtifactError::PublicationCustody("legacy worker start failed".into()))?;
    }
    std::future::poll_fn(|context| {
        let mut workers = match inventory.workers.lock() {
            Ok(workers) => workers,
            Err(_) => return std::task::Poll::Ready(Err(ArtifactError::PublicationCustody("legacy worker inventory poisoned".into()))),
        };
        let Some(worker) = workers.last_mut() else {
            return std::task::Poll::Ready(Err(ArtifactError::PublicationCustody("legacy worker join owner missing".into())));
        };
        let mut settled = match inventory.settled.lock() {
            Ok(settled) => settled,
            Err(_) => return std::task::Poll::Ready(Err(ArtifactError::PublicationCustody("legacy settled inventory poisoned".into()))),
        };
        let outcome = match std::pin::Pin::new(worker).poll(context) {
            std::task::Poll::Pending => return std::task::Poll::Pending,
            std::task::Poll::Ready(outcome) => outcome,
        };
        let error = match &outcome {
            Ok(Ok(())) => None,
            Ok(Err(ArtifactError::Io(error))) => Some(ArtifactError::Io(
                error.raw_os_error().map(io::Error::from_raw_os_error)
                    .unwrap_or_else(|| io::Error::new(error.kind(), error.to_string())))),
            Ok(Err(error)) => Some(ArtifactError::PublicationCustody(error.to_string())),
            Err(error) => Some(ArtifactError::PublicationCustody(format!("legacy I/O worker terminated: {error}"))),
        };
        settled.push(outcome);
        workers.pop();
        std::task::Poll::Ready(error.map_or(Ok(()), Err))
    }).await
}

fn with_common_artifact_gate<T>(
    root: &Path,
    operation: impl FnOnce() -> Result<T, ArtifactError>,
) -> Result<T, ArtifactError> {
    #[cfg(windows)]
    let mut gate = LegacyArtifactPhysicalFlight::acquire(root)?;
    #[cfg(not(windows))]
    let _ = root;
    let outcome = operation();
    #[cfg(windows)]
    return gate.finish(outcome);
    #[cfg(not(windows))]
    outcome
}

pub fn write_file_atomic(root: &Path, target_path: &Path, bytes: &[u8], overwrite: bool)
    -> Result<(), ArtifactError> {
    with_common_artifact_gate(root, || write_file_atomic_inner(root, target_path, bytes, overwrite))
}

pub fn materialize_local_dir(root: &Path, entries: &[ArtifactWriteEntry], overwrite: bool)
    -> Result<Vec<String>, ArtifactError> {
    with_common_artifact_gate(root, || materialize_local_dir_inner(root, entries, overwrite))
}

#[cfg(windows)]
async fn write_file_artifact_streaming_owned_legacy<S, B, E>(
    spec: StreamingFileArtifactSpec,
    mut stream: S,
    flight: &LegacyArtifactPhysicalFlight,
) -> Result<ArtifactManifest, ArtifactError>
where
    S: futures_util::Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    E: std::fmt::Display,
{
    if matches!(spec.classification, ArtifactClassification::High) && spec.retention_ttl_days.is_none() {
        return Err(ArtifactError::MissingRetentionTtlDays {
            artifact_id: spec.artifact_id, kind: ArtifactPayloadKind::File,
        });
    }
    if spec.max_bytes == 0 {
        return Err(ArtifactError::SizeLimitExceeded { limit_bytes: 0, observed_bytes: 0 });
    }
    let relative_root = flight.target_relative.join(HANDSHAKE_DIR).join(ARTIFACTS_DIR)
        .join(spec.layer.as_str()).join(spec.artifact_id.to_string());
    let temp = relative_root.join(format!(".hsk_tmp_{}", Uuid::now_v7()));
    let open_temp = temp.clone();
    run_legacy_blocking_io(flight, move |native| native.legacy_create_stream_file(&open_temp)).await?;
    let mut hasher = Sha256::new();
    let mut observed = 0u64;
    while let Some(chunk) = futures_util::StreamExt::next(&mut stream).await {
        let chunk = chunk.map_err(|error| ArtifactError::Stream(error.to_string()))?;
        let bytes = chunk.as_ref();
        let next = observed.saturating_add(bytes.len() as u64);
        if next > spec.max_bytes {
            return Err(ArtifactError::SizeLimitExceeded { limit_bytes: spec.max_bytes, observed_bytes: next });
        }
        hasher.update(bytes);
        let owned_bytes = bytes.to_vec();
        let write_temp = temp.clone();
        run_legacy_blocking_io(flight, move |native| {
            native.legacy_write_stream_chunk(&write_temp, &owned_bytes, false)
        }).await?;
        observed = next;
    }
    if observed == 0 {
        return Err(ArtifactError::Io(io::Error::new(io::ErrorKind::InvalidData, "empty payload stream")));
    }
    let manifest = ArtifactManifest {
        artifact_id: spec.artifact_id,
        layer: spec.layer,
        kind: ArtifactPayloadKind::File,
        mime: spec.mime,
        filename_hint: spec.filename_hint,
        created_at: Utc::now(),
        created_by_job_id: spec.created_by_job_id,
        source_entity_refs: spec.source_entity_refs,
        source_artifact_refs: spec.source_artifact_refs,
        content_hash: hex::encode(hasher.finalize()),
        size_bytes: observed,
        classification: spec.classification,
        exportable: spec.exportable,
        retention_ttl_days: spec.retention_ttl_days,
        pinned: spec.pinned,
        hash_basis: None,
        hash_exclude_paths: Vec::new(),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    let payload = relative_root.join("payload");
    let manifest_path = relative_root.join(ARTIFACT_MANIFEST_FILENAME);
    run_legacy_blocking_io(flight, move |native| {
        native.legacy_write_stream_chunk(&temp, &[], true)?;
        native.legacy_publish_stream_file(&temp, &payload)?;
        native.legacy_write_stream_manifest(&manifest_path, &manifest_bytes)
    }).await?;
    Ok(manifest)
}

pub async fn write_file_artifact_streaming<S, B, E>(
    root: &Path, spec: StreamingFileArtifactSpec, stream: S,
) -> Result<ArtifactManifest, ArtifactError>
where
    S: futures_util::Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    E: std::fmt::Display,
{
    #[cfg(windows)]
    let mut gate = LegacyArtifactPhysicalFlight::acquire(root)?;
    #[cfg(windows)]
    let outcome = write_file_artifact_streaming_owned_legacy(spec, stream, &gate).await;
    #[cfg(not(windows))]
    let outcome = write_file_artifact_streaming_inner(root, spec, stream).await;
    #[cfg(windows)]
    return gate.finish(outcome);
    #[cfg(not(windows))]
    outcome
}

pub fn write_artifact_manifest_atomic(root: &Path, manifest: &ArtifactManifest)
    -> Result<(), ArtifactError> {
    with_common_artifact_gate(root, || write_artifact_manifest_atomic_inner(root, manifest))
}

pub fn write_file_artifact(root: &Path, manifest: &ArtifactManifest, payload: &[u8])
    -> Result<(), ArtifactError> {
    with_common_artifact_gate(root, || write_file_artifact_inner(root, manifest, payload))
}

pub fn remove_file_artifact(root: &Path, layer: ArtifactLayer, id: Uuid)
    -> Result<(), ArtifactError> {
    with_common_artifact_gate(root, || remove_file_artifact_inner(root, layer, id))
}

pub fn write_dir_artifact(root: &Path, manifest: &ArtifactManifest, entries: &[ArtifactWriteEntry])
    -> Result<(), ArtifactError> {
    with_common_artifact_gate(root, || write_dir_artifact_inner(root, manifest, entries))
}

pub fn read_artifact_manifest(
    workspace_root: &Path,
    layer: ArtifactLayer,
    artifact_id: Uuid,
) -> Result<ArtifactManifest, ArtifactError> {
    let artifact_root = artifact_root_dir(workspace_root, layer, artifact_id);
    let manifest_path = artifact_root.join(ARTIFACT_MANIFEST_FILENAME);
    let raw = fs::read_to_string(&manifest_path)?;
    let manifest: ArtifactManifest = serde_json::from_str(&raw)?;
    Ok(manifest)
}

/// Read the raw payload bytes of a single-file artifact, fail-closed on a missing payload or a
/// content-hash / size mismatch against the manifest. This is the byte-fetch READ counterpart to
/// [`write_file_artifact`]: it reuses the same manifest-vs-payload validation as
/// [`validate_artifact_content_hash`] (file case) but returns the verified bytes so a runtime
/// surface (the Posekit source-image viewport, the Studio placed-asset resolver, the CKC asset
/// viewport) can display the REAL stored payload. Directory (bundle) artifacts are rejected
/// here — this reader serves the single `payload` file case only; callers needing bundle bytes
/// must enumerate the payload directory themselves. Never returns a fabricated or partial body:
/// any hash/size drift is a hard [`ArtifactError::ContentHashMismatch`].
///
/// Ported from `feat/WP-CKC-posekit-overhaul` (MT-043) onto the SurrealDB tree; the function is
/// storage-agnostic and identical in contract to the reference implementation.
pub fn read_file_artifact(
    workspace_root: &Path,
    layer: ArtifactLayer,
    artifact_id: Uuid,
) -> Result<Vec<u8>, ArtifactError> {
    let manifest = read_artifact_manifest(workspace_root, layer, artifact_id)?;
    read_file_artifact_with_manifest(workspace_root, &manifest)
}

/// Same contract as [`read_file_artifact`] for a caller that already holds the manifest and must
/// not pay a second manifest read (the media byte route cross-checks the manifest against the
/// catalog row before deciding whether to touch the payload at all).
pub fn read_file_artifact_with_manifest(
    workspace_root: &Path,
    manifest: &ArtifactManifest,
) -> Result<Vec<u8>, ArtifactError> {
    let artifact_root = artifact_root_dir(workspace_root, manifest.layer, manifest.artifact_id);
    let payload_path = artifact_root.join("payload");
    let meta = fs::metadata(&payload_path)?;
    if !meta.is_file() {
        // A directory (bundle) or other resolved non-file payload is not byte-servable.
        // Metadata follows symlinks; the resolved file still requires size and hash checks.
        return Err(ArtifactError::InvalidRelPath {
            path: payload_path.to_string_lossy().to_string(),
        });
    }
    if meta.len() != manifest.size_bytes {
        // Cheap pre-check: refuse before reading a payload whose on-disk size already disagrees
        // with the manifest (a torn write or truncated copy). The full hash check below still
        // runs for the equal-size case.
        return Err(ArtifactError::ContentHashMismatch);
    }

    let bytes = fs::read(&payload_path)?;
    if sha256_hex(&bytes) != manifest.content_hash || bytes.len() as u64 != manifest.size_bytes {
        return Err(ArtifactError::ContentHashMismatch);
    }
    Ok(bytes)
}

pub fn validate_artifact_content_hash(
    workspace_root: &Path,
    layer: ArtifactLayer,
    artifact_id: Uuid,
) -> Result<(), ArtifactError> {
    let manifest = read_artifact_manifest(workspace_root, layer, artifact_id)?;
    let artifact_root = artifact_root_dir(workspace_root, layer, artifact_id);

    let payload_path = artifact_root.join("payload");
    let meta = fs::metadata(&payload_path)?;
    if meta.is_file() {
        let bytes = fs::read(payload_path)?;
        if sha256_hex(&bytes) != manifest.content_hash
            || bytes.len() as u64 != manifest.size_bytes
        {
            return Err(ArtifactError::ContentHashMismatch);
        }
        return Ok(());
    }

    if !meta.is_dir() {
        return Err(ArtifactError::InvalidRelPath {
            path: payload_path.to_string_lossy().to_string(),
        });
    }

    let payload_root = payload_path;
    let mut entries: Vec<ArtifactWriteEntry> = Vec::new();
    collect_dir_entries(&payload_root, &payload_root, &mut entries)?;
    let (index, total_size) = compute_entries_index(&entries, &manifest.hash_exclude_paths)?;
    let index_json = bundle_index_json(&index)?;
    let computed = bundle_index_content_hash(&index_json);
    if computed != manifest.content_hash || total_size != manifest.size_bytes {
        return Err(ArtifactError::ContentHashMismatch);
    }
    Ok(())
}

fn collect_dir_entries(
    root: &Path,
    dir: &Path,
    out: &mut Vec<ArtifactWriteEntry>,
) -> Result<(), ArtifactError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            return Err(ArtifactError::InvalidRelPath {
                path: entry.path().to_string_lossy().to_string(),
            });
        }
        let path = entry.path();
        if ty.is_dir() {
            collect_dir_entries(root, &path, out)?;
            continue;
        }
        if !ty.is_file() {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .map_err(|_| ArtifactError::InvalidRelPath {
                path: path.to_string_lossy().to_string(),
            })?;
        let rel = normalize_rel_path(&rel.to_string_lossy());
        let bytes = fs::read(&path)?;
        out.push(ArtifactWriteEntry {
            rel_path: rel,
            bytes,
        });
    }
    Ok(())
}

pub fn normalize_materialized_path(path: &str) -> Result<String, ArtifactError> {
    let rel = normalize_rel_path(path);
    if rel.is_empty() || rel.contains(':') || rel.starts_with('/') {
        return Err(ArtifactError::InvalidRelPath { path: rel });
    }
    if rel.split('/').any(|c| c == "..") {
        return Err(ArtifactError::PathTraversalBlocked { path: rel });
    }
    Ok(rel)
}

