use std::collections::HashMap;
use std::path::PathBuf;

/// Tracks active session-level workspace allocations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionWorktreeAllocation {
    pub session_id: String,
    pub worktree_path: PathBuf,
}

impl SessionWorktreeAllocation {
    pub fn new(session_id: impl Into<String>, worktree_path: impl Into<PathBuf>) -> Self {
        Self {
            session_id: session_id.into(),
            worktree_path: worktree_path.into(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SessionWorktreeRegistry {
    allocations: HashMap<String, PathBuf>,
}

impl SessionWorktreeRegistry {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
        }
    }

    pub fn put(&mut self, allocation: SessionWorktreeAllocation) {
        self.allocations
            .insert(allocation.session_id, allocation.worktree_path);
    }

    pub fn get(&self, session_id: &str) -> Option<&PathBuf> {
        self.allocations.get(session_id)
    }

    pub fn remove(&mut self, session_id: &str) -> Option<SessionWorktreeAllocation> {
        self.allocations
            .remove(session_id)
            .map(|worktree_path| SessionWorktreeAllocation::new(session_id, worktree_path))
    }

    pub fn len(&self) -> usize {
        self.allocations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.allocations.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionWorktreeAllocation, SessionWorktreeRegistry};

    #[test]
    fn registry_put_get_remove_roundtrip() {
        let mut registry = SessionWorktreeRegistry::new();
        let session_id = "session-1".to_string();

        registry.put(SessionWorktreeAllocation::new(
            &session_id,
            "tmp/session-1-worktree",
        ));

        assert_eq!(
            registry.get(&session_id).unwrap().to_string_lossy().as_ref(),
            "tmp/session-1-worktree"
        );
        assert_eq!(registry.len(), 1);

        let removed = registry
            .remove(&session_id)
            .expect("session allocation should be present");
        assert_eq!(removed.session_id, session_id);
        assert_eq!(
            removed.worktree_path.to_string_lossy().as_ref(),
            "tmp/session-1-worktree"
        );
        assert!(registry.get(&removed.session_id).is_none());
        assert_eq!(registry.len(), 0);
    }
}
