use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use sha2::{Digest, Sha256};
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue, Value as SurrealValueData};
use thiserror::Error;

use super::schema::{canonicalize_info, info_entry_name, parse_named_array};
use super::{SurrealStorage, SurrealStorageError};

/// A feature-gated, read-only view of the live SurrealDB catalog and rows.
///
/// This type intentionally owns only the lifecycle-aware storage facade. It
/// cannot expose an SDK client, change namespace/database, run caller-authored
/// SurrealQL, or mutate records.
#[derive(Clone)]
pub struct SurrealTestInspector {
    storage: SurrealStorage,
}

#[derive(Debug, Error)]
pub enum SurrealTestInspectorError {
    #[error(transparent)]
    Storage(#[from] SurrealStorageError),
    #[error("invalid structured SurrealDB catalog: {0}")]
    InvalidCatalog(String),
    #[error("unknown catalog table `{0}`")]
    UnknownTable(String),
    #[error("unknown catalog field `{field}` on table `{table}`")]
    UnknownField { table: String, field: String },
    #[error("catalog identifier `{0}` is not safe for closed test inspection")]
    UnsafeIdentifier(String),
    #[error("selector for table `{actual}` cannot be used with table `{expected}`")]
    SelectorTableMismatch { expected: String, actual: String },
    #[error("projection must include at least one catalog field")]
    EmptyProjection,
    #[error("field `{field}` on table `{table}` is not a record reference")]
    NotAReference { table: String, field: String },
    #[error("required reference `{table}.{field}` returned NONE or NULL")]
    NullRequiredReference { table: String, field: String },
    #[error(
        "reference `{table}.{field}` returned a record from `{actual}` instead of `{expected}`"
    )]
    UnexpectedReferenceTable {
        table: String,
        field: String,
        expected: String,
        actual: String,
    },
    #[error("observed row has an invalid shape: {0}")]
    InvalidRow(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SchemaCatalogSnapshot {
    pub schema_version: String,
    pub schema_revision: i64,
    pub info_fingerprint_sha256: String,
    pub tables_defined: usize,
    pub fields_defined: usize,
    pub indexes_defined: usize,
    pub tables: Vec<TableCatalog>,
    pub references: Vec<ReferenceCatalog>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TableCatalog {
    pub name: String,
    pub schemafull: bool,
    pub kind: String,
    pub fields: Vec<FieldCatalog>,
    pub indexes: Vec<IndexCatalog>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldCatalog {
    pub name: String,
    pub kind: String,
    pub readonly: bool,
    pub reference_on_delete: Option<String>,
    pub referenced_tables: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IndexCatalog {
    pub name: String,
    pub columns: Vec<String>,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReferenceCatalog {
    source_table: String,
    source_field: String,
    target_table: String,
    on_delete: String,
}

impl ReferenceCatalog {
    pub fn source_table(&self) -> &str {
        &self.source_table
    }

    pub fn source_field(&self) -> &str {
        &self.source_field
    }

    pub fn target_table(&self) -> &str {
        &self.target_table
    }

    pub fn on_delete(&self) -> &str {
        &self.on_delete
    }
}

/// A validated table capability produced from structured `INFO` output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableSelector {
    name: String,
    fields: BTreeSet<String>,
}

impl TableSelector {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn field(
        &self,
        field: impl AsRef<str>,
    ) -> Result<FieldSelector, SurrealTestInspectorError> {
        let field = field.as_ref();
        require_safe_identifier(field)?;
        if !self.fields.contains(field) {
            return Err(SurrealTestInspectorError::UnknownField {
                table: self.name.clone(),
                field: field.to_owned(),
            });
        }
        Ok(FieldSelector {
            table: self.name.clone(),
            name: field.to_owned(),
        })
    }
}

/// A validated direct field capability. Nested paths and wildcard fields are
/// deliberately excluded from this first inspector slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSelector {
    table: String,
    name: String,
}

impl FieldSelector {
    pub fn table(&self) -> &str {
        &self.table
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScalarValue {
    String(String),
    Bool(bool),
    I64(i64),
    F64(f64),
}

impl From<String> for ScalarValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for ScalarValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<bool> for ScalarValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for ScalarValue {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

impl From<f64> for ScalarValue {
    fn from(value: f64) -> Self {
        Self::F64(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RowFilter {
    All,
    IdEquals(String),
    FieldEquals {
        field: FieldSelector,
        value: ScalarValue,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RecordIdentity {
    pub table: String,
    pub key: serde_json::Value,
}

impl RecordIdentity {
    pub fn key_string(&self) -> Option<&str> {
        self.key.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProjectedRow {
    pub record_id: RecordIdentity,
    pub values: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, SurrealValue)]
struct CountRow {
    count: i64,
}

#[derive(Debug, SurrealValue)]
struct RecordedSchemaState {
    apply_state: String,
    info_fingerprint_sha256: String,
}

impl SurrealTestInspector {
    pub(super) fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub async fn schema_catalog(&self) -> Result<SchemaCatalogSnapshot, SurrealTestInspectorError> {
        let database_info: SurrealValueData = self
            .storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut db_response = database.query("INFO FOR DB STRUCTURE;").await?;
                    Ok(canonicalize_info(db_response.take(0)?))
                })
            })
            .await?;
        let table_names = parse_named_array(&database_info, "tables")
            .map_err(SurrealTestInspectorError::InvalidCatalog)?;
        let table_definitions = named_objects(&database_info, "tables")?;
        for table_name in &table_names {
            require_safe_identifier(table_name)?;
            if !table_definitions.contains_key(table_name) {
                return Err(SurrealTestInspectorError::InvalidCatalog(format!(
                    "table `{table_name}` has no definition"
                )));
            }
        }
        let statements = table_names
            .iter()
            .map(|table_name| format!("INFO FOR TABLE `{table_name}` STRUCTURE;"))
            .collect::<Vec<_>>();
        let table_infos: Vec<SurrealValueData> = self
            .storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut table_infos = Vec::with_capacity(statements.len());
                    for statement in statements {
                        let mut table_response = database.query(statement).await?;
                        table_infos.push(canonicalize_info(table_response.take(0)?));
                    }
                    Ok(table_infos)
                })
            })
            .await?;
        if table_infos.len() != table_names.len() {
            return Err(SurrealTestInspectorError::InvalidCatalog(
                "catalog table detail count differs from database table count".to_owned(),
            ));
        }
        let canonical_tables = table_names
            .iter()
            .cloned()
            .zip(table_infos.iter().cloned())
            .collect::<BTreeMap<_, _>>();
        let info_fingerprint_sha256 = catalog_fingerprint(&database_info, &canonical_tables)?;
        if super::EXPECTED_SCHEMA_INFO_SHA256
            .bytes()
            .all(|byte| byte == b'0')
        {
            return Err(SurrealTestInspectorError::InvalidCatalog(
                "canonical schema fingerprint is not pinned".to_owned(),
            ));
        }
        // Provenance, not a live recompute. The live catalog legitimately carries feature
        // schemas applied on top of the canonical wave (ModelLane alone adds 8 tables), so a
        // live fingerprint can never equal the canonical pin in a real product database.
        // What must hold is that THIS database was bootstrapped from the pinned canonical
        // schema and finalized, which the durable bootstrap-state receipt records.
        let recorded_state: Option<RecordedSchemaState> = self
            .storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query(format!(
                            "SELECT apply_state, info_fingerprint_sha256 FROM {} LIMIT 1;",
                            super::schema::BOOTSTRAP_STATE_TABLE
                        ))
                        .await?;
                    let rows: Vec<RecordedSchemaState> = response.take(0)?;
                    Ok(rows.into_iter().next())
                })
            })
            .await?;
        let Some(recorded_state) = recorded_state else {
            return Err(SurrealTestInspectorError::InvalidCatalog(
                "canonical schema bootstrap receipt is absent".to_owned(),
            ));
        };
        if recorded_state.apply_state != "complete" {
            return Err(SurrealTestInspectorError::InvalidCatalog(format!(
                "canonical schema bootstrap is not finalized: apply_state={}",
                recorded_state.apply_state
            )));
        }
        if recorded_state.info_fingerprint_sha256 != super::EXPECTED_SCHEMA_INFO_SHA256 {
            return Err(SurrealTestInspectorError::InvalidCatalog(format!(
                "canonical schema fingerprint mismatch: expected {}, recorded {}",
                super::EXPECTED_SCHEMA_INFO_SHA256,
                recorded_state.info_fingerprint_sha256
            )));
        }
        let mut tables = Vec::with_capacity(table_names.len());
        let mut references = Vec::new();
        for (table_name, table_info) in table_names.into_iter().zip(table_infos) {
            let table_definition = &table_definitions[&table_name];
            let schemafull = optional_bool(table_definition, "schemafull").unwrap_or(false);
            let kind =
                optional_string(table_definition, "kind").unwrap_or_else(|| "UNKNOWN".to_owned());
            let fields = parse_fields(&table_name, &table_info, &mut references)?;
            let indexes = parse_indexes(&table_name, &table_info)?;
            tables.push(TableCatalog {
                name: table_name,
                schemafull,
                kind,
                fields,
                indexes,
            });
        }
        tables.sort_by(|left, right| left.name.cmp(&right.name));
        references.sort_by(|left, right| {
            (
                left.target_table.as_str(),
                left.source_table.as_str(),
                left.source_field.as_str(),
            )
                .cmp(&(
                    right.target_table.as_str(),
                    right.source_table.as_str(),
                    right.source_field.as_str(),
                ))
        });

        let tables_defined = tables.len();
        let fields_defined = tables.iter().map(|table| table.fields.len()).sum();
        let indexes_defined = tables.iter().map(|table| table.indexes.len()).sum();

        Ok(SchemaCatalogSnapshot {
            schema_version: super::SCHEMA_VERSION.to_owned(),
            schema_revision: super::SCHEMA_REVISION,
            info_fingerprint_sha256,
            tables_defined,
            fields_defined,
            indexes_defined,
            tables,
            references,
        })
    }

    pub async fn table_names(&self) -> Result<Vec<String>, SurrealTestInspectorError> {
        Ok(self
            .schema_catalog()
            .await?
            .tables
            .into_iter()
            .map(|table| table.name)
            .collect())
    }

    pub async fn table_catalog(
        &self,
        table: impl AsRef<str>,
    ) -> Result<TableCatalog, SurrealTestInspectorError> {
        let table = table.as_ref();
        require_safe_identifier(table)?;
        self.schema_catalog()
            .await?
            .tables
            .into_iter()
            .find(|entry| entry.name == table)
            .ok_or_else(|| SurrealTestInspectorError::UnknownTable(table.to_owned()))
    }

    pub async fn table_selector(
        &self,
        table: impl AsRef<str>,
    ) -> Result<TableSelector, SurrealTestInspectorError> {
        let catalog = self.table_catalog(table).await?;
        Ok(TableSelector {
            name: catalog.name,
            fields: catalog
                .fields
                .into_iter()
                .filter(|field| is_safe_identifier(&field.name))
                .map(|field| field.name)
                .collect(),
        })
    }

    pub async fn references_to(
        &self,
        target: &TableSelector,
    ) -> Result<Vec<ReferenceCatalog>, SurrealTestInspectorError> {
        Ok(self
            .schema_catalog()
            .await?
            .references
            .into_iter()
            .filter(|reference| reference.target_table == target.name)
            .collect())
    }

    pub async fn row_count(
        &self,
        table: &TableSelector,
        filter: RowFilter,
    ) -> Result<u64, SurrealTestInspectorError> {
        let rows = self.count_rows(table, filter).await?;
        let count = match rows.as_slice() {
            [] => 0,
            [row] if row.count >= 0 => row.count as u64,
            [row] => {
                return Err(SurrealTestInspectorError::InvalidRow(format!(
                    "negative count {}",
                    row.count
                )))
            }
            _ => {
                return Err(SurrealTestInspectorError::InvalidRow(
                    "count query returned multiple aggregate rows".to_owned(),
                ))
            }
        };
        Ok(count)
    }

    pub async fn exists(
        &self,
        table: &TableSelector,
        filter: RowFilter,
    ) -> Result<bool, SurrealTestInspectorError> {
        Ok(self.row_count(table, filter).await? != 0)
    }

    pub async fn project(
        &self,
        table: &TableSelector,
        fields: &[FieldSelector],
        filter: RowFilter,
    ) -> Result<Vec<ProjectedRow>, SurrealTestInspectorError> {
        if fields.is_empty() {
            return Err(SurrealTestInspectorError::EmptyProjection);
        }
        let field_names = fields
            .iter()
            .map(|field| {
                validate_field_for_table(table, field)?;
                Ok(field.name.clone())
            })
            .collect::<Result<Vec<_>, SurrealTestInspectorError>>()?;
        let projection = field_names
            .iter()
            .map(|field| format!("`{}`", checked_identifier(field).expect("validated field")))
            .collect::<Vec<_>>()
            .join(", ");
        let statement = format!(
            "SELECT id, {projection} FROM `{}`{};",
            checked_identifier(&table.name)?,
            filter_clause(table, &filter)?
        );
        let rows = self.query_values(statement, table, &filter).await?;
        rows.into_iter()
            .map(|row| parse_projected_row(row, &field_names))
            .collect()
    }

    pub async fn referenced_ids(
        &self,
        reference: &ReferenceCatalog,
        filter: RowFilter,
    ) -> Result<Vec<RecordIdentity>, SurrealTestInspectorError> {
        let source_catalog = self.table_catalog(&reference.source_table).await?;
        let source_field_catalog = source_catalog
            .fields
            .iter()
            .find(|field| field.name == reference.source_field)
            .ok_or_else(|| SurrealTestInspectorError::UnknownField {
                table: reference.source_table.clone(),
                field: reference.source_field.clone(),
            })?;
        let allows_none = field_kind_allows_none(&source_field_catalog.kind);
        let table = TableSelector {
            name: source_catalog.name,
            fields: source_catalog
                .fields
                .into_iter()
                .filter(|field| is_safe_identifier(&field.name))
                .map(|field| field.name)
                .collect(),
        };
        let field = table.field(&reference.source_field)?;
        let live_reference = self
            .references_to(&self.table_selector(&reference.target_table).await?)
            .await?
            .into_iter()
            .any(|candidate| candidate == *reference);
        if !live_reference {
            return Err(SurrealTestInspectorError::NotAReference {
                table: reference.source_table.clone(),
                field: reference.source_field.clone(),
            });
        }
        let statement = format!(
            "SELECT `{}` AS referenced_id FROM `{}`{};",
            checked_identifier(field.name())?,
            checked_identifier(table.name())?,
            filter_clause(&table, &filter)?
        );
        let rows = self.query_values(statement, &table, &filter).await?;
        rows.into_iter()
            .map(|row| parse_referenced_id_row(row, reference, allows_none))
            .collect::<Result<Vec<_>, _>>()
            .map(|rows| rows.into_iter().flatten().collect())
    }

    async fn count_rows(
        &self,
        table: &TableSelector,
        filter: RowFilter,
    ) -> Result<Vec<CountRow>, SurrealTestInspectorError> {
        let statement = format!(
            "SELECT count() AS count FROM `{}`{} GROUP ALL;",
            checked_identifier(&table.name)?,
            filter_clause(table, &filter)?
        );
        let binding = filter_binding(table, &filter)?;
        self.storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = match binding {
                        None => database.query(statement).await?,
                        Some(binding) => {
                            database
                                .query_bound(statement, ("filter_value", binding))
                                .await?
                        }
                    };
                    Ok(response.take(0)?)
                })
            })
            .await
            .map_err(Into::into)
    }

    async fn query_values(
        &self,
        statement: String,
        table: &TableSelector,
        filter: &RowFilter,
    ) -> Result<Vec<SurrealValueData>, SurrealTestInspectorError> {
        let binding = filter_binding(table, filter)?;
        self.storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = match binding {
                        None => database.query(statement).await?,
                        Some(binding) => {
                            database
                                .query_bound(statement, ("filter_value", binding))
                                .await?
                        }
                    };
                    Ok(response.take(0)?)
                })
            })
            .await
            .map_err(Into::into)
    }
}

#[derive(Serialize)]
struct CanonicalCatalogEnvelope<'a> {
    database: &'a SurrealValueData,
    tables: &'a BTreeMap<String, SurrealValueData>,
}

fn catalog_fingerprint(
    database: &SurrealValueData,
    tables: &BTreeMap<String, SurrealValueData>,
) -> Result<String, SurrealTestInspectorError> {
    let canonical_json = serde_json::to_string(&CanonicalCatalogEnvelope { database, tables })
        .map_err(|error| SurrealTestInspectorError::InvalidCatalog(error.to_string()))?;
    let digest = Sha256::digest(canonical_json.as_bytes());
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn parse_fields(
    table: &str,
    info: &SurrealValueData,
    references: &mut Vec<ReferenceCatalog>,
) -> Result<Vec<FieldCatalog>, SurrealTestInspectorError> {
    let entries = named_objects(info, "fields")?;
    let mut fields = Vec::with_capacity(entries.len());
    for (name, definition) in entries {
        let kind = optional_string(&definition, "kind").unwrap_or_else(|| "any".to_owned());
        let readonly = optional_bool(&definition, "readonly").unwrap_or(false);
        let on_delete = optional_object(&definition, "reference")
            .and_then(|reference| optional_string(reference, "on_delete"));
        let referenced_tables = if on_delete.is_some() {
            record_targets(&kind)?
        } else {
            Vec::new()
        };
        if let Some(on_delete) = &on_delete {
            for target_table in &referenced_tables {
                references.push(ReferenceCatalog {
                    source_table: table.to_owned(),
                    source_field: name.clone(),
                    target_table: target_table.clone(),
                    on_delete: on_delete.clone(),
                });
            }
        }
        fields.push(FieldCatalog {
            name,
            kind,
            readonly,
            reference_on_delete: on_delete,
            referenced_tables,
        });
    }
    fields.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(fields)
}

fn parse_indexes(
    table: &str,
    info: &SurrealValueData,
) -> Result<Vec<IndexCatalog>, SurrealTestInspectorError> {
    let entries = named_objects(info, "indexes")?;
    let mut indexes = Vec::with_capacity(entries.len());
    for (name, definition) in entries {
        let columns = required_array(&definition, "cols")?
            .iter()
            .map(value_text)
            .collect::<Result<Vec<_>, _>>()?;
        let kind = definition
            .get("index")
            .map(value_text)
            .transpose()?
            .unwrap_or_else(|| "INDEX".to_owned());
        if optional_string(&definition, "table").as_deref() != Some(table) {
            return Err(SurrealTestInspectorError::InvalidCatalog(format!(
                "index `{name}` table does not match `{table}`"
            )));
        }
        indexes.push(IndexCatalog {
            name,
            columns,
            kind,
        });
    }
    indexes.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(indexes)
}

fn named_objects(
    value: &SurrealValueData,
    key: &str,
) -> Result<BTreeMap<String, surrealdb::types::Object>, SurrealTestInspectorError> {
    let SurrealValueData::Object(object) = value else {
        return Err(SurrealTestInspectorError::InvalidCatalog(
            "expected catalog object".to_owned(),
        ));
    };
    let Some(SurrealValueData::Array(array)) = object.get(key) else {
        return Err(SurrealTestInspectorError::InvalidCatalog(format!(
            "missing `{key}` array"
        )));
    };
    array
        .iter()
        .map(|entry| {
            let name = info_entry_name(entry).ok_or_else(|| {
                SurrealTestInspectorError::InvalidCatalog(format!("`{key}` entry has no name"))
            })?;
            let SurrealValueData::Object(definition) = entry else {
                unreachable!("named INFO entries are objects")
            };
            Ok((name.to_owned(), definition.clone()))
        })
        .collect()
}

fn required_array<'a>(
    object: &'a surrealdb::types::Object,
    key: &str,
) -> Result<&'a surrealdb::types::Array, SurrealTestInspectorError> {
    match object.get(key) {
        Some(SurrealValueData::Array(value)) => Ok(value),
        _ => Err(SurrealTestInspectorError::InvalidCatalog(format!(
            "missing `{key}` array"
        ))),
    }
}

fn optional_object<'a>(
    object: &'a surrealdb::types::Object,
    key: &str,
) -> Option<&'a surrealdb::types::Object> {
    match object.get(key) {
        Some(SurrealValueData::Object(value)) => Some(value),
        _ => None,
    }
}

fn optional_string(object: &surrealdb::types::Object, key: &str) -> Option<String> {
    match object.get(key) {
        Some(SurrealValueData::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn optional_bool(object: &surrealdb::types::Object, key: &str) -> Option<bool> {
    match object.get(key) {
        Some(SurrealValueData::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn value_text(value: &SurrealValueData) -> Result<String, SurrealTestInspectorError> {
    match value {
        SurrealValueData::String(value) => Ok(value.clone()),
        _ => serde_json::to_value(value)
            .map(|value| value.to_string())
            .map_err(|error| SurrealTestInspectorError::InvalidCatalog(error.to_string())),
    }
}

fn record_targets(kind: &str) -> Result<Vec<String>, SurrealTestInspectorError> {
    let mut rest = kind;
    let mut targets = BTreeSet::new();
    while let Some(start) = rest.find("record<") {
        let after = &rest[start + "record<".len()..];
        let end = after.find('>').ok_or_else(|| {
            SurrealTestInspectorError::InvalidCatalog(format!("unterminated record kind `{kind}`"))
        })?;
        for target in after[..end].split('|').map(str::trim) {
            require_safe_identifier(target)?;
            targets.insert(target.to_owned());
        }
        rest = &after[end + 1..];
    }
    if targets.is_empty() {
        return Err(SurrealTestInspectorError::InvalidCatalog(format!(
            "reference has non-record kind `{kind}`"
        )));
    }
    Ok(targets.into_iter().collect())
}

fn filter_clause(
    table: &TableSelector,
    filter: &RowFilter,
) -> Result<String, SurrealTestInspectorError> {
    match filter {
        RowFilter::All => Ok(String::new()),
        RowFilter::IdEquals(_) => Ok(" WHERE id = $filter_value".to_owned()),
        RowFilter::FieldEquals { field, .. } => {
            validate_field_for_table(table, field)?;
            Ok(format!(
                " WHERE `{}` = $filter_value",
                checked_identifier(field.name())?
            ))
        }
    }
}

fn filter_binding(
    table: &TableSelector,
    filter: &RowFilter,
) -> Result<Option<SurrealValueData>, SurrealTestInspectorError> {
    match filter {
        RowFilter::All => Ok(None),
        RowFilter::IdEquals(id) => Ok(Some(SurrealValueData::RecordId(RecordId::new(
            table.name.clone(),
            id.clone(),
        )))),
        RowFilter::FieldEquals { field, value } => {
            validate_field_for_table(table, field)?;
            Ok(Some(match value {
                ScalarValue::String(value) => value.clone().into_value(),
                ScalarValue::Bool(value) => (*value).into_value(),
                ScalarValue::I64(value) => (*value).into_value(),
                ScalarValue::F64(value) => (*value).into_value(),
            }))
        }
    }
}

fn validate_field_for_table(
    table: &TableSelector,
    field: &FieldSelector,
) -> Result<(), SurrealTestInspectorError> {
    if field.table != table.name {
        return Err(SurrealTestInspectorError::SelectorTableMismatch {
            expected: table.name.clone(),
            actual: field.table.clone(),
        });
    }
    if !table.fields.contains(&field.name) {
        return Err(SurrealTestInspectorError::UnknownField {
            table: table.name.clone(),
            field: field.name.clone(),
        });
    }
    require_safe_identifier(&field.name)
}

fn parse_projected_row(
    row: SurrealValueData,
    fields: &[String],
) -> Result<ProjectedRow, SurrealTestInspectorError> {
    let SurrealValueData::Object(object) = row else {
        return Err(SurrealTestInspectorError::InvalidRow(
            "projection did not return an object".to_owned(),
        ));
    };
    let record_id = object
        .get("id")
        .cloned()
        .ok_or_else(|| SurrealTestInspectorError::InvalidRow("missing record id".to_owned()))
        .and_then(parse_record_identity)?;
    let mut values = BTreeMap::new();
    for field in fields {
        let value = object.get(field).cloned().ok_or_else(|| {
            SurrealTestInspectorError::InvalidRow(format!("missing projected field `{field}`"))
        })?;
        let value = projected_json(&value)?;
        values.insert(field.clone(), value);
    }
    Ok(ProjectedRow { record_id, values })
}

/// Projects one stored field as plain JSON for assertions.
///
/// `serde_json::to_value` on a Surreal value serializes the externally tagged enum, so a
/// stored string arrives as {"String": "..."} rather than "...". Callers compare projected
/// scope columns against plain strings, so the tag is unwrapped here for the scalar shapes a
/// projection can return; anything else keeps its structural serialization.
fn projected_json(
    value: &SurrealValueData,
) -> Result<serde_json::Value, SurrealTestInspectorError> {
    let tagged = serde_json::to_value(value)
        .map_err(|error| SurrealTestInspectorError::InvalidRow(error.to_string()))?;
    let serde_json::Value::Object(map) = &tagged else {
        return Ok(tagged);
    };
    if map.len() != 1 {
        return Ok(tagged);
    }
    let (variant, inner) = map
        .iter()
        .next()
        .expect("single-entry object has one entry");
    match (variant.as_str(), inner) {
        ("String", inner @ serde_json::Value::String(_)) => Ok(inner.clone()),
        ("Bool", inner @ serde_json::Value::Bool(_)) => Ok(inner.clone()),
        ("Number" | "Int" | "Float" | "Decimal", inner) if inner.is_number() => {
            Ok(inner.clone())
        }
        ("Null" | "None", _) => Ok(serde_json::Value::Null),
        ("Uuid" | "Datetime" | "Strand", inner @ serde_json::Value::String(_)) => {
            Ok(inner.clone())
        }
        _ => Ok(tagged),
    }
}

fn parse_record_identity(
    value: SurrealValueData,
) -> Result<RecordIdentity, SurrealTestInspectorError> {
    let SurrealValueData::RecordId(record_id) = value else {
        return Err(SurrealTestInspectorError::InvalidRow(
            "expected record id".to_owned(),
        ));
    };
    let key = match record_id.key {
        RecordIdKey::String(value) => serde_json::Value::String(value),
        key => serde_json::to_value(key)
            .map_err(|error| SurrealTestInspectorError::InvalidRow(error.to_string()))?,
    };
    Ok(RecordIdentity {
        table: record_id.table.as_str().to_owned(),
        key,
    })
}

fn field_kind_allows_none(kind: &str) -> bool {
    let kind = kind.trim();
    if kind.starts_with("option<") {
        return true;
    }
    let mut depth = 0usize;
    let mut token_start = 0usize;
    for (index, character) in kind.char_indices() {
        match character {
            '<' => depth = depth.saturating_add(1),
            '>' => depth = depth.saturating_sub(1),
            '|' if depth == 0 => {
                if kind[token_start..index].trim() == "none" {
                    return true;
                }
                token_start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    kind[token_start..].trim() == "none"
}

fn parse_referenced_id_row(
    row: SurrealValueData,
    reference: &ReferenceCatalog,
    allows_none: bool,
) -> Result<Option<RecordIdentity>, SurrealTestInspectorError> {
    let SurrealValueData::Object(object) = row else {
        return Err(SurrealTestInspectorError::InvalidRow(
            "reference projection did not return an object".to_owned(),
        ));
    };
    let value = object.get("referenced_id").cloned().ok_or_else(|| {
        SurrealTestInspectorError::InvalidRow(format!(
            "reference projection is missing `{}`.{}",
            reference.source_table, reference.source_field
        ))
    })?;
    if matches!(value, SurrealValueData::None | SurrealValueData::Null) {
        return if allows_none {
            Ok(None)
        } else {
            Err(SurrealTestInspectorError::NullRequiredReference {
                table: reference.source_table.clone(),
                field: reference.source_field.clone(),
            })
        };
    }
    let identity = parse_record_identity(value)?;
    if identity.table != reference.target_table {
        return Err(SurrealTestInspectorError::UnexpectedReferenceTable {
            table: reference.source_table.clone(),
            field: reference.source_field.clone(),
            expected: reference.target_table.clone(),
            actual: identity.table,
        });
    }
    Ok(Some(identity))
}

fn checked_identifier(identifier: &str) -> Result<&str, SurrealTestInspectorError> {
    require_safe_identifier(identifier)?;
    Ok(identifier)
}

fn require_safe_identifier(identifier: &str) -> Result<(), SurrealTestInspectorError> {
    if is_safe_identifier(identifier) {
        Ok(())
    } else {
        Err(SurrealTestInspectorError::UnsafeIdentifier(
            identifier.to_owned(),
        ))
    }
}

fn is_safe_identifier(identifier: &str) -> bool {
    let mut chars = identifier.chars();
    matches!(chars.next(), Some('_' | 'a'..='z' | 'A'..='Z'))
        && chars.all(|character| matches!(character, '_' | 'a'..='z' | 'A'..='Z' | '0'..='9'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_reference() -> ReferenceCatalog {
        ReferenceCatalog {
            source_table: "documents".to_owned(),
            source_field: "workspace_id".to_owned(),
            target_table: "workspaces".to_owned(),
            on_delete: "CASCADE".to_owned(),
        }
    }

    fn reference_row(value: SurrealValueData) -> SurrealValueData {
        let mut object = surrealdb::types::Object::new();
        object.insert("referenced_id".to_owned(), value);
        SurrealValueData::Object(object)
    }

    #[test]
    fn reference_row_parser_fails_closed_and_enforces_target_table() {
        let reference = workspace_reference();

        assert!(matches!(
            parse_referenced_id_row(
                SurrealValueData::String("bad-row".to_owned()),
                &reference,
                false
            ),
            Err(SurrealTestInspectorError::InvalidRow(_))
        ));
        assert!(matches!(
            parse_referenced_id_row(
                SurrealValueData::Object(surrealdb::types::Object::new()),
                &reference,
                false,
            ),
            Err(SurrealTestInspectorError::InvalidRow(_))
        ));
        assert!(matches!(
            parse_referenced_id_row(
                reference_row(SurrealValueData::String("not-a-record".to_owned())),
                &reference,
                false,
            ),
            Err(SurrealTestInspectorError::InvalidRow(_))
        ));
        assert!(matches!(
            parse_referenced_id_row(reference_row(SurrealValueData::None), &reference, false,),
            Err(SurrealTestInspectorError::NullRequiredReference { .. })
        ));
        assert_eq!(
            parse_referenced_id_row(reference_row(SurrealValueData::Null), &reference, true,)
                .expect("optional reference permits an explicit null"),
            None
        );
        assert!(matches!(
            parse_referenced_id_row(
                reference_row(SurrealValueData::RecordId(RecordId::new(
                    "documents",
                    "wrong-table",
                ))),
                &reference,
                false,
            ),
            Err(SurrealTestInspectorError::UnexpectedReferenceTable { .. })
        ));

        let observed = parse_referenced_id_row(
            reference_row(SurrealValueData::RecordId(RecordId::new(
                "workspaces",
                "workspace-1",
            ))),
            &reference,
            false,
        )
        .expect("matching reference is accepted")
        .expect("required reference produces an identity");
        assert_eq!(observed.table, "workspaces");
        assert_eq!(observed.key_string(), Some("workspace-1"));
    }

    #[test]
    fn reference_optionality_is_derived_from_the_catalog_kind() {
        assert!(field_kind_allows_none("option<record<workspaces>>"));
        assert!(field_kind_allows_none("none | record<workspaces>"));
        assert!(field_kind_allows_none("record<workspaces> | none"));
        assert!(!field_kind_allows_none("record<workspaces>"));
        assert!(!field_kind_allows_none("record<none | workspaces>"));
    }
}
