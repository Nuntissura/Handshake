use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tempfile::tempdir;
use uuid::Uuid;

use handshake_core::ai_ready_data::chunking::{chunk_code_treesitter, sha256_hex, CodeLanguage};
use handshake_core::ai_ready_data::paths::ShadowWorkspacePaths;
use handshake_core::ai_ready_data::pipeline::{AiReadyDataPipeline, DocIngestSpec};
use handshake_core::ai_ready_data::retrieval::{HybridQuery, HybridRetrievalParams, HybridWeights};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    RecorderError,
};
use handshake_core::storage::tests::sqlite_backend;
use handshake_core::storage::{NewWorkspace, WriteContext};

#[derive(Clone, Default)]
struct InMemoryFlightRecorder {
    events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
}

#[async_trait]
impl FlightRecorder for InMemoryFlightRecorder {
    async fn record_event(&self, mut event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        event.normalize_payload();
        self.events.lock().expect("events lock").push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events.lock().expect("events lock").clone())
    }
}

fn json_contains_string(value: &serde_json::Value, needle: &str) -> bool {
    match value {
        serde_json::Value::String(s) => s == needle,
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| json_contains_string(value, needle)),
        serde_json::Value::Object(map) => map
            .values()
            .any(|value| json_contains_string(value, needle)),
        _ => false,
    }
}

#[test]
fn shadow_workspace_paths_do_not_duplicate_workspace_segment() {
    let paths = ShadowWorkspacePaths::new(std::path::PathBuf::from("handshake_root"), "ws-1");
    let bronze = paths.bronze_dir().to_string_lossy().replace('\\', "/");
    let silver = paths.silver_dir().to_string_lossy().replace('\\', "/");
    let gold_indexes = paths
        .gold_indexes_dir()
        .to_string_lossy()
        .replace('\\', "/");
    let gold_graph = paths.gold_graph_dir().to_string_lossy().replace('\\', "/");

    assert!(!bronze.contains("workspace/workspace"));
    assert!(!silver.contains("workspace/workspace"));
    assert!(!gold_indexes.contains("workspace/workspace"));
    assert!(!gold_graph.contains("workspace/workspace"));

    assert!(bronze.ends_with("data/workspaces/ws-1/workspace/raw"));
    assert!(silver.ends_with("data/workspaces/ws-1/workspace/derived"));
    assert!(gold_indexes.ends_with("data/workspaces/ws-1/workspace/indexes"));
    assert!(gold_graph.ends_with("data/workspaces/ws-1/workspace/graph"));
}

#[test]
fn treesitter_chunking_is_deterministic_for_rust_and_typescript() {
    let rust_source = r#"
use std::fmt;

pub fn greet() -> &'static str {
    "hello"
}

struct Thing {
    value: i32,
}
"#;

    let chunks_a = chunk_code_treesitter(
        "brz_test",
        rust_source,
        CodeLanguage::Rust,
        "ai_ready_pipeline_v1",
        "test_model",
        "v1",
    )
    .expect("rust treesitter chunking");
    let chunks_b = chunk_code_treesitter(
        "brz_test",
        rust_source,
        CodeLanguage::Rust,
        "ai_ready_pipeline_v1",
        "test_model",
        "v1",
    )
    .expect("rust treesitter chunking");

    assert_eq!(chunks_a.len(), chunks_b.len());
    for (left, right) in chunks_a.iter().zip(chunks_b.iter()) {
        assert_eq!(left.silver_id, right.silver_id);
        assert_eq!(left.content_hash, right.content_hash);
        assert_eq!(left.byte_start, right.byte_start);
        assert_eq!(left.byte_end, right.byte_end);
    }
    assert_eq!(chunks_a[0].byte_start, 0);

    let ts_source = r#"
export function add(a: number, b: number): number {
  return a + b;
}

export class Greeter {
  greet(): string {
    return "hi";
  }
}
"#;

    let ts_chunks_a = chunk_code_treesitter(
        "brz_test_ts",
        ts_source,
        CodeLanguage::TypeScript,
        "ai_ready_pipeline_v1",
        "test_model",
        "v1",
    )
    .expect("ts treesitter chunking");
    let ts_chunks_b = chunk_code_treesitter(
        "brz_test_ts",
        ts_source,
        CodeLanguage::TypeScript,
        "ai_ready_pipeline_v1",
        "test_model",
        "v1",
    )
    .expect("ts treesitter chunking");

    assert_eq!(ts_chunks_a.len(), ts_chunks_b.len());
    for (left, right) in ts_chunks_a.iter().zip(ts_chunks_b.iter()) {
        assert_eq!(left.silver_id, right.silver_id);
        assert_eq!(left.content_hash, right.content_hash);
    }
    assert_eq!(ts_chunks_a[0].byte_start, 0);
}

#[test]
fn treesitter_chunking_returns_error_on_parse_failure() {
    let bad_rust = "fn {";
    let err = chunk_code_treesitter(
        "brz_bad",
        bad_rust,
        CodeLanguage::Rust,
        "ai_ready_pipeline_v1",
        "test_model",
        "v1",
    )
    .expect_err("parse failure should return error");
    let msg = err.to_string();
    assert!(msg.contains("chunking failed"));
}

#[tokio::test]
async fn pipeline_emits_validation_failed_on_treesitter_parse_error_and_skips_silver() {
    let temp = tempdir().expect("tempdir");
    let handshake_root = temp.path().to_path_buf();

    let db = sqlite_backend().await.expect("sqlite backend");
    let ctx = WriteContext::human(None);
    let workspace = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("ws-{}", Uuid::new_v4()),
            },
        )
        .await
        .expect("create workspace");

    let bad_rel = "src/bad.rs";
    let bad_abs = handshake_root.join(bad_rel);
    std::fs::create_dir_all(bad_abs.parent().expect("parent")).expect("create src dir");
    std::fs::write(&bad_abs, b"fn {").expect("write bad source");

    let recorder = InMemoryFlightRecorder::default();
    let paths = ShadowWorkspacePaths::new(handshake_root.clone(), workspace.id.clone());
    let write_ctx = WriteContext::system(None);
    let pipeline = AiReadyDataPipeline::new(
        paths,
        db.as_ref(),
        &write_ctx,
        &recorder,
        Uuid::new_v4(),
        None,
        None,
    );

    pipeline
        .run_doc_ingest(DocIngestSpec {
            workspace_id: workspace.id.clone(),
            paths: vec![bad_rel.to_string()],
            embedding_model_id: "test_model".to_string(),
            embedding_model_version: "v1".to_string(),
            retrieval_query: None,
            golden_query: None,
        })
        .await
        .expect("pipeline ingest");

    let silver_records = db
        .list_ai_silver_records(&workspace.id)
        .await
        .expect("list silver");
    assert!(silver_records.is_empty());

    let events = recorder
        .list_events(EventFilter::default())
        .await
        .expect("list events");
    assert!(events.iter().any(|event| {
        event.event_type == FlightRecorderEventType::DataValidationFailed
            && event.payload.get("type").and_then(|v| v.as_str()) == Some("data_validation_failed")
    }));
}

#[tokio::test]
async fn pipeline_hashes_query_in_retrieval_events() {
    let temp = tempdir().expect("tempdir");
    let handshake_root = temp.path().to_path_buf();

    let db = sqlite_backend().await.expect("sqlite backend");
    let ctx = WriteContext::human(None);
    let workspace = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("ws-{}", Uuid::new_v4()),
            },
        )
        .await
        .expect("create workspace");

    let doc_rel = "docs/readme.md";
    let doc_abs = handshake_root.join(doc_rel);
    std::fs::create_dir_all(doc_abs.parent().expect("parent")).expect("create docs dir");
    std::fs::write(
        &doc_abs,
        b"# Title\n\nThis file mentions magic_word for retrieval.\n",
    )
    .expect("write doc");

    let recorder = InMemoryFlightRecorder::default();
    let paths = ShadowWorkspacePaths::new(handshake_root.clone(), workspace.id.clone());
    let write_ctx = WriteContext::system(None);
    let pipeline = AiReadyDataPipeline::new(
        paths,
        db.as_ref(),
        &write_ctx,
        &recorder,
        Uuid::new_v4(),
        None,
        None,
    );

    let query_text = "magic_word";
    pipeline
        .run_doc_ingest(DocIngestSpec {
            workspace_id: workspace.id.clone(),
            paths: vec![doc_rel.to_string()],
            embedding_model_id: "test_model".to_string(),
            embedding_model_version: "v1".to_string(),
            retrieval_query: Some(HybridQuery {
                query: query_text.to_string(),
                query_intent: "factual_lookup".to_string(),
                weights: HybridWeights {
                    vector: 0.5,
                    keyword: 0.5,
                    graph: 0.0,
                },
                retrieval: HybridRetrievalParams {
                    k: 5,
                    vector_candidates: 10,
                    keyword_candidates: 10,
                    graph_hops: 0,
                },
                rerank: false,
            }),
            golden_query: None,
        })
        .await
        .expect("pipeline ingest");

    let events = recorder
        .list_events(EventFilter::default())
        .await
        .expect("list events");

    let retrieval_event = events
        .iter()
        .find(|event| event.event_type == FlightRecorderEventType::DataRetrievalExecuted)
        .expect("data_retrieval_executed event");

    let query_hash = retrieval_event
        .payload
        .get("query_hash")
        .and_then(|v| v.as_str())
        .expect("query_hash present");
    assert!(retrieval_event.payload.get("query").is_none());

    let normalized = handshake_core::ace::normalize_query(query_text);
    let expected_hash = sha256_hex(normalized.as_bytes());
    assert_eq!(query_hash, expected_hash);

    assert!(!events
        .iter()
        .any(|event| json_contains_string(&event.payload, query_text)));
}

#[test]
fn data_retrieval_executed_rejects_null_rerank_ms() {
    let payload = serde_json::json!({
        "type": "data_retrieval_executed",
        "request_id": "req-1",
        "query_hash": sha256_hex(b"query"),
        "query_intent": "code_search",
        "weights": {
            "vector": 0.0,
            "keyword": 1.0,
            "graph": 0.0,
        },
        "results": {
            "vector_candidates": 0,
            "keyword_candidates": 1,
            "after_fusion": 1,
            "final_count": 1,
        },
        "latency": {
            "embedding_ms": 0,
            "vector_search_ms": 0,
            "keyword_search_ms": 0,
            "rerank_ms": serde_json::Value::Null,
            "total_ms": 0,
        },
        "reranking_used": false,
    });

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::DataRetrievalExecuted,
        FlightRecorderActor::System,
        Uuid::new_v4(),
        payload,
    );
    assert!(event.validate().is_err());
}

#[test]
fn data_relationship_extracted_rejects_null_confidence() {
    let payload = serde_json::json!({
        "type": "data_relationship_extracted",
        "relationship_type": "depends_on",
        "source_id": "src-1",
        "target_id": "tgt-1",
        "confidence": serde_json::Value::Null,
    });

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::DataRelationshipExtracted,
        FlightRecorderActor::System,
        Uuid::new_v4(),
        payload,
    );
    assert!(event.validate().is_err());
}
