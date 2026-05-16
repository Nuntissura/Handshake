use std::collections::HashSet;

use super::fold_manifest::{FoldManifest, KERNEL002_WP_ID, LEGACY_CACHE_OFFLINE_BOUNDARY_STUB_ID};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LegacyResetTopic {
    LegacyLocalAuthority,
    MarkdownAuthority,
    MailboxChronology,
    UiLocalTruth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetDisposition {
    PostgresEventLedgerCrdtAuthority,
    ProjectionOrAdvisory,
    PromotionGatedAction,
}

impl ResetDisposition {
    pub fn is_allowed_kernel002_disposition(self) -> bool {
        matches!(
            self,
            ResetDisposition::PostgresEventLedgerCrdtAuthority
                | ResetDisposition::ProjectionOrAdvisory
                | ResetDisposition::PromotionGatedAction
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetReconciliation {
    pub source_stub_id: &'static str,
    pub topic: LegacyResetTopic,
    pub legacy_assumption: &'static str,
    pub reset_disposition: ResetDisposition,
    pub kernel_semantics: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetInvariantMatrix {
    pub wp_id: &'static str,
    pub reconciliations: &'static [ResetReconciliation],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResetInvariantError {
    WpMismatch {
        matrix_wp_id: &'static str,
        manifest_wp_id: &'static str,
    },
    MissingFoldSource {
        source_stub_id: &'static str,
    },
    ResetOverrideWithoutReconciliation {
        source_stub_id: &'static str,
    },
    MissingTopic {
        topic: LegacyResetTopic,
    },
}

impl ResetInvariantMatrix {
    pub fn entries_for_topic(&self, topic: LegacyResetTopic) -> Vec<&'static ResetReconciliation> {
        self.reconciliations
            .iter()
            .filter(|entry| entry.topic == topic)
            .collect()
    }

    pub fn verify_against_fold_manifest(
        &self,
        manifest: &FoldManifest,
    ) -> Result<(), Vec<ResetInvariantError>> {
        let mut errors = Vec::new();

        if self.wp_id != manifest.wp_id {
            errors.push(ResetInvariantError::WpMismatch {
                matrix_wp_id: self.wp_id,
                manifest_wp_id: manifest.wp_id,
            });
        }

        let folded_source_ids: HashSet<&str> = manifest
            .source_stubs
            .iter()
            .map(|source| source.stub_id)
            .collect();
        let reconciliation_source_ids: HashSet<&str> = self
            .reconciliations
            .iter()
            .map(|entry| entry.source_stub_id)
            .collect();

        for entry in self.reconciliations {
            if !folded_source_ids.contains(entry.source_stub_id) {
                errors.push(ResetInvariantError::MissingFoldSource {
                    source_stub_id: entry.source_stub_id,
                });
            }
        }

        for source in manifest
            .source_stubs
            .iter()
            .filter(|source| source.reset_override.is_some())
        {
            if !reconciliation_source_ids.contains(source.stub_id) {
                errors.push(ResetInvariantError::ResetOverrideWithoutReconciliation {
                    source_stub_id: source.stub_id,
                });
            }
        }

        for topic in [
            LegacyResetTopic::LegacyLocalAuthority,
            LegacyResetTopic::MarkdownAuthority,
            LegacyResetTopic::MailboxChronology,
            LegacyResetTopic::UiLocalTruth,
        ] {
            if !self
                .reconciliations
                .iter()
                .any(|entry| entry.topic == topic)
            {
                errors.push(ResetInvariantError::MissingTopic { topic });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

pub fn kernel002_reset_invariant_matrix() -> ResetInvariantMatrix {
    ResetInvariantMatrix {
        wp_id: KERNEL002_WP_ID,
        reconciliations: KERNEL002_RESET_RECONCILIATIONS,
    }
}

const KERNEL002_RESET_RECONCILIATIONS: &[ResetReconciliation] = &[
    ResetReconciliation {
        source_stub_id: LEGACY_CACHE_OFFLINE_BOUNDARY_STUB_ID,
        topic: LegacyResetTopic::LegacyLocalAuthority,
        legacy_assumption: "Legacy cache/offline boundary work can be mistaken for local runtime authority.",
        reset_disposition: ResetDisposition::PostgresEventLedgerCrdtAuthority,
        kernel_semantics: "Preserve offline/cache intent only; Kernel002 authority is Postgres/EventLedger/CRDT and promotion commits authority events through EventLedger.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-FEMS-Write-Time-Safeguards-v1",
        topic: LegacyResetTopic::LegacyLocalAuthority,
        legacy_assumption: "FEMS safeguard records and FTS-style search were scoped around legacy local storage.",
        reset_disposition: ResetDisposition::PostgresEventLedgerCrdtAuthority,
        kernel_semantics: "Preserve novelty, contradiction, dedup, and audit semantics; authoritative FEMS writes use Postgres/EventLedger/CRDT storage/search primitives.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Locus-Work-Tracking-System-Phase1-v1",
        topic: LegacyResetTopic::LegacyLocalAuthority,
        legacy_assumption: "Locus work tracking rows could remain authoritative in legacy local task-board state.",
        reset_disposition: ResetDisposition::PostgresEventLedgerCrdtAuthority,
        kernel_semantics: "Preserve work graph and task-board intent; authoritative Locus state is Postgres/EventLedger/CRDT and generated views remain projections.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Markdown-Mirror-Sync-Drift-Guard-v1",
        topic: LegacyResetTopic::MarkdownAuthority,
        legacy_assumption: "Markdown mirrors can be edited or read as the current source of truth.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "Markdown mirrors are generated projections; manual or model edits become MirrorAdvisoryBox input until a registered normalization action accepts them.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Software-Delivery-Governance-Overlay-Boundary-v1",
        topic: LegacyResetTopic::MarkdownAuthority,
        legacy_assumption: "Imported repo governance files can be treated as live runtime truth after the fold.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "Imported .GOV artifacts are frozen evidence/source overlays; product runtime truth comes from kernel records and generated projections.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Governance-Pack-v1",
        topic: LegacyResetTopic::MarkdownAuthority,
        legacy_assumption: "Governance-pack docs or manifests can become operator-facing authority by being present on disk.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "Pack text is import evidence or generated projection; authority lives in machine contracts and advisory edits require normalization.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Inbox-Role-Mailbox-Alignment-v1",
        topic: LegacyResetTopic::MailboxChronology,
        legacy_assumption: "Inbox labels and message order can imply current role-mailbox state.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "Inbox alignment is a projection over typed Role Mailbox state; labels and chronology are advisory evidence only.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Role-Mailbox-Message-Thread-Contract-v1",
        topic: LegacyResetTopic::MailboxChronology,
        legacy_assumption: "Mailbox thread order can define lifecycle, current responder, or permitted responses.",
        reset_disposition: ResetDisposition::PromotionGatedAction,
        kernel_semantics: "Lifecycle, delivery state, response eligibility, and action requests are typed records changed by catalog-backed promotion-gated actions.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1",
        topic: LegacyResetTopic::MailboxChronology,
        legacy_assumption: "Retry, verifier outcome, escalation, and completion can be inferred from message chronology.",
        reset_disposition: ResetDisposition::PromotionGatedAction,
        kernel_semantics: "Loop checkpoints, retry budget, completion, dead-letter, and escalation are compact replayable records changed through governed actions.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Role-Mailbox-Triage-Queue-Controls-v1",
        topic: LegacyResetTopic::MailboxChronology,
        legacy_assumption: "Queue urgency, snooze, expiry, and reroute state can be derived from mailbox order.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "Triage queue posture is field-backed record state; queue views are projections and mailbox chronology is advisory evidence.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1",
        topic: LegacyResetTopic::MailboxChronology,
        legacy_assumption: "Claimant, lease age, takeover legality, and responder eligibility can be inferred from latest mailbox messages.",
        reset_disposition: ResetDisposition::PromotionGatedAction,
        kernel_semantics: "Claim, lease, takeover, and responder eligibility are explicit action results with stable ids and replayable receipts.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1",
        topic: LegacyResetTopic::MailboxChronology,
        legacy_assumption: "Handoff or announce-back chronology can decide whether work is advisory or complete.",
        reset_disposition: ResetDisposition::PromotionGatedAction,
        kernel_semantics: "Handoff bundles, transcription targets, provenance, advisory state, and completion state are typed records advanced by registered actions.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1",
        topic: LegacyResetTopic::MailboxChronology,
        legacy_assumption: "Debug bundle exports can imply mailbox truth because they capture recent message order.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "Debug bundles are leak-safe evidence projections with stable provenance refs and never define mailbox authority.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Software-Delivery-Overlay-Coordination-Records-v1",
        topic: LegacyResetTopic::MailboxChronology,
        legacy_assumption: "Queued steering, follow-up, takeover, or actor eligibility can be reconstructed from mailbox chronology.",
        reset_disposition: ResetDisposition::PromotionGatedAction,
        kernel_semantics: "Coordination state is queryable by stable ids and mutated only through catalog-backed claim, lease, steering, and takeover actions.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Dev-Command-Center-MVP-v1",
        topic: LegacyResetTopic::UiLocalTruth,
        legacy_assumption: "DCC selected work, diffs, approvals, or action buttons can become local UI truth.",
        reset_disposition: ResetDisposition::PromotionGatedAction,
        kernel_semantics: "DCC may preview and trigger catalog actions, but authority changes only through write-box promotion and EventLedger outcomes.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1",
        topic: LegacyResetTopic::UiLocalTruth,
        legacy_assumption: "Rendered artifacts or mirrored text can be treated as canonical because the UI shows them.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "Structured viewer renders canonical fields before mirrors; rendered and raw drilldown views are projections over machine records.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Dev-Command-Center-Layout-Projection-Registry-v1",
        topic: LegacyResetTopic::UiLocalTruth,
        legacy_assumption: "Board, queue, list, roadmap, inbox, or execution views can define work state.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "DCC layouts are registered projections and action bindings; they do not mutate or define authority without catalog actions.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-DCC-Postgres-Control-Plane-Projections-v1",
        topic: LegacyResetTopic::UiLocalTruth,
        legacy_assumption: "DCC control-plane projection rows can be used as authority when stale or locally cached.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "DCC projection rows are rebuildable Postgres/EventLedger/CRDT views with freshness status; stale views cannot promote authority.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Session-Spawn-Tree-DCC-Visualization-v1",
        topic: LegacyResetTopic::UiLocalTruth,
        legacy_assumption: "Spawn hierarchy, child counts, or cancel posture shown in DCC can become local UI truth.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "Spawn tree visualization is a projection from runtime records with action-backed cascade cancel and announce-back badges.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Product-Screenshot-Visual-Validation-v1",
        topic: LegacyResetTopic::UiLocalTruth,
        legacy_assumption: "Screenshot evidence can be mistaken for current product or validation authority.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "Screenshots are evidence artifacts with metadata refs; validation verdicts and promotions remain typed kernel records.",
    },
    ResetReconciliation {
        source_stub_id: "WP-1-Visual-Debugging-Loop-v1",
        topic: LegacyResetTopic::UiLocalTruth,
        legacy_assumption: "Visual baseline comparisons can decide runtime state directly from UI observations.",
        reset_disposition: ResetDisposition::ProjectionOrAdvisory,
        kernel_semantics: "Visual debug outputs are evidence projections feeding validator steering; accepted authority still requires registered action outcomes.",
    },
];
