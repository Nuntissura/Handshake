#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManualTopic {
    Purpose,
    Startup,
    ActionCatalog,
    WriteBoxes,
    DccPaths,
    CrdtWorkflow,
    SafetyConstraints,
    FailureModes,
    DenialRecovery,
    ValidationEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelModelManualSection {
    pub topic: ManualTopic,
    pub title: &'static str,
    pub instructions: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelModelManualV1 {
    pub schema_id: &'static str,
    pub manual_id: &'static str,
    pub no_prior_context_required: bool,
    pub startup_commands: &'static [&'static str],
    pub sections: &'static [KernelModelManualSection],
}

impl KernelModelManualV1 {
    pub fn section(&self, topic: ManualTopic) -> Option<&'static KernelModelManualSection> {
        self.sections.iter().find(|section| section.topic == topic)
    }
}

pub fn kernel002_no_context_model_manual() -> KernelModelManualV1 {
    KernelModelManualV1 {
        schema_id: "hsk.kernel_model_manual@1",
        manual_id: "kernel002-no-context-model-manual-v1",
        no_prior_context_required: true,
        startup_commands: &[
            "cmd /c kbstart",
            "just mt-board <wp-id>",
            "just mt-claim <wp-id> <session-key>",
            "cargo test -p handshake_core --test <focused_kernel_test>",
        ],
        sections: KERNEL002_MANUAL_SECTIONS,
    }
}

const KERNEL002_MANUAL_SECTIONS: &[KernelModelManualSection] = &[
    KernelModelManualSection {
        topic: ManualTopic::Purpose,
        title: "Purpose",
        instructions: &[
            "Handshake Kernel002 moves repo-governance behavior into product-owned kernel records.",
            "The kernel treats CRDT workspaces, write boxes, action catalog entries, receipts, and EventLedger mappings as mechanical product surfaces.",
            "Generated markdown, chat chronology, DCC renderings, and local UI state are projections or evidence, never authority by themselves.",
        ],
    },
    KernelModelManualSection {
        topic: ManualTopic::Startup,
        title: "Startup",
        instructions: &[
            "Run kbstart before acting in a kernel-governed session.",
            "Read the active packet and microtask contract before editing product code.",
            "Use the microtask board to claim exactly one MT at a time unless the packet explicitly permits grouping.",
            "Record receipts for intent, blockers, proof, and handoff using the packet's receipt helpers.",
        ],
    },
    KernelModelManualSection {
        topic: ManualTopic::ActionCatalog,
        title: "Action Catalog",
        instructions: &[
            "Start with kernel.action_catalog.view to discover lawful model-facing actions.",
            "Each action declares action_id, input schema, result schema, role eligibility, capability requirements, expected write boxes, authority effect, approval posture, validation hooks, and DCC preview metadata.",
            "Do not invent ad hoc write paths when the catalog lacks an action; deny or capture an advisory instead.",
        ],
    },
    KernelModelManualSection {
        topic: ManualTopic::WriteBoxes,
        title: "Write Boxes",
        instructions: &[
            "Use DraftBox, CRDTWorkspaceBox, ProposalBox, PatchBox, ArtifactBox, MirrorAdvisoryBox, MemoryBox, ExecutionBox, and PromotionBox as pre-promotion work containers.",
            "Every write box carries lifecycle state, owner, allowed transitions, authority effect, evidence refs, validation status, and projection rules.",
            "Only PromotionBox is allowed to describe an EventLedger authority write, and only after promotion-gate validation.",
        ],
    },
    KernelModelManualSection {
        topic: ManualTopic::DccPaths,
        title: "DCC Paths",
        instructions: &[
            "Use DCC panels as projections over kernel records: action catalog, write-box queue, CRDT workspace, mirror advisory queue, direct-edit denials, and promotion queue.",
            "DCC controls may request catalog-backed actions but must not mutate authority records directly.",
            "Freshness badges and validation state must be visible before promotion decisions.",
        ],
    },
    KernelModelManualSection {
        topic: ManualTopic::CrdtWorkflow,
        title: "CRDT Workflow",
        instructions: &[
            "Use Yjs-compatible CRDT updates as workspace evidence and store updates, state vectors, hashes, actor ids, and schema ids through kernel records.",
            "CRDT merge is not authority; it becomes authority only when a registered promotion action validates and appends EventLedger events.",
            "Schema drift, stale state vectors, unsupported Tiptap/ProseMirror nodes, or idempotency failures must deny promotion and preserve replay evidence.",
        ],
    },
    KernelModelManualSection {
        topic: ManualTopic::SafetyConstraints,
        title: "Safety Constraints",
        instructions: &[
            "Never directly edit authority artifacts, generated mirrors, task status, or runtime truth records to change kernel state.",
            "Never treat SQLite, markdown, mailbox chronology, or UI-local truth as Kernel002 authority.",
            "Keep product code and repo-governance overlays separate; imported .GOV artifacts are source evidence only.",
        ],
    },
    KernelModelManualSection {
        topic: ManualTopic::FailureModes,
        title: "Failure Modes",
        instructions: &[
            "Missing catalog action: stop or deny instead of inventing a direct mutation.",
            "Hash, schema, state-vector, or idempotency mismatch: retain evidence and mark validation failed.",
            "Projection drift: regenerate or capture a MirrorAdvisoryBox; do not edit the projection as authority.",
            "Proof command timeout: record a procedural memory and rerun the focused proof serially.",
        ],
    },
    KernelModelManualSection {
        topic: ManualTopic::DenialRecovery,
        title: "Denial Recovery",
        instructions: &[
            "Read KernelActionDenialV1 denial_code, reason, evidence_refs, and lawful_replacement_action_ids.",
            "Choose one lawful replacement action from lawful_replacement_action_ids, usually kernel.mirror_advisory.capture or kernel.crdt_workspace.propose_patch.",
            "Preserve denial evidence and correlation ids so another model can replay why the direct edit was blocked.",
        ],
    },
    KernelModelManualSection {
        topic: ManualTopic::ValidationEvidence,
        title: "Validation Evidence",
        instructions: &[
            "Attach focused test commands, formatter checks, diff checks, receipts, and known blockers to coder handoff evidence.",
            "Distinguish coder proof from Integration Validator verdicts; coder proof is not PASS/FAIL authority.",
            "Use receipt correlation ids, write_box_id, trace_id, state_vector, schema_id, and EventLedger refs so validation can reproduce decisions.",
        ],
    },
];
