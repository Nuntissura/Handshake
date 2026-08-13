//! Cross-process exclusive ownership for a declared worktree/checkout.
//!
//! The coordinator acquires these non-blocking OS file locks before a factory
//! can create any runtime resource. A guard moves from pending-spawn state into
//! the live session and is dropped only after teardown, ProcessOwnershipLedger
//! STOP, and the completed cleanup receipt all succeed.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use fs2::FileExt;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::error::{SwarmError, SwarmResult};
#[cfg(test)]
use super::ids::ModelInstanceId;
use super::ids::{CheckoutLeaseRef, SpawnRequest};

#[derive(Debug)]
pub(crate) struct CheckoutLeaseGuard {
    lease_ref: CheckoutLeaseRef,
    locked_files: Vec<File>,
}

impl CheckoutLeaseGuard {
    pub(crate) fn acquire(
        request: &SpawnRequest,
        owner_generation: u64,
    ) -> SwarmResult<Option<Self>> {
        let worktree_id = request
            .worktree_id()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let canonical_working_dir = request
            .working_dir()
            .map(canonical_checkout_root)
            .transpose()?;

        // A grouping-only worktree id has no host checkout to protect. Warm VM
        // requests are the exception: their remote worktree identity is itself
        // the exclusive substrate key even when no host cwd exists.
        if canonical_working_dir.is_none() && !request.wants_warm_vm_execution() {
            return Ok(None);
        }

        let lease_id = Uuid::now_v7();
        let lease_ref = CheckoutLeaseRef {
            lease_id,
            owner_generation,
            owner_instance_id: request.instance_id,
            worktree_id: worktree_id.clone(),
            canonical_working_dir: canonical_working_dir
                .as_ref()
                .map(|path| path.display().to_string()),
        };

        let mut keys = Vec::with_capacity(2);
        if let Some(worktree_id) = worktree_id {
            keys.push((
                "worktree_id",
                normalized_identity(&worktree_id),
                worktree_id,
            ));
        }
        if let Some(path) = canonical_working_dir {
            let display = path.display().to_string();
            keys.push(("canonical_path", normalized_identity(&display), display));
        }
        keys.sort_by(|left, right| (left.0, &left.1).cmp(&(right.0, &right.1)));

        let lock_root = std::env::temp_dir()
            .join("handshake")
            .join("worktree_checkout_leases");
        std::fs::create_dir_all(&lock_root).map_err(|error| {
            SwarmError::CheckoutLeaseFailed(format!(
                "cannot create checkout lease root {}: {error}",
                lock_root.display()
            ))
        })?;

        let mut locked_files = Vec::with_capacity(keys.len());
        for (key_kind, normalized, display) in keys {
            let lock_path = lock_path(&lock_root, key_kind, &normalized);
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&lock_path)
                .map_err(|error| {
                    SwarmError::CheckoutLeaseFailed(format!(
                        "cannot open {key_kind} checkout lease {}: {error}",
                        lock_path.display()
                    ))
                })?;
            if let Err(error) = file.try_lock_exclusive() {
                if error.kind() == fs2::lock_contended_error().kind() {
                    return Err(SwarmError::CheckoutLeaseConflict {
                        key_kind: key_kind.to_string(),
                        key: display,
                    });
                }
                return Err(SwarmError::CheckoutLeaseFailed(format!(
                    "cannot acquire {key_kind} checkout lease {}: {error}",
                    lock_path.display()
                )));
            }
            locked_files.push(file);
        }

        Ok(Some(Self {
            lease_ref,
            locked_files,
        }))
    }

    pub(crate) fn lease_ref(&self) -> &CheckoutLeaseRef {
        &self.lease_ref
    }
}

impl Drop for CheckoutLeaseGuard {
    fn drop(&mut self) {
        for file in self.locked_files.iter().rev() {
            let _ = FileExt::unlock(file);
        }
    }
}

pub(crate) fn canonical_checkout_root(raw: &str) -> SwarmResult<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SwarmError::CheckoutLeaseFailed(
            "working_dir is blank".to_string(),
        ));
    }
    let canonical = std::fs::canonicalize(trimmed).map_err(|error| {
        SwarmError::CheckoutLeaseFailed(format!(
            "cannot canonicalize checkout working_dir {trimmed}: {error}"
        ))
    })?;
    if !canonical.is_dir() {
        return Err(SwarmError::CheckoutLeaseFailed(format!(
            "checkout working_dir is not a directory: {}",
            canonical.display()
        )));
    }

    Ok(canonical
        .ancestors()
        .find(|candidate| candidate.join(".git").exists())
        .unwrap_or(canonical.as_path())
        .to_path_buf())
}

fn normalized_identity(value: &str) -> String {
    let normalized = value.trim().replace('\\', "/");
    if cfg!(windows) {
        normalized.to_ascii_lowercase()
    } else {
        normalized
    }
}

fn lock_path(root: &Path, key_kind: &str, normalized_identity: &str) -> PathBuf {
    let digest = Sha256::digest(normalized_identity.as_bytes());
    root.join(format!("{key_kind}-{}.lock", hex::encode(digest)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::{registry::RuntimeBinding, ModelId};

    fn request(instance: u32, worktree: &str, working_dir: &Path) -> SpawnRequest {
        SpawnRequest::new(
            ModelInstanceId::new(ModelId::new_v7(), instance),
            RuntimeBinding::Candle,
            "lease-test",
            "lease-parent",
        )
        .with_worktree(worktree)
        .with_working_dir(working_dir.display().to_string())
    }

    fn checkout(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("handshake-{name}-{}", Uuid::now_v7()));
        std::fs::create_dir_all(root.join(".git")).expect("create checkout marker");
        root
    }

    #[test]
    fn canonical_path_and_worktree_id_are_both_exclusive_until_guard_drop() {
        let root = checkout("checkout-lease-dual-key");
        let sibling = checkout("checkout-lease-sibling");
        let first_request = request(1, "wt-exclusive", &root);
        let same_path_other_id = request(2, "wt-spoof", &root);
        let same_id_other_path = request(3, "wt-exclusive", &sibling);

        let first = CheckoutLeaseGuard::acquire(&first_request, 1)
            .expect("first lease")
            .expect("guard");
        assert!(matches!(
            CheckoutLeaseGuard::acquire(&same_path_other_id, 2),
            Err(SwarmError::CheckoutLeaseConflict { key_kind, .. }) if key_kind == "canonical_path"
        ));
        assert!(matches!(
            CheckoutLeaseGuard::acquire(&same_id_other_path, 3),
            Err(SwarmError::CheckoutLeaseConflict { key_kind, .. }) if key_kind == "worktree_id"
        ));

        drop(first);
        let reacquired = CheckoutLeaseGuard::acquire(&same_path_other_id, 4)
            .expect("lease released")
            .expect("reacquired guard");
        assert_eq!(reacquired.lease_ref().owner_generation, 4);

        drop(reacquired);
        std::fs::remove_dir_all(root).expect("remove checkout");
        std::fs::remove_dir_all(sibling).expect("remove sibling");
    }
}
