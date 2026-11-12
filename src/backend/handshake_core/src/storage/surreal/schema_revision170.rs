use super::*;

pub(super) const REVISION: i64 = 170;
const BASE_GENERATED: &str = "d8f17e8be49a849622ea730822b14404995b349b9c2d291fb960bd76d36fcb37";
const PRODUCTION_GENERATED: &str = "ab370c60727923a06c1de08f398a0095d49c318f0743a8a0060d26a0b016c36f";
const PRODUCTION_SOURCE: &str = "34270b1088be95c6ff867ccb9fa0cc44a891ccbd24d8863446712788e5703bd5";
const BASE_CATALOG: (&str, usize, usize, usize) =
    ("cd5bf4df67b91146ed9dbd3c47802184107419fbe8710bf66a610e6cc66dba2f", 311, 3987, 859);
const PRODUCTION_CATALOG: (&str, usize, usize, usize) =
    ("d3aaf84e49b7f19b8dffad1be10bef41e5859f3502ce047d89ac32988885e294", 339, 4570, 936);

pub(super) const RECEIPT_FUNCTIONS: [&str; 2] = [
    "wp012_first_owner_workspace_source",
    "wp012_first_owner_workspace_sources",
];

pub(super) fn receipt_pins(production: bool) -> (&'static str, &'static str, &'static str) {
    if production {
        (PRODUCTION_SOURCE, PRODUCTION_GENERATED, PRODUCTION_CATALOG.0)
    } else {
        (SCHEMA_LINEAGE_SHA256, BASE_GENERATED, BASE_CATALOG.0)
    }
}

pub(super) fn recognizes(state: &SchemaState, production: bool, config: &SurrealStorageConfig) -> bool {
    let (source, generated, info) = receipt_pins(production);
    state.version == SCHEMA_VERSION
        && state.revision == REVISION
        && state.target_revision == REVISION
        && state.namespace == config.namespace()
        && state.database == config.database()
        && state.source_manifest_sha256 == source
        && state.generated_surql_sha256 == generated
        && match state.apply_state.as_str() {
            "complete" => state.info_fingerprint_sha256 == info,
            "schema_applied" => state.info_fingerprint_sha256 == PENDING_SCHEMA_INFO_SHA256,
            _ => false,
        }
}

pub(super) fn matches_observed_catalog(observed: &ObservedSchema, production: bool) -> bool {
    let (info, tables, fields, indexes) = if production { PRODUCTION_CATALOG } else { BASE_CATALOG };
    observed.info_fingerprint_sha256 == info
        && observed.tables_defined == tables
        && observed.fields_defined == fields
        && observed.indexes_defined == indexes
}

pub(super) fn restore_shared_schema(mut source: String) -> String {
    for name in RECEIPT_FUNCTIONS {
        let current = schema_function_definition(name);
        assert!(source.matches(current).count() <= 1, "revision 170 receipt function duplicated");
        source = source.replacen(current, &current.replace("$receipt_session", "$session"), 1);
    }
    source
}
