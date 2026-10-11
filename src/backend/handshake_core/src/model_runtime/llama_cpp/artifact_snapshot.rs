//! Exact-byte capture boundary for llama.cpp's path-only GGUF loader.
//!
//! The configured source is opened once and copied through that handle into a
//! private retained stage (a sealed memfd on Linux/Android, a deny-write/delete
//! named file on Windows). Every digest, GGUF, split-model, and tokenizer check
//! is then performed against that stage. The native wrapper receives only the
//! bound staged path. Other Unix targets fail closed until an equivalent sealed
//! fresh-offset descriptor path is proven.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::io::{Seek, SeekFrom};

use sha2::{Digest, Sha256};
#[cfg(windows)]
use tempfile::Builder;
use tempfile::TempDir;

use crate::model_runtime::{
    LlamaCppArtifactIntegrityReceipt, ModelArtifactComponentIntegrity, ModelRuntimeError,
    RuntimeArtifactIntegrityReceipt, RuntimeKind,
};

use super::{gguf_loader, tokenizer_cache::LlamaTokenizer};

#[derive(Debug)]
pub(super) struct CapturedGgufArtifact {
    stage: RetainedGgufStage,
    receipt: RuntimeArtifactIntegrityReceipt,
    tokenizer: LlamaTokenizer,
}

#[derive(Debug)]
struct RetainedGgufStage {
    // On Linux/Android this owns a sealed memfd. On Windows it is opened with
    // read-only sharing, so other handles cannot write, replace, or delete the
    // stage. Other Unix targets fail before constructing this value. The guard
    // is dropped before optional TempDir cleanup.
    immutability_guard: Option<File>,
    _directory: Option<TempDir>,
    original_path: Option<PathBuf>,
    load_path: PathBuf,
}

impl CapturedGgufArtifact {
    pub(super) fn staged_path(&self) -> &Path {
        &self.stage.load_path
    }

    pub(super) fn receipt(&self) -> &RuntimeArtifactIntegrityReceipt {
        &self.receipt
    }

    pub(super) fn tokenizer(&self) -> &LlamaTokenizer {
        &self.tokenizer
    }

    /// Re-read the retained stage after native construction. This catches any
    /// mutation between preflight and the point the runtime publishes the
    /// model, including changes by same-user processes on platforms where
    /// read-only file permissions are advisory.
    pub(super) fn post_verify(&self) -> Result<(), ModelRuntimeError> {
        let (component, tokenizer) = inspect_staged_gguf(&self.stage.load_path)?;
        let expected = match &self.receipt {
            RuntimeArtifactIntegrityReceipt::LlamaCpp(receipt) => &receipt.gguf,
            RuntimeArtifactIntegrityReceipt::Candle(_) => {
                return Err(ModelRuntimeError::LoadError(
                    "llama.cpp staged artifact carried a non-GGUF integrity receipt".to_string(),
                ))
            }
        };
        if &component != expected {
            return Err(ModelRuntimeError::LoadError(format!(
                "llama.cpp staged GGUF changed after capture: expected sha256 {} length {}, got sha256 {} length {}",
                expected.sha256,
                expected.length_bytes,
                component.sha256,
                component.length_bytes
            )));
        }
        if tokenizer != self.tokenizer {
            return Err(ModelRuntimeError::LoadError(
                "llama.cpp staged GGUF tokenizer metadata changed after capture".to_string(),
            ));
        }
        Ok(())
    }
}

impl Drop for RetainedGgufStage {
    fn drop(&mut self) {
        // Closing the deny-write guard first lets Windows cleanup clear the
        // read-only attribute and remove the private stage.
        drop(self.immutability_guard.take());
        #[cfg(windows)]
        if let Some(original_path) = self.original_path.as_ref() {
            if let Ok(metadata) = fs::metadata(original_path) {
                let mut permissions = metadata.permissions();
                permissions.set_readonly(false);
                let _ = fs::set_permissions(original_path, permissions);
            }
        }
    }
}

pub(super) fn capture_gguf_artifact(
    source_path: &Path,
    expected_sha256: [u8; 32],
) -> Result<CapturedGgufArtifact, ModelRuntimeError> {
    // A path-level preflight prevents a FIFO/device open from blocking. The
    // post-open metadata check below remains authoritative against replacement
    // between this check and the single source open.
    let path_metadata = fs::metadata(source_path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to inspect llama.cpp artifact {}: {error}",
            source_path.display()
        ))
    })?;
    if !path_metadata.is_file() {
        return Err(non_regular_source(source_path));
    }

    let mut source = open_source_once(source_path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to open llama.cpp artifact {}: {error}",
            source_path.display()
        ))
    })?;
    let source_metadata = source.metadata().map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to inspect opened llama.cpp artifact {}: {error}",
            source_path.display()
        ))
    })?;
    if !source_metadata.is_file() {
        return Err(non_regular_source(source_path));
    }
    let source_length = source_metadata.len();
    if source_length == 0 {
        return Err(ModelRuntimeError::LoadError(format!(
            "llama.cpp artifact is empty: {}",
            source_path.display()
        )));
    }

    let (mut staged, stage) = create_writable_stage()?;

    // Bound the copy to the opened file's initial length. Short reads prove
    // truncation; one trailing read proves the source did not grow underneath
    // the capture. No path reopen is used.
    let copied = {
        let mut bounded = (&mut source).take(source_length);
        std::io::copy(&mut bounded, &mut staged).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to stage llama.cpp artifact {}: {error}",
                source_path.display()
            ))
        })?
    };
    if copied != source_length {
        return Err(ModelRuntimeError::LoadError(format!(
            "llama.cpp artifact changed length during capture: expected {source_length} bytes, copied {copied}"
        )));
    }
    let mut trailing = [0_u8; 1];
    if source.read(&mut trailing).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to verify llama.cpp artifact length after capture: {error}"
        ))
    })? != 0
    {
        return Err(ModelRuntimeError::LoadError(
            "llama.cpp artifact grew during capture".to_string(),
        ));
    }
    staged.flush().map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to flush staged llama.cpp artifact: {error}"
        ))
    })?;
    staged.sync_all().map_err(|error| {
        ModelRuntimeError::LoadError(format!("failed to sync staged llama.cpp artifact: {error}"))
    })?;
    let stage = finalize_writable_stage(stage, staged)?;

    // Bind the staged bytes to the configured digest before parsing any
    // attacker-controlled metadata beyond the fixed header.
    let gguf = hash_and_measure(&stage.load_path)?;
    let receipt = RuntimeArtifactIntegrityReceipt::from(
        LlamaCppArtifactIntegrityReceipt::from_gguf_component(gguf)?,
    );
    receipt.validate_for_runtime_expected(RuntimeKind::LlamaCpp, expected_sha256)?;
    gguf_loader::validate_gguf_magic(&stage.load_path)?;
    let tokenizer = gguf_loader::parse_gguf_tokenizer_metadata(&stage.load_path)?;

    Ok(CapturedGgufArtifact {
        stage,
        receipt,
        tokenizer,
    })
}

fn inspect_staged_gguf(
    staged_path: &Path,
) -> Result<(ModelArtifactComponentIntegrity, LlamaTokenizer), ModelRuntimeError> {
    let component = hash_and_measure(staged_path)?;
    gguf_loader::validate_gguf_magic(staged_path)?;
    let tokenizer = gguf_loader::parse_gguf_tokenizer_metadata(staged_path)?;
    Ok((component, tokenizer))
}

fn hash_and_measure(path: &Path) -> Result<ModelArtifactComponentIntegrity, ModelRuntimeError> {
    let mut file = File::open(path).map_err(|error| {
        ModelRuntimeError::LoadError(format!("failed to open staged llama.cpp artifact: {error}"))
    })?;
    let mut hasher = Sha256::new();
    let mut length_bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to read staged llama.cpp artifact: {error}"
            ))
        })?;
        if read == 0 {
            break;
        }
        length_bytes = length_bytes
            .checked_add(read as u64)
            .ok_or_else(|| ModelRuntimeError::LoadError("GGUF length overflow".to_string()))?;
        hasher.update(&buffer[..read]);
    }
    Ok(ModelArtifactComponentIntegrity {
        sha256: hex::encode(hasher.finalize()),
        length_bytes,
    })
}

fn non_regular_source(path: &Path) -> ModelRuntimeError {
    ModelRuntimeError::LoadError(format!(
        "llama.cpp artifact must be a regular file: {}",
        path.display()
    ))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn create_writable_stage() -> Result<(File, RetainedGgufStage), ModelRuntimeError> {
    use std::{ffi::CString, os::fd::FromRawFd};

    let name = CString::new("handshake-llama-model.gguf").expect("static memfd name has no NUL");
    // SAFETY: `name` is a live NUL-terminated C string. memfd_create does not
    // retain the pointer and returns a newly owned descriptor on success.
    let raw_fd =
        unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    if raw_fd < 0 {
        return Err(ModelRuntimeError::LoadError(format!(
            "failed to create sealable llama.cpp memfd stage: {}",
            std::io::Error::last_os_error()
        )));
    }
    // SAFETY: memfd_create returned a fresh owned descriptor and this File is
    // its sole Rust owner.
    let file = unsafe { File::from_raw_fd(raw_fd) };
    Ok((
        file,
        RetainedGgufStage {
            immutability_guard: None,
            _directory: None,
            original_path: None,
            load_path: PathBuf::new(),
        },
    ))
}

#[cfg(windows)]
fn create_writable_stage() -> Result<(File, RetainedGgufStage), ModelRuntimeError> {
    let directory = Builder::new()
        .prefix("handshake-llama-gguf-")
        .tempdir()
        .map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to create private llama.cpp staging directory: {error}"
            ))
        })?;
    let staged_path = directory.path().join("model.gguf");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let staged = options.open(&staged_path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to create private llama.cpp staged artifact: {error}"
        ))
    })?;
    Ok((
        staged,
        RetainedGgufStage {
            immutability_guard: None,
            _directory: Some(directory),
            original_path: Some(staged_path.clone()),
            load_path: staged_path,
        },
    ))
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
fn create_writable_stage() -> Result<(File, RetainedGgufStage), ModelRuntimeError> {
    Err(ModelRuntimeError::LoadError(
        "llama.cpp exact-byte staging is unsupported on this Unix target; a sealed fresh-offset descriptor path is required"
            .to_string(),
    ))
}

#[cfg(not(any(unix, windows)))]
fn create_writable_stage() -> Result<(File, RetainedGgufStage), ModelRuntimeError> {
    Err(ModelRuntimeError::LoadError(
        "llama.cpp exact-byte staging is unsupported on this platform".to_string(),
    ))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn finalize_writable_stage(
    mut stage: RetainedGgufStage,
    mut staged: File,
) -> Result<RetainedGgufStage, ModelRuntimeError> {
    staged.seek(SeekFrom::Start(0)).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to rewind llama.cpp memfd stage before sealing: {error}"
        ))
    })?;
    seal_linux_memfd(&staged)?;
    stage.immutability_guard = Some(staged);
    bind_retained_load_path(&mut stage)?;
    Ok(stage)
}

#[cfg(windows)]
fn finalize_writable_stage(
    mut stage: RetainedGgufStage,
    staged: File,
) -> Result<RetainedGgufStage, ModelRuntimeError> {
    drop(staged);
    let original_path = stage.original_path.as_ref().ok_or_else(|| {
        ModelRuntimeError::LoadError("llama.cpp named stage path is missing".to_string())
    })?;
    set_staged_read_only(original_path)?;
    stage.immutability_guard = Some(open_immutability_guard(original_path)?);
    bind_retained_load_path(&mut stage)?;
    Ok(stage)
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
fn finalize_writable_stage(
    _stage: RetainedGgufStage,
    _staged: File,
) -> Result<RetainedGgufStage, ModelRuntimeError> {
    Err(ModelRuntimeError::LoadError(
        "llama.cpp exact-byte staging is unsupported on this Unix target".to_string(),
    ))
}

#[cfg(not(any(unix, windows)))]
fn finalize_writable_stage(
    _stage: RetainedGgufStage,
    _staged: File,
) -> Result<RetainedGgufStage, ModelRuntimeError> {
    Err(ModelRuntimeError::LoadError(
        "llama.cpp exact-byte staging is unsupported on this platform".to_string(),
    ))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn seal_linux_memfd(file: &File) -> Result<(), ModelRuntimeError> {
    use std::os::fd::AsRawFd;

    let required = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
    // SAFETY: fcntl operates on the live owned memfd and the command's third
    // argument is the documented integer seal bitmask.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, required) } < 0 {
        return Err(ModelRuntimeError::LoadError(format!(
            "failed to seal llama.cpp memfd stage: {}",
            std::io::Error::last_os_error()
        )));
    }
    // SAFETY: F_GET_SEALS takes no variadic payload and only reads seal state
    // from the live descriptor.
    let actual = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
    if actual != required {
        return Err(ModelRuntimeError::LoadError(format!(
            "llama.cpp memfd stage seal verification failed: required {required:#x}, got {actual:#x}"
        )));
    }
    Ok(())
}

fn open_source_once(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A path swapped from a regular file to a FIFO/device between the
        // preflight and open cannot block model boot. Post-open metadata remains
        // authoritative and rejects every non-regular descriptor.
        options.custom_flags(libc::O_NONBLOCK);
    }
    options.open(path)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn bind_retained_load_path(stage: &mut RetainedGgufStage) -> Result<(), ModelRuntimeError> {
    use std::os::{fd::AsRawFd, unix::fs::MetadataExt};

    let guard = stage.immutability_guard.as_ref().ok_or_else(|| {
        ModelRuntimeError::LoadError(
            "llama.cpp staged descriptor guard was not established".to_string(),
        )
    })?;
    let load_path = retained_descriptor_path(guard.as_raw_fd());
    let expected = guard.metadata().map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to inspect retained llama.cpp descriptor: {error}"
        ))
    })?;
    let alias = fs::metadata(&load_path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "retained llama.cpp descriptor path {} is unavailable: {error}",
            load_path.display()
        ))
    })?;
    if expected.dev() != alias.dev() || expected.ino() != alias.ino() {
        return Err(ModelRuntimeError::LoadError(
            "retained llama.cpp descriptor path did not resolve to the staged inode".to_string(),
        ));
    }

    let sealed_alias = fs::metadata(&load_path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "sealed llama.cpp descriptor became unavailable: {error}"
        ))
    })?;
    if expected.dev() != sealed_alias.dev() || expected.ino() != sealed_alias.ino() {
        return Err(ModelRuntimeError::LoadError(
            "sealed llama.cpp descriptor path changed inode".to_string(),
        ));
    }
    stage.load_path = load_path;
    Ok(())
}

#[cfg(all(unix, any(target_os = "linux", target_os = "android")))]
fn retained_descriptor_path(raw_fd: std::os::fd::RawFd) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{raw_fd}"))
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn bind_retained_load_path(_stage: &mut RetainedGgufStage) -> Result<(), ModelRuntimeError> {
    Ok(())
}

#[cfg(windows)]
fn set_staged_read_only(path: &Path) -> Result<(), ModelRuntimeError> {
    let mut permissions = fs::metadata(path)
        .map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to inspect staged llama.cpp permissions: {error}"
            ))
        })?
        .permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to make staged llama.cpp artifact read-only: {error}"
        ))
    })
}

#[cfg(windows)]
fn open_immutability_guard(path: &Path) -> Result<File, ModelRuntimeError> {
    use std::os::windows::fs::OpenOptionsExt;

    // FILE_SHARE_READ only: llama.cpp and post-verification may open readers,
    // while writers, replacement, and deletion remain denied.
    OpenOptions::new()
        .read(true)
        .share_mode(0x0000_0001)
        .open(path)
        .map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to lock staged llama.cpp artifact against mutation: {error}"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_digest(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }

    fn push_string(bytes: &mut Vec<u8>, value: &str) {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_u32_metadata(bytes: &mut Vec<u8>, key: &str, value: u32) {
        push_string(bytes, key);
        bytes.extend_from_slice(&4_u32.to_le_bytes()); // GGUF_TYPE_UINT32
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn minimal_tokenizer_gguf(tokens: &[&str], split: bool) -> Vec<u8> {
        let metadata_count = 3_u64 + u64::from(split);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"GGUF");
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&0_u64.to_le_bytes()); // tensor_count
        bytes.extend_from_slice(&metadata_count.to_le_bytes());

        push_string(&mut bytes, "tokenizer.ggml.tokens");
        bytes.extend_from_slice(&9_u32.to_le_bytes()); // GGUF_TYPE_ARRAY
        bytes.extend_from_slice(&8_u32.to_le_bytes()); // GGUF_TYPE_STRING
        bytes.extend_from_slice(&(tokens.len() as u64).to_le_bytes());
        for token in tokens {
            push_string(&mut bytes, token);
        }
        push_u32_metadata(&mut bytes, "tokenizer.ggml.bos_token_id", 0);
        push_u32_metadata(
            &mut bytes,
            "tokenizer.ggml.eos_token_id",
            u32::try_from(tokens.len().saturating_sub(1)).unwrap(),
        );
        if split {
            push_u32_metadata(&mut bytes, "split.count", 2);
        }
        bytes
    }

    #[cfg(any(target_os = "linux", target_os = "android", windows))]
    #[test]
    fn mt013_source_replacement_cannot_change_captured_bytes_or_tokenizer() {
        let temp = tempfile::tempdir().expect("source tempdir");
        let source = temp.path().join("configured.gguf");
        let original = minimal_tokenizer_gguf(&["<s>", "original", "</s>"], false);
        fs::write(&source, &original).expect("write original GGUF");

        let captured =
            capture_gguf_artifact(&source, expected_digest(&original)).expect("capture exact GGUF");
        let replacement = minimal_tokenizer_gguf(&["<s>", "replacement", "</s>"], false);
        fs::write(&source, replacement).expect("replace configured source");

        captured
            .post_verify()
            .expect("retained stage is independent of source replacement");
        assert_eq!(captured.tokenizer().vocab_size, 3);
        assert_eq!(
            captured.receipt().primary_artifact_sha256(),
            hex::encode(expected_digest(&original))
        );
        assert_eq!(
            fs::read(captured.staged_path()).expect("read retained stage"),
            original
        );
    }

    #[cfg(any(target_os = "linux", target_os = "android", windows))]
    #[test]
    fn mt013_capture_rejects_missing_non_regular_digest_magic_tokenizer_and_split() {
        let temp = tempfile::tempdir().expect("source tempdir");
        let missing = temp.path().join("missing.gguf");
        assert!(capture_gguf_artifact(&missing, [0; 32])
            .expect_err("missing source must fail")
            .to_string()
            .contains("failed to inspect"));
        assert!(capture_gguf_artifact(temp.path(), [0; 32])
            .expect_err("directory source must fail")
            .to_string()
            .contains("regular file"));

        let source = temp.path().join("configured.gguf");
        let valid = minimal_tokenizer_gguf(&["<s>", "token", "</s>"], false);
        fs::write(&source, &valid).expect("write valid GGUF");
        assert!(capture_gguf_artifact(&source, [0; 32])
            .expect_err("wrong digest must fail")
            .to_string()
            .contains("GGUF sha256 mismatch"));

        let bad_magic = [b'B', b'A', b'D', b'!'];
        fs::write(&source, bad_magic).expect("write bad magic");
        assert!(capture_gguf_artifact(&source, expected_digest(&bad_magic))
            .expect_err("bad magic must fail")
            .to_string()
            .contains("not a GGUF"));

        let mut no_tokenizer = Vec::new();
        no_tokenizer.extend_from_slice(b"GGUF");
        no_tokenizer.extend_from_slice(&3_u32.to_le_bytes());
        no_tokenizer.extend_from_slice(&0_u64.to_le_bytes());
        no_tokenizer.extend_from_slice(&0_u64.to_le_bytes());
        fs::write(&source, &no_tokenizer).expect("write tokenizer-free GGUF");
        assert!(
            capture_gguf_artifact(&source, expected_digest(&no_tokenizer))
                .expect_err("missing tokenizer metadata must fail")
                .to_string()
                .contains("tokenizer.ggml.tokens")
        );

        let split = minimal_tokenizer_gguf(&["<s>", "token", "</s>"], true);
        fs::write(&source, &split).expect("write split GGUF");
        assert!(capture_gguf_artifact(&source, expected_digest(&split))
            .expect_err("split GGUF must fail")
            .to_string()
            .contains("split GGUF artifacts are not supported"));
    }

    #[cfg(unix)]
    #[test]
    fn mt013_source_open_is_nonblocking_if_regular_path_is_swapped_to_fifo() {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};

        let temp = tempfile::tempdir().expect("source tempdir");
        let fifo = temp.path().join("swapped.gguf");
        let fifo_c = CString::new(fifo.as_os_str().as_bytes()).expect("FIFO path has no NUL");
        // SAFETY: `fifo_c` is a live NUL-terminated path and mode has no
        // pointer-bearing data; mkfifo does not retain the pointer.
        let result = unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) };
        assert_eq!(
            result,
            0,
            "create FIFO: {}",
            std::io::Error::last_os_error()
        );

        let opened = open_source_once(&fifo).expect("O_NONBLOCK FIFO open returns without writer");
        assert!(!opened.metadata().expect("FIFO metadata").is_file());
    }

    #[cfg(windows)]
    #[test]
    fn mt013_retained_stage_is_read_only_and_post_verify_detects_tamper() {
        let temp = tempfile::tempdir().expect("source tempdir");
        let source = temp.path().join("configured.gguf");
        let valid = minimal_tokenizer_gguf(&["<s>", "token", "</s>"], false);
        fs::write(&source, &valid).expect("write valid GGUF");
        let mut captured =
            capture_gguf_artifact(&source, expected_digest(&valid)).expect("capture exact GGUF");

        assert!(OpenOptions::new()
            .write(true)
            .open(captured.staged_path())
            .is_err());

        drop(captured.stage.immutability_guard.take());
        let mut permissions = fs::metadata(captured.staged_path())
            .expect("stage metadata")
            .permissions();
        permissions.set_readonly(false);
        fs::set_permissions(captured.staged_path(), permissions).expect("make stage writable");
        fs::write(captured.staged_path(), &valid[..valid.len() - 1]).expect("tamper stage");
        assert!(captured
            .post_verify()
            .expect_err("post-verification must detect stage tamper")
            .to_string()
            .contains("changed after capture"));
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn mt013_sealed_memfd_rejects_write_grow_shrink_and_has_no_swap_entry() {
        use std::os::fd::AsRawFd;

        let temp = tempfile::tempdir().expect("source tempdir");
        let source = temp.path().join("configured.gguf");
        let original = minimal_tokenizer_gguf(&["<s>", "original", "</s>"], false);
        let replacement = minimal_tokenizer_gguf(&["<s>", "replacement", "</s>"], false);
        fs::write(&source, &original).expect("write original GGUF");
        let captured =
            capture_gguf_artifact(&source, expected_digest(&original)).expect("capture exact GGUF");

        assert!(captured.stage.original_path.is_none());
        assert!(captured.staged_path().starts_with("/proc/self/fd"));
        let guard = captured
            .stage
            .immutability_guard
            .as_ref()
            .expect("sealed memfd guard");
        let required =
            libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
        // SAFETY: F_GET_SEALS only reads seal state from the live memfd.
        let actual = unsafe { libc::fcntl(guard.as_raw_fd(), libc::F_GET_SEALS) };
        assert_eq!(actual, required);

        let write_result = OpenOptions::new()
            .write(true)
            .open(captured.staged_path())
            .and_then(|mut writer| writer.write_all(&replacement));
        assert!(write_result.is_err(), "F_SEAL_WRITE rejects writes");
        assert!(guard
            .set_len((original.len() as u64).saturating_add(1))
            .is_err());
        assert!(guard
            .set_len((original.len() as u64).saturating_sub(1))
            .is_err());
        // SAFETY: the requested mapping length is the live memfd length and the
        // returned pointer is never dereferenced. A surprising success is
        // immediately unmapped before failing the assertion.
        let writable_mapping = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                original.len(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                guard.as_raw_fd(),
                0,
            )
        };
        if writable_mapping != libc::MAP_FAILED {
            // SAFETY: this is exactly the pointer/length returned by mmap.
            unsafe { libc::munmap(writable_mapping, original.len()) };
        }
        assert_eq!(writable_mapping, libc::MAP_FAILED);

        let swap_decoy = temp.path().join("model.gguf");
        fs::write(&swap_decoy, &replacement).expect("create swap decoy");
        assert_eq!(
            fs::read(captured.staged_path()).expect("read sealed native path during swap"),
            original,
            "native boundary remains bound to the sealed memfd"
        );
        fs::write(&swap_decoy, &original).expect("restore swap decoy");
        captured
            .post_verify()
            .expect("sealed memfd remains exact across swap-and-restore decoy");
    }
}
