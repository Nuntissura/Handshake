use std::{
    ffi::c_void,
    fs::{self, File, OpenOptions},
    io::Write,
    os::windows::fs::OpenOptionsExt,
    os::windows::io::AsRawHandle,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicU8, Ordering},
        Mutex, OnceLock,
    },
};

use serde::{Deserialize, Serialize};
use windows_sys::{
    core::PWSTR,
    Win32::{
        Foundation::{
            CloseHandle, LocalFree, ERROR_SHARING_VIOLATION, ERROR_SUCCESS, HANDLE, HLOCAL,
            WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
        },
        Security::{
            Authorization::{
                ConvertStringSidToSidW, GetNamedSecurityInfoW, SetEntriesInAclW,
                SetNamedSecurityInfoW, EXPLICIT_ACCESS_W, REVOKE_ACCESS, SE_FILE_OBJECT,
                TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
            },
            ACL, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
        },
        Storage::FileSystem::{
            FileIdInfo, GetFileInformationByHandleEx, MoveFileExW, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_ID_INFO, FILE_SHARE_READ, FILE_SHARE_WRITE, MOVEFILE_REPLACE_EXISTING,
            MOVEFILE_WRITE_THROUGH,
        },
        System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject},
    },
};

use super::restricted_appcontainer::StartupCancellation;

const ACL_JOURNAL_SCHEMA_VERSION: u32 = 2;
const ACL_JOURNAL_DIR: &str = "acl-journals";
const DETACHED_PROFILE_PREFIX: &str = "handshake.mt046.";
const ATTACHED_PROFILE_PREFIX: &str = "handshake.native.attached.";

static APP_CONTAINER_PROFILE_API_LOCK: Mutex<()> = Mutex::new(());
static APP_CONTAINER_ACL_API_LOCK: Mutex<()> = Mutex::new(());
static ACL_GRANT_STAGE_DETAIL: Mutex<String> = Mutex::new(String::new());
static DEFAULT_ACL_RECOVERY: OnceLock<Result<(), String>> = OnceLock::new();
static DEFAULT_ACL_RECOVERY_STAGE: AtomicU8 = AtomicU8::new(0);
const PROFILE_API_MUTEX_NAME: &str = "Local\\Handshake.AppContainerProfileApi";
const PROFILE_API_MUTEX_TIMEOUT_MS: u32 = 5_000;

struct NamedProfileApiMutexGuard {
    handle: HANDLE,
}

impl NamedProfileApiMutexGuard {
    fn acquire() -> Result<Self, String> {
        let name = to_wide(std::ffi::OsStr::new(PROFILE_API_MUTEX_NAME));
        let handle = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(format!(
                "CreateMutexW for AppContainer profile API serialization failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let wait = unsafe { WaitForSingleObject(handle, PROFILE_API_MUTEX_TIMEOUT_MS) };
        if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {
            Ok(Self { handle })
        } else {
            let error = if wait == WAIT_TIMEOUT {
                format!(
                    "cross-process AppContainer profile API mutex exceeded {} ms",
                    PROFILE_API_MUTEX_TIMEOUT_MS
                )
            } else {
                format!(
                    "WaitForSingleObject for AppContainer profile API mutex failed: {}",
                    std::io::Error::last_os_error()
                )
            };
            unsafe {
                let _ = CloseHandle(handle);
            }
            Err(error)
        }
    }
}

impl Drop for NamedProfileApiMutexGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.handle);
            let _ = CloseHandle(self.handle);
        }
    }
}

pub(super) fn default_acl_recovery_stage_name() -> &'static str {
    match DEFAULT_ACL_RECOVERY_STAGE.load(Ordering::Acquire) {
        1 => "journal discovery",
        2 => "journal lock acquisition",
        3 => "journal read and validation",
        4 => "profile SID derivation",
        5 => "ACL grant revocation",
        6 => "AppContainer profile deletion",
        7 => "journal removal",
        _ => "not started",
    }
}

pub(super) fn acl_grant_stage_detail() -> String {
    ACL_GRANT_STAGE_DETAIL
        .lock()
        .map(|detail| detail.clone())
        .unwrap_or_else(|_| "ACL grant diagnostic lock poisoned".to_string())
}

fn set_acl_grant_stage_detail(operation: &str, path: &Path) {
    if let Ok(mut detail) = ACL_GRANT_STAGE_DETAIL.lock() {
        *detail = format!("{operation} for {}", path.display());
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AclGrantTargetKind {
    File,
    Directory,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct AclGrantTarget {
    pub(super) path: PathBuf,
    pub(super) kind: AclGrantTargetKind,
    pub(super) access_mask: u32,
    #[serde(default)]
    identity: Option<AclObjectIdentity>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct AclObjectIdentity {
    volume_serial_number: u64,
    file_id: [u8; 16],
}

impl AclGrantTarget {
    pub(super) fn file(path: PathBuf, access_mask: u32) -> Self {
        Self {
            path,
            kind: AclGrantTargetKind::File,
            access_mask,
            identity: None,
        }
    }

    pub(super) fn directory(path: PathBuf, access_mask: u32) -> Self {
        Self {
            path,
            kind: AclGrantTargetKind::Directory,
            access_mask,
            identity: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AclJournalPhase {
    Prepared,
    ProfileCreated,
    GrantsApplied,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AclGrantJournal {
    schema_version: u32,
    profile_name: String,
    package_sid_sddl: Option<String>,
    phase: AclJournalPhase,
    targets: Vec<AclGrantTarget>,
}

#[derive(Clone, Debug)]
struct AclJournalStore {
    root: PathBuf,
}

impl AclJournalStore {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path_for(&self, profile_name: &str) -> PathBuf {
        self.root.join(format!("{profile_name}.json"))
    }

    fn lock_path_for(&self, profile_name: &str) -> PathBuf {
        self.root.join(format!("{profile_name}.lock"))
    }

    fn acquire_lock(&self, profile_name: &str) -> std::io::Result<File> {
        fs::create_dir_all(&self.root)?;
        OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .share_mode(0)
            .open(self.lock_path_for(profile_name))
    }

    fn persist(&self, journal: &AclGrantJournal) -> Result<PathBuf, String> {
        fs::create_dir_all(&self.root).map_err(|error| {
            format!(
                "create AppContainer ACL journal directory {}: {error}",
                self.root.display()
            )
        })?;
        let destination = self.path_for(&journal.profile_name);
        let temporary = self.root.join(format!(
            "{}.{}.tmp",
            journal.profile_name,
            uuid::Uuid::now_v7().simple()
        ));
        let bytes = serde_json::to_vec(journal)
            .map_err(|error| format!("serialize AppContainer ACL journal: {error}"))?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| {
                format!(
                    "create AppContainer ACL journal temporary file {}: {error}",
                    temporary.display()
                )
            })?;
        if let Err(error) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&temporary);
            return Err(format!(
                "durably write AppContainer ACL journal {}: {error}",
                temporary.display()
            ));
        }
        drop(file);
        move_file_replace_write_through(&temporary, &destination).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            format!(
                "commit AppContainer ACL journal {}: {error}",
                destination.display()
            )
        })?;
        Ok(destination)
    }

    fn remove(&self, profile_name: &str) -> Result<(), String> {
        let path = self.path_for(profile_name);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "remove completed AppContainer ACL journal {}: {error}",
                path.display()
            )),
        }
    }

    fn remove_lock(&self, profile_name: &str) -> Result<(), String> {
        let path = self.lock_path_for(profile_name);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "remove completed AppContainer ACL journal lock {}: {error}",
                path.display()
            )),
        }
    }
}

pub(super) struct AppContainerAclTransaction {
    store: AclJournalStore,
    journal: AclGrantJournal,
    lock_file: Option<File>,
    profile: Option<rappct::AppContainerProfile>,
    target_handles: Vec<File>,
    completed: bool,
}

impl AppContainerAclTransaction {
    pub(super) fn begin(
        profile_name: String,
        targets: Vec<AclGrantTarget>,
    ) -> Result<Self, String> {
        let journal_root = default_acl_journal_root()?;
        recover_pending_acl_transactions(journal_root.clone())?;
        Self::begin_in_store(journal_root, profile_name, targets)
    }

    fn begin_in_store(
        journal_root: PathBuf,
        profile_name: String,
        targets: Vec<AclGrantTarget>,
    ) -> Result<Self, String> {
        validate_profile_name(&profile_name)?;
        if let Some(target) = targets.iter().find(|target| !target.path.is_absolute()) {
            return Err(format!(
                "AppContainer ACL journal target must be absolute: {}",
                target.path.display()
            ));
        }
        let store = AclJournalStore::new(journal_root);
        let lock_file = store.acquire_lock(&profile_name).map_err(|error| {
            format!("acquire AppContainer ACL transaction lock for {profile_name}: {error}")
        })?;
        let journal = AclGrantJournal {
            schema_version: ACL_JOURNAL_SCHEMA_VERSION,
            profile_name,
            package_sid_sddl: None,
            phase: AclJournalPhase::Prepared,
            targets,
        };
        if let Err(error) = store.persist(&journal) {
            drop(lock_file);
            let _ = store.remove_lock(&journal.profile_name);
            return Err(error);
        }
        Ok(Self {
            store,
            journal,
            lock_file: Some(lock_file),
            profile: None,
            target_handles: Vec::new(),
            completed: false,
        })
    }

    pub(super) fn create_profile(
        &mut self,
        display: &str,
        description: Option<&str>,
    ) -> Result<(), String> {
        let profile =
            ensure_appcontainer_profile(&self.journal.profile_name, display, description)?;
        self.journal.package_sid_sddl = Some(profile.sid.as_string().to_string());
        self.journal.phase = AclJournalPhase::ProfileCreated;
        self.profile = Some(profile);
        self.store.persist(&self.journal)?;
        Ok(())
    }

    pub(super) fn profile(&self) -> Result<&rappct::AppContainerProfile, String> {
        self.profile
            .as_ref()
            .ok_or_else(|| "AppContainer ACL transaction has no created profile".to_string())
    }

    pub(super) fn grant_all(&mut self) -> Result<(), String> {
        self.grant_all_inner(None)
    }

    pub(super) fn grant_all_cancellable(
        &mut self,
        cancellation: &StartupCancellation,
    ) -> Result<(), String> {
        self.grant_all_inner(Some(cancellation))
    }

    fn grant_all_inner(
        &mut self,
        cancellation: Option<&StartupCancellation>,
    ) -> Result<(), String> {
        let profile = self.profile()?;
        let sid = profile.sid.clone();
        for index in 0..self.journal.targets.len() {
            if cancellation
                .map(StartupCancellation::is_cancelled)
                .unwrap_or(false)
            {
                return Err(self.rollback_grant_failure(format!(
                    "attached startup cancelled before ACL grant {}",
                    self.journal.targets[index].path.display()
                )));
            }
            let (handle, identity) = match lock_target_identity(&self.journal.targets[index]) {
                Ok(locked) => locked,
                Err(error) => return Err(self.rollback_grant_failure(error)),
            };
            self.journal.targets[index].identity = Some(identity);
            if let Err(error) = self.store.persist(&self.journal) {
                return Err(self.rollback_grant_failure(error));
            }
            self.target_handles.push(handle);
            if let Err(error) = grant_target_to_package(&self.journal.targets[index], &sid) {
                return Err(self.rollback_grant_failure(error));
            }
            if cancellation
                .map(StartupCancellation::is_cancelled)
                .unwrap_or(false)
            {
                return Err(self.rollback_grant_failure(format!(
                    "attached startup cancelled after ACL grant {}",
                    self.journal.targets[index].path.display()
                )));
            }
        }
        self.journal.phase = AclJournalPhase::GrantsApplied;
        if let Err(error) = self.store.persist(&self.journal) {
            let cleanup_error = self.cleanup().err();
            return Err(match cleanup_error {
                Some(cleanup_error) => format!(
                    "{error}; rollback also failed and recovery journal was retained: {cleanup_error}"
                ),
                None => error,
            });
        }
        Ok(())
    }

    fn rollback_grant_failure(&mut self, error: String) -> String {
        match self.cleanup() {
            Ok(()) => error,
            Err(cleanup_error) => format!(
                "{error}; rollback also failed and recovery journal was retained: {cleanup_error}"
            ),
        }
    }

    pub(super) fn cleanup(&mut self) -> Result<(), String> {
        if self.completed {
            return Ok(());
        }
        let sid_sddl = self
            .profile
            .as_ref()
            .map(|profile| profile.sid.as_string().to_string())
            .or_else(|| self.journal.package_sid_sddl.clone());
        if let Some(sid_sddl) = sid_sddl.as_deref() {
            revoke_all_targets(&self.journal.targets, sid_sddl)?;
        }
        self.target_handles.clear();
        if let Some(profile) = self.profile.take() {
            delete_appcontainer_profile(profile)?;
        } else if self.journal.package_sid_sddl.is_some() {
            let profile = derive_appcontainer_profile(&self.journal.profile_name)?;
            delete_appcontainer_profile(profile)?;
        }
        self.store.remove(&self.journal.profile_name)?;
        drop(self.lock_file.take());
        self.store.remove_lock(&self.journal.profile_name)?;
        self.completed = true;
        Ok(())
    }
}

impl Drop for AppContainerAclTransaction {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!(
                "windows_native_jail: AppContainer ACL cleanup incomplete for {}; recovery journal retained: {error}",
                self.journal.profile_name
            );
        }
    }
}

pub(super) fn ensure_default_acl_recovery() -> Result<(), String> {
    DEFAULT_ACL_RECOVERY
        .get_or_init(|| default_acl_journal_root().and_then(recover_pending_acl_transactions))
        .clone()
}

pub(super) fn ensure_appcontainer_profile(
    name: &str,
    display: &str,
    description: Option<&str>,
) -> Result<rappct::AppContainerProfile, String> {
    let _guard = APP_CONTAINER_PROFILE_API_LOCK
        .lock()
        .map_err(|_| "AppContainer profile API lock poisoned".to_string())?;
    let _cross_process_guard = NamedProfileApiMutexGuard::acquire()?;
    rappct::AppContainerProfile::ensure(name, display, description)
        .map_err(|error| error.to_string())
}

pub(super) fn delete_appcontainer_profile(
    profile: rappct::AppContainerProfile,
) -> Result<(), String> {
    let _guard = APP_CONTAINER_PROFILE_API_LOCK
        .lock()
        .map_err(|_| "AppContainer profile API lock poisoned".to_string())?;
    let _cross_process_guard = NamedProfileApiMutexGuard::acquire()?;
    profile.delete().map_err(|error| error.to_string())
}

fn derive_appcontainer_profile(name: &str) -> Result<rappct::AppContainerProfile, String> {
    let _guard = APP_CONTAINER_PROFILE_API_LOCK
        .lock()
        .map_err(|_| "AppContainer profile API lock poisoned".to_string())?;
    let _cross_process_guard = NamedProfileApiMutexGuard::acquire()?;
    let sid = rappct::derive_sid_from_name(name).map_err(|error| error.to_string())?;
    Ok(rappct::AppContainerProfile {
        name: name.to_string(),
        sid,
    })
}

fn default_acl_journal_root() -> Result<PathBuf, String> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        "LOCALAPPDATA is unavailable; refusing non-durable AppContainer ACL grants".to_string()
    })?;
    Ok(PathBuf::from(local_app_data)
        .join("Handshake")
        .join("runtime")
        .join("windows-native-jail")
        .join(ACL_JOURNAL_DIR))
}

fn recover_pending_acl_transactions(root: PathBuf) -> Result<(), String> {
    DEFAULT_ACL_RECOVERY_STAGE.store(1, Ordering::Release);
    let store = AclJournalStore::new(root.clone());
    if !root.exists() {
        return Ok(());
    }
    let mut journals = Vec::new();
    for entry in fs::read_dir(&root)
        .map_err(|error| format!("read AppContainer ACL journal directory: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("read AppContainer ACL journal entry: {error}"))?
            .path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            journals.push(path);
        }
    }
    journals.sort();
    for path in journals {
        let profile_name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                format!(
                    "invalid AppContainer ACL journal filename {}",
                    path.display()
                )
            })?;
        validate_profile_name(profile_name)?;
        DEFAULT_ACL_RECOVERY_STAGE.store(2, Ordering::Release);
        let lock_file = match store.acquire_lock(profile_name) {
            Ok(lock_file) => lock_file,
            Err(error) if error.raw_os_error() == Some(ERROR_SHARING_VIOLATION as i32) => {
                continue;
            }
            Err(error) => {
                return Err(format!(
                    "acquire AppContainer ACL recovery lock for {profile_name}: {error}"
                ));
            }
        };
        DEFAULT_ACL_RECOVERY_STAGE.store(3, Ordering::Release);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                drop(lock_file);
                store.remove_lock(profile_name)?;
                continue;
            }
            Err(error) => {
                return Err(format!(
                    "read AppContainer ACL journal {}: {error}",
                    path.display()
                ));
            }
        };
        let journal: AclGrantJournal = serde_json::from_slice(&bytes).map_err(|error| {
            format!(
                "parse AppContainer ACL journal {} fail-closed: {error}",
                path.display()
            )
        })?;
        if journal.schema_version != ACL_JOURNAL_SCHEMA_VERSION {
            return Err(format!(
                "unsupported AppContainer ACL journal schema {} in {}",
                journal.schema_version,
                path.display()
            ));
        }
        validate_profile_name(&journal.profile_name)?;
        if path.file_stem().and_then(|value| value.to_str()) != Some(&journal.profile_name) {
            return Err(format!(
                "AppContainer ACL journal filename/profile mismatch in {}",
                path.display()
            ));
        }
        if let Some(target) = journal
            .targets
            .iter()
            .find(|target| !target.path.is_absolute())
        {
            return Err(format!(
                "AppContainer ACL journal contains non-absolute target {}",
                target.path.display()
            ));
        }
        // Recovery must never create new OS state. SID derivation is
        // deterministic from the validated profile name and lets us verify
        // and revoke any persisted grants before deleting the old profile.
        // Windows documents that deleting a non-existent AppContainer profile
        // succeeds, which also covers a crash before profile creation.
        DEFAULT_ACL_RECOVERY_STAGE.store(4, Ordering::Release);
        let profile = derive_appcontainer_profile(&journal.profile_name)?;
        if let Some(expected_sid) = journal.package_sid_sddl.as_deref() {
            if !profile.sid.as_string().eq_ignore_ascii_case(expected_sid) {
                return Err(format!(
                    "AppContainer ACL journal SID mismatch for {}",
                    journal.profile_name
                ));
            }
        }
        DEFAULT_ACL_RECOVERY_STAGE.store(5, Ordering::Release);
        revoke_all_targets(&journal.targets, profile.sid.as_string())?;
        DEFAULT_ACL_RECOVERY_STAGE.store(6, Ordering::Release);
        delete_appcontainer_profile(profile)?;
        DEFAULT_ACL_RECOVERY_STAGE.store(7, Ordering::Release);
        store.remove(&journal.profile_name)?;
        drop(lock_file);
        store.remove_lock(&journal.profile_name)?;
    }
    Ok(())
}

fn validate_profile_name(profile_name: &str) -> Result<(), String> {
    if profile_name.starts_with(DETACHED_PROFILE_PREFIX)
        || profile_name.starts_with(ATTACHED_PROFILE_PREFIX)
        || profile_name.starts_with("handshake.acl.test.")
    {
        Ok(())
    } else {
        Err(format!(
            "refusing AppContainer ACL journal for unrecognized profile name {profile_name}"
        ))
    }
}

fn grant_target_to_package(
    target: &AclGrantTarget,
    sid: &rappct::sid::AppContainerSid,
) -> Result<(), String> {
    let _guard = APP_CONTAINER_ACL_API_LOCK
        .lock()
        .map_err(|_| "AppContainer ACL API lock poisoned".to_string())?;
    set_acl_grant_stage_detail("DACL inspection", &target.path);
    require_non_null_dacl(&target.path)?;
    let resource = match target.kind {
        AclGrantTargetKind::File => rappct::acl::ResourcePath::File(target.path.clone()),
        AclGrantTargetKind::Directory => rappct::acl::ResourcePath::Directory(target.path.clone()),
    };
    set_acl_grant_stage_detail("DACL grant write", &target.path);
    rappct::acl::grant_to_package(resource, sid, rappct::acl::AccessMask(target.access_mask))
        .map_err(|error| {
            format!(
                "rappct AppContainer ACL grant failed for {}: {error}",
                target.path.display()
            )
        })
}

fn require_non_null_dacl(path: &Path) -> Result<(), String> {
    let path_w = to_wide(path.as_os_str());
    let mut dacl: *mut ACL = null_mut();
    let mut descriptor: PSECURITY_DESCRIPTOR = null_mut();
    let status = unsafe {
        GetNamedSecurityInfoW(
            path_w.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut descriptor,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!(
            "GetNamedSecurityInfoW failed for {} before AppContainer ACL grant: Win32 error {status}",
            path.display()
        ));
    }
    let _descriptor_guard = LocalAllocGuard::new(descriptor);
    if dacl.is_null() {
        return Err(format!(
            "refusing AppContainer ACL grant on {} because it has a null DACL whose semantics cannot be preserved by SID-specific revoke",
            path.display()
        ));
    }
    Ok(())
}

fn lock_target_identity(target: &AclGrantTarget) -> Result<(File, AclObjectIdentity), String> {
    let handle = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(&target.path)
        .map_err(|error| {
            format!(
                "open AppContainer ACL target {} with rename/delete exclusion: {error}",
                target.path.display()
            )
        })?;
    let identity = identity_from_handle(&handle, &target.path)?;
    Ok((handle, identity))
}

fn identity_from_handle(handle: &File, path: &Path) -> Result<AclObjectIdentity, String> {
    let mut info = FILE_ID_INFO::default();
    let ok = unsafe {
        GetFileInformationByHandleEx(
            handle.as_raw_handle() as _,
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if ok == 0 {
        return Err(format!(
            "GetFileInformationByHandleEx(FileIdInfo) failed for AppContainer ACL target {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(AclObjectIdentity {
        volume_serial_number: info.VolumeSerialNumber,
        file_id: info.FileId.Identifier,
    })
}

fn revoke_package_sid(target: &AclGrantTarget, sid_sddl: &str) -> Result<(), String> {
    let expected_identity = target.identity.as_ref().ok_or_else(|| {
        format!(
            "refusing AppContainer SID revoke for {} because its durable file identity is absent",
            target.path.display()
        )
    })?;
    let (_identity_lock, actual_identity) = lock_target_identity(target)?;
    if &actual_identity != expected_identity {
        return Err(format!(
            "refusing AppContainer SID revoke for {} because the current path identity does not match the journaled grant target",
            target.path.display()
        ));
    }
    // The journal persists target identity before the ACL write. A crash or
    // deadline can therefore leave an ambiguous record where no ACE was ever
    // applied. Inspecting first avoids an unnecessary DACL rewrite (and its
    // directory inheritance propagation) while still revoking every SID that
    // is actually present.
    if !acl_contains_sid(&target.path, sid_sddl)? {
        return Ok(());
    }
    let _guard = APP_CONTAINER_ACL_API_LOCK
        .lock()
        .map_err(|_| "AppContainer ACL API lock poisoned".to_string())?;
    let path_w = to_wide(target.path.as_os_str());
    let sid_w = to_wide(std::ffi::OsStr::new(sid_sddl));
    let mut sid: PSID = null_mut();
    if unsafe { ConvertStringSidToSidW(sid_w.as_ptr(), &mut sid) } == 0 {
        return Err(format!(
            "ConvertStringSidToSidW failed for AppContainer SID {sid_sddl}: {}",
            std::io::Error::last_os_error()
        ));
    }
    let _sid_guard = LocalAllocGuard::new(sid);

    let mut old_dacl: *mut ACL = null_mut();
    let mut security_descriptor: PSECURITY_DESCRIPTOR = null_mut();
    let status = unsafe {
        GetNamedSecurityInfoW(
            path_w.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            &mut old_dacl,
            null_mut(),
            &mut security_descriptor,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!(
            "GetNamedSecurityInfoW failed for {} while revoking AppContainer SID: Win32 error {status}",
            target.path.display()
        ));
    }
    let _descriptor_guard = LocalAllocGuard::new(security_descriptor);

    let mut trustee = TRUSTEE_W::default();
    trustee.TrusteeForm = TRUSTEE_IS_SID;
    trustee.TrusteeType = TRUSTEE_IS_UNKNOWN;
    trustee.ptstrName = sid.cast::<u16>() as PWSTR;
    let entry = EXPLICIT_ACCESS_W {
        grfAccessPermissions: 0,
        grfAccessMode: REVOKE_ACCESS,
        grfInheritance: 0,
        Trustee: trustee,
    };
    let mut new_dacl: *mut ACL = null_mut();
    let status = unsafe { SetEntriesInAclW(1, &entry, old_dacl, &mut new_dacl) };
    if status != ERROR_SUCCESS {
        return Err(format!(
            "SetEntriesInAclW failed for {} while revoking AppContainer SID: Win32 error {status}",
            target.path.display()
        ));
    }
    let _new_dacl_guard = LocalAllocGuard::new(new_dacl);
    let status = unsafe {
        SetNamedSecurityInfoW(
            path_w.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            new_dacl,
            null(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!(
            "SetNamedSecurityInfoW failed for {} while revoking AppContainer SID: Win32 error {status}",
            target.path.display()
        ));
    }
    Ok(())
}

fn acl_contains_sid(path: &Path, sid_sddl: &str) -> Result<bool, String> {
    use windows_sys::Win32::Security::{
        AclSizeInformation, EqualSid, GetAce, GetAclInformation, ACCESS_ALLOWED_ACE, ACE_HEADER,
        ACL_SIZE_INFORMATION,
    };

    const ACCESS_ALLOWED_ACE_TYPE_VALUE: u8 = 0;

    let path_w = to_wide(path.as_os_str());
    let sid_w = to_wide(std::ffi::OsStr::new(sid_sddl));
    let mut sid: PSID = null_mut();
    if unsafe { ConvertStringSidToSidW(sid_w.as_ptr(), &mut sid) } == 0 {
        return Err(format!(
            "ConvertStringSidToSidW failed while inspecting AppContainer SID {sid_sddl}: {}",
            std::io::Error::last_os_error()
        ));
    }
    let _sid_guard = LocalAllocGuard::new(sid);
    let mut dacl: *mut ACL = null_mut();
    let mut descriptor: PSECURITY_DESCRIPTOR = null_mut();
    let status = unsafe {
        GetNamedSecurityInfoW(
            path_w.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut descriptor,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!(
            "GetNamedSecurityInfoW failed for {} while inspecting AppContainer SID: Win32 error {status}",
            path.display()
        ));
    }
    let _descriptor_guard = LocalAllocGuard::new(descriptor);
    if dacl.is_null() {
        return Ok(false);
    }
    let mut info = ACL_SIZE_INFORMATION::default();
    if unsafe {
        GetAclInformation(
            dacl,
            (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
            std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    } == 0
    {
        return Err(format!(
            "GetAclInformation failed for {} while inspecting AppContainer SID: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    for index in 0..info.AceCount {
        let mut ace: *mut c_void = null_mut();
        if unsafe { GetAce(dacl, index, &mut ace) } == 0 {
            return Err(format!(
                "GetAce failed for {} while inspecting AppContainer SID: {}",
                path.display(),
                std::io::Error::last_os_error()
            ));
        }
        let header = unsafe { &*(ace.cast::<ACE_HEADER>()) };
        if header.AceType != ACCESS_ALLOWED_ACE_TYPE_VALUE {
            continue;
        }
        let allowed = ace.cast::<ACCESS_ALLOWED_ACE>();
        let ace_sid = unsafe { (&mut (*allowed).SidStart as *mut u32).cast::<c_void>() };
        if unsafe { EqualSid(ace_sid, sid) } != 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

fn revoke_all_targets(targets: &[AclGrantTarget], sid_sddl: &str) -> Result<(), String> {
    let errors = targets
        .iter()
        .rev()
        .filter(|target| target.identity.is_some())
        .filter_map(|target| revoke_package_sid(target, sid_sddl).err())
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "AppContainer SID revoke failed for {} target(s): {}",
            errors.len(),
            errors.join(" | ")
        ))
    }
}

fn move_file_replace_write_through(source: &Path, destination: &Path) -> Result<(), String> {
    let source_w = to_wide(source.as_os_str());
    let destination_w = to_wide(destination.as_os_str());
    let ok = unsafe {
        MoveFileExW(
            source_w.as_ptr(),
            destination_w.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

fn to_wide(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().chain(std::iter::once(0)).collect()
}

struct LocalAllocGuard(*mut c_void);

impl LocalAllocGuard {
    fn new<T>(pointer: *mut T) -> Self {
        Self(pointer.cast())
    }
}

impl Drop for LocalAllocGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0 as HLOCAL);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profile_name(label: &str) -> String {
        format!(
            "handshake.acl.test.{label}.{}",
            uuid::Uuid::now_v7().simple()
        )
    }

    #[test]
    fn sid_specific_revoke_preserves_unrelated_appcontainer_ace() {
        let temp = tempfile::tempdir().expect("create ACL test directory");
        let target = temp.path().join("shared.txt");
        fs::write(&target, b"acl-test").expect("write ACL target");
        let first = ensure_appcontainer_profile(
            &test_profile_name("first"),
            "Handshake ACL test first",
            None,
        )
        .expect("create first profile");
        let second = ensure_appcontainer_profile(
            &test_profile_name("second"),
            "Handshake ACL test second",
            None,
        )
        .expect("create second profile");
        let grant =
            AclGrantTarget::file(target.clone(), rappct::acl::AccessMask::FILE_GENERIC_READ.0);
        grant_target_to_package(&grant, &first.sid).expect("grant first SID");
        grant_target_to_package(&grant, &second.sid).expect("grant second SID");

        let (_identity_lock, identity) = lock_target_identity(&grant).expect("lock grant identity");
        let mut grant = grant;
        grant.identity = Some(identity);
        revoke_package_sid(&grant, first.sid.as_string()).expect("revoke first SID");
        assert!(!acl_contains_sid(&target, first.sid.as_string()).expect("inspect first SID"));
        assert!(acl_contains_sid(&target, second.sid.as_string()).expect("inspect second SID"));

        revoke_package_sid(&grant, second.sid.as_string()).expect("revoke second SID");
        delete_appcontainer_profile(first).expect("delete first profile");
        delete_appcontainer_profile(second).expect("delete second profile");
    }

    #[test]
    fn pending_journal_recovery_revokes_acl_and_removes_journal() {
        let journal_root = tempfile::tempdir().expect("create journal root");
        let target_root = tempfile::tempdir().expect("create ACL target root");
        let target = target_root.path().join("recovery.txt");
        fs::write(&target, b"recovery-test").expect("write recovery target");
        let profile_name = test_profile_name("recovery");
        let mut transaction = AppContainerAclTransaction::begin_in_store(
            journal_root.path().to_path_buf(),
            profile_name.clone(),
            vec![AclGrantTarget::file(
                target.clone(),
                rappct::acl::AccessMask::FILE_GENERIC_READ.0,
            )],
        )
        .expect("prepare transaction");
        transaction
            .create_profile("Handshake ACL recovery test", None)
            .expect("create recovery profile");
        let sid = transaction
            .profile()
            .expect("profile")
            .sid
            .as_string()
            .to_string();
        transaction.grant_all().expect("apply recovery grant");
        assert!(acl_contains_sid(&target, &sid).expect("inspect granted SID"));
        drop(transaction.lock_file.take());
        std::mem::forget(transaction);

        recover_pending_acl_transactions(journal_root.path().to_path_buf())
            .expect("recover pending transaction");
        assert!(!acl_contains_sid(&target, &sid).expect("inspect recovered SID"));
        assert!(!journal_root
            .path()
            .join(format!("{profile_name}.json"))
            .exists());
    }

    #[test]
    fn live_transaction_handle_blocks_target_rename() {
        let journal_root = tempfile::tempdir().expect("create journal root");
        let target_root = tempfile::tempdir().expect("create ACL target root");
        let target = target_root.path().join("live-rename.txt");
        let renamed = target_root.path().join("renamed.txt");
        fs::write(&target, b"live-rename-test").expect("write ACL target");
        let profile_name = test_profile_name("live-rename");
        let mut transaction = AppContainerAclTransaction::begin_in_store(
            journal_root.path().to_path_buf(),
            profile_name,
            vec![AclGrantTarget::file(
                target.clone(),
                rappct::acl::AccessMask::FILE_GENERIC_READ.0,
            )],
        )
        .expect("prepare transaction");
        transaction
            .create_profile("Handshake ACL live rename test", None)
            .expect("create profile");
        transaction.grant_all().expect("apply grant");

        fs::rename(&target, &renamed)
            .expect_err("identity lock must prevent rename while the grant is live");
        transaction.cleanup().expect("cleanup transaction");
    }

    #[test]
    fn recovery_rejects_missing_renamed_target_and_retains_journal() {
        let journal_root = tempfile::tempdir().expect("create journal root");
        let target_root = tempfile::tempdir().expect("create ACL target root");
        let target = target_root.path().join("original.txt");
        let renamed = target_root.path().join("renamed.txt");
        fs::write(&target, b"rename-recovery-test").expect("write ACL target");
        let profile_name = test_profile_name("rename");
        let mut transaction = AppContainerAclTransaction::begin_in_store(
            journal_root.path().to_path_buf(),
            profile_name.clone(),
            vec![AclGrantTarget::file(
                target.clone(),
                rappct::acl::AccessMask::FILE_GENERIC_READ.0,
            )],
        )
        .expect("prepare transaction");
        transaction
            .create_profile("Handshake ACL rename recovery test", None)
            .expect("create profile");
        let sid = transaction
            .profile()
            .expect("profile")
            .sid
            .as_string()
            .to_string();
        transaction.grant_all().expect("apply grant");
        transaction.target_handles.clear();
        drop(transaction.lock_file.take());
        std::mem::forget(transaction);
        fs::rename(&target, &renamed).expect("simulate post-crash target rename");

        let error = recover_pending_acl_transactions(journal_root.path().to_path_buf())
            .expect_err("recovery must fail closed when the journaled path is missing");
        assert!(error.contains("open AppContainer ACL target"), "{error}");
        assert!(journal_root
            .path()
            .join(format!("{profile_name}.json"))
            .exists());
        assert!(acl_contains_sid(&renamed, &sid).expect("renamed object retains SID"));

        fs::rename(&renamed, &target).expect("restore original path for recovery");
        recover_pending_acl_transactions(journal_root.path().to_path_buf())
            .expect("recover after restoring original identity");
        assert!(!acl_contains_sid(&target, &sid).expect("SID revoked after identity restore"));
        assert!(!journal_root
            .path()
            .join(format!("{profile_name}.json"))
            .exists());
    }

    #[test]
    fn recovery_rejects_replacement_target_and_retains_journal() {
        let journal_root = tempfile::tempdir().expect("create journal root");
        let target_root = tempfile::tempdir().expect("create ACL target root");
        let target = target_root.path().join("original.txt");
        let granted_object = target_root.path().join("granted-object.txt");
        fs::write(&target, b"replacement-recovery-test").expect("write ACL target");
        let profile_name = test_profile_name("replace");
        let mut transaction = AppContainerAclTransaction::begin_in_store(
            journal_root.path().to_path_buf(),
            profile_name.clone(),
            vec![AclGrantTarget::file(
                target.clone(),
                rappct::acl::AccessMask::FILE_GENERIC_READ.0,
            )],
        )
        .expect("prepare transaction");
        transaction
            .create_profile("Handshake ACL replacement recovery test", None)
            .expect("create profile");
        let sid = transaction
            .profile()
            .expect("profile")
            .sid
            .as_string()
            .to_string();
        transaction.grant_all().expect("apply grant");
        transaction.target_handles.clear();
        drop(transaction.lock_file.take());
        std::mem::forget(transaction);
        fs::rename(&target, &granted_object).expect("move granted object after simulated crash");
        fs::write(&target, b"replacement").expect("place replacement at journaled path");

        let error = recover_pending_acl_transactions(journal_root.path().to_path_buf())
            .expect_err("recovery must reject a replacement object");
        assert!(error.contains("identity does not match"), "{error}");
        assert!(journal_root
            .path()
            .join(format!("{profile_name}.json"))
            .exists());
        assert!(acl_contains_sid(&granted_object, &sid).expect("granted object retains SID"));

        fs::remove_file(&target).expect("remove replacement");
        fs::rename(&granted_object, &target).expect("restore granted object identity");
        recover_pending_acl_transactions(journal_root.path().to_path_buf())
            .expect("recover after restoring granted object");
        assert!(!acl_contains_sid(&target, &sid).expect("SID revoked after identity restore"));
        assert!(!journal_root
            .path()
            .join(format!("{profile_name}.json"))
            .exists());
    }

    #[test]
    fn recovery_skips_journal_owned_by_live_transaction() {
        let journal_root = tempfile::tempdir().expect("create journal root");
        let target_root = tempfile::tempdir().expect("create ACL target root");
        let target = target_root.path().join("active.txt");
        fs::write(&target, b"active-test").expect("write active target");
        let profile_name = test_profile_name("active");
        let mut transaction = AppContainerAclTransaction::begin_in_store(
            journal_root.path().to_path_buf(),
            profile_name.clone(),
            vec![AclGrantTarget::file(
                target.clone(),
                rappct::acl::AccessMask::FILE_GENERIC_READ.0,
            )],
        )
        .expect("prepare active transaction");
        transaction
            .create_profile("Handshake ACL active test", None)
            .expect("create active profile");
        let sid = transaction
            .profile()
            .expect("profile")
            .sid
            .as_string()
            .to_string();
        transaction.grant_all().expect("apply active grant");

        recover_pending_acl_transactions(journal_root.path().to_path_buf())
            .expect("active transaction must be skipped");
        assert!(acl_contains_sid(&target, &sid).expect("inspect active SID"));
        assert!(journal_root
            .path()
            .join(format!("{profile_name}.json"))
            .exists());

        transaction.cleanup().expect("cleanup active transaction");
        assert!(!acl_contains_sid(&target, &sid).expect("inspect cleaned SID"));
    }

    #[test]
    fn partial_grant_failure_rolls_back_prior_grant() {
        let journal_root = tempfile::tempdir().expect("create journal root");
        let target_root = tempfile::tempdir().expect("create ACL target root");
        let valid_target = target_root.path().join("valid.txt");
        fs::write(&valid_target, b"rollback-test").expect("write rollback target");
        let missing_target = target_root.path().join("missing.txt");
        let profile_name = test_profile_name("rollback");
        let mut transaction = AppContainerAclTransaction::begin_in_store(
            journal_root.path().to_path_buf(),
            profile_name.clone(),
            vec![
                AclGrantTarget::file(
                    valid_target.clone(),
                    rappct::acl::AccessMask::FILE_GENERIC_READ.0,
                ),
                AclGrantTarget::file(missing_target, rappct::acl::AccessMask::FILE_GENERIC_READ.0),
            ],
        )
        .expect("prepare transaction");
        transaction
            .create_profile("Handshake ACL rollback test", None)
            .expect("create rollback profile");
        let sid = transaction
            .profile()
            .expect("profile")
            .sid
            .as_string()
            .to_string();
        transaction
            .grant_all()
            .expect_err("missing second target must fail grant transaction");
        assert!(!acl_contains_sid(&valid_target, &sid).expect("inspect rolled back SID"));
        assert!(!journal_root
            .path()
            .join(format!("{profile_name}.json"))
            .exists());
    }
}
