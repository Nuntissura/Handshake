//! WP-KERNEL-012 MT-167: code-nav index cost against Master Spec §2.3.14.17.3
//! (index update propagation: target < 5 s, maximum 30 s) and batch/per-file parity.
//!
//! * `mt167_code_nav_index_median_within_spec_baseline` (AC-167-3) times the full
//!   `POST /workspaces/{id}/code-nav/index` request as the authenticated record user on
//!   the Handshake-managed embedded SurrealDB harness for the MT-008 fixture root and the
//!   FEMS two-file seed root: 1 warm-up + N=5 timed samples per shape, a fresh workspace
//!   and fixture directory per sample, non-vacuous counts per run, printed measurements.
//! * `mt167_batch_and_per_file_writer_produce_equal_rows` (MT-167-EQUAL-COUNTS, AC-167-2)
//!   indexes each shape (plus a richer parity-only shape) TWICE through the route
//!   (clean-code batch) and TWICE through the per-file writer (`index_code_source`), and
//!   asserts equal totals AND equal sets of natural keys for entities, spans,
//!   entity_spans, edges, edge_spans and code-file rows after the first and second pass,
//!   and that the totals stay stable across passes (no evidence-link accumulation).
//!
//! Fixture trees are written only below `HANDSHAKE_TEST_STAGE_BINDING_ROOT` (an external
//! Handshake_Artifacts directory supplied by the runner) and removed by the test.

#[allow(dead_code)]
mod user_manual_support;

#[path = "account_session_support/mod.rs"]
mod account_session_support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use account_session_support::AccountFixture;
use handshake_core::api::code_nav_index as index_api;
use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_code_index::engine::{CodeIndexContext, CodeIndexEngine};
use handshake_core::knowledge_code_index::CODE_EXTRACTOR_VERSION;
use handshake_core::storage::knowledge::{
    KnowledgeEntityKind, KnowledgeIndexingEligibility, KnowledgeRootKind, KnowledgeStore,
    NewKnowledgeSourceRoot,
};
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::Database;
use serde_json::{json, Value};
use user_manual_support::{app_state_for, manual_test_backend, start_server};
use uuid::Uuid;

/// The MT-008 native fixture (tests/mt008_code_nav_support/mod.rs): one doc comment and
/// one in-file call, i.e. the shape that used to fall back to the per-file writer.
const MT008_LIB_RS: &str = "/// Adds two numbers.\npub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn caller() -> i32 { add(1, 2) }\n";

/// The FEMS seed (test_fems_interop_proofs.rs seed_code_authority) with a fixed symbol.
const FEMS_TARGET_RS: &str =
    "pub fn fems_target_mt167() -> &'static str {\n    \"canonical-fems-café\"\n}\n";
const FEMS_ANCHOR_RS: &str = "pub fn anchor() {}\n";

/// Parity-only shape covering the other batched passage/relationship forms: an inner doc
/// comment documenting the file, an outer doc comment documenting a symbol, a TODO marker,
/// two operator strings on one line (one shared concept key), an implements edge, a
/// resolved call and an unresolved call (`String::new`).
const RICH_LIB_RS: &str = "//! Crate docs for the MT-167 parity shape.

/// Greets someone.
pub trait Greet {
    fn greet(&self) -> String;
}

pub struct Person;

impl Greet for Person {
    fn greet(&self) -> String {
        String::new()
    }
}

// TODO: tidy the shout helper
pub fn shout() { println!(\"loud\"); println!(\"again\"); }

pub fn caller() {
    shout();
}
";

const RICH_SHAPE: Shape = Shape {
    name: "rich-parity",
    files: &[("lib.rs", RICH_LIB_RS)],
    min_symbols: 4,
};

const WARM_UPS: usize = 1;
const SAMPLES: usize = 5;
const TARGET_MEDIAN: Duration = Duration::from_secs(5);
const MAXIMUM_SAMPLE: Duration = Duration::from_secs(30);

struct Shape {
    name: &'static str,
    files: &'static [(&'static str, &'static str)],
    min_symbols: u64,
}

const SHAPES: [Shape; 2] = [
    Shape {
        name: "mt008-fixture",
        files: &[("lib.rs", MT008_LIB_RS)],
        min_symbols: 2,
    },
    Shape {
        name: "fems-seed",
        files: &[("target.rs", FEMS_TARGET_RS), ("anchor.rs", FEMS_ANCHOR_RS)],
        min_symbols: 2,
    },
];

fn artifact_root() -> PathBuf {
    let root: PathBuf = std::env::var("HANDSHAKE_TEST_STAGE_BINDING_ROOT")
        .expect(
            "MT-167 requires HANDSHAKE_TEST_STAGE_BINDING_ROOT=<absolute dir below the external \
             Handshake_Artifacts root> for its fixture trees; nothing is written into the repo",
        )
        .into();
    assert!(root.is_absolute(), "artifact root must be absolute");
    root.join("wp-kernel-012").join("mt-167")
}

/// Write one shape into a fresh directory below the artifact root.
fn write_fixture(shape: &Shape, label: &str) -> PathBuf {
    let dir = artifact_root().join(format!(
        "{}-{label}-{}",
        shape.name,
        Uuid::now_v7().simple()
    ));
    std::fs::create_dir_all(&dir).expect("create MT-167 fixture directory");
    for (relative_path, text) in shape.files {
        std::fs::write(dir.join(relative_path), text).expect("write MT-167 fixture file");
    }
    dir.canonicalize()
        .expect("canonicalize MT-167 fixture directory")
}

fn nav_headers(client: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    client
        .header("x-hsk-actor-kind", "operator")
        .header("x-hsk-actor-id", format!("mt167-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-MT167-{label}"))
        .header("x-hsk-session-run-id", format!("SR-MT167-{label}"))
        .header("x-hsk-correlation-id", format!("CORR-MT167-{label}"))
}

/// POST the full code-nav index request; returns the elapsed wall time and the body.
async fn index_route(
    http: &reqwest::Client,
    base: &str,
    workspace_id: &str,
    root: &PathBuf,
    label: &str,
) -> (Duration, Value) {
    let started = Instant::now();
    let response = nav_headers(
        http.post(format!("{base}/workspaces/{workspace_id}/code-nav/index")),
        label,
    )
    .json(&json!({"root_path": root.to_string_lossy()}))
    .send()
    .await
    .expect("MT-167 code-nav index request");
    let elapsed = started.elapsed();
    let status = response.status();
    let body: Value = response.json().await.expect("MT-167 index response JSON");
    assert_eq!(status, 200, "MT-167 code-nav index route: {body}");
    (elapsed, body)
}

fn assert_non_vacuous(shape: &Shape, body: &Value) {
    assert_eq!(
        body["files_indexed"].as_u64(),
        Some(shape.files.len() as u64),
        "{}: every fixture file must be indexed: {body}",
        shape.name
    );
    assert_eq!(
        body["files_failed"].as_u64(),
        Some(0),
        "{}: {body}",
        shape.name
    );
    assert!(
        body["symbol_count"].as_u64().unwrap_or(0) >= shape.min_symbols,
        "{}: timed run must index its symbols: {body}",
        shape.name
    );
}

fn median(samples: &[Duration]) -> Duration {
    let mut sorted = samples.to_vec();
    sorted.sort();
    sorted[sorted.len() / 2]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt167_code_nav_index_median_within_spec_baseline() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("handshake_core::code_nav_index=info")
        .with_test_writer()
        .try_init();
    let backend = manual_test_backend()
        .await
        .expect("open embedded backend for MT-167 timing");
    let account = AccountFixture::install(backend.db.storage()).await;
    let state = app_state_for(&backend.db).await;
    let (base, server) = start_server(index_api::routes(state.clone())).await;
    let http = reqwest::Client::builder()
        .default_headers(account.owner.headers())
        .timeout(MAXIMUM_SAMPLE + Duration::from_secs(15))
        .build()
        .expect("MT-167 request client");

    let mut created = Vec::new();
    let mut report = Vec::new();
    for shape in &SHAPES {
        let mut samples = Vec::with_capacity(SAMPLES);
        for run in 0..WARM_UPS + SAMPLES {
            let workspace_id = account.owner.create_workspace(&state).await;
            let root = write_fixture(shape, &format!("run{run}"));
            created.push(root.clone());
            let (elapsed, body) = index_route(
                &http,
                &base,
                &workspace_id,
                &root,
                &format!("{}-{run}", shape.name),
            )
            .await;
            assert_non_vacuous(shape, &body);
            println!(
                "MT167_TIMING shape={} run={} warm_up={} elapsed_ms={} symbols={}",
                shape.name,
                run,
                run < WARM_UPS,
                elapsed.as_millis(),
                body["symbol_count"]
            );
            if run >= WARM_UPS {
                samples.push(elapsed);
            }
        }
        let median = median(&samples);
        let max = samples.iter().max().copied().unwrap_or_default();
        println!(
            "MT167_TIMING_SUMMARY shape={} samples_ms={:?} median_ms={} max_ms={}",
            shape.name,
            samples.iter().map(Duration::as_millis).collect::<Vec<_>>(),
            median.as_millis(),
            max.as_millis()
        );
        report.push((shape.name, samples, median, max));
    }
    server.abort();
    for dir in created {
        std::fs::remove_dir_all(&dir).expect("remove MT-167 fixture directory");
    }

    for (name, samples, median, max) in &report {
        assert!(
            *max < MAXIMUM_SAMPLE,
            "{name}: every sample must stay below the {MAXIMUM_SAMPLE:?} maximum: {samples:?}"
        );
        assert!(
            *median < TARGET_MEDIAN,
            "{name}: median {median:?} must meet the {TARGET_MEDIAN:?} target: {samples:?}"
        );
    }
}

/// Natural-key snapshot of every code-index row of one workspace.
#[derive(Debug, PartialEq, Eq)]
struct RowSets {
    entities: BTreeSet<String>,
    spans: BTreeSet<String>,
    entity_spans: BTreeSet<String>,
    edges: BTreeSet<String>,
    edge_spans: BTreeSet<String>,
    code_files: BTreeSet<String>,
    totals: BTreeMap<&'static str, usize>,
}

async fn code_index_rows(db: &SurrealDatabase, workspace_id: &str, root_id: &str) -> RowSets {
    let source_paths: BTreeMap<String, String> = db
        .list_knowledge_sources_for_root(root_id)
        .await
        .expect("list MT-167 sources")
        .into_iter()
        .filter(|source| source.workspace_id == workspace_id)
        .map(|source| (source.source_id, source.relative_path.unwrap_or_default()))
        .collect();

    let mut entity_keys = BTreeMap::new();
    let mut entities = BTreeSet::new();
    let mut entity_rows = 0usize;
    for kind in [
        KnowledgeEntityKind::File,
        KnowledgeEntityKind::Symbol,
        KnowledgeEntityKind::Concept,
    ] {
        for entity in db
            .list_knowledge_entities_by_kind(workspace_id, kind)
            .await
            .expect("list MT-167 entities")
        {
            if entity.detection_provenance["extractor"] != json!("knowledge_code_index") {
                continue;
            }
            let natural = format!("{}|{}", entity.entity_kind.as_str(), entity.entity_key);
            let primary = entity
                .primary_source_id
                .as_ref()
                .and_then(|id| source_paths.get(id))
                .cloned()
                .unwrap_or_default();
            entities.insert(format!(
                "{natural}|{}|{}|{primary}",
                entity.display_name, entity.detection_provenance
            ));
            entity_keys.insert(entity.entity_id.clone(), natural);
            entity_rows += 1;
        }
    }

    let mut span_keys = BTreeMap::new();
    for source_id in source_paths.keys() {
        for span in db
            .list_knowledge_spans_for_source(source_id)
            .await
            .expect("list MT-167 spans")
        {
            span_keys.insert(
                span.span_id.clone(),
                format!(
                    "{}|{}|{}|{}|{:?}|{:?}|{:?}|{}|{:?}",
                    source_paths[source_id],
                    span.span_kind.as_str(),
                    span.range_start,
                    span.range_end,
                    span.line_start,
                    span.line_end,
                    span.section_path,
                    span.content_sha256,
                    span.display_snippet
                ),
            );
        }
    }

    let mut spans = BTreeSet::new();
    let mut entity_spans = BTreeSet::new();
    let mut entity_span_rows = 0usize;
    let mut edges = BTreeSet::new();
    let mut edge_rows = BTreeMap::new();
    let mut edge_spans = BTreeSet::new();
    let mut edge_span_rows = 0usize;
    for (entity_id, natural) in &entity_keys {
        for span_id in db
            .list_knowledge_entity_span_ids(entity_id)
            .await
            .expect("list MT-167 entity spans")
        {
            let span = span_keys
                .get(&span_id)
                .unwrap_or_else(|| panic!("entity span {span_id} is not a fixture span"));
            spans.insert(span.clone());
            entity_spans.insert(format!("{natural}=>{span}"));
            entity_span_rows += 1;
        }
        for edge in db
            .list_knowledge_edges_for_entity(entity_id)
            .await
            .expect("list MT-167 edges")
        {
            let (Some(source), Some(target)) = (
                entity_keys.get(&edge.source_entity_id),
                entity_keys.get(&edge.target_entity_id),
            ) else {
                continue;
            };
            if edge.extractor_version != CODE_EXTRACTOR_VERSION {
                continue;
            }
            let natural_edge = format!(
                "{}|{source}|{target}|{}",
                edge.edge_type.as_str(),
                edge.confidence
            );
            edges.insert(natural_edge.clone());
            edge_rows.insert(edge.edge_id.clone(), natural_edge);
        }
    }
    for (edge_id, natural_edge) in &edge_rows {
        for span_id in db
            .list_knowledge_edge_span_ids(edge_id)
            .await
            .expect("list MT-167 edge spans")
        {
            let span = span_keys
                .get(&span_id)
                .unwrap_or_else(|| panic!("edge span {span_id} is not a fixture span"));
            spans.insert(span.clone());
            edge_spans.insert(format!("{natural_edge}=>{span}"));
            edge_span_rows += 1;
        }
    }

    let code_files: BTreeSet<String> = db
        .list_knowledge_code_files(workspace_id)
        .await
        .expect("list MT-167 code files")
        .into_iter()
        .filter_map(|file| {
            let path = source_paths.get(&file.source_id)?;
            Some(format!(
                "{path}|{:?}|{}|{}|{}",
                file.parse_status, file.symbols_indexed, file.edges_indexed, file.stale
            ))
        })
        .collect();

    let totals = BTreeMap::from([
        ("entities", entity_rows),
        ("spans", spans.len()),
        ("entity_spans", entity_span_rows),
        ("edges", edge_rows.len()),
        ("edge_spans", edge_span_rows),
    ]);
    RowSets {
        entities,
        spans,
        entity_spans,
        edges,
        edge_spans,
        code_files,
        totals,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt167_batch_and_per_file_writer_produce_equal_rows() {
    let backend = manual_test_backend()
        .await
        .expect("open embedded backend for MT-167 parity");
    let account = AccountFixture::install(backend.db.storage()).await;
    let state = app_state_for(&backend.db).await;
    let (base, server) = start_server(index_api::routes(state.clone())).await;
    let http = reqwest::Client::builder()
        .default_headers(account.owner.headers())
        .timeout(MAXIMUM_SAMPLE + Duration::from_secs(15))
        .build()
        .expect("MT-167 request client");
    let engine = CodeIndexEngine::new(Arc::new(backend.db.clone()));
    let context = CodeIndexContext {
        actor: KernelActor::System("mt167-per-file-writer".to_string()),
        kernel_task_run_id: "KTR-MT167-PER-FILE".to_string(),
        session_run_id: "SR-MT167-PER-FILE".to_string(),
        correlation_id: None,
    };

    let mut created = Vec::new();
    for shape in SHAPES.iter().chain(std::iter::once(&RICH_SHAPE)) {
        // Batch: the real route as the record user.
        let batch_workspace = account.owner.create_workspace(&state).await;
        let root = write_fixture(shape, "batch");
        created.push(root.clone());
        let (_, body) = index_route(
            &http,
            &base,
            &batch_workspace,
            &root,
            &format!("{}-batch", shape.name),
        )
        .await;
        assert_non_vacuous(shape, &body);
        let batch_root = body["root_id"].as_str().expect("batch root id").to_owned();
        assert_batch_path(&backend.db, shape, &body).await;

        // Per-file writer: same relative paths and text, separate workspace.
        let per_file_workspace = account.owner.create_workspace(&state).await;
        let per_file_root = backend
            .db
            .create_knowledge_source_root(NewKnowledgeSourceRoot {
                workspace_id: per_file_workspace.clone(),
                display_name: format!("mt167-{}", shape.name),
                root_kind: KnowledgeRootKind::ProjectRepo,
                repo_relative_path: format!("mt167/{}", Uuid::now_v7().simple()),
                allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
                indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
            })
            .await
            .expect("create MT-167 per-file root")
            .root_id;
        let mut per_file_sources = Vec::new();
        for (relative_path, text) in shape.files {
            let source_id = engine
                .register_code_source(
                    &per_file_workspace,
                    Some(&per_file_root),
                    relative_path,
                    text,
                )
                .await
                .expect("register MT-167 per-file source");
            per_file_sources.push((source_id, *relative_path, *text));
        }

        let mut first_pass_totals = None;
        for pass in 1..=2 {
            if pass == 2 {
                // Re-index the same content: both writers must replace this source's older
                // evidence links (symbols, passages, edges) identically (no accumulation).
                let (_, body) = index_route(
                    &http,
                    &base,
                    &batch_workspace,
                    &root,
                    &format!("{}-batch-reindex", shape.name),
                )
                .await;
                assert_non_vacuous(shape, &body);
                assert_eq!(body["root_id"].as_str(), Some(batch_root.as_str()));
                assert_batch_path(&backend.db, shape, &body).await;
            }
            for (source_id, relative_path, text) in &per_file_sources {
                engine
                    .index_code_source(
                        &context,
                        &per_file_workspace,
                        source_id,
                        relative_path,
                        text,
                        None,
                    )
                    .await
                    .expect("MT-167 per-file writer");
            }
            let totals = assert_equal_rows(
                &backend.db,
                shape,
                pass,
                (&batch_workspace, &batch_root),
                (&per_file_workspace, &per_file_root),
            )
            .await;
            // MT-167 (DX-MT-167-20261002-EVIDENCE-LINK-ACCUMULATION): re-indexing unchanged
            // content must not accumulate evidence links in either writer.
            match &first_pass_totals {
                None => first_pass_totals = Some(totals),
                Some(first) => assert_eq!(
                    &totals, first,
                    "{}: row totals must stay stable across re-index passes (pass 1 vs pass {pass})",
                    shape.name
                ),
            }
        }
    }
    server.abort();
    for dir in created {
        std::fs::remove_dir_all(&dir).expect("remove MT-167 fixture directory");
    }
}

/// The route run recorded in `body` took the single-transaction batch path.
async fn assert_batch_path(db: &SurrealDatabase, shape: &Shape, body: &Value) {
    let index_run_id = body["index_run_id"].as_str().expect("batch index run id");
    let run_receipts = db
        .list_kernel_events_for_aggregate("knowledge_code_index_run", index_run_id)
        .await
        .expect("list MT-167 index run receipts");
    assert!(
        run_receipts
            .iter()
            .any(|event| event.payload["kind"] == json!("code_files_indexed_batch")),
        "{}: the route must take the single-transaction batch path",
        shape.name
    );
}

async fn assert_equal_rows(
    db: &SurrealDatabase,
    shape: &Shape,
    pass: usize,
    batch: (&str, &str),
    per_file: (&str, &str),
) -> BTreeMap<&'static str, usize> {
    {
        let batch_rows = code_index_rows(db, batch.0, batch.1).await;
        let per_file_rows = code_index_rows(db, per_file.0, per_file.1).await;
        let shape = &format!("{} pass {pass}", shape.name);
        println!(
            "MT167_EQUAL_COUNTS shape={} batch={:?} per_file={:?}",
            shape, batch_rows.totals, per_file_rows.totals
        );
        assert!(
            batch_rows.totals.values().all(|count| *count > 0),
            "{}: parity must compare non-empty row sets: {:?}",
            shape,
            batch_rows.totals
        );
        assert_eq!(batch_rows.totals, per_file_rows.totals, "{}: totals", shape);
        assert_eq!(
            batch_rows.entities, per_file_rows.entities,
            "{}: entities",
            shape
        );
        assert_eq!(batch_rows.spans, per_file_rows.spans, "{}: spans", shape);
        assert_eq!(
            batch_rows.entity_spans, per_file_rows.entity_spans,
            "{}: entity_spans",
            shape
        );
        assert_eq!(batch_rows.edges, per_file_rows.edges, "{}: edges", shape);
        assert_eq!(
            batch_rows.edge_spans, per_file_rows.edge_spans,
            "{}: edge_spans",
            shape
        );
        assert_eq!(
            batch_rows.code_files, per_file_rows.code_files,
            "{}: code_files",
            shape
        );
        batch_rows.totals
    }
}
