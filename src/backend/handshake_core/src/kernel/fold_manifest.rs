use std::collections::HashMap;

pub const KERNEL002_WP_ID: &str = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";

const SOURCE_IMPORT_FULL: &str =
    "Import full source identity, intent, scope, out-of-scope, acceptance, dependencies, risks, UI notes, research notes, and activation cautions into Kernel002 unless superseded by stricter reset invariants.";
const SOURCE_IMPORT_POSTGRES_RESET: &str =
    "Import full source identity, intent, scope, out-of-scope, acceptance, dependencies, risks, UI notes, research notes, and activation cautions into Kernel002; preserve work intent while overriding storage authority to Postgres/EventLedger/CRDT and forbidding SQLite authority.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldClassification {
    Direct,
    Transitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldedSourceStub {
    pub stub_id: &'static str,
    pub source_path: &'static str,
    pub pre_fold_sha256: &'static str,
    pub fold_classification: FoldClassification,
    pub reset_override: Option<&'static str>,
    pub source_scope_import: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldManifest {
    pub wp_id: &'static str,
    pub import_rule: &'static str,
    pub source_stubs: &'static [FoldedSourceStub],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoldManifestError<'a> {
    MissingSource {
        source_path: &'static str,
    },
    HashMismatch {
        source_path: &'static str,
        expected_sha256: &'static str,
        observed_sha256: &'a str,
    },
}

impl FoldManifest {
    pub fn verify_observed_sources<'a>(
        &self,
        observed_sources: &'a [(&'a str, &'a str)],
    ) -> Result<(), Vec<FoldManifestError<'a>>> {
        let observed_by_path: HashMap<&str, &str> = observed_sources.iter().copied().collect();
        let mut errors = Vec::new();

        for source in self.source_stubs {
            match observed_by_path.get(source.source_path).copied() {
                None => errors.push(FoldManifestError::MissingSource {
                    source_path: source.source_path,
                }),
                Some(observed_sha256) if observed_sha256 != source.pre_fold_sha256 => {
                    errors.push(FoldManifestError::HashMismatch {
                        source_path: source.source_path,
                        expected_sha256: source.pre_fold_sha256,
                        observed_sha256,
                    });
                }
                Some(_) => {}
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

pub fn kernel002_fold_manifest() -> FoldManifest {
    FoldManifest {
        wp_id: KERNEL002_WP_ID,
        import_rule: SOURCE_IMPORT_FULL,
        source_stubs: KERNEL002_SOURCE_STUBS,
    }
}

const KERNEL002_SOURCE_STUBS: &[FoldedSourceStub] = &[
    FoldedSourceStub {
        stub_id: "WP-1-Postgres-Control-Plane-Shift-Bundle-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Postgres-Control-Plane-Shift-Bundle-v1.md",
        pre_fold_sha256: "f160424f7dd05647fec455d6eee7acbd0f1774d58d4b948963d4af9c58cce5a7",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Postgres-Dev-Test-Container-Matrix-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Postgres-Dev-Test-Container-Matrix-v1.md",
        pre_fold_sha256: "fa7d06125f95851335964035492d67096e8e3051ebd80bab9557fca7eccbfb5e",
        fold_classification: FoldClassification::Transitive,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Postgres-Control-Plane-Leases-Backpressure-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Postgres-Control-Plane-Leases-Backpressure-v1.md",
        pre_fold_sha256: "a7483370b3e309abcbee0b1e5c18615410c2ada92db22c57f1246f0ede802f93",
        fold_classification: FoldClassification::Transitive,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-ModelSession-Postgres-Queue-Workers-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-ModelSession-Postgres-Queue-Workers-v1.md",
        pre_fold_sha256: "42b20ed2dd8c520a019032edb51973ab49b57c67c90212bc1efd47b79c6ff745",
        fold_classification: FoldClassification::Transitive,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-FEMS-Postgres-Memory-Store-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-FEMS-Postgres-Memory-Store-v1.md",
        pre_fold_sha256: "f3d77fc67144ccfd3e6aeee01a683318f1cfdf69f07ed3c4dccc9de8e5b21a29",
        fold_classification: FoldClassification::Transitive,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Workflow-Engine-Postgres-Durable-Execution-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Workflow-Engine-Postgres-Durable-Execution-v1.md",
        pre_fold_sha256: "2abaaf481d34acade35ea172b26db947d1a9525f77dde0cd42493832af01f84c",
        fold_classification: FoldClassification::Transitive,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-DCC-Postgres-Control-Plane-Projections-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-DCC-Postgres-Control-Plane-Projections-v1.md",
        pre_fold_sha256: "bcc5adb8d8cb0cdabf20bdb50fa49ea1d3ae40ca56cff7ecf22e4c53e980d018",
        fold_classification: FoldClassification::Transitive,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-SQLite-Cache-Offline-Boundaries-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-SQLite-Cache-Offline-Boundaries-v1.md",
        pre_fold_sha256: "b0377704e89922c158720fe9bf32237f8358ef6847e4339726cbaeacb8ccccb1",
        fold_classification: FoldClassification::Transitive,
        reset_override: Some("reset_invariant"),
        source_scope_import: SOURCE_IMPORT_POSTGRES_RESET,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Software-Delivery-Runtime-Truth-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Software-Delivery-Runtime-Truth-v1.md",
        pre_fold_sha256: "85906f631d2d45d6ac946d63cf627fe32be5081b3f085b79379a5258c57e3e78",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Workflow-Transition-Automation-Registry-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Workflow-Transition-Automation-Registry-v1.md",
        pre_fold_sha256: "31e8380e93c8dff47d2ac1b8d93a89984ac9db32e6d9d08ba8112f15caa65b59",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Dev-Command-Center-MVP-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-MVP-v1.md",
        pre_fold_sha256: "c08b0372b562475a0f45dd9374c1b21de90cb8dc0c0adc1053bf4821364e78b6",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1.md",
        pre_fold_sha256: "48d658515ebc042061fff0b2b72eee868e66af5b836a61d6bac424c5530953c4",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Dev-Command-Center-Layout-Projection-Registry-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-Layout-Projection-Registry-v1.md",
        pre_fold_sha256: "35a4dad6c2a6463b0ba2bf28309a9f410c2eef8e7c45c92463bb76c60b464c71",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1.md",
        pre_fold_sha256: "3d334db27543a1209895d7865644ad7cd7ea78aba022e3ab6b76f35b3cd08c75",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-FEMS-Write-Time-Safeguards-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-FEMS-Write-Time-Safeguards-v1.md",
        pre_fold_sha256: "478b61d701afed3d04e4ac4224bc6299c998f74271adb4fe170ac9be577a4701",
        fold_classification: FoldClassification::Direct,
        reset_override: Some("storage_override"),
        source_scope_import: SOURCE_IMPORT_POSTGRES_RESET,
    },
    FoldedSourceStub {
        stub_id: "WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1.md",
        pre_fold_sha256: "81d08167ecd52d00b4057a3d898392cc890596c94eced1c99844f7c66b2c7776",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-FEMS-MT-Handoff-Memory-Context-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-FEMS-MT-Handoff-Memory-Context-v1.md",
        pre_fold_sha256: "2fcf8539ce8fdeafe983c4585f04c8001e00896849d7dc90421c5a4edb2dfed3",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Session-Spawn-Tree-DCC-Visualization-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Session-Spawn-Tree-DCC-Visualization-v1.md",
        pre_fold_sha256: "4ee8fcb66087a2a03bb37e2699ff3f5aa29ec175839e16786cea9501ab2517f0",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Session-Spawn-Conversation-Distillation-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Session-Spawn-Conversation-Distillation-v1.md",
        pre_fold_sha256: "e5cd2f010cd1ed02307792079a556526c37af673c8f857d47ffa176cf89f36f0",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Visual-Debugging-Loop-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Visual-Debugging-Loop-v1.md",
        pre_fold_sha256: "e7c521856929e4d96a23046d1f3d10ee2793010d61785cf7e69ce4db8ecf542a",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Product-Screenshot-Visual-Validation-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Product-Screenshot-Visual-Validation-v1.md",
        pre_fold_sha256: "aa410c75ca5c0a85bbae90b3451d388579a1d033a5aee260bd2c7c5be5d94bcf",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Markdown-Mirror-Sync-Drift-Guard-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Markdown-Mirror-Sync-Drift-Guard-v1.md",
        pre_fold_sha256: "aa5ad6b368a1777ef33d4b41e78ac1bc0f5a3a1df9cadaf03863722be98ffa0b",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Software-Delivery-Governance-Overlay-Boundary-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Software-Delivery-Governance-Overlay-Boundary-v1.md",
        pre_fold_sha256: "949cb0143adb074ec46d5c84bfe200987103e23c442a4b5e7530a5e07e06ed09",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Software-Delivery-Overlay-Coordination-Records-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Software-Delivery-Overlay-Coordination-Records-v1.md",
        pre_fold_sha256: "34cb80503244efcc8d71f552feec11a806ffea9d30acf39cfd508c4b56a88527",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1.md",
        pre_fold_sha256: "4c9c9de3ebea742c40fd3072cd26d219de0a86faf9bcaf0ad6009df18b54df58",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Role-Turn-Isolation-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Role-Turn-Isolation-v1.md",
        pre_fold_sha256: "0d390abb2fb0460fcc5debb01406c613b926ca0e591be9b02a14ac5e9105fcd4",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-LocalFirst-Agentic-MCP-Posture-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-LocalFirst-Agentic-MCP-Posture-v1.md",
        pre_fold_sha256: "eaaf9ea2f49e7baab0deec309750fbd8d9c48a155ee06774ae19d4d8de37a7cf",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Git-Engine-Decision-Gate-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Git-Engine-Decision-Gate-v1.md",
        pre_fold_sha256: "edfb98b8ca3bcc137ecfcc2766396ada0776d971a3d080616894b91005d4e7b8",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Session-Anti-Pattern-Registry-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Session-Anti-Pattern-Registry-v1.md",
        pre_fold_sha256: "207ae7c5a6f3080155c1ad1a7f6ab3a484f2618d202c37c2ef9fb616020aaa36",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Work-Profiles-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Work-Profiles-v1.md",
        pre_fold_sha256: "4ac98b03e6af6d5ba41980575b98391f5c6f5e70b846a3dce84b283711a7b36f",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Governance-Pack-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Governance-Pack-v1.md",
        pre_fold_sha256: "f443c20ffc9895c12a9896c385e9fcfb6581f1aa179f459402d45acaef417d3e",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Locus-Work-Tracking-System-Phase1-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Locus-Work-Tracking-System-Phase1-v1.md",
        pre_fold_sha256: "6a4fa49233f7d36102c4b585bdbaa67703e12f14aef1a4dc1841fc49ff11900d",
        fold_classification: FoldClassification::Direct,
        reset_override: Some("storage_override"),
        source_scope_import: SOURCE_IMPORT_POSTGRES_RESET,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Inbox-Role-Mailbox-Alignment-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Inbox-Role-Mailbox-Alignment-v1.md",
        pre_fold_sha256: "4af0d85eb3693f4f33a4de1917422831d342dce89c3860fe53e8520a41578031",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1.md",
        pre_fold_sha256: "95e912f9df5267e96150b7b8045b9b3f403f16ad165f10671ce10332a80ec6a1",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Role-Mailbox-Message-Thread-Contract-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Message-Thread-Contract-v1.md",
        pre_fold_sha256: "bf325c9033b1daa42d68d433e1c56ef39efb05e36e9eef9cbd3ea6f593d9be00",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1.md",
        pre_fold_sha256: "d3bb810783b291489c0ac14a2ddd2448fe410b2cfe6e324b929d6585ee5a72c0",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Role-Mailbox-Triage-Queue-Controls-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Triage-Queue-Controls-v1.md",
        pre_fold_sha256: "3206b9b86e84af4e8bcc431bc342adb9b4f40f3818db5c03d71bad4a4eca8f9f",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Executor-Routing-Claim-Lease-v1.md",
        pre_fold_sha256: "87f289e590265545d732edca5e4faf41f10697ababbabbe990bac647e820fb98",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
    FoldedSourceStub {
        stub_id: "WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1",
        source_path: ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1.md",
        pre_fold_sha256: "7bc0a81a45d5ddd5d60bca63fc7e56f959bf641b2de8a7b10abfac8ce6f9557f",
        fold_classification: FoldClassification::Direct,
        reset_override: None,
        source_scope_import: SOURCE_IMPORT_FULL,
    },
];
