//! MT-178 Deterministic repo export of Role Mailbox.
//!
//! Writes a `thread_index.json` summary plus per-thread JSON Lines files under
//! a configurable target directory. Default is
//! `.GOV/roles_shared/exports/role_mailbox/` resolved relative to the repo
//! root (per `crate::runtime_governance::RuntimeGovernancePaths`). The exporter
//! is idempotent (no writes when no new state) and append-only (existing lines
//! never rewritten).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

use super::message::RoleMailboxMessage;
use super::thread::RoleMailboxThread;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxExporterConfig {
    pub target_dir: PathBuf,
}

impl MailboxExporterConfig {
    pub fn default_under_repo_root(repo_root: &Path) -> Self {
        Self {
            target_dir: repo_root
                .join(".GOV")
                .join("roles_shared")
                .join("exports")
                .join("role_mailbox"),
        }
    }
}

#[derive(Debug, Error)]
pub enum ExporterError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportReport {
    pub threads_written: u32,
    pub lines_appended: u32,
    pub bytes_written: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ThreadIndexEntry {
    thread_id: String,
    title: String,
    lifecycle_state: String,
    linked_record_kind: String,
    linked_record_id: Option<String>,
    updated_at_utc: String,
    message_count: u32,
}

pub struct MailboxExporter {
    cfg: MailboxExporterConfig,
}

impl MailboxExporter {
    pub fn new(cfg: MailboxExporterConfig) -> Self {
        Self { cfg }
    }

    /// Synchronous export entrypoint. Caller supplies the snapshot of threads
    /// + messages (production wires this to `RoleMailboxRepository::list_threads_by_state`
    /// + `list_thread_messages` per thread; this keeps the exporter
    /// pool-independent for testability).
    pub fn export(
        &self,
        threads: &[RoleMailboxThread],
        messages_by_thread: &BTreeMap<uuid::Uuid, Vec<RoleMailboxMessage>>,
    ) -> Result<ExportReport, ExporterError> {
        fs::create_dir_all(&self.cfg.target_dir)?;
        let threads_dir = self.cfg.target_dir.join("threads");
        fs::create_dir_all(&threads_dir)?;
        let mut report = ExportReport {
            threads_written: 0,
            lines_appended: 0,
            bytes_written: 0,
        };

        // Build thread_index sorted by updated_at_utc DESC.
        let mut sorted = threads.to_vec();
        sorted.sort_by(|a, b| b.updated_at_utc.cmp(&a.updated_at_utc));
        let index_entries: Vec<ThreadIndexEntry> = sorted
            .iter()
            .filter(|t| t.archived_at_utc.is_none())
            .map(|t| {
                let id_uuid = t.thread_id.as_uuid();
                let messages = messages_by_thread.get(&id_uuid);
                ThreadIndexEntry {
                    thread_id: id_uuid.to_string(),
                    title: t.title.clone(),
                    lifecycle_state: t.lifecycle_state.as_str().to_string(),
                    linked_record_kind: serde_json::to_value(&t.linked_record_kind)
                        .map(|v| v.as_str().unwrap_or("").to_string())
                        .unwrap_or_default(),
                    linked_record_id: t.linked_record_id.clone(),
                    updated_at_utc: t
                        .updated_at_utc
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    message_count: messages.map(|m| m.len() as u32).unwrap_or(0),
                }
            })
            .collect();
        let index_json = serde_json::to_string_pretty(&index_entries)? + "\n";
        let index_path = self.cfg.target_dir.join("thread_index.json");
        if path_content_equals(&index_path, index_json.as_bytes()) {
            // Idempotency: no write needed.
        } else {
            fs::write(&index_path, index_json.as_bytes())?;
            report.bytes_written += index_json.as_bytes().len() as u64;
        }

        // Per-thread JSONL files with append-only watermark.
        for thread in &sorted {
            let id_uuid = thread.thread_id.as_uuid();
            let Some(messages) = messages_by_thread.get(&id_uuid) else {
                continue;
            };
            let path = threads_dir.join(format!("{}.jsonl", id_uuid));
            let watermark_path = threads_dir.join(format!("{}.watermark", id_uuid));
            let last_id = load_watermark(&watermark_path);
            // Filter messages to those newer than watermark (chronological order).
            let mut sorted_msgs = messages.clone();
            sorted_msgs.sort_by(|a, b| a.created_at_utc.cmp(&b.created_at_utc));
            let new_msgs: Vec<&RoleMailboxMessage> = match &last_id {
                Some(lid) => sorted_msgs
                    .iter()
                    .skip_while(|m| &m.message_id.as_uuid() != lid)
                    .skip(1)
                    .collect(),
                None => sorted_msgs.iter().collect(),
            };
            if new_msgs.is_empty() {
                continue;
            }
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            for msg in &new_msgs {
                let line = serde_json::to_string(msg)? + "\n";
                file.write_all(line.as_bytes())?;
                report.bytes_written += line.as_bytes().len() as u64;
                report.lines_appended += 1;
            }
            if let Some(last) = new_msgs.last() {
                fs::write(&watermark_path, last.message_id.as_uuid().to_string())?;
            }
            report.threads_written += 1;
        }
        Ok(report)
    }
}

fn load_watermark(path: &Path) -> Option<uuid::Uuid> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| uuid::Uuid::parse_str(s.trim()).ok())
}

fn path_content_equals(path: &Path, content: &[u8]) -> bool {
    fs::read(path)
        .map(|existing| existing == content)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role_mailbox::RoleId;
    use crate::role_mailbox_v1::lease::TakeoverPolicy;
    use crate::role_mailbox_v1::message::{MessageType, RoleMailboxMessage};
    use crate::role_mailbox_v1::router::ExecutorKind;
    use crate::role_mailbox_v1::thread::{ClaimMode, LinkedRecordKind, ResponseAuthorityScope};

    fn tmp_cfg() -> (tempfile::TempDir, MailboxExporterConfig) {
        let dir = tempfile::tempdir().unwrap();
        let cfg = MailboxExporterConfig {
            target_dir: dir.path().to_path_buf(),
        };
        (dir, cfg)
    }

    fn make_thread() -> RoleMailboxThread {
        RoleMailboxThread::open(
            "t",
            LinkedRecordKind::Wp,
            Some("WP-1".to_string()),
            vec![ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            TakeoverPolicy::Never,
            ResponseAuthorityScope::LeaseHolder,
        )
    }

    #[test]
    fn empty_repo_export_writes_empty_index() {
        let (_d, cfg) = tmp_cfg();
        let ex = MailboxExporter::new(cfg.clone());
        let report = ex.export(&[], &BTreeMap::new()).unwrap();
        assert_eq!(report.threads_written, 0);
        let idx = fs::read_to_string(cfg.target_dir.join("thread_index.json")).unwrap();
        assert!(idx.contains("[]"));
    }

    #[test]
    fn three_threads_two_messages_each_writes_six_lines() {
        let (_d, cfg) = tmp_cfg();
        let ex = MailboxExporter::new(cfg.clone());
        let mut threads = Vec::new();
        let mut messages = BTreeMap::new();
        for _ in 0..3 {
            let t = make_thread();
            let mut msgs = Vec::new();
            for _ in 0..2 {
                msgs.push(RoleMailboxMessage::new(
                    t.thread_id,
                    MessageType::DelegateWork,
                    RoleId::Orchestrator,
                    vec![RoleId::Coder],
                    serde_json::json!({"k": "v"}),
                ));
            }
            messages.insert(t.thread_id.as_uuid(), msgs);
            threads.push(t);
        }
        let report = ex.export(&threads, &messages).unwrap();
        assert_eq!(report.threads_written, 3);
        assert_eq!(report.lines_appended, 6);
    }

    #[test]
    fn second_run_with_no_new_state_is_idempotent() {
        let (_d, cfg) = tmp_cfg();
        let ex = MailboxExporter::new(cfg.clone());
        let t = make_thread();
        let msg = RoleMailboxMessage::new(
            t.thread_id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"k": "v"}),
        );
        let mut messages = BTreeMap::new();
        messages.insert(t.thread_id.as_uuid(), vec![msg]);
        let r1 = ex.export(&[t.clone()], &messages).unwrap();
        let r2 = ex.export(&[t], &messages).unwrap();
        assert_eq!(r1.lines_appended, 1);
        assert_eq!(r2.lines_appended, 0, "idempotent re-export must not append");
    }

    #[test]
    fn new_message_appended_only() {
        let (_d, cfg) = tmp_cfg();
        let ex = MailboxExporter::new(cfg.clone());
        let t = make_thread();
        let msg1 = RoleMailboxMessage::new(
            t.thread_id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"k": 1}),
        );
        let mut messages = BTreeMap::new();
        messages.insert(t.thread_id.as_uuid(), vec![msg1.clone()]);
        ex.export(&[t.clone()], &messages).unwrap();
        // Append a second message with a strictly later created_at_utc.
        let mut msg2 = RoleMailboxMessage::new(
            t.thread_id,
            MessageType::AnnounceBack,
            RoleId::Coder,
            vec![RoleId::Orchestrator],
            serde_json::json!({"k": 2}),
        );
        msg2.created_at_utc = msg1.created_at_utc + chrono::Duration::milliseconds(10);
        messages.insert(t.thread_id.as_uuid(), vec![msg1, msg2]);
        let r2 = ex.export(&[t], &messages).unwrap();
        assert_eq!(
            r2.lines_appended, 1,
            "only the new message should be appended"
        );
    }
}
