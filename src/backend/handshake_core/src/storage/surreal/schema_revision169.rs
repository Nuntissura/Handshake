use super::*;

pub(super) const REVISION: i64 = 169;
const BASE_GENERATED: &str = "b1bc55b01549fbeb17a5345c95d271ffd0453d39eaf2fffb0cdaaaa23ca88203";
const PRODUCTION_GENERATED: &str = "c460f1cfbcd8ba376c83e05777951b755ef79782136909a49062a1083d82c20e";
const PRODUCTION_SOURCE: &str = "b2d1649ec6da14dde3ee74bce20d7920204823c412a92e55fb97a512632e9e87";
const BASE_CATALOG: (&str, usize, usize, usize) =
    ("8aad971fedcdb5605ac1ebe64667a8eef24c1b3be79498af8c3ef38f4a4cce3f", 311, 3987, 859);
const PRODUCTION_CATALOG: (&str, usize, usize, usize) =
    ("1bd1c82774112973553bb290d0813fbd3c64c7176663ff5f525fcf0f971524cf", 339, 4570, 936);

const FIELD_HEADERS: &[&str] = &[
    "DEFINE FIELD OVERWRITE password_verifier ON TABLE local_accounts ",
    "DEFINE FIELD OVERWRITE credential_revision ON TABLE local_accounts ",
];
pub(super) const PREVIOUS_PASSWORD_VERIFIER: &str = r#"DEFINE FIELD OVERWRITE password_verifier ON TABLE local_accounts TYPE option<string> PERMISSIONS FOR select WHERE fn::wp012_member_redemption_password_read($this) FOR create NONE FOR update WHERE fn::wp012_member_redemption_identity_update('account', $before, $after);"#;
const PREVIOUS_CREDENTIAL_REVISION: &str = r#"DEFINE FIELD OVERWRITE credential_revision ON TABLE local_accounts TYPE int DEFAULT 0 ASSERT $value >= 0
    PERMISSIONS FOR select WHERE (fn::mt109_live_session() AND id = $auth.account_id) OR fn::wp012_member_recipient_revision_read($this)
    FOR create WHERE $value = 0 FOR update WHERE fn::wp012_member_redemption_identity_update('account', $before, $after);"#;

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
        && state.apply_state == "complete"
        && state.source_manifest_sha256 == source
        && state.generated_surql_sha256 == generated
        && state.info_fingerprint_sha256 == info
}

pub(super) fn matches_observed_catalog(observed: &ObservedSchema, production: bool) -> bool {
    let (info, tables, fields, indexes) = if production { PRODUCTION_CATALOG } else { BASE_CATALOG };
    observed.info_fingerprint_sha256 == info
        && observed.tables_defined == tables
        && observed.fields_defined == fields
        && observed.indexes_defined == indexes
}

pub(super) fn field_statements() -> impl Iterator<Item = &'static str> {
    canonical_upgrade_spans(SCHEMA, FIELD_HEADERS)
        .into_iter()
        .map(|(start, end)| &SCHEMA[start..end])
}

pub(super) fn upgrade_statements() -> String {
    let mut statements = String::new();
    for field in field_statements() {
        statements.push_str(field);
        statements.push('\n');
    }
    statements
}

pub(super) fn restore_shared_schema(mut source: String) -> String {
    let fields: Vec<_> = field_statements().collect();
    if fields.iter().all(|field| !source.contains(*field)) {
        return source;
    }
    for (current, previous) in fields.into_iter().zip([PREVIOUS_PASSWORD_VERIFIER, PREVIOUS_CREDENTIAL_REVISION]) {
        assert_eq!(source.matches(current).count(), 1, "revision 169 field source delta drifted");
        source = source.replacen(current, previous, 1);
    }
    source
}
