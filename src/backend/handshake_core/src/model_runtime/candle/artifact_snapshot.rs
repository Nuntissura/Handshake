#![cfg(feature = "candle-runtime-engine")]

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use memmap2::{Mmap, MmapOptions};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::tokenizer::tokenizer_json_path_for_artifact;
use crate::model_runtime::{
    LoadSpec, ModelArtifactComponentIntegrity, ModelArtifactIntegrityReceipt, ModelRuntimeError,
};

const CONFIG_COMPONENT_NAME: &str = "config.json";
const MAX_CANDLE_CONFIG_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CANDLE_TOKENIZER_BYTES: u64 = 256 * 1024 * 1024;

/// One immutable, path-independent snapshot of every behavior-bearing input to
/// a Candle load. The weights map is backed by an unnamed temporary file; no
/// downstream loader can reopen or observe replacement of the source path.
pub(super) struct CapturedCandleArtifact {
    pub(super) weights: Mmap,
    pub(super) config: Value,
    pub(super) tokenizer: Option<Arc<tokenizers::Tokenizer>>,
    pub(super) receipt: ModelArtifactIntegrityReceipt,
}

pub(super) fn capture_candle_artifact(
    spec: &LoadSpec,
) -> Result<CapturedCandleArtifact, ModelRuntimeError> {
    let weights = capture_weights(&spec.artifact_path)?;
    let weights_integrity = integrity(weights.as_ref());
    if !weights_integrity
        .sha256
        .eq_ignore_ascii_case(spec.sha256_expected.trim())
    {
        return Err(ModelRuntimeError::LoadError(format!(
            "candle artifact sha256 mismatch: expected {}, got {}",
            spec.sha256_expected, weights_integrity.sha256
        )));
    }

    let config_path = config_json_path_for_artifact(&spec.artifact_path);
    let config_bytes =
        read_required_regular_file(&config_path, "Candle config", MAX_CANDLE_CONFIG_BYTES)?;
    let config_integrity = integrity(&config_bytes);
    let config = serde_json::from_slice::<Value>(&config_bytes).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to parse captured Candle config {}: {error}",
            config_path.display()
        ))
    })?;

    let tokenizer_path = tokenizer_json_path_for_artifact(&spec.artifact_path);
    let (tokenizer, tokenizer_integrity) = match read_optional_regular_file(
        &tokenizer_path,
        "Candle tokenizer",
        MAX_CANDLE_TOKENIZER_BYTES,
    )? {
        Some(bytes) => {
            let component_integrity = integrity(&bytes);
            let tokenizer = tokenizers::Tokenizer::from_bytes(&bytes).map_err(|error| {
                ModelRuntimeError::LoadError(format!(
                    "failed to parse captured Candle tokenizer {}: {error}",
                    tokenizer_path.display()
                ))
            })?;
            (Some(Arc::new(tokenizer)), Some(component_integrity))
        }
        None => (None, None),
    };

    let receipt = ModelArtifactIntegrityReceipt::from_candle_components(
        weights_integrity,
        config_integrity,
        tokenizer_integrity,
    )?;

    Ok(CapturedCandleArtifact {
        weights,
        config,
        tokenizer,
        receipt,
    })
}

fn capture_weights(path: &Path) -> Result<Mmap, ModelRuntimeError> {
    let (mut source, source_length) = open_required_regular_file(path, "Candle artifact")?;
    if source_length == 0 {
        return Err(ModelRuntimeError::LoadError(format!(
            "Candle artifact is empty: {}",
            path.display()
        )));
    }
    let mut staging = tempfile::tempfile().map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to create private Candle artifact staging file: {error}"
        ))
    })?;
    let copied = {
        let mut bounded = (&mut source).take(source_length);
        io::copy(&mut bounded, &mut staging).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to capture Candle artifact {}: {error}",
                path.display()
            ))
        })?
    };
    if copied != source_length {
        return Err(ModelRuntimeError::LoadError(format!(
            "Candle artifact changed length during capture: expected {source_length} bytes, copied {copied}"
        )));
    }
    reject_trailing_growth(&mut source, path, "Candle artifact")?;
    staging.flush().map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to flush captured Candle artifact bytes: {error}"
        ))
    })?;
    staging.sync_all().map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to sync captured Candle artifact bytes: {error}"
        ))
    })?;

    // SAFETY: `staging` is an unnamed private file. No write occurs after this
    // map is created, and the sole writable handle is dropped before the map is
    // returned. The returned mapping is read-only and is the exact slice later
    // passed to Candle's `VarBuilder::from_slice_safetensors`.
    let mapped = unsafe { MmapOptions::new().map(&staging) }.map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to map captured Candle artifact bytes: {error}"
        ))
    })?;
    drop(staging);
    Ok(mapped)
}

fn read_required_regular_file(
    path: &Path,
    label: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, ModelRuntimeError> {
    let (file, source_length) = open_required_regular_file(path, label)?;
    read_opened_regular_file(file, source_length, path, label, max_bytes)
}

fn read_optional_regular_file(
    path: &Path,
    label: &str,
    max_bytes: u64,
) -> Result<Option<Vec<u8>>, ModelRuntimeError> {
    let Some((file, source_length)) = open_optional_regular_file(path, label)? else {
        return Ok(None);
    };
    read_opened_regular_file(file, source_length, path, label, max_bytes).map(Some)
}

fn read_opened_regular_file(
    mut file: File,
    source_length: u64,
    path: &Path,
    label: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, ModelRuntimeError> {
    if source_length > max_bytes {
        return Err(ModelRuntimeError::LoadError(format!(
            "{label} {} is {source_length} bytes, exceeding the {max_bytes}-byte capture limit",
            path.display()
        )));
    }
    let capacity = usize::try_from(source_length).map_err(|_| {
        ModelRuntimeError::LoadError(format!(
            "{label} {} length cannot be represented in memory",
            path.display()
        ))
    })?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to reserve {source_length} bytes for captured {label} {}: {error}",
            path.display()
        ))
    })?;
    bytes.resize(capacity, 0);
    let mut copied = 0_usize;
    while copied < capacity {
        let read = file.read(&mut bytes[copied..]).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to capture {label} {}: {error}",
                path.display()
            ))
        })?;
        if read == 0 {
            break;
        }
        copied += read;
    }
    if copied as u64 != source_length {
        return Err(ModelRuntimeError::LoadError(format!(
            "{label} changed length during capture: expected {source_length} bytes, copied {copied}"
        )));
    }
    reject_trailing_growth(&mut file, path, label)?;
    Ok(bytes)
}

fn reject_trailing_growth(
    file: &mut File,
    path: &Path,
    label: &str,
) -> Result<(), ModelRuntimeError> {
    let mut trailing = [0_u8; 1];
    if file.read(&mut trailing).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to verify {label} length after capture {}: {error}",
            path.display()
        ))
    })? != 0
    {
        return Err(ModelRuntimeError::LoadError(format!(
            "{label} grew during capture: {}",
            path.display()
        )));
    }
    Ok(())
}

fn open_required_regular_file(path: &Path, label: &str) -> Result<(File, u64), ModelRuntimeError> {
    let path_metadata = fs::metadata(path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to inspect {label} {}: {error}",
            path.display()
        ))
    })?;
    if !path_metadata.is_file() {
        return Err(non_regular_file(path, label));
    }
    let file = open_source_once(path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to open {label} {}: {error}",
            path.display()
        ))
    })?;
    let source_length = opened_regular_file_length(&file, path, label)?;
    Ok((file, source_length))
}

fn open_optional_regular_file(
    path: &Path,
    label: &str,
) -> Result<Option<(File, u64)>, ModelRuntimeError> {
    let path_metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(ModelRuntimeError::LoadError(format!(
                "failed to inspect {label} {}: {error}",
                path.display()
            )))
        }
    };
    if !path_metadata.is_file() {
        return Err(non_regular_file(path, label));
    }
    let file = match open_source_once(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(ModelRuntimeError::LoadError(format!(
                "failed to open {label} {}: {error}",
                path.display()
            )))
        }
    };
    let source_length = opened_regular_file_length(&file, path, label)?;
    Ok(Some((file, source_length)))
}

fn opened_regular_file_length(
    file: &File,
    path: &Path,
    label: &str,
) -> Result<u64, ModelRuntimeError> {
    let metadata = file.metadata().map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to inspect opened {label} {}: {error}",
            path.display()
        ))
    })?;
    if !metadata.is_file() {
        return Err(non_regular_file(path, label));
    }
    Ok(metadata.len())
}

fn non_regular_file(path: &Path, label: &str) -> ModelRuntimeError {
    ModelRuntimeError::LoadError(format!(
        "{label} must be a regular file: {}",
        path.display()
    ))
}

fn open_source_once(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    options.open(path)
}

fn integrity(bytes: &[u8]) -> ModelArtifactComponentIntegrity {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    ModelArtifactComponentIntegrity {
        sha256: hex::encode(hasher.finalize()),
        length_bytes: bytes.len() as u64,
    }
}

fn config_json_path_for_artifact(artifact_path: &Path) -> PathBuf {
    artifact_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(CONFIG_COMPONENT_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mt013_candle_sidecar_capture_rejects_oversize_and_length_changes() {
        let temp = tempfile::tempdir().expect("Candle snapshot tempdir");
        let oversize = temp.path().join("oversize-config.json");
        File::create(&oversize)
            .expect("create sparse oversize config")
            .set_len(MAX_CANDLE_CONFIG_BYTES + 1)
            .expect("size sparse oversize config");
        assert!(
            read_required_regular_file(&oversize, "Candle config", MAX_CANDLE_CONFIG_BYTES)
                .expect_err("oversize sidecar must fail before allocation")
                .to_string()
                .contains("capture limit")
        );

        let changing = temp.path().join("changing-tokenizer.json");
        fs::write(&changing, b"abcdef").expect("write changing sidecar fixture");
        let short = open_source_once(&changing).expect("open truncation fixture");
        assert!(read_opened_regular_file(
            short,
            7,
            &changing,
            "Candle tokenizer",
            MAX_CANDLE_TOKENIZER_BYTES
        )
        .expect_err("short capture must fail")
        .to_string()
        .contains("changed length"));

        let grown = open_source_once(&changing).expect("open growth fixture");
        assert!(read_opened_regular_file(
            grown,
            5,
            &changing,
            "Candle tokenizer",
            MAX_CANDLE_TOKENIZER_BYTES
        )
        .expect_err("growth after bounded capture must fail")
        .to_string()
        .contains("grew during capture"));
    }

    #[cfg(unix)]
    #[test]
    fn mt013_candle_source_open_is_nonblocking_for_fifo() {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};

        let temp = tempfile::tempdir().expect("Candle FIFO tempdir");
        let fifo = temp.path().join("swapped.safetensors");
        let fifo_c = CString::new(fifo.as_os_str().as_bytes()).expect("FIFO path has no NUL");
        // SAFETY: `fifo_c` is a live NUL-terminated path and `mkfifo` does not
        // retain its pointer.
        assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
        let opened = open_source_once(&fifo).expect("O_NONBLOCK FIFO open returns without writer");
        assert!(!opened.metadata().expect("FIFO metadata").is_file());
    }
}
