use handshake_core::kernel::{
    crdt_adr::{
        kernel002_crdt_adr, CrdtLibrary, CrdtSelection, KernelCrdtAuthorityBoundary,
        KernelCrdtStorageModel, RuntimeIntegrationBoundary,
    },
    fold_manifest::kernel002_fold_manifest,
};

#[test]
fn kernel002_crdt_adr_compares_required_options_and_selects_yjs_boundary() {
    let adr = kernel002_crdt_adr();
    let manifest = kernel002_fold_manifest();

    assert_eq!(adr.wp_id, manifest.wp_id);
    assert_eq!(adr.decision, CrdtSelection::YjsCompatibleUpdateLog);

    for library in [
        CrdtLibrary::Yjs,
        CrdtLibrary::Loro,
        CrdtLibrary::Automerge,
        CrdtLibrary::ExistingProductDependencies,
    ] {
        let option = adr
            .option_for(library)
            .expect("required ADR option missing");
        assert!(
            !option.fit_notes.is_empty(),
            "ADR option must preserve fit notes for {:?}",
            library
        );
        assert!(
            !option.risks.is_empty(),
            "ADR option must preserve risks for {:?}",
            library
        );
    }

    assert_eq!(
        adr.option_for(CrdtLibrary::Yjs)
            .expect("Yjs option")
            .decision,
        CrdtSelection::YjsCompatibleUpdateLog
    );
    assert_ne!(
        adr.option_for(CrdtLibrary::Loro)
            .expect("Loro option")
            .decision,
        adr.decision
    );
    assert_ne!(
        adr.option_for(CrdtLibrary::Automerge)
            .expect("Automerge option")
            .decision,
        adr.decision
    );
}

#[test]
fn kernel002_crdt_adr_defines_storage_sync_schema_and_validation_plan() {
    let adr = kernel002_crdt_adr();

    assert_eq!(
        adr.storage_model,
        KernelCrdtStorageModel::PostgresEventLedgerUpdateLog
    );
    assert_eq!(
        adr.runtime_boundary,
        RuntimeIntegrationBoundary::TypeScriptYjsRustValidatedBytes
    );
    assert_eq!(
        adr.authority_boundary,
        KernelCrdtAuthorityBoundary::CrdtIsPrePromotionEvidence
    );

    for required_term in [
        "document_id",
        "workspace_id",
        "actor_id",
        "crdt_site_id",
        "schema_id",
        "state_vector",
        "update_sha256",
        "EventLedger",
    ] {
        assert!(
            adr.storage_contract.contains(required_term),
            "storage contract must name {required_term}"
        );
    }

    assert!(
        adr.schema_compatibility
            .contains("Tiptap/ProseMirror schema"),
        "schema compatibility must cover editor schema drift"
    );
    assert!(
        adr.validation_plan
            .iter()
            .any(|step| step.contains("replay updates")),
        "validation plan must prove update replay"
    );
    assert!(
        adr.validation_plan
            .iter()
            .any(|step| step.contains("promotion gate")),
        "validation plan must keep CRDT merge from mutating authority directly"
    );
}

#[test]
fn kernel002_crdt_adr_records_current_research_basis() {
    let adr = kernel002_crdt_adr();

    for source in [
        "docs.yjs.dev/api/document-updates",
        "github.com/y-crdt/y-crdt",
        "docs.rs/loro",
        "automerge.org/automerge/automerge",
        "tiptap.dev/docs/collaboration",
        "app/package.json",
        "app/src/components/TiptapEditor.tsx",
    ] {
        assert!(
            adr.research_sources
                .iter()
                .any(|entry| entry.contains(source)),
            "research basis missing source: {source}"
        );
    }
}
