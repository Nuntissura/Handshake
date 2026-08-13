use crate::models::ErrorResponse;
use crate::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

use crate::api::account_scope::RequestAccountScope;

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

fn invalid_event() -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "HSK-400-INVALID-EVENT",
        }),
    )
}

fn db_error(err: impl std::fmt::Display) -> ApiError {
    tracing::error!(target: "handshake_core", error = %err, "flight_recorder_db_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "HSK-500-DB",
        }),
    )
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FlightEvent {
    pub event_id: String,
    pub trace_id: String,
    pub timestamp: String,
    pub actor: String,
    pub actor_id: String,
    pub event_type: String,
    pub job_id: Option<String>,
    pub workflow_id: Option<String>,
    pub model_id: Option<String>,
    pub model_session_id: Option<String>,
    pub wsids: Vec<String>,
    pub activity_span_id: Option<String>,
    pub session_span_id: Option<String>,
    pub capability_id: Option<String>,
    pub policy_decision_id: Option<String>,
    pub payload: Value,
}

#[derive(Deserialize, Debug, Default)]
pub struct EventFilter {
    pub event_id: Option<Uuid>,
    pub job_id: Option<String>,
    pub trace_id: Option<Uuid>,
    pub model_session_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub actor: Option<String>,
    pub surface: Option<String>,
    pub event_type: Option<String>,
    pub wsid: Option<String>,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/flight_recorder", get(list_events))
        .route("/events", get(list_events)) // backward-compatible path
        .route(
            "/flight_recorder/runtime_chat_event",
            post(record_runtime_chat_event),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeChatEventType {
    RuntimeChatMessageAppended,
    RuntimeChatAns001Validation,
    RuntimeChatSessionClosed,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeChatEventV0_1 {
    pub schema_version: String,
    pub event_id: String,
    pub ts_utc: String,
    pub session_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_packet_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsid: Option<String>,

    #[serde(rename = "type")]
    pub event_type: RuntimeChatEventType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_role: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ans001_sha256: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ans001_compliant: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violation_clauses: Option<Vec<String>>,
}

async fn record_runtime_chat_event(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Json(event): Json<RuntimeChatEventV0_1>,
) -> ApiResult<Json<Value>> {
    let trace_id = match Uuid::parse_str(event.session_id.trim()) {
        Ok(parsed) if parsed != Uuid::nil() => parsed,
        _ => return Err(invalid_event()),
    };

    let event_type = match event.event_type {
        RuntimeChatEventType::RuntimeChatMessageAppended => {
            crate::flight_recorder::FlightRecorderEventType::RuntimeChatMessageAppended
        }
        RuntimeChatEventType::RuntimeChatAns001Validation => {
            crate::flight_recorder::FlightRecorderEventType::RuntimeChatAns001Validation
        }
        RuntimeChatEventType::RuntimeChatSessionClosed => {
            crate::flight_recorder::FlightRecorderEventType::RuntimeChatSessionClosed
        }
    };

    let mut payload = match serde_json::to_value(&event) {
        Ok(value) => value,
        Err(err) => return Err(db_error(err)),
    };
    scope
        .exact()
        .stamp_json_object(&mut payload)
        .map_err(db_error)?;
    let mut fr_event = crate::flight_recorder::FlightRecorderEvent::new(
        event_type,
        crate::flight_recorder::FlightRecorderActor::System,
        trace_id,
        payload,
    )
    .with_actor_id("runtime_chat");

    if let Some(job_id) = event.job_id {
        fr_event = fr_event.with_job_id(job_id);
    }
    if let Some(wsid) = event.wsid {
        fr_event = fr_event.with_wsids(vec![wsid]);
    }

    state
        .flight_recorder
        .record_event(fr_event)
        .await
        .map_err(|e| match e {
            crate::flight_recorder::RecorderError::InvalidEvent(_) => invalid_event(),
            other => db_error(other),
        })?;

    Ok(Json(json!({ "ok": true })))
}

async fn list_events(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Query(filter): Query<EventFilter>,
) -> ApiResult<Json<Vec<FlightEvent>>> {
    let exact_scope = scope.exact().clone();
    let internal_filter = crate::flight_recorder::EventFilter {
        event_id: filter.event_id,
        job_id: filter.job_id,
        trace_id: filter.trace_id,
        model_session_id: filter.model_session_id,
        from: filter.from,
        to: filter.to,
        resource_scope: Some(scope.into_query()),
    };

    let mut events = state
        .flight_recorder
        .list_events(internal_filter)
        .await
        .map_err(db_error)?;

    events.retain(|event| {
        serde_json::from_value::<
            crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
        >(event.payload.clone())
        .is_ok_and(|stored| stored == exact_scope)
    });

    if let Some(actor) = filter.actor {
        events.retain(|e| e.actor.to_string() == actor);
    }
    if let Some(kind) = filter.event_type {
        events.retain(|e| e.event_type.to_string() == kind);
    }
    if let Some(wsid) = filter.wsid {
        events.retain(|e| e.wsids.contains(&wsid));
    }
    if let Some(surface) = filter.surface {
        let target = surface.as_str();
        let mut filtered = Vec::new();
        for event in events {
            let surface_match = match event.event_type {
                crate::flight_recorder::FlightRecorderEventType::Diagnostic => {
                    let diag_id = event
                        .payload
                        .get("diagnostic_id")
                        .and_then(|v| v.as_str())
                        .and_then(|raw| Uuid::parse_str(raw).ok());

                    match diag_id {
                        Some(id) => match state.diagnostics.get_diagnostic(id).await {
                            Ok(diag) => diag.surface.as_str() == target,
                            Err(_) => false,
                        },
                        None => false,
                    }
                }
                crate::flight_recorder::FlightRecorderEventType::TerminalCommand => {
                    target == "system" || target == "terminal"
                }
                crate::flight_recorder::FlightRecorderEventType::EditorEdit => {
                    if target == "system" {
                        true
                    } else {
                        matches!(
                            event.payload.get("editor_surface").and_then(|v| v.as_str()),
                            Some(surface) if surface == target
                        )
                    }
                }
                _ => target == "system",
            };

            if surface_match {
                filtered.push(event);
            }
        }
        events = filtered;
    }

    let api_events = events
        .into_iter()
        .map(|e| FlightEvent {
            event_id: e.event_id.to_string(),
            trace_id: e.trace_id.to_string(),
            timestamp: e.timestamp.to_rfc3339(),
            actor: e.actor.to_string(),
            actor_id: e.actor_id,
            event_type: e.event_type.to_string(),
            job_id: e.job_id,
            workflow_id: e.workflow_id,
            model_id: e.model_id,
            model_session_id: e.model_session_id,
            wsids: e.wsids,
            activity_span_id: e.activity_span_id,
            session_span_id: e.session_span_id,
            capability_id: e.capability_id,
            policy_decision_id: e.policy_decision_id,
            payload: e.payload,
        })
        .collect();

    Ok(Json(api_events))
}

#[cfg(all(test, feature = "duckdb-flight-recorder"))]
mod tests {
    use super::*;
    use crate::api::account_scope::ProductLocalResourceScope;
    use crate::api::operator_chat::{self, OperatorChatState};
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
    use crate::flight_recorder::{
        FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    };
    use crate::llm::{
        CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
    };
    use crate::storage::{tests::optional_postgres_backend_with_pool_from_env, Database};
    use crate::swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
        OwnerAccountId, WorkspaceScopeRef,
    };
    use crate::workflows::{SessionRegistry, SessionSchedulerConfig};
    use crate::AppState;
    use std::sync::Arc;
    use tower::ServiceExt;
    use uuid::Uuid;

    struct TestLlmClient {
        profile: ModelProfile,
    }

    impl TestLlmClient {
        fn new() -> Self {
            Self {
                profile: ModelProfile::new("flight-recorder-api-test".to_string(), 4096),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmClient for TestLlmClient {
        async fn completion(
            &self,
            _req: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            Ok(CompletionResponse {
                text: "ok".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                latency_ms: 0,
            })
        }

        fn profile(&self) -> &ModelProfile {
            &self.profile
        }
    }

    async fn setup_state() -> Result<Option<AppState>, Box<dyn std::error::Error>> {
        let Some(backend) = optional_postgres_backend_with_pool_from_env().await? else {
            return Ok(None);
        };

        let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(32)?);

        Ok(Some(AppState {
            storage: backend.database,
            postgres_pool: backend.postgres_pool,
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(TestLlmClient::new()),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        }))
    }

    #[tokio::test]
    async fn list_events_preserves_model_session_id_filter_and_payload(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state()
            .await?
            .expect("real PostgreSQL authority is required for exact event-route proof");
        let trace_id = Uuid::now_v7();
        let owner = OwnerAccountId::mint();
        let workspace = WorkspaceScopeRef::new(format!("fr-api-{}", Uuid::now_v7()))?;
        let attribution = ExactResourceScopeAttribution {
            owner_account_id: owner,
            actor_principal_id: ActorPrincipalId::mint(),
            authenticated_session_id: AuthenticatedSessionRef::mint(),
            access_space_id: AccessSpaceRef::mint(),
            workspace_id: workspace.clone(),
        };
        let mut scoped_payload = json!({
            "type": "system",
            "event_id": "FR-EVT-SYS-001",
        });
        attribution.stamp_json_object(&mut scoped_payload)?;

        state
            .flight_recorder
            .record_event(
                FlightRecorderEvent::new(
                    FlightRecorderEventType::System,
                    FlightRecorderActor::System,
                    trace_id,
                    scoped_payload,
                )
                .with_model_session_id("sess-keep"),
            )
            .await?;

        state
            .flight_recorder
            .record_event(FlightRecorderEvent::new(
                FlightRecorderEventType::System,
                FlightRecorderActor::System,
                trace_id,
                json!({
                    "type": "system",
                    "event_id": "FR-EVT-SYS-000",
                }),
            ))
            .await?;

        let response = list_events(
            State(state),
            RequestAccountScope::from_exact(attribution),
            Query(EventFilter {
                model_session_id: Some("sess-keep".to_string()),
                ..Default::default()
            }),
        )
        .await;
        let Json(events) = match response {
            Ok(payload) => payload,
            Err(_) => panic!("filtered flight recorder api response failed"),
        };

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model_session_id.as_deref(), Some("sess-keep"));
        assert_eq!(events[0].event_type, "system");

        Ok(())
    }

    fn exact_event_router(state: AppState, scope: Option<ExactResourceScopeAttribution>) -> Router {
        match scope {
            Some(exact) => crate::api::routes_with_product_local_scope(
                state,
                ProductLocalResourceScope::from_exact(exact).expect("valid exact server scope"),
            ),
            None => {
                let operator_state =
                    OperatorChatState::production().with_recorder(state.flight_recorder.clone());
                operator_chat::scoped_routes(operator_state).merge(routes(state))
            }
        }
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("bounded response body");
        serde_json::from_slice(&bytes).expect("JSON response")
    }

    #[tokio::test]
    async fn scoped_selection_and_runtime_chat_are_top_level_visible_only_to_exact_scope(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state()
            .await?
            .expect("real PostgreSQL authority is required for exact scoped event-route proof");
        let exact = ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::mint(),
            actor_principal_id: ActorPrincipalId::mint(),
            authenticated_session_id: AuthenticatedSessionRef::mint(),
            access_space_id: AccessSpaceRef::mint(),
            workspace_id: WorkspaceScopeRef::new(format!("scoped-event-{}", Uuid::now_v7()))?,
        };
        let missing = exact_event_router(state.clone(), None);
        let selection_body = json!({
            "selected_model_id": "runtime-test-model",
            "lane_kind": "local",
            "actor": "operator",
            "reason": "exact-scope route proof"
        });
        let chat_body = json!({
            "schema_version": "hsk.fr.runtime_chat@0.1",
            "event_id": "FR-EVT-RUNTIME-CHAT-101",
            "ts_utc": Utc::now().to_rfc3339(),
            "session_id": Uuid::now_v7().to_string(),
            "type": "runtime_chat_message_appended",
            "message_id": Uuid::now_v7().to_string(),
            "role": "assistant",
            "model_role": "frontend",
            "body_sha256": "00".repeat(32)
        });
        for (path, body) in [
            ("/operator-chat/selection", selection_body.clone()),
            ("/flight_recorder/runtime_chat_event", chat_body.clone()),
        ] {
            let response = missing
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("POST")
                        .uri(path)
                        .header("content-type", "application/json")
                        .body(axum::body::Body::from(body.to_string()))?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        }
        assert!(state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter::system_raw())
            .await?
            .is_empty());

        let scoped = exact_event_router(state.clone(), Some(exact.clone()));
        let mismatch = scoped
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/operator-chat/selection")
                    .header("content-type", "application/json")
                    .header(
                        "x-handshake-owner-account",
                        OwnerAccountId::mint().to_string(),
                    )
                    .body(axum::body::Body::from(selection_body.to_string()))?,
            )
            .await?;
        assert_eq!(mismatch.status(), StatusCode::FORBIDDEN);

        for (path, body) in [
            ("/operator-chat/selection", selection_body),
            ("/flight_recorder/runtime_chat_event", chat_body),
        ] {
            let response = scoped
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("POST")
                        .uri(path)
                        .header("content-type", "application/json")
                        .body(axum::body::Body::from(body.to_string()))?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
        }

        let visible = scoped
            .oneshot(
                axum::http::Request::builder()
                    .uri("/events")
                    .body(axum::body::Body::empty())?,
            )
            .await?;
        assert_eq!(visible.status(), StatusCode::OK);
        let visible = response_json(visible).await;
        let events = visible.as_array().expect("event array");
        assert_eq!(events.len(), 2);
        for event in events {
            let decoded: ExactResourceScopeAttribution =
                serde_json::from_value(event["payload"].clone())?;
            assert_eq!(
                decoded, exact,
                "scope must be top-level on final event payload"
            );
        }

        for foreign in [
            ExactResourceScopeAttribution {
                owner_account_id: OwnerAccountId::mint(),
                ..exact.clone()
            },
            ExactResourceScopeAttribution {
                actor_principal_id: ActorPrincipalId::mint(),
                ..exact.clone()
            },
            ExactResourceScopeAttribution {
                authenticated_session_id: AuthenticatedSessionRef::mint(),
                ..exact.clone()
            },
            ExactResourceScopeAttribution {
                access_space_id: AccessSpaceRef::mint(),
                ..exact.clone()
            },
            ExactResourceScopeAttribution {
                workspace_id: WorkspaceScopeRef::new("foreign-workspace")?,
                ..exact.clone()
            },
        ] {
            let hidden = exact_event_router(state.clone(), Some(foreign))
                .oneshot(
                    axum::http::Request::builder()
                        .uri("/events")
                        .body(axum::body::Body::empty())?,
                )
                .await?;
            assert_eq!(hidden.status(), StatusCode::OK);
            assert_eq!(response_json(hidden).await, json!([]));
        }

        Ok(())
    }
}
