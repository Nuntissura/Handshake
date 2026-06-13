//! WP-KERNEL-009 MT-063 RollbackAndRepairMigrations.
//!
//! Two proof layers:
//!   1. Coverage audit (filesystem, always runs): EVERY `NNNN_knowledge*`
//!      migration ships a `.down.sql` twin — the audit found none missing in
//!      the PostgresEventLedgerCore range 0130-0142 (0143-0149 unused), and
//!      the test keeps future knowledge families honest.
//!   2. Scratch-schema apply -> rollback -> re-apply (REAL PostgreSQL): the
//!      full migration chain through 0142 plus the hardening band 0200-0209
//!      applies on a fresh schema, the WP-009 PostgresEventLedgerCore
//!      migrations roll back in reverse order leaving ZERO knowledge_* tables
//!      AND ZERO knowledge_* routines, and re-applying them restores the exact
//!      registry rows and routine set — the repair path after a failed or
//!      abandoned rollback.
//!
//! Routines coverage (MT-063 hardening): rollback completeness is asserted over
//! information_schema.routines as well as BASE TABLE, so a future `.down.sql`
//! that dropped its tables but forgot a `DROP FUNCTION` (orphaning a trigger
//! function) fails this test instead of passing green.
//!
//! Scope note: rollback covers this group's chain 0130..=0142 plus the
//! WP-KERNEL-009 hardening band 0200..=0209 (additive trigger guards that
//! depend on 0130-0142 tables; the hardening band rolls back first so its
//! trigger/function drop while their owning tables still exist). The CRDT
//! (0150..) and ingestion (0160..) families ship their own `.down.sql` files
//! and are out of this scratch chain's scope; in a full-chain database those
//! must roll back first because ingestion tables FK into `knowledge_sources`.
//! The scratch chain executes only committed files, so this test is
//! deterministic.

// Shared proof-path support; this binary only needs the URL resolution, so
// the unused helpers are expected here.
#[allow(dead_code)]
mod knowledge_pg_support;

use knowledge_pg_support::base_database_url;
use sqlx::{Connection, Executor};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn migrations_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

/// (version, path) for every non-down migration file with a numeric prefix.
fn versioned_migrations() -> Vec<(u32, PathBuf)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(migrations_dir()).expect("read migrations dir") {
        let path = entry.expect("dir entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".sql") || name.ends_with(".down.sql") {
            continue;
        }
        let Some(prefix) = name.split('_').next() else {
            continue;
        };
        let Ok(version) = prefix.parse::<u32>() else {
            continue;
        };
        out.push((version, path));
    }
    out.sort_by_key(|(version, _)| *version);
    out
}

#[test]
fn every_knowledge_migration_ships_a_down_file() {
    let mut checked = 0;
    for entry in fs::read_dir(migrations_dir()).expect("read migrations dir") {
        let path = entry.expect("dir entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".sql") || name.ends_with(".down.sql") {
            continue;
        }
        // Knowledge namespace only: NNNN_knowledge*.sql.
        let mut parts = name.splitn(2, '_');
        let numeric_prefix = parts
            .next()
            .map(|prefix| prefix.chars().all(|c| c.is_ascii_digit()) && prefix.len() == 4)
            .unwrap_or(false);
        let knowledge_named = parts
            .next()
            .map(|rest| rest.starts_with("knowledge"))
            .unwrap_or(false);
        if !(numeric_prefix && knowledge_named) {
            continue;
        }
        let down = path.with_file_name(name.replace(".sql", ".down.sql"));
        assert!(
            down.exists(),
            "knowledge migration {name} has no .down.sql twin (MT-063 rollback rule)"
        );
        let down_sql = fs::read_to_string(&down).expect("read down file");
        assert!(
            down_sql.to_ascii_lowercase().contains("drop"),
            "down file for {name} must actually roll the family back"
        );
        checked += 1;
    }
    assert!(
        checked >= 13,
        "expected at least the 13 PostgresEventLedgerCore knowledge migrations (0130-0142), found {checked}"
    );
}

/// All knowledge_* base tables in the given schema.
async fn knowledge_tables(conn: &mut sqlx::PgConnection, schema: &str) -> Vec<String> {
    sqlx::query_scalar(
        r#"
        SELECT table_name::text
        FROM information_schema.tables
        WHERE table_schema = $1
          AND table_type = 'BASE TABLE'
          AND table_name LIKE 'knowledge\_%' ESCAPE '\'
        ORDER BY table_name
        "#,
    )
    .bind(schema)
    .fetch_all(conn)
    .await
    .expect("list knowledge tables")
}

/// All knowledge_* routines (functions/procedures) in the given schema.
///
/// MT-063 hardening: rollback completeness was previously asserted only over
/// BASE TABLE, so a future `.down.sql` that forgot a `DROP FUNCTION` would pass
/// green while leaving an orphan trigger function behind. Trigger functions are
/// the most likely leftover (every commit-time/transition guard is one), so the
/// rollback test asserts this set is empty too.
async fn knowledge_routines(conn: &mut sqlx::PgConnection, schema: &str) -> Vec<String> {
    sqlx::query_scalar(
        r#"
        SELECT routine_name::text
        FROM information_schema.routines
        WHERE routine_schema = $1
          AND routine_name LIKE 'knowledge\_%' ESCAPE '\'
        ORDER BY routine_name
        "#,
    )
    .bind(schema)
    .fetch_all(conn)
    .await
    .expect("list knowledge routines")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scratch_schema_apply_rollback_reapply() {
    let Some(url) = base_database_url().await else {
        eprintln!("SKIP scratch_schema_apply_rollback_reapply: no PostgreSQL");
        return;
    };

    // Scratch schema with the same digest shims the proof-path support
    // installs (migrations call digest() unqualified through search_path).
    let schema = format!("knowledge_mig_{}", Uuid::now_v7().simple());
    let mut setup = sqlx::PgConnection::connect(&url)
        .await
        .expect("connect for schema setup");
    setup
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await
        .expect("create scratch schema");
    setup
        .execute("CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public")
        .await
        .expect("ensure pgcrypto");
    for shim in [
        format!(
            "CREATE OR REPLACE FUNCTION {schema}.digest(input text, algorithm text)
             RETURNS bytea LANGUAGE SQL IMMUTABLE PARALLEL SAFE
             AS $$ SELECT public.digest(input::bytea, algorithm) $$"
        ),
        format!(
            "CREATE OR REPLACE FUNCTION {schema}.digest(input bytea, algorithm text)
             RETURNS bytea LANGUAGE SQL IMMUTABLE PARALLEL SAFE
             AS $$ SELECT public.digest(input, algorithm) $$"
        ),
    ] {
        setup.execute(shim.as_str()).await.expect("install shim");
    }
    drop(setup);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");
    let mut conn = sqlx::PgConnection::connect(&schema_url)
        .await
        .expect("connect into scratch schema");

    // APPLY: the full base chain through 0142 (this group's core last
    // migration) PLUS WP-KERNEL-009 hardening migrations (additive trigger
    // guards that depend on the 0130-0142 tables), in version order, executing
    // the exact committed migration files. The hardening migrations sort last,
    // so version order already applies them after their 0137 dependency.
    let chain: Vec<(u32, PathBuf)> = versioned_migrations()
        .into_iter()
        .filter(|(version, _)| *version <= 142 || (200..=209).contains(version) || *version == 320)
        .collect();
    assert!(
        chain.iter().any(|(version, _)| *version == 142),
        "expected migration 0142 in the chain"
    );
    for (version, path) in &chain {
        let sql = fs::read_to_string(path).expect("read migration");
        conn.execute(sql.as_str())
            .await
            .unwrap_or_else(|err| panic!("apply migration {version:04} failed: {err}"));
    }

    let applied_tables = knowledge_tables(&mut conn, &schema).await;
    assert!(
        applied_tables.len() >= 14,
        "expected the WP-009 knowledge tables after apply, found {applied_tables:?}"
    );
    let registry_before: Vec<(String, String)> = sqlx::query_as(
        "SELECT family_key::text, table_name::text FROM knowledge_schema_registry
         ORDER BY family_key",
    )
    .fetch_all(&mut conn)
    .await
    .expect("registry before rollback");
    for (_, table_name) in &registry_before {
        assert!(
            applied_tables.contains(table_name),
            "registered table {table_name} missing after apply"
        );
    }

    // MT-063 routines coverage: the hardening band installs a trigger FUNCTION
    // (knowledge_claim_transition_guard), and 0130-0142 install the span guard
    // functions. They must all exist after apply so the post-rollback
    // empty-routines assertion is a real proof, not a vacuous one.
    let routines_before = knowledge_routines(&mut conn, &schema).await;
    assert!(
        routines_before
            .iter()
            .any(|r| r == "knowledge_claim_transition_guard"),
        "expected the MT-056 transition-guard function after apply, found {routines_before:?}"
    );
    assert!(
        routines_before
            .iter()
            .any(|r| r == "knowledge_claim_conflict_resolution_receipt_guard"),
        "expected the MT-231 conflict receipt-guard function after apply, found {routines_before:?}"
    );
    assert!(
        routines_before
            .iter()
            .any(|r| r == "knowledge_claim_conflict_unresolved_state_guard"),
        "expected the MT-231 conflict state-guard function after apply, found {routines_before:?}"
    );

    // Two-band rollback. The hardening migrations roll back FIRST so their
    // triggers + functions drop while knowledge_claims still exists, then the
    // base band 0142 -> 0130. Both bands' down files must leave ZERO
    // knowledge_* tables AND ZERO knowledge_* routines.
    let knowledge_chain: Vec<(u32, PathBuf)> = chain
        .iter()
        .filter(|(version, _)| (130..=142).contains(version))
        .cloned()
        .collect();
    assert_eq!(
        knowledge_chain.len(),
        13,
        "PostgresEventLedgerCore owns exactly 13 migrations in 0130..=0142"
    );
    let hardening_chain: Vec<(u32, PathBuf)> = chain
        .iter()
        .filter(|(version, _)| (200..=209).contains(version) || *version == 320)
        .cloned()
        .collect();
    assert!(
        hardening_chain.iter().any(|(version, _)| *version == 320),
        "expected the MT-231 hardening migration (0320) in the chain"
    );
    // Reverse-order rollback across BOTH bands: hardening first, then base.
    let rollback_order: Vec<(u32, PathBuf)> = hardening_chain
        .iter()
        .rev()
        .chain(knowledge_chain.iter().rev())
        .cloned()
        .collect();
    for (version, path) in &rollback_order {
        let name = path.file_name().and_then(|n| n.to_str()).expect("name");
        let down = path.with_file_name(name.replace(".sql", ".down.sql"));
        let sql = fs::read_to_string(&down).expect("read down migration");
        conn.execute(sql.as_str())
            .await
            .unwrap_or_else(|err| panic!("rollback migration {version:04} failed: {err}"));
    }
    let after_rollback = knowledge_tables(&mut conn, &schema).await;
    assert!(
        after_rollback.is_empty(),
        "rollback must drop every knowledge_* table, leftover: {after_rollback:?}"
    );
    // MT-063 hardening: routines must be clean too. A future down that forgot a
    // DROP FUNCTION would leave an orphan here and fail this assertion.
    let routines_after = knowledge_routines(&mut conn, &schema).await;
    assert!(
        routines_after.is_empty(),
        "rollback must drop every knowledge_* routine, leftover: {routines_after:?}"
    );

    // RE-APPLY (repair): the same committed files restore the family set across
    // both bands, in version order (base first, hardening last).
    let reapply_order: Vec<(u32, PathBuf)> = knowledge_chain
        .iter()
        .chain(hardening_chain.iter())
        .cloned()
        .collect();
    for (version, path) in &reapply_order {
        let sql = fs::read_to_string(path).expect("re-read migration");
        conn.execute(sql.as_str())
            .await
            .unwrap_or_else(|err| panic!("re-apply migration {version:04} failed: {err}"));
    }
    let reapplied_tables = knowledge_tables(&mut conn, &schema).await;
    assert_eq!(
        reapplied_tables, applied_tables,
        "re-apply must restore the exact knowledge table set"
    );
    let reapplied_routines = knowledge_routines(&mut conn, &schema).await;
    assert_eq!(
        reapplied_routines, routines_before,
        "re-apply must restore the exact knowledge routine set"
    );
    let registry_after: Vec<(String, String)> = sqlx::query_as(
        "SELECT family_key::text, table_name::text FROM knowledge_schema_registry
         ORDER BY family_key",
    )
    .fetch_all(&mut conn)
    .await
    .expect("registry after re-apply");
    assert_eq!(
        registry_after, registry_before,
        "re-apply must restore the exact schema registry rows"
    );

    // Cleanup: the scratch schema is disposable by design.
    conn.execute(format!("DROP SCHEMA {schema} CASCADE").as_str())
        .await
        .expect("drop scratch schema");
}
