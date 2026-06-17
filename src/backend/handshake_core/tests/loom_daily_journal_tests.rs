//! WP-KERNEL-009 MT-257 DailyNotesJournal route-level proof against REAL
//! PostgreSQL over a quiet loopback listener.

mod knowledge_pg_support;
#[allow(dead_code)]
mod user_manual_support;

use handshake_core::api;
use knowledge_pg_support::KnowledgePg;
use serde_json::{json, Value};
use sqlx::{Connection, Row};
use user_manual_support::{app_state_for, start_server};

struct ApiFixture {
    kpg: KnowledgePg,
    base: String,
    _server: tokio::task::JoinHandle<()>,
    http: reqwest::Client,
}

async fn fixture() -> Option<ApiFixture> {
    let kpg = knowledge_pg_support::knowledge_pg().await?;
    let state = app_state_for(&kpg.schema_url).await;
    let (base, server) = start_server(api::loom::routes(state)).await;
    Some(ApiFixture {
        kpg,
        base,
        _server: server,
        http: reqwest::Client::new(),
    })
}

async fn journal_row_count(kpg: &KnowledgePg, workspace_id: &str, journal_date: &str) -> i64 {
    let mut conn = kpg.raw_connection().await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM loom_blocks \
         WHERE workspace_id = $1 AND content_type = 'journal' AND journal_date = $2",
    )
    .bind(workspace_id)
    .bind(journal_date)
    .fetch_one(&mut conn)
    .await
    .expect("journal row count");
    conn.close().await.ok();
    count
}

async fn knowledge_bridge_count(kpg: &KnowledgePg, workspace_id: &str, block_id: &str) -> i64 {
    let mut conn = kpg.raw_connection().await;
    let row = sqlx::query(
        "SELECT COUNT(*)::BIGINT AS bridge_count \
         FROM loom_block_knowledge_bridge b \
         JOIN knowledge_entities e ON e.entity_id = b.entity_id \
         WHERE b.workspace_id = $1 \
           AND b.block_id = $2 \
           AND e.entity_kind = 'loom_block' \
           AND e.entity_key = $2",
    )
    .bind(workspace_id)
    .bind(block_id)
    .fetch_one(&mut conn)
    .await
    .expect("journal knowledge bridge count");
    conn.close().await.ok();
    row.get("bridge_count")
}

async fn open_journal(fx: &ApiFixture, workspace_id: &str, journal_date: &str) -> Value {
    fx.http
        .put(format!(
            "{}/workspaces/{}/loom/journals/{}",
            fx.base, workspace_id, journal_date
        ))
        .send()
        .await
        .expect("daily journal open")
        .error_for_status()
        .expect("daily journal open succeeds")
        .json()
        .await
        .expect("daily journal JSON")
}

async fn create_note(fx: &ApiFixture, workspace_id: &str, title: &str) -> Value {
    fx.http
        .post(format!(
            "{}/workspaces/{}/loom/blocks",
            fx.base, workspace_id
        ))
        .json(&json!({
            "content_type": "note",
            "title": title,
        }))
        .send()
        .await
        .expect("create note block")
        .error_for_status()
        .expect("create note block succeeds")
        .json()
        .await
        .expect("note block JSON")
}

async fn create_mention_edge(
    fx: &ApiFixture,
    workspace_id: &str,
    source_block_id: &str,
    target_block_id: &str,
) {
    fx.http
        .post(format!(
            "{}/workspaces/{}/loom/edges",
            fx.base, workspace_id
        ))
        .json(&json!({
            "source_block_id": source_block_id,
            "target_block_id": target_block_id,
            "edge_type": "mention",
            "created_by": "user",
        }))
        .send()
        .await
        .expect("create mention edge")
        .error_for_status()
        .expect("create mention edge succeeds");
}

#[tokio::test]
async fn daily_journal_open_is_idempotent_and_bridged() {
    let Some(fx) = fixture().await else {
        eprintln!("SKIP MT-257 daily journal proof: PostgreSQL unavailable");
        return;
    };
    let workspace_id = fx.kpg.create_workspace().await;
    let journal_date = "2026-06-16";
    let first = open_journal(&fx, &workspace_id, journal_date).await;
    let second = open_journal(&fx, &workspace_id, journal_date).await;

    assert_eq!(first["block_id"], second["block_id"]);
    assert_eq!(first["content_type"], "journal");
    assert_eq!(first["journal_date"], journal_date);
    assert_eq!(first["title"], "Daily Note 2026-06-16");
    assert_eq!(
        journal_row_count(&fx.kpg, &workspace_id, journal_date).await,
        1
    );

    let block_id = first["block_id"]
        .as_str()
        .expect("daily journal block id is a string");
    assert_eq!(
        knowledge_bridge_count(&fx.kpg, &workspace_id, block_id).await,
        1,
        "daily journal is resolved through the LoomBlock knowledge bridge"
    );
}

#[tokio::test]
async fn journal_date_filters_all_and_sorted_views() {
    let Some(fx) = fixture().await else {
        eprintln!("SKIP MT-257 journal date view proof: PostgreSQL unavailable");
        return;
    };
    let workspace_id = fx.kpg.create_workspace().await;

    let june15 = open_journal(&fx, &workspace_id, "2026-06-15").await;
    let june16 = open_journal(&fx, &workspace_id, "2026-06-16").await;
    let target = create_note(&fx, &workspace_id, "Roadmap").await;
    let target_id = target["block_id"].as_str().expect("target block id");
    let june15_id = june15["block_id"].as_str().expect("june 15 block id");
    let june16_id = june16["block_id"].as_str().expect("june 16 block id");
    create_mention_edge(&fx, &workspace_id, june15_id, target_id).await;
    create_mention_edge(&fx, &workspace_id, june16_id, target_id).await;

    let date_window =
        "content_type=journal&date_from=2026-06-15T00:00:00Z&date_to=2026-06-15T23:59:59Z";
    let all: Value = fx
        .http
        .get(format!(
            "{}/workspaces/{}/loom/views/all?{}",
            fx.base, workspace_id, date_window
        ))
        .send()
        .await
        .expect("all view request")
        .error_for_status()
        .expect("all view succeeds")
        .json()
        .await
        .expect("all view JSON");
    let all_ids: Vec<&str> = all["blocks"]
        .as_array()
        .expect("all view blocks")
        .iter()
        .map(|block| block["block_id"].as_str().expect("block id"))
        .collect();
    assert_eq!(all_ids, vec![june15_id]);
    assert!(
        !all_ids.contains(&june16_id),
        "All view date window is driven by journal_date, not updated_at"
    );

    let sorted: Value = fx
        .http
        .get(format!(
            "{}/workspaces/{}/loom/views/sorted?{}",
            fx.base, workspace_id, date_window
        ))
        .send()
        .await
        .expect("sorted view request")
        .error_for_status()
        .expect("sorted view succeeds")
        .json()
        .await
        .expect("sorted view JSON");
    let sorted_ids: Vec<&str> = sorted["groups"]
        .as_array()
        .expect("sorted groups")
        .iter()
        .flat_map(|group| group["blocks"].as_array().expect("group blocks"))
        .map(|block| block["block_id"].as_str().expect("block id"))
        .collect();
    assert_eq!(sorted_ids, vec![june15_id]);
}

#[tokio::test]
async fn journal_mentions_surface_as_backlinks() {
    let Some(fx) = fixture().await else {
        eprintln!("SKIP MT-257 journal backlink proof: PostgreSQL unavailable");
        return;
    };
    let workspace_id = fx.kpg.create_workspace().await;

    let journal = open_journal(&fx, &workspace_id, "2026-06-16").await;
    let target = create_note(&fx, &workspace_id, "Roadmap").await;
    let journal_id = journal["block_id"].as_str().expect("journal block id");
    let target_id = target["block_id"].as_str().expect("target block id");
    create_mention_edge(&fx, &workspace_id, journal_id, target_id).await;

    let backlinks: Value = fx
        .http
        .get(format!(
            "{}/workspaces/{}/loom/blocks/{}/backlinks",
            fx.base, workspace_id, target_id
        ))
        .send()
        .await
        .expect("backlinks request")
        .error_for_status()
        .expect("backlinks request succeeds")
        .json()
        .await
        .expect("backlinks JSON");
    let incoming = backlinks.as_array().expect("backlinks array");
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0]["source_block"]["block_id"], journal_id);
    assert_eq!(incoming[0]["source_block"]["content_type"], "journal");
    assert_eq!(incoming[0]["source_block"]["journal_date"], "2026-06-16");
}

#[tokio::test]
async fn daily_journal_rejects_non_canonical_dates() {
    let Some(fx) = fixture().await else {
        eprintln!("SKIP MT-257 journal date validation proof: PostgreSQL unavailable");
        return;
    };
    let workspace_id = fx.kpg.create_workspace().await;

    let response = fx
        .http
        .put(format!(
            "{}/workspaces/{}/loom/journals/2026-6-16",
            fx.base, workspace_id
        ))
        .send()
        .await
        .expect("invalid date request");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(
        journal_row_count(&fx.kpg, &workspace_id, "2026-06-16").await,
        0,
        "invalid date does not create a canonical journal row"
    );
}
