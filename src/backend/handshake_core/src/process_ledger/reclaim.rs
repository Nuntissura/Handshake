use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    str::FromStr,
    sync::{Arc, OnceLock},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgConnectOptions, PgConnection, PgPool, Row};
use thiserror::Error;
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use uuid::Uuid;

use crate::managed_postgres::ProvenLocalPostgresEndpoint;

use super::{
    LedgerEventKind, PostgresProcessLedgerStore, ProcessEngineKind, ProcessLedgerError,
    ProcessLedgerWriter, ProcessRuntimeOwner, ProcessStop, ReservedProcessStop,
};

/// MT-008: atomically claim the active (un-stopped) rows for a session.
///
/// The previous form was `SELECT ... FOR UPDATE` executed in a transaction that
/// was committed immediately, releasing the row locks *before* the reclaim
/// decision (sandbox kill + stop-event write) ran. Two concurrent reclaims could
/// therefore both read the same active rows and both act on them, double-killing
/// a process and writing duplicate STOP rows (or, under interleaving, missing a
/// row).
///
/// The atomic update writes a time-bounded reclaim claim while deliberately
/// leaving `stopped_at` NULL. Only a proven successful kill may append STOP.
/// Concurrent reclaimers skip a fresh claim, abandoned claims become eligible
/// after 30 seconds, and a failed kill releases its claim immediately so the
/// process remains truthfully open and retryable.
pub const POSTGRES_ACTIVE_RECLAIM_QUERY_SQL: &str = r#"
WITH locked AS (
    SELECT process_uuid, stop_reason, metadata_jsonb
    FROM kernel_process_lifecycle
    WHERE parent_session_id = $1
      AND stopped_at IS NULL
      AND (
          stop_reason = 'kill_succeeded_pending_stop'
          OR stop_reason NOT IN ('reclaim_claimed', 'reclaim_kill_in_progress')
          OR stop_reason IS NULL
          OR (
            stop_reason = 'reclaim_claimed'
            AND CASE
              WHEN jsonb_typeof(metadata_jsonb->'reclaim_claim'->'lease_expires_at_unix_ms') = 'number'
              THEN (metadata_jsonb->'reclaim_claim'->>'lease_expires_at_unix_ms')::numeric
              ELSE 0::numeric
            END < (extract(epoch FROM clock_timestamp()) * 1000)::numeric
          )
      )
    FOR UPDATE
)
UPDATE kernel_process_lifecycle AS k
SET stop_reason = CASE
        WHEN locked.stop_reason = 'kill_succeeded_pending_stop'
        THEN 'kill_succeeded_pending_stop'
        ELSE 'reclaim_claimed'
    END,
    metadata_jsonb = jsonb_set(
        COALESCE(k.metadata_jsonb, '{}'::jsonb),
        '{reclaim_claim}',
        jsonb_build_object(
            'claimant_uuid', $2,
            'kill_operation_uuid', COALESCE(
                locked.metadata_jsonb->'reclaim_claim'->>'kill_operation_uuid',
                locked.metadata_jsonb->'reclaim_last_kill_operation'->>'kill_operation_uuid',
                $2
            ),
            'generation', CASE
                WHEN jsonb_typeof(locked.metadata_jsonb->'reclaim_claim'->'generation') = 'number'
                THEN (locked.metadata_jsonb->'reclaim_claim'->>'generation')::bigint + 1
                ELSE 1::bigint
            END,
            'claimed_at_unix_ms', (extract(epoch FROM clock_timestamp()) * 1000)::bigint,
            'lease_expires_at_unix_ms',
                (extract(epoch FROM clock_timestamp()) * 1000)::bigint + 30000::bigint
        ),
        true
    )
FROM locked
WHERE k.process_uuid = locked.process_uuid
  AND k.stopped_at IS NULL
RETURNING
    k.process_uuid::text AS process_uuid,
    k.os_pid,
    k.parent_session_id,
    k.parent_process_id::text AS parent_process_id,
    k.sandbox_adapter_id,
    k.sandbox_internal_id,
    k.engine_kind,
    k.started_at,
    k.model_artifact_sha256,
    k.work_profile_id,
    k.owner_role,
    k.owner_wp,
    k.role_id,
    k.wp_id,
    k.mt_id,
    k.owner_runtime_instance_id::text AS owner_runtime_instance_id,
    k.owner_host_scope_id,
    k.owner_lease_schema_id,
    k.owner_lease_protocol,
    k.owner_lease_address,
    k.owner_lease_port,
    k.stop_reason,
    k.sandbox_capabilities_snapshot::text AS sandbox_capabilities_snapshot,
    k.metadata_jsonb::text AS metadata_jsonb
"#;

pub const EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID: &str = "hsk.embedded_runtime.instance@2";
pub const EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL: &str =
    "hsk.embedded_runtime.loopback_udp_exclusive@1";
pub const HANDSHAKE_HOST_SCOPE_ID_ENV: &str = "HANDSHAKE_HOST_SCOPE_ID";
// Keep boot reconciliation bounded on slower managed-PostgreSQL hosts. The
// cursor makes the sweep cyclic, so later boots continue from the next
// instance instead of sacrificing liveness to one oversized startup batch.
pub const PIDLESS_RECLAIM_INSTANCE_CAP: usize = 16;
pub const EMBEDDED_RUNTIME_MANAGED_LOCAL_HOST_SCOPE_V2_PREFIX: &str = "local-pg-v2-sha256:";

const EMBEDDED_RUNTIME_LEGACY_LOCAL_HOST_SCOPE_PREFIX: &str = "local-pg-sha256:";

const RUNTIME_INSTANCE_SCHEMA_KEY: &str = "runtime_instance_schema_id";
const RUNTIME_INSTANCE_ID_KEY: &str = "runtime_instance_id";
const RUNTIME_HOST_SCOPE_ID_KEY: &str = "runtime_host_scope_id";
const RUNTIME_LEASE_PROTOCOL_KEY: &str = "runtime_lease_protocol";
const RUNTIME_LEASE_ADDRESS_KEY: &str = "runtime_lease_address";
const RUNTIME_LEASE_PORT_KEY: &str = "runtime_lease_port";
const PIDLESS_RECLAIM_LOCK_TIMEOUT: &str = "1500ms";
const PIDLESS_RECLAIM_STATEMENT_TIMEOUT: &str = "2500ms";
const PIDLESS_RECLAIM_AUTHORITY_STATEMENT_TIMEOUT: &str = "15000ms";

/// Non-optional process-liveness evidence carried by every pid-less embedded
/// runtime START row.
///
/// The UDP socket is deliberately loopback-only and never carries traffic. Its
/// exclusive OS bind is the lease: process crash releases the port, while a
/// PostgreSQL restart cannot. `host_scope_id` prevents a reclaimer connected to
/// a shared remote database from treating its own loopback namespace as another
/// host's namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddedRuntimeInstanceDescriptor {
    #[serde(rename = "runtime_instance_id")]
    pub instance_id: Uuid,
    #[serde(rename = "runtime_host_scope_id")]
    pub host_scope_id: String,
    #[serde(rename = "runtime_lease_protocol")]
    pub lease_protocol: String,
    #[serde(rename = "runtime_lease_address")]
    pub loopback_address: IpAddr,
    #[serde(rename = "runtime_lease_port")]
    pub loopback_port: u16,
}

impl EmbeddedRuntimeInstanceDescriptor {
    pub fn metadata_fields(&self) -> Value {
        serde_json::json!({
            "runtime_instance_schema_id": EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID,
            "runtime_instance_id": self.instance_id.to_string(),
            "runtime_host_scope_id": self.host_scope_id.clone(),
            "runtime_lease_protocol": self.lease_protocol.clone(),
            "runtime_lease_address": self.loopback_address.to_string(),
            "runtime_lease_port": self.loopback_port,
        })
    }

    pub fn process_runtime_owner(&self) -> super::ProcessRuntimeOwner {
        super::ProcessRuntimeOwner {
            runtime_instance_id: self.instance_id,
            host_scope_id: self.host_scope_id.clone(),
            lease_schema_id: EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID.to_string(),
            lease_protocol: self.lease_protocol.clone(),
            lease_address: self.loopback_address.to_string(),
            lease_port: self.loopback_port,
        }
    }
}

/// An OS-owned liveness lease for one Handshake backend instance.
///
/// Keeping `_socket` alive keeps the exact loopback endpoint exclusively bound.
/// No database connection is held, so database restart/session loss cannot be
/// confused with process death and the application pool loses no capacity.
pub struct EmbeddedRuntimeInstanceLease {
    descriptor: EmbeddedRuntimeInstanceDescriptor,
    _socket: UdpSocket,
}

impl EmbeddedRuntimeInstanceLease {
    pub fn instance_id(&self) -> Uuid {
        self.descriptor.instance_id
    }

    pub fn descriptor(&self) -> &EmbeddedRuntimeInstanceDescriptor {
        &self.descriptor
    }

    pub async fn release(self) -> Result<(), ProcessLedgerError> {
        drop(self);
        Ok(())
    }
}

/// Resolve the host scope used to prevent cross-host loopback false positives.
///
/// An explicit host id always wins. Automatic derivation is deliberately
/// unavailable from a URL alone because localhost may be an SSH/container/WSL
/// tunnel. The production entrypoint may supply provenance only through an
/// opaque proof issued by `ManagedPostgres` for its configured local cluster.
pub fn resolve_embedded_runtime_host_scope(
    database_url: &str,
) -> Result<String, ProcessLedgerError> {
    let explicit = std::env::var(HANDSHAKE_HOST_SCOPE_ID_ENV).ok();
    resolve_embedded_runtime_host_scope_with_managed_local(database_url, explicit.as_deref(), None)
}

/// Deterministic host-scope resolver used by the production environment seam
/// and by tests that must not mutate process-global environment variables.
pub fn resolve_embedded_runtime_host_scope_with_override(
    database_url: &str,
    explicit_host_scope: Option<&str>,
) -> Result<String, ProcessLedgerError> {
    resolve_embedded_runtime_host_scope_with_managed_local(database_url, explicit_host_scope, None)
}

/// Resolve host scope with an optional proven-local managed PostgreSQL endpoint.
/// Adopted and newly started local endpoints can carry proof; external
/// endpoints remain untrusted even when their URL names localhost. The proof
/// token cannot be constructed from a URL: only `ManagedPostgres` returns it
/// after validating the SQL endpoint's data_directory/system_identifier,
/// pg_ctl status, postmaster.pid, and the configured port. Adoption after a
/// Handshake crash is therefore trusted without granting shutdown ownership.
pub fn resolve_embedded_runtime_host_scope_with_managed_local(
    database_url: &str,
    explicit_host_scope: Option<&str>,
    proven_local_endpoint: Option<&ProvenLocalPostgresEndpoint>,
) -> Result<String, ProcessLedgerError> {
    let options = PgConnectOptions::from_str(database_url).map_err(|error| {
        ProcessLedgerError::InvalidConfig(format!(
            "invalid PostgreSQL DATABASE_URL for embedded runtime host scope: {error}"
        ))
    })?;
    let explicit = explicit_host_scope
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(explicit) = explicit {
        if explicit.len() > 256 {
            return Err(ProcessLedgerError::InvalidConfig(format!(
                "{HANDSHAKE_HOST_SCOPE_ID_ENV} exceeds 256 bytes"
            )));
        }
        return Ok(explicit.to_string());
    }

    let host = options.get_host().trim();
    if postgres_host_is_loopback(host) {
        let proof = proven_local_endpoint.ok_or_else(|| {
                ProcessLedgerError::InvalidConfig(format!(
                    "unproven loopback PostgreSQL requires explicit {HANDSHAKE_HOST_SCOPE_ID_ENV}; automatic scope is allowed only for a ManagedPostgres endpoint proven from its local data directory"
                ))
            })?;
        let trusted_options = PgConnectOptions::from_str(proof.database_url()).map_err(|error| {
            ProcessLedgerError::InvalidConfig(format!(
                "invalid proven-local managed PostgreSQL URL for embedded runtime host scope: {error}"
            ))
        })?;
        let trusted_host = trusted_options.get_host().trim();
        let database = options.get_database().unwrap_or("postgres");
        let trusted_database = trusted_options.get_database().unwrap_or("postgres");
        if !postgres_host_is_loopback(trusted_host)
            || proof.port() != trusted_options.get_port()
            || options.get_port() != trusted_options.get_port()
            || database != trusted_database
        {
            return Err(ProcessLedgerError::InvalidConfig(format!(
                "DATABASE_URL does not match the proven local managed PostgreSQL endpoint; set explicit {HANDSHAKE_HOST_SCOPE_ID_ENV}"
            )));
        }
        let material = format!(
            "hsk.embedded_runtime.host_scope@2\0proven-local-managed-postgres-system-identity\0{}\0{}",
            proof.system_identifier(),
            database
        );
        let digest = Sha256::digest(material.as_bytes());
        return Ok(format!(
            "{EMBEDDED_RUNTIME_MANAGED_LOCAL_HOST_SCOPE_V2_PREFIX}{}",
            hex::encode(digest)
        ));
    }

    Err(ProcessLedgerError::InvalidConfig(format!(
        "non-loopback PostgreSQL host {host:?} requires explicit {HANDSHAKE_HOST_SCOPE_ID_ENV}"
    )))
}

fn postgres_host_is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// Re-bind an opaque managed-local proof to the PostgreSQL endpoint currently
/// serving a live control-plane pool.
///
/// `ensure_running` proves the endpoint before storage initialization, but an
/// endpoint can be replaced between that proof and crash reclaim or lease
/// acquisition. Callers use this check immediately before those authority
/// actions. PostgreSQL exposes the unsigned control-system identifier as a
/// signed BIGINT, so the i64 value is reinterpreted as its full u64 bit pattern.
pub async fn verify_proven_local_postgres_endpoint_pool(
    pool: &PgPool,
    proof: &ProvenLocalPostgresEndpoint,
) -> Result<(), ProcessLedgerError> {
    let actual_signed: i64 = sqlx::query_scalar(
        "SELECT control.system_identifier FROM pg_catalog.pg_control_system() AS control",
    )
    .fetch_one(pool)
    .await?;
    let actual = actual_signed as u64;
    let expected = proof.system_identifier().parse::<u64>().map_err(|error| {
        ProcessLedgerError::InvalidConfig(format!(
            "managed PostgreSQL proof contains an invalid system_identifier: {error}"
        ))
    })?;
    if actual != expected {
        return Err(ProcessLedgerError::InvalidConfig(format!(
            "control-plane PostgreSQL system_identifier {actual} does not match proven managed endpoint {expected}"
        )));
    }
    Ok(())
}

/// Acquire the process-lifetime OS lease before any model artifact is opened.
/// This is synchronous so callers cannot accidentally spawn work between lease
/// selection and ownership. The second-bind self-test fails closed on a platform
/// whose ordinary UDP bind semantics do not provide exclusivity.
pub fn acquire_embedded_runtime_instance_lease(
    instance_id: Uuid,
    host_scope_id: impl Into<String>,
) -> Result<EmbeddedRuntimeInstanceLease, ProcessLedgerError> {
    let host_scope_id = host_scope_id.into();
    if host_scope_id.trim().is_empty() {
        return Err(ProcessLedgerError::InvalidConfig(
            "embedded runtime host_scope_id must not be empty".to_string(),
        ));
    }
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|error| {
        ProcessLedgerError::Store(format!(
            "failed to bind embedded runtime loopback UDP lease: {error}"
        ))
    })?;
    let address = socket.local_addr().map_err(|error| {
        ProcessLedgerError::Store(format!(
            "failed to inspect embedded runtime loopback UDP lease: {error}"
        ))
    })?;
    verify_second_udp_bind_is_rejected(address)?;
    Ok(EmbeddedRuntimeInstanceLease {
        descriptor: EmbeddedRuntimeInstanceDescriptor {
            instance_id,
            host_scope_id,
            lease_protocol: EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL.to_string(),
            loopback_address: address.ip(),
            loopback_port: address.port(),
        },
        _socket: socket,
    })
}

fn verify_second_udp_bind_is_rejected(address: SocketAddr) -> Result<(), ProcessLedgerError> {
    match UdpSocket::bind(address) {
        Ok(second) => {
            drop(second);
            Err(ProcessLedgerError::Store(format!(
                "loopback UDP lease endpoint {address} accepted a second ordinary bind"
            )))
        }
        // The platform-specific error kind differs (notably AddrInUse vs
        // PermissionDenied on Windows). The invariant is simply that the exact
        // second ordinary bind was rejected after the first succeeded.
        Err(_) => Ok(()),
    }
}

#[derive(Debug)]
enum UdpLeaseClaim {
    Claimed(UdpSocket),
    Protected,
    Ambiguous(io::Error),
}

fn try_claim_udp_lease(descriptor: &EmbeddedRuntimeInstanceDescriptor) -> UdpLeaseClaim {
    let address = SocketAddr::new(descriptor.loopback_address, descriptor.loopback_port);
    match UdpSocket::bind(address) {
        Ok(socket) => match UdpSocket::bind(address) {
            Ok(second) => {
                drop(second);
                drop(socket);
                UdpLeaseClaim::Ambiguous(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "claimed UDP endpoint did not enforce exclusive second-bind rejection",
                ))
            }
            // As in acquisition, the platform-specific rejection kind is not
            // stable. The first exact bind succeeded and the second exact bind
            // failed, which is the OS-ownership invariant we need.
            Err(_) => UdpLeaseClaim::Claimed(socket),
        },
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => UdpLeaseClaim::Protected,
        Err(error) => UdpLeaseClaim::Ambiguous(error),
    }
}

#[derive(Debug, Clone)]
pub(super) struct ProcessLedgerAuthorityRelation {
    pub(super) schema: String,
    pub(super) qualified_table: String,
    pub(super) relation_oid: i64,
}

#[derive(Debug, Clone)]
struct PidlessReclaimCursorAuthorityRelation {
    schema: String,
    qualified_table: String,
    relation_oid: i64,
}

const PROCESS_RUNTIME_OWNER_DESCRIPTOR_GUARD_BODY: &str = r#"
BEGIN
    IF NEW.owner_runtime_instance_id IS NULL THEN
        RETURN NEW;
    END IF;

    IF TG_OP = 'UPDATE'
       AND OLD.owner_runtime_instance_id IS NOT NULL
       AND (
           OLD.owner_runtime_instance_id IS DISTINCT FROM NEW.owner_runtime_instance_id
           OR OLD.owner_host_scope_id IS DISTINCT FROM NEW.owner_host_scope_id
           OR OLD.owner_lease_schema_id IS DISTINCT FROM NEW.owner_lease_schema_id
           OR OLD.owner_lease_protocol IS DISTINCT FROM NEW.owner_lease_protocol
           OR OLD.owner_lease_address IS DISTINCT FROM NEW.owner_lease_address
           OR OLD.owner_lease_port IS DISTINCT FROM NEW.owner_lease_port
       ) THEN
        RAISE EXCEPTION
            'typed runtime-owner descriptor is immutable for process %',
            NEW.process_uuid
            USING ERRCODE = '23514';
    END IF;

    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(NEW.owner_runtime_instance_id::text, 359)
    );

    IF EXISTS (
        SELECT 1
        FROM kernel_process_lifecycle AS existing
        WHERE existing.owner_runtime_instance_id = NEW.owner_runtime_instance_id
          AND existing.process_uuid <> NEW.process_uuid
          AND (
              existing.owner_host_scope_id IS DISTINCT FROM NEW.owner_host_scope_id
              OR existing.owner_lease_schema_id IS DISTINCT FROM NEW.owner_lease_schema_id
              OR existing.owner_lease_protocol IS DISTINCT FROM NEW.owner_lease_protocol
              OR existing.owner_lease_address IS DISTINCT FROM NEW.owner_lease_address
              OR existing.owner_lease_port IS DISTINCT FROM NEW.owner_lease_port
          )
    ) THEN
        RAISE EXCEPTION
            'runtime instance % already has a different typed lease descriptor',
            NEW.owner_runtime_instance_id
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
"#;

/// Migration 0359 deliberately installs one enabled user trigger on the
/// lifecycle authority. Validate its complete catalog contract and PL/pgSQL
/// body instead of treating every user trigger as disqualifying.
async fn process_runtime_owner_descriptor_guard_matches(
    connection: &mut PgConnection,
    relation_oid: i64,
) -> Result<bool, ProcessLedgerError> {
    let valid: bool = sqlx::query_scalar(
        r#"
        SELECT
            pg_catalog.count(*) = 1
            AND pg_catalog.bool_and(
                trigger_row.tgname = 'trg_kernel_process_runtime_owner_descriptor_guard'
                AND trigger_row.tgenabled = 'O'
                AND trigger_row.tgtype = 23
                AND trigger_row.tgnargs = 0
                AND trigger_row.tgargs = '\x'::pg_catalog.bytea
                AND trigger_row.tgqual IS NULL
                AND trigger_row.tgconstraint = 0
                AND NOT trigger_row.tgdeferrable
                AND NOT trigger_row.tginitdeferred
                AND procedure_row.proname = 'kernel_process_runtime_owner_descriptor_guard'
                AND procedure_row.pronargs = 0
                AND procedure_row.prorettype = 'pg_catalog.trigger'::pg_catalog.regtype
                AND procedure_row.prokind = 'f'
                AND procedure_row.provolatile = 'v'
                AND NOT procedure_row.prosecdef
                AND NOT procedure_row.proleakproof
                AND NOT procedure_row.proisstrict
                AND procedure_row.proparallel = 'u'
                AND procedure_row.proconfig IS NULL
                AND language_row.lanname = 'plpgsql'
                AND procedure_row.prosrc = $2
                AND ARRAY(
                    SELECT attribute_row.attname::pg_catalog.text
                    FROM pg_catalog.unnest(trigger_row.tgattr::pg_catalog.int2[])
                        WITH ORDINALITY AS trigger_column(attnum, ordinality)
                    JOIN pg_catalog.pg_attribute AS attribute_row
                      ON attribute_row.attrelid = trigger_row.tgrelid
                     AND attribute_row.attnum = trigger_column.attnum
                    ORDER BY trigger_column.ordinality
                ) = ARRAY[
                    'owner_runtime_instance_id',
                    'owner_host_scope_id',
                    'owner_lease_schema_id',
                    'owner_lease_protocol',
                    'owner_lease_address',
                    'owner_lease_port'
                ]::pg_catalog.text[]
            )
        FROM pg_catalog.pg_trigger AS trigger_row
        JOIN pg_catalog.pg_proc AS procedure_row ON procedure_row.oid = trigger_row.tgfoid
        JOIN pg_catalog.pg_language AS language_row ON language_row.oid = procedure_row.prolang
        WHERE trigger_row.tgrelid::pg_catalog.int8 = $1
          AND NOT trigger_row.tgisinternal
          AND trigger_row.tgenabled <> 'D'
        "#,
    )
    .bind(relation_oid)
    .bind(PROCESS_RUNTIME_OWNER_DESCRIPTOR_GUARD_BODY)
    .fetch_one(&mut *connection)
    .await?;
    Ok(valid)
}

/// Resolve the one lifecycle table in the explicit schema search path whose
/// shape matches migration 0021. Incomplete shadows are ignored; two complete
/// candidates fail closed instead of selecting by search-path order.
pub(super) async fn resolve_process_ledger_authority_relation(
    pool: &PgPool,
) -> Result<ProcessLedgerAuthorityRelation, ProcessLedgerError> {
    let mut connection = pool.acquire().await?;
    resolve_process_ledger_authority_relation_on_connection(&mut connection).await
}

async fn resolve_process_ledger_authority_relation_on_connection(
    connection: &mut PgConnection,
) -> Result<ProcessLedgerAuthorityRelation, ProcessLedgerError> {
    let rows = sqlx::query(
        r#"
        SELECT
            CAST(n.nspname AS pg_catalog.text) AS schema_name,
            CAST(c.oid AS pg_catalog.int8) AS relation_oid,
            CAST(a.attname AS pg_catalog.text) AS column_name,
            CAST(a.atttypid AS pg_catalog.int8) AS type_oid,
            a.attnotnull AS not_null,
            CAST(a.attgenerated AS pg_catalog.text) AS generated_kind
        FROM pg_catalog.pg_class AS c
        JOIN pg_catalog.pg_namespace AS n ON n.oid = c.relnamespace
        JOIN pg_catalog.pg_attribute AS a ON a.attrelid = c.oid
        JOIN pg_catalog.pg_type AS t ON t.oid = a.atttypid
        WHERE c.relname = 'kernel_process_lifecycle'
          AND c.relkind = 'r'
          AND c.relpersistence = 'p'
          AND NOT c.relrowsecurity
          AND NOT c.relforcerowsecurity
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_policy AS policy
              WHERE policy.polrelid = c.oid
          )
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_rewrite AS rewrite
              WHERE rewrite.ev_class = c.oid
          )
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_inherits AS inheritance
              WHERE inheritance.inhparent = c.oid OR inheritance.inhrelid = c.oid
          )
          AND n.nspname = ANY(pg_catalog.current_schemas(false))
          AND a.attnum > 0
          AND NOT a.attisdropped
        ORDER BY n.nspname, c.oid, a.attnum
        "#,
    )
    .fetch_all(&mut *connection)
    .await?;

    let mut relations: BTreeMap<(String, i64), HashMap<String, (i64, bool, String)>> =
        BTreeMap::new();
    for row in rows {
        let schema: String = row.try_get("schema_name")?;
        let oid: i64 = row.try_get("relation_oid")?;
        let column: String = row.try_get("column_name")?;
        let type_oid: i64 = row.try_get("type_oid")?;
        let not_null: bool = row.try_get("not_null")?;
        let generated_kind: String = row.try_get("generated_kind")?;
        relations
            .entry((schema, oid))
            .or_default()
            .insert(column, (type_oid, not_null, generated_kind));
    }

    let type_oids = sqlx::query(
        r#"
        SELECT 'pg_catalog.uuid'::pg_catalog.regtype::pg_catalog.oid::pg_catalog.int8 AS uuid_oid,
               'pg_catalog.int8'::pg_catalog.regtype::pg_catalog.oid::pg_catalog.int8 AS int8_oid,
               'pg_catalog.int4'::pg_catalog.regtype::pg_catalog.oid::pg_catalog.int8 AS int4_oid,
               'pg_catalog.text'::pg_catalog.regtype::pg_catalog.oid::pg_catalog.int8 AS text_oid,
               'pg_catalog.timestamptz'::pg_catalog.regtype::pg_catalog.oid::pg_catalog.int8 AS timestamptz_oid,
               'pg_catalog.jsonb'::pg_catalog.regtype::pg_catalog.oid::pg_catalog.int8 AS jsonb_oid
        "#,
    )
    .fetch_one(&mut *connection)
    .await?;
    let uuid_oid: i64 = type_oids.try_get("uuid_oid")?;
    let int8_oid: i64 = type_oids.try_get("int8_oid")?;
    let int4_oid: i64 = type_oids.try_get("int4_oid")?;
    let text_oid: i64 = type_oids.try_get("text_oid")?;
    let timestamptz_oid: i64 = type_oids.try_get("timestamptz_oid")?;
    let jsonb_oid: i64 = type_oids.try_get("jsonb_oid")?;
    let expected = [
        ("process_uuid", uuid_oid, true, ""),
        ("process_id", uuid_oid, false, "s"),
        ("os_pid", int8_oid, false, ""),
        ("parent_session_id", text_oid, false, ""),
        ("parent_process_id", uuid_oid, false, ""),
        ("sandbox_adapter_id", text_oid, false, ""),
        ("adapter_id", text_oid, false, "s"),
        ("sandbox_internal_id", text_oid, false, ""),
        ("engine_kind", text_oid, true, ""),
        ("started_at", timestamptz_oid, true, ""),
        ("spawned_at_utc", timestamptz_oid, false, "s"),
        ("stopped_at", timestamptz_oid, false, ""),
        ("stopped_at_utc", timestamptz_oid, false, "s"),
        ("exit_code", int4_oid, false, ""),
        ("model_artifact_sha256", text_oid, false, ""),
        ("work_profile_id", text_oid, false, ""),
        ("owner_role", text_oid, true, ""),
        ("owner_wp", text_oid, false, ""),
        ("role_id", text_oid, false, ""),
        ("wp_id", text_oid, false, ""),
        ("mt_id", text_oid, false, ""),
        ("owner_runtime_instance_id", uuid_oid, false, ""),
        ("owner_host_scope_id", text_oid, false, ""),
        ("owner_lease_schema_id", text_oid, false, ""),
        ("owner_lease_protocol", text_oid, false, ""),
        ("owner_lease_address", text_oid, false, ""),
        ("owner_lease_port", int4_oid, false, ""),
        ("stop_reason", text_oid, false, ""),
        ("sandbox_capabilities_snapshot", jsonb_oid, true, ""),
        ("metadata_jsonb", jsonb_oid, true, ""),
    ];
    let shape_matching: Vec<(String, i64)> = relations
        .into_iter()
        .filter_map(|((schema, oid), columns)| {
            (columns.len() == expected.len()
                && expected
                    .iter()
                    .all(|(name, type_oid, not_null, generated)| {
                        columns.get(*name).is_some_and(|actual| {
                            actual.0 == *type_oid
                                && actual.1 == *not_null
                                && actual.2.as_str() == *generated
                        })
                    }))
            .then_some((schema, oid))
        })
        .collect();
    let mut matching = Vec::new();
    for (schema, oid) in shape_matching {
        let primary_key_matches: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM pg_catalog.pg_constraint AS constraint_row
                WHERE constraint_row.conrelid::pg_catalog.int8 = $1
                  AND constraint_row.contype = 'p'
                  AND (
                      SELECT pg_catalog.array_agg(attribute_row.attname::pg_catalog.text ORDER BY key_row.ordinality)
                      FROM pg_catalog.unnest(constraint_row.conkey)
                          WITH ORDINALITY AS key_row(attnum, ordinality)
                      JOIN pg_catalog.pg_attribute AS attribute_row
                        ON attribute_row.attrelid = constraint_row.conrelid
                       AND attribute_row.attnum = key_row.attnum
                  ) = ARRAY['process_uuid']::pg_catalog.text[]
            )
            "#,
        )
        .bind(oid)
        .fetch_one(&mut *connection)
        .await?;
        let generated_expressions_match: bool = sqlx::query_scalar(
            r#"
            SELECT pg_catalog.count(*) = 4
            FROM pg_catalog.pg_attribute AS attribute_row
            JOIN pg_catalog.pg_attrdef AS default_row
              ON default_row.adrelid = attribute_row.attrelid
             AND default_row.adnum = attribute_row.attnum
            WHERE attribute_row.attrelid::pg_catalog.int8 = $1
              AND (
                  (attribute_row.attname = 'process_id'
                      AND pg_catalog.pg_get_expr(default_row.adbin, default_row.adrelid, false) = 'process_uuid')
                  OR (attribute_row.attname = 'adapter_id'
                      AND pg_catalog.pg_get_expr(default_row.adbin, default_row.adrelid, false) = 'sandbox_adapter_id')
                  OR (attribute_row.attname = 'spawned_at_utc'
                      AND pg_catalog.pg_get_expr(default_row.adbin, default_row.adrelid, false) = 'started_at')
                  OR (attribute_row.attname = 'stopped_at_utc'
                      AND pg_catalog.pg_get_expr(default_row.adbin, default_row.adrelid, false) = 'stopped_at')
              )
            "#,
        )
        .bind(oid)
        .fetch_one(&mut *connection)
        .await?;
        let runtime_owner_guard_matches =
            process_runtime_owner_descriptor_guard_matches(connection, oid).await?;
        if primary_key_matches && generated_expressions_match && runtime_owner_guard_matches {
            matching.push((schema, oid));
        }
    }
    let (schema, relation_oid) = match matching.as_slice() {
        [(schema, relation_oid)] => (schema.clone(), *relation_oid),
        [] => {
            return Err(ProcessLedgerError::Store(
                "no migration-0021/0359-shaped, permanent, non-inherited, exact-runtime-owner-guard/RLS/rule-free kernel_process_lifecycle authority relation exists in the explicit PostgreSQL search path".to_string(),
            ))
        }
        schemas => {
            return Err(ProcessLedgerError::Store(format!(
                "ambiguous kernel_process_lifecycle authority relations in schemas: {}",
                schemas
                    .iter()
                    .map(|(schema, _)| schema.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            )))
        }
    };
    Ok(ProcessLedgerAuthorityRelation {
        qualified_table: format!(
            "{}.{}",
            quote_pg_identifier(&schema),
            quote_pg_identifier("kernel_process_lifecycle")
        ),
        schema,
        relation_oid,
    })
}

async fn resolve_pidless_reclaim_cursor_authority_relation_on_connection(
    connection: &mut PgConnection,
    schema: &str,
) -> Result<PidlessReclaimCursorAuthorityRelation, ProcessLedgerError> {
    let relation_oid = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT c.oid::pg_catalog.int8
        FROM pg_catalog.pg_class AS c
        JOIN pg_catalog.pg_namespace AS n ON n.oid = c.relnamespace
        WHERE n.nspname = $1
          AND c.relname = 'kernel_pidless_embedded_reclaim_cursor'
          AND c.relkind = 'r'
          AND c.relpersistence = 'p'
          AND NOT c.relrowsecurity
          AND NOT c.relforcerowsecurity
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_policy AS policy
              WHERE policy.polrelid = c.oid
          )
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_trigger AS trigger
              WHERE trigger.tgrelid = c.oid
                AND NOT trigger.tgisinternal
                AND trigger.tgenabled <> 'D'
          )
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_rewrite AS rewrite
              WHERE rewrite.ev_class = c.oid
          )
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_inherits AS inheritance
              WHERE inheritance.inhparent = c.oid OR inheritance.inhrelid = c.oid
          )
        "#,
    )
    .bind(schema)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| {
        ProcessLedgerError::Store(format!(
            "{}.kernel_pidless_embedded_reclaim_cursor is absent or is not a permanent logged, non-inherited, hook/RLS/rule-free ordinary table",
            quote_pg_identifier(schema)
        ))
    })?;
    Ok(PidlessReclaimCursorAuthorityRelation {
        schema: schema.to_string(),
        qualified_table: format!(
            "{}.{}",
            quote_pg_identifier(schema),
            quote_pg_identifier("kernel_pidless_embedded_reclaim_cursor")
        ),
        relation_oid,
    })
}

pub(super) async fn assert_process_ledger_authority_relation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &ProcessLedgerAuthorityRelation,
) -> Result<(), ProcessLedgerError> {
    let valid: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM pg_catalog.pg_class AS c
            JOIN pg_catalog.pg_namespace AS n ON n.oid = c.relnamespace
            WHERE c.oid::pg_catalog.int8 = $1
              AND n.nspname = $2
              AND c.relname = 'kernel_process_lifecycle'
              AND c.relkind = 'r'
              AND c.relpersistence = 'p'
              AND NOT c.relrowsecurity
              AND NOT c.relforcerowsecurity
              AND NOT EXISTS (
                  SELECT 1
                  FROM pg_catalog.pg_policy AS policy
                  WHERE policy.polrelid = c.oid
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM pg_catalog.pg_rewrite AS rewrite
                  WHERE rewrite.ev_class = c.oid
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM pg_catalog.pg_inherits AS inheritance
                  WHERE inheritance.inhparent = c.oid OR inheritance.inhrelid = c.oid
              )
              AND (
                  SELECT pg_catalog.count(*)
                  FROM pg_catalog.pg_attribute AS a
                  WHERE a.attrelid = c.oid
                    AND a.attnum > 0
                    AND NOT a.attisdropped
              ) = 30
              AND (
                  SELECT pg_catalog.count(*)
                  FROM pg_catalog.pg_attribute AS a
                  WHERE a.attrelid = c.oid
                    AND a.attnum > 0
                    AND NOT a.attisdropped
                    AND CASE a.attname
                        WHEN 'process_uuid' THEN a.atttypid = 'pg_catalog.uuid'::pg_catalog.regtype AND a.attnotnull AND a.attgenerated = ''
                        WHEN 'process_id' THEN a.atttypid = 'pg_catalog.uuid'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = 's'
                        WHEN 'os_pid' THEN a.atttypid = 'pg_catalog.int8'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'parent_session_id' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'parent_process_id' THEN a.atttypid = 'pg_catalog.uuid'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'sandbox_adapter_id' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'adapter_id' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = 's'
                        WHEN 'sandbox_internal_id' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'engine_kind' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND a.attnotnull AND a.attgenerated = ''
                        WHEN 'started_at' THEN a.atttypid = 'pg_catalog.timestamptz'::pg_catalog.regtype AND a.attnotnull AND a.attgenerated = ''
                        WHEN 'spawned_at_utc' THEN a.atttypid = 'pg_catalog.timestamptz'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = 's'
                        WHEN 'stopped_at' THEN a.atttypid = 'pg_catalog.timestamptz'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'stopped_at_utc' THEN a.atttypid = 'pg_catalog.timestamptz'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = 's'
                        WHEN 'exit_code' THEN a.atttypid = 'pg_catalog.int4'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'model_artifact_sha256' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'work_profile_id' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'owner_role' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND a.attnotnull AND a.attgenerated = ''
                        WHEN 'owner_wp' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'role_id' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'wp_id' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'mt_id' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'owner_runtime_instance_id' THEN a.atttypid = 'pg_catalog.uuid'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'owner_host_scope_id' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'owner_lease_schema_id' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'owner_lease_protocol' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'owner_lease_address' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'owner_lease_port' THEN a.atttypid = 'pg_catalog.int4'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'stop_reason' THEN a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = ''
                        WHEN 'sandbox_capabilities_snapshot' THEN a.atttypid = 'pg_catalog.jsonb'::pg_catalog.regtype AND a.attnotnull AND a.attgenerated = ''
                        WHEN 'metadata_jsonb' THEN a.atttypid = 'pg_catalog.jsonb'::pg_catalog.regtype AND a.attnotnull AND a.attgenerated = ''
                        ELSE false
                    END
              ) = 30
              AND EXISTS (
                  SELECT 1
                  FROM pg_catalog.pg_constraint AS constraint_row
                  WHERE constraint_row.conrelid = c.oid
                    AND constraint_row.contype = 'p'
                    AND (
                        SELECT pg_catalog.array_agg(attribute_row.attname::pg_catalog.text ORDER BY key_row.ordinality)
                        FROM pg_catalog.unnest(constraint_row.conkey)
                            WITH ORDINALITY AS key_row(attnum, ordinality)
                        JOIN pg_catalog.pg_attribute AS attribute_row
                          ON attribute_row.attrelid = c.oid
                         AND attribute_row.attnum = key_row.attnum
                    ) = ARRAY['process_uuid']::pg_catalog.text[]
              )
              AND (
                  SELECT pg_catalog.count(*)
                  FROM pg_catalog.pg_attribute AS attribute_row
                  JOIN pg_catalog.pg_attrdef AS default_row
                    ON default_row.adrelid = attribute_row.attrelid
                   AND default_row.adnum = attribute_row.attnum
                  WHERE attribute_row.attrelid = c.oid
                    AND (
                        (attribute_row.attname = 'process_id'
                            AND pg_catalog.pg_get_expr(default_row.adbin, default_row.adrelid, false) = 'process_uuid')
                        OR (attribute_row.attname = 'adapter_id'
                            AND pg_catalog.pg_get_expr(default_row.adbin, default_row.adrelid, false) = 'sandbox_adapter_id')
                        OR (attribute_row.attname = 'spawned_at_utc'
                            AND pg_catalog.pg_get_expr(default_row.adbin, default_row.adrelid, false) = 'started_at')
                        OR (attribute_row.attname = 'stopped_at_utc'
                            AND pg_catalog.pg_get_expr(default_row.adbin, default_row.adrelid, false) = 'stopped_at')
                    )
              ) = 4
        )
        "#,
    )
    .bind(authority.relation_oid)
    .bind(&authority.schema)
    .fetch_one(&mut **tx)
    .await?;
    let guard_valid =
        process_runtime_owner_descriptor_guard_matches(&mut **tx, authority.relation_oid).await?;
    if !valid || !guard_valid {
        return Err(ProcessLedgerError::Store(format!(
            "kernel_process_lifecycle authority relation {} changed identity, exact built-in column/generated-expression shape, UUID primary key, inheritance state, migration-0359 runtime-owner guard, RLS/rule-free state, or crash-durability class",
            authority.qualified_table
        )));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProcessLedgerAuthorityLockMode {
    AccessShare,
    RowExclusive,
}

pub(super) async fn lock_process_ledger_authority_relation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &ProcessLedgerAuthorityRelation,
    mode: ProcessLedgerAuthorityLockMode,
) -> Result<(), ProcessLedgerError> {
    let mode = match mode {
        ProcessLedgerAuthorityLockMode::AccessShare => "ACCESS SHARE",
        ProcessLedgerAuthorityLockMode::RowExclusive => "ROW EXCLUSIVE",
    };
    let statement = format!(
        "LOCK TABLE ONLY {} IN {mode} MODE",
        authority.qualified_table
    );
    sqlx::query(&statement).execute(&mut **tx).await?;
    Ok(())
}

async fn lock_pidless_reclaim_cursor_authority_relation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &PidlessReclaimCursorAuthorityRelation,
) -> Result<(), ProcessLedgerError> {
    let statement = format!(
        "LOCK TABLE ONLY {} IN ROW EXCLUSIVE MODE",
        authority.qualified_table
    );
    sqlx::query(&statement).execute(&mut **tx).await?;
    Ok(())
}

async fn assert_pidless_reclaim_cursor_authority_relation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &PidlessReclaimCursorAuthorityRelation,
) -> Result<(), ProcessLedgerError> {
    let valid: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM pg_catalog.pg_class AS c
            JOIN pg_catalog.pg_namespace AS n ON n.oid = c.relnamespace
            WHERE c.oid::pg_catalog.int8 = $1
              AND n.nspname = $2
              AND c.relname = 'kernel_pidless_embedded_reclaim_cursor'
              AND c.relkind = 'r'
              AND c.relpersistence = 'p'
              AND NOT c.relrowsecurity
              AND NOT c.relforcerowsecurity
              AND NOT EXISTS (
                  SELECT 1
                  FROM pg_catalog.pg_policy AS policy
                  WHERE policy.polrelid = c.oid
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM pg_catalog.pg_trigger AS trigger
                  WHERE trigger.tgrelid = c.oid
                    AND NOT trigger.tgisinternal
                    AND trigger.tgenabled <> 'D'
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM pg_catalog.pg_rewrite AS rewrite
                  WHERE rewrite.ev_class = c.oid
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM pg_catalog.pg_inherits AS inheritance
                  WHERE inheritance.inhparent = c.oid OR inheritance.inhrelid = c.oid
              )
              AND (
                  SELECT pg_catalog.count(*)
                  FROM pg_catalog.pg_attribute AS a
                  WHERE a.attrelid = c.oid
                    AND a.attnum > 0
                    AND NOT a.attisdropped
              ) = 3
              AND (
                  SELECT pg_catalog.count(*)
                  FROM pg_catalog.pg_attribute AS a
                  WHERE a.attrelid = c.oid
                    AND a.attnum > 0
                    AND NOT a.attisdropped
                    AND (
                        (a.attname = 'host_scope_id' AND a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND a.attnotnull AND a.attgenerated = '')
                        OR (a.attname = 'last_instance_id' AND a.atttypid = 'pg_catalog.text'::pg_catalog.regtype AND NOT a.attnotnull AND a.attgenerated = '')
                        OR (a.attname = 'updated_at_utc' AND a.atttypid = 'pg_catalog.timestamptz'::pg_catalog.regtype AND a.attnotnull AND a.attgenerated = '')
                    )
              ) = 3
              AND EXISTS (
                  SELECT 1
                  FROM pg_catalog.pg_constraint AS constraint_row
                  WHERE constraint_row.conrelid = c.oid
                    AND constraint_row.contype = 'p'
                    AND (
                        SELECT pg_catalog.array_agg(attribute_row.attname::pg_catalog.text ORDER BY key_row.ordinality)
                        FROM pg_catalog.unnest(constraint_row.conkey)
                            WITH ORDINALITY AS key_row(attnum, ordinality)
                        JOIN pg_catalog.pg_attribute AS attribute_row
                          ON attribute_row.attrelid = c.oid
                         AND attribute_row.attnum = key_row.attnum
                    ) = ARRAY['host_scope_id']::pg_catalog.text[]
              )
        )
        "#,
    )
    .bind(authority.relation_oid)
    .bind(&authority.schema)
    .fetch_one(&mut **tx)
    .await?;
    if !valid {
        return Err(ProcessLedgerError::Store(format!(
            "pid-less reclaim cursor authority {} changed identity, shape, primary key, inheritance state, hook/RLS/rule-free state, or crash-durability class",
            authority.qualified_table
        )));
    }
    Ok(())
}

pub(super) async fn force_all_constraints_immediate(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), ProcessLedgerError> {
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(super) async fn require_synchronous_commit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    operation: &str,
) -> Result<(), ProcessLedgerError> {
    let synchronous_commit: String =
        sqlx::query_scalar("SELECT pg_catalog.set_config('synchronous_commit', 'on', true)")
            .fetch_one(&mut **tx)
            .await?;
    if synchronous_commit != "on" {
        return Err(ProcessLedgerError::Store(format!(
            "failed to require synchronous PostgreSQL commit for {operation}; got {synchronous_commit}"
        )));
    }
    Ok(())
}

pub(super) async fn require_postgres_crash_durability(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    operation: &str,
) -> Result<(), ProcessLedgerError> {
    let (fsync, full_page_writes): (String, String) = sqlx::query_as(
        r#"
        SELECT pg_catalog.current_setting('fsync'),
               pg_catalog.current_setting('full_page_writes')
        "#,
    )
    .fetch_one(&mut **tx)
    .await?;
    if fsync != "on" || full_page_writes != "on" {
        return Err(ProcessLedgerError::Store(format!(
            "{operation} requires PostgreSQL fsync=on and full_page_writes=on for crash-durable acknowledgement; got fsync={fsync}, full_page_writes={full_page_writes}"
        )));
    }
    Ok(())
}

fn quote_pg_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub(super) async fn pin_transaction_search_path(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority_schema: &str,
) -> Result<(), ProcessLedgerError> {
    let search_path = format!(
        "pg_catalog,{},pg_temp",
        quote_pg_identifier(authority_schema)
    );
    sqlx::query("SELECT pg_catalog.set_config('search_path', $1, true)")
        .bind(search_path)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn configure_pidless_reclaim_timeouts(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), ProcessLedgerError> {
    sqlx::query("SELECT pg_catalog.set_config('lock_timeout', $1, true)")
        .bind(PIDLESS_RECLAIM_LOCK_TIMEOUT)
        .execute(&mut **tx)
        .await?;
    sqlx::query("SELECT pg_catalog.set_config('statement_timeout', $1, true)")
        .bind(PIDLESS_RECLAIM_STATEMENT_TIMEOUT)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn prepare_pidless_reclaim_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &ProcessLedgerAuthorityRelation,
    cursor_authority: Option<&PidlessReclaimCursorAuthorityRelation>,
    lock_mode: ProcessLedgerAuthorityLockMode,
) -> Result<(), ProcessLedgerError> {
    configure_pidless_reclaim_timeouts(tx).await?;
    pin_transaction_search_path(tx, &authority.schema).await?;
    lock_process_ledger_authority_relation(tx, authority, lock_mode).await?;
    assert_process_ledger_authority_relation(tx, authority).await?;
    if let Some(cursor_authority) = cursor_authority {
        lock_pidless_reclaim_cursor_authority_relation(tx, cursor_authority).await?;
        assert_pidless_reclaim_cursor_authority_relation(tx, cursor_authority).await?;
    }
    require_postgres_crash_durability(tx, "pid-less process-ledger reclaim").await?;
    require_synchronous_commit(tx, "pid-less process-ledger reclaim").await
}

fn postgres_sqlstate(error: &sqlx::Error) -> Option<String> {
    match error {
        sqlx::Error::Database(database_error) => {
            database_error.code().map(|code| code.into_owned())
        }
        _ => None,
    }
}

fn is_pidless_reclaim_timeout(error: &sqlx::Error) -> bool {
    postgres_sqlstate(error).is_some_and(|code| matches!(code.as_str(), "55P03" | "57014"))
}

fn is_pidless_reclaim_process_error_timeout(error: &ProcessLedgerError) -> bool {
    matches!(
        error,
        ProcessLedgerError::Postgres { source } if is_pidless_reclaim_timeout(source)
    )
}

fn parse_descriptor_from_metadata(
    metadata: &Value,
) -> Result<EmbeddedRuntimeInstanceDescriptor, String> {
    let object = metadata
        .as_object()
        .ok_or_else(|| "metadata_jsonb is not an object".to_string())?;
    let schema_id = object
        .get(RUNTIME_INSTANCE_SCHEMA_KEY)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string {RUNTIME_INSTANCE_SCHEMA_KEY}"))?;
    if schema_id != EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID {
        return Err(format!("unsupported runtime instance schema {schema_id:?}"));
    }
    let instance_raw = object
        .get(RUNTIME_INSTANCE_ID_KEY)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string {RUNTIME_INSTANCE_ID_KEY}"))?;
    let instance_id = Uuid::parse_str(instance_raw)
        .map_err(|error| format!("malformed runtime instance id: {error}"))?;
    let host_scope_id = object
        .get(RUNTIME_HOST_SCOPE_ID_KEY)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing nonempty string {RUNTIME_HOST_SCOPE_ID_KEY}"))?
        .to_string();
    let lease_protocol = object
        .get(RUNTIME_LEASE_PROTOCOL_KEY)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string {RUNTIME_LEASE_PROTOCOL_KEY}"))?;
    if lease_protocol != EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL {
        return Err(format!(
            "unsupported runtime lease protocol {lease_protocol:?}"
        ));
    }
    let loopback_address = object
        .get(RUNTIME_LEASE_ADDRESS_KEY)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string {RUNTIME_LEASE_ADDRESS_KEY}"))?
        .parse::<IpAddr>()
        .map_err(|error| format!("malformed runtime lease address: {error}"))?;
    if !loopback_address.is_loopback() {
        return Err("runtime lease address is not loopback".to_string());
    }
    let port = object
        .get(RUNTIME_LEASE_PORT_KEY)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing integer {RUNTIME_LEASE_PORT_KEY}"))?;
    let loopback_port = u16::try_from(port)
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| "runtime lease port is outside 1..=65535".to_string())?;
    Ok(EmbeddedRuntimeInstanceDescriptor {
        instance_id,
        host_scope_id,
        lease_protocol: lease_protocol.to_string(),
        loopback_address,
        loopback_port,
    })
}

fn metadata_instance_id(metadata: &Value) -> Option<Uuid> {
    metadata
        .as_object()?
        .get(RUNTIME_INSTANCE_ID_KEY)?
        .as_str()
        .and_then(|raw| Uuid::parse_str(raw).ok())
}

fn embedded_runtime_mutex_key(descriptor: &EmbeddedRuntimeInstanceDescriptor) -> i64 {
    let material = format!(
        "hsk.embedded_runtime.reclaim_mutex@1\0{}\0{}",
        descriptor.host_scope_id, descriptor.instance_id
    );
    let digest = Sha256::digest(material.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

/// Close only positively stale pid-less embedded-runtime rows.
///
/// PostgreSQL is used only for a short transaction-scoped concurrency mutex.
/// Liveness comes from exclusive ownership of the descriptor's loopback UDP
/// endpoint, which survives database restart but is released by process death.
/// Corrupt or foreign-host metadata is skipped independently and never prevents
/// valid candidates from being reconciled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyHostScopeOpenRowProbe {
    /// The bounded read-only probe found no open rows using the ambiguous
    /// pre-v2 `local-pg-sha256:` scope format.
    NoneDetected,
    /// At least one ambiguous legacy row remains open. It is never mutated by
    /// the v2 reclaimer and requires operator inspection.
    Detected,
    /// PostgreSQL did not complete the read-only probe within the reclaim
    /// transaction bounds, so absence of legacy rows was not proven.
    TimedOut,
}

impl Default for LegacyHostScopeOpenRowProbe {
    fn default() -> Self {
        // Unknown must fail closed until the bounded probe records a result.
        Self::TimedOut
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PidlessEmbeddedReclaimReport {
    pub closed_rows: u64,
    pub deferred_instances: u64,
    pub candidate_scan_timed_out: bool,
    pub candidate_instance_limit_reached: bool,
    pub legacy_host_scope_open_rows: LegacyHostScopeOpenRowProbe,
}

impl PidlessEmbeddedReclaimReport {
    pub fn is_complete(self) -> bool {
        !self.candidate_scan_timed_out
            && !self.candidate_instance_limit_reached
            && self.deferred_instances == 0
            && self.legacy_host_scope_open_rows == LegacyHostScopeOpenRowProbe::NoneDetected
    }
}

async fn probe_legacy_host_scope_open_rows(
    pool: &PgPool,
    authority: &ProcessLedgerAuthorityRelation,
    started_before: DateTime<Utc>,
) -> Result<LegacyHostScopeOpenRowProbe, ProcessLedgerError> {
    let mut tx = pool.begin().await?;
    if let Err(error) = prepare_pidless_reclaim_transaction(
        &mut tx,
        authority,
        None,
        ProcessLedgerAuthorityLockMode::AccessShare,
    )
    .await
    {
        tx.rollback().await?;
        if is_pidless_reclaim_process_error_timeout(&error) {
            return Ok(LegacyHostScopeOpenRowProbe::TimedOut);
        }
        return Err(error);
    }

    // Read-only and statement-timeout bounded. Deliberately use EXISTS rather
    // than loading row identities: legacy v1 and early-v2 hashes are
    // indistinguishable, so this path may diagnose but must never classify,
    // claim, close, or advance a cursor for them.
    let probe_sql = format!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM ONLY {}
            WHERE parent_session_id IS NULL
              AND os_pid IS NULL
              AND stopped_at IS NULL
              AND engine_kind IN ('llamacpp', 'candle')
              AND started_at < $1
              AND metadata_jsonb->>'runtime_host_scope_id' LIKE $2
        )
        "#,
        authority.qualified_table
    );
    let legacy_prefix_pattern = format!("{EMBEDDED_RUNTIME_LEGACY_LOCAL_HOST_SCOPE_PREFIX}%");
    let detected = match sqlx::query_scalar::<_, bool>(&probe_sql)
        .bind(started_before)
        .bind(legacy_prefix_pattern)
        .fetch_one(&mut *tx)
        .await
    {
        Ok(detected) => detected,
        Err(error) if is_pidless_reclaim_timeout(&error) => {
            tx.rollback().await?;
            return Ok(LegacyHostScopeOpenRowProbe::TimedOut);
        }
        Err(error) => return Err(error.into()),
    };
    tx.commit().await?;
    Ok(if detected {
        LegacyHostScopeOpenRowProbe::Detected
    } else {
        LegacyHostScopeOpenRowProbe::NoneDetected
    })
}

pub async fn reclaim_pidless_embedded_orphans(
    pool: &PgPool,
    started_before: DateTime<Utc>,
    local_host_scope_id: &str,
) -> Result<PidlessEmbeddedReclaimReport, ProcessLedgerError> {
    if local_host_scope_id.trim().is_empty() {
        return Err(ProcessLedgerError::InvalidConfig(
            "local embedded runtime host scope must not be empty".to_string(),
        ));
    }
    let mut resolution_tx = pool.begin().await?;
    configure_pidless_reclaim_timeouts(&mut resolution_tx).await?;
    // Authority attestation performs several catalog queries and generated
    // expression checks. It needs more execution time than a mutation query,
    // while the lower lock_timeout still bounds hostile DDL contention.
    sqlx::query("SELECT pg_catalog.set_config('statement_timeout', $1, true)")
        .bind(PIDLESS_RECLAIM_AUTHORITY_STATEMENT_TIMEOUT)
        .execute(&mut *resolution_tx)
        .await?;
    let authority =
        match resolve_process_ledger_authority_relation_on_connection(&mut resolution_tx).await {
            Ok(authority) => authority,
            Err(error) if is_pidless_reclaim_process_error_timeout(&error) => {
                resolution_tx.rollback().await?;
                return Ok(PidlessEmbeddedReclaimReport {
                    candidate_scan_timed_out: true,
                    ..PidlessEmbeddedReclaimReport::default()
                });
            }
            Err(error) => {
                resolution_tx.rollback().await?;
                return Err(error);
            }
        };
    let cursor_authority = match resolve_pidless_reclaim_cursor_authority_relation_on_connection(
        &mut resolution_tx,
        &authority.schema,
    )
    .await
    {
        Ok(authority) => authority,
        Err(error) if is_pidless_reclaim_process_error_timeout(&error) => {
            resolution_tx.rollback().await?;
            return Ok(PidlessEmbeddedReclaimReport {
                candidate_scan_timed_out: true,
                ..PidlessEmbeddedReclaimReport::default()
            });
        }
        Err(error) => {
            resolution_tx.rollback().await?;
            return Err(error);
        }
    };
    resolution_tx.rollback().await?;
    let legacy_host_scope_open_rows =
        probe_legacy_host_scope_open_rows(pool, &authority, started_before).await?;
    match legacy_host_scope_open_rows {
        LegacyHostScopeOpenRowProbe::Detected => tracing::warn!(
            target: "handshake_core::process_ledger",
            legacy_scope_prefix = EMBEDDED_RUNTIME_LEGACY_LOCAL_HOST_SCOPE_PREFIX,
            "ambiguous legacy host-scope pid-less embedded rows remain open; v2 reclaim left them untouched for operator inspection"
        ),
        LegacyHostScopeOpenRowProbe::TimedOut => tracing::warn!(
            target: "handshake_core::process_ledger",
            legacy_scope_prefix = EMBEDDED_RUNTIME_LEGACY_LOCAL_HOST_SCOPE_PREFIX,
            "bounded legacy host-scope row probe timed out; absence of ambiguous open rows was not proven"
        ),
        LegacyHostScopeOpenRowProbe::NoneDetected => {}
    }
    // A timed-out legacy probe means we could not prove that ambiguous v1
    // rows are absent. Continuing into the candidate scan would both weaken
    // that fail-closed boundary and stack a second lock/statement timeout onto
    // the boot path. Return an explicitly incomplete report and retry later.
    if legacy_host_scope_open_rows == LegacyHostScopeOpenRowProbe::TimedOut {
        return Ok(PidlessEmbeddedReclaimReport {
            candidate_scan_timed_out: true,
            legacy_host_scope_open_rows,
            ..PidlessEmbeddedReclaimReport::default()
        });
    }
    let cursor_table = &cursor_authority.qualified_table;
    let candidate_sql = format!(
        r#"
        WITH forward_candidates AS (
            SELECT DISTINCT ON (owner_runtime_instance_id)
                process_uuid,
                metadata_jsonb,
                exit_code,
                stop_reason,
                owner_runtime_instance_id::text AS raw_instance_id,
                owner_runtime_instance_id::text AS owner_runtime_instance_id,
                owner_host_scope_id,
                owner_lease_schema_id,
                owner_lease_protocol,
                owner_lease_address,
                owner_lease_port
            FROM ONLY {}
            WHERE parent_session_id IS NULL
              AND os_pid IS NULL
              AND stopped_at IS NULL
              AND exit_code IS NULL
              AND stop_reason IS NULL
              AND engine_kind IN ('llamacpp', 'candle')
              AND started_at < $1
              AND owner_lease_schema_id = $2
              AND owner_host_scope_id = $3
              AND owner_lease_protocol = $4
              AND owner_runtime_instance_id IS NOT NULL
              AND NULLIF(owner_lease_address, '') IS NOT NULL
              AND owner_lease_port BETWEEN 1 AND 65535
              AND ($5::text IS NULL OR owner_runtime_instance_id::text > $5)
            ORDER BY owner_runtime_instance_id, process_uuid
            LIMIT $6
        ), wrapped_candidates AS (
            SELECT DISTINCT ON (owner_runtime_instance_id)
                process_uuid,
                metadata_jsonb,
                exit_code,
                stop_reason,
                owner_runtime_instance_id::text AS raw_instance_id,
                owner_runtime_instance_id::text AS owner_runtime_instance_id,
                owner_host_scope_id,
                owner_lease_schema_id,
                owner_lease_protocol,
                owner_lease_address,
                owner_lease_port
            FROM ONLY {}
            WHERE parent_session_id IS NULL
              AND os_pid IS NULL
              AND stopped_at IS NULL
              AND exit_code IS NULL
              AND stop_reason IS NULL
              AND engine_kind IN ('llamacpp', 'candle')
              AND started_at < $1
              AND owner_lease_schema_id = $2
              AND owner_host_scope_id = $3
              AND owner_lease_protocol = $4
              AND owner_runtime_instance_id IS NOT NULL
              AND NULLIF(owner_lease_address, '') IS NOT NULL
              AND owner_lease_port BETWEEN 1 AND 65535
              AND $5::text IS NOT NULL
              AND owner_runtime_instance_id::text <= $5
            ORDER BY owner_runtime_instance_id, process_uuid
            LIMIT $6
        ), cyclic_candidates AS (
            SELECT *, 0 AS scan_phase FROM forward_candidates
            UNION ALL
            SELECT *, 1 AS scan_phase FROM wrapped_candidates
        )
        SELECT process_uuid, metadata_jsonb, exit_code, stop_reason, raw_instance_id,
               owner_runtime_instance_id, owner_host_scope_id, owner_lease_schema_id,
               owner_lease_protocol, owner_lease_address, owner_lease_port
        FROM cyclic_candidates
        ORDER BY scan_phase, raw_instance_id, process_uuid
        LIMIT $6
        "#,
        authority.qualified_table, authority.qualified_table
    );
    let mut report = PidlessEmbeddedReclaimReport {
        legacy_host_scope_open_rows,
        ..PidlessEmbeddedReclaimReport::default()
    };
    let mut candidate_tx = pool.begin().await?;
    if let Err(error) = prepare_pidless_reclaim_transaction(
        &mut candidate_tx,
        &authority,
        Some(&cursor_authority),
        ProcessLedgerAuthorityLockMode::AccessShare,
    )
    .await
    {
        candidate_tx.rollback().await?;
        if is_pidless_reclaim_process_error_timeout(&error) {
            report.candidate_scan_timed_out = true;
            return Ok(report);
        }
        return Err(error);
    }
    let cursor_sql = format!(
        r#"
        INSERT INTO {} (host_scope_id, last_instance_id, updated_at_utc)
        VALUES ($1, NULL, pg_catalog.clock_timestamp())
        ON CONFLICT (host_scope_id) DO UPDATE
        SET host_scope_id = EXCLUDED.host_scope_id
        RETURNING last_instance_id
        "#,
        cursor_table
    );
    let last_instance_id = match sqlx::query_scalar::<_, Option<String>>(&cursor_sql)
        .bind(local_host_scope_id)
        .fetch_one(&mut *candidate_tx)
        .await
    {
        Ok(cursor) => cursor,
        Err(error) if is_pidless_reclaim_timeout(&error) => {
            candidate_tx.rollback().await?;
            report.candidate_scan_timed_out = true;
            return Ok(report);
        }
        Err(error) => return Err(error.into()),
    };
    let mut candidate_rows = match sqlx::query(&candidate_sql)
        .bind(started_before)
        .bind(EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID)
        .bind(local_host_scope_id)
        .bind(EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL)
        .bind(last_instance_id.as_deref())
        .bind((PIDLESS_RECLAIM_INSTANCE_CAP + 1) as i64)
        .fetch_all(&mut *candidate_tx)
        .await
    {
        Ok(rows) => rows,
        Err(error) if is_pidless_reclaim_timeout(&error) => {
            let sqlstate = postgres_sqlstate(&error).unwrap_or_else(|| "unknown".to_string());
            tracing::warn!(
                target: "handshake_core::process_ledger",
                postgres_sqlstate = %sqlstate,
                reclaim_outcome = "candidate_scan_timeout_skipped",
                "pid-less embedded orphan candidate scan exceeded its bounded PostgreSQL deadline; leaving all rows open for a later sweep"
            );
            candidate_tx.rollback().await?;
            report.candidate_scan_timed_out = true;
            return Ok(report);
        }
        Err(error) => return Err(error.into()),
    };
    let excluded_unsafe_sql = format!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM ONLY {}
            WHERE parent_session_id IS NULL
              AND os_pid IS NULL
              AND stopped_at IS NULL
              AND engine_kind IN ('llamacpp', 'candle')
              AND started_at < $1
              AND (
                  owner_host_scope_id = $3
                  OR NULLIF(owner_host_scope_id, '') IS NULL
              )
              AND (
                  exit_code IS NOT NULL
                  OR stop_reason IS NOT NULL
                  OR owner_lease_schema_id IS DISTINCT FROM $2
                  OR owner_lease_protocol IS DISTINCT FROM $4
                  OR owner_runtime_instance_id IS NULL
                  OR NULLIF(owner_lease_address, '') IS NULL
                  OR owner_lease_port NOT BETWEEN 1 AND 65535
              )
        )
        "#,
        authority.qualified_table
    );
    let excluded_unsafe_exists = match sqlx::query_scalar::<_, bool>(&excluded_unsafe_sql)
        .bind(started_before)
        .bind(EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID)
        .bind(local_host_scope_id)
        .bind(EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL)
        .fetch_one(&mut *candidate_tx)
        .await
    {
        Ok(exists) => exists,
        Err(error) if is_pidless_reclaim_timeout(&error) => {
            candidate_tx.rollback().await?;
            report.candidate_scan_timed_out = true;
            return Ok(report);
        }
        Err(error) => return Err(error.into()),
    };
    if excluded_unsafe_exists {
        report.deferred_instances = report.deferred_instances.saturating_add(1);
        tracing::error!(
            target: "handshake_core::process_ledger",
            local_host_scope_id,
            "open pid-less embedded rows excluded by safe descriptor prefilters remain unresolved"
        );
    }
    if candidate_rows.len() > PIDLESS_RECLAIM_INSTANCE_CAP {
        candidate_rows.truncate(PIDLESS_RECLAIM_INSTANCE_CAP);
        report.candidate_instance_limit_reached = true;
    }
    let next_cursor = candidate_rows
        .last()
        .and_then(|row| row.try_get::<String, _>("raw_instance_id").ok());
    if let Some(next_cursor) = next_cursor.as_deref() {
        let update_cursor_sql = format!(
            "UPDATE {} SET last_instance_id = $2, updated_at_utc = pg_catalog.clock_timestamp() WHERE host_scope_id = $1",
            cursor_table
        );
        sqlx::query(&update_cursor_sql)
            .bind(local_host_scope_id)
            .bind(next_cursor)
            .execute(&mut *candidate_tx)
            .await?;
    }
    force_all_constraints_immediate(&mut candidate_tx).await?;
    assert_process_ledger_authority_relation(&mut candidate_tx, &authority).await?;
    assert_pidless_reclaim_cursor_authority_relation(&mut candidate_tx, &cursor_authority).await?;
    let persisted_cursor_sql = format!(
        "SELECT last_instance_id FROM {} WHERE host_scope_id = $1",
        cursor_table
    );
    let persisted_cursor = sqlx::query_scalar::<_, Option<String>>(&persisted_cursor_sql)
        .bind(local_host_scope_id)
        .fetch_one(&mut *candidate_tx)
        .await?;
    let expected_cursor = next_cursor.as_ref().or(last_instance_id.as_ref());
    if persisted_cursor.as_ref() != expected_cursor {
        return Err(ProcessLedgerError::Store(format!(
            "pid-less reclaim cursor final readback mismatch for host scope {local_host_scope_id}"
        )));
    }
    require_synchronous_commit(
        &mut candidate_tx,
        "pid-less process-ledger cursor mutation commit",
    )
    .await?;
    candidate_tx.commit().await?;

    let mut descriptors: BTreeMap<Uuid, EmbeddedRuntimeInstanceDescriptor> = BTreeMap::new();
    let mut conflicts = BTreeSet::new();
    for row in candidate_rows {
        let process_uuid = match row.try_get::<Uuid, _>("process_uuid") {
            Ok(value) => value,
            Err(error) => {
                report.deferred_instances = report.deferred_instances.saturating_add(1);
                tracing::error!(
                    target: "handshake_core::process_ledger",
                    error = %error,
                    "pid-less embedded row has unreadable process_uuid; skipping independently"
                );
                continue;
            }
        };
        let exit_code = match row.try_get::<Option<i32>, _>("exit_code") {
            Ok(value) => value,
            Err(error) => {
                report.deferred_instances = report.deferred_instances.saturating_add(1);
                tracing::error!(
                    target: "handshake_core::process_ledger",
                    process_uuid = %process_uuid,
                    error = %error,
                    "open pid-less embedded row has unreadable exit_code; skipping independently"
                );
                continue;
            }
        };
        let stop_reason = match row.try_get::<Option<String>, _>("stop_reason") {
            Ok(value) => value,
            Err(error) => {
                report.deferred_instances = report.deferred_instances.saturating_add(1);
                tracing::error!(
                    target: "handshake_core::process_ledger",
                    process_uuid = %process_uuid,
                    error = %error,
                    "open pid-less embedded row has unreadable stop_reason; skipping independently"
                );
                continue;
            }
        };
        if exit_code.is_some() || stop_reason.is_some() {
            report.deferred_instances = report.deferred_instances.saturating_add(1);
            tracing::error!(
                target: "handshake_core::process_ledger",
                process_uuid = %process_uuid,
                exit_code = ?exit_code,
                stop_reason = ?stop_reason,
                "open pid-less embedded row already carries terminal metadata; refusing automatic reclassification"
            );
            continue;
        }
        let runtime_owner = match process_runtime_owner_from_row(&row) {
            Ok(Some(value)) => value,
            Ok(None) => {
                report.deferred_instances = report.deferred_instances.saturating_add(1);
                tracing::error!(
                    target: "handshake_core::process_ledger",
                    process_uuid = %process_uuid,
                    "pid-less embedded row has no typed runtime owner; skipping independently"
                );
                continue;
            }
            Err(error) => {
                report.deferred_instances = report.deferred_instances.saturating_add(1);
                tracing::error!(
                    target: "handshake_core::process_ledger",
                    process_uuid = %process_uuid,
                    error = %error,
                    "pid-less embedded row has invalid typed runtime owner; skipping independently"
                );
                continue;
            }
        };
        let descriptor = EmbeddedRuntimeInstanceDescriptor {
            instance_id: runtime_owner.runtime_instance_id,
            host_scope_id: runtime_owner.host_scope_id,
            lease_protocol: runtime_owner.lease_protocol,
            loopback_address: match runtime_owner.lease_address.parse::<IpAddr>() {
                Ok(address) if address.is_loopback() => address,
                _ => {
                    report.deferred_instances = report.deferred_instances.saturating_add(1);
                    continue;
                }
            },
            loopback_port: runtime_owner.lease_port,
        };
        match (runtime_owner.lease_schema_id == EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID)
            .then_some(descriptor)
            .ok_or_else(|| "unsupported typed runtime owner schema".to_string())
        {
            Ok(descriptor) => match descriptors.get(&descriptor.instance_id) {
                Some(existing) if existing != &descriptor => {
                    conflicts.insert(descriptor.instance_id);
                    tracing::error!(
                        target: "handshake_core::process_ledger",
                        process_uuid = %process_uuid,
                        runtime_instance_id = %descriptor.instance_id,
                        "conflicting embedded runtime descriptors share one instance id; skipping that instance"
                    );
                }
                Some(_) => {}
                None => {
                    descriptors.insert(descriptor.instance_id, descriptor);
                }
            },
            Err(error) => {
                conflicts.insert(runtime_owner.runtime_instance_id);
                tracing::error!(
                    target: "handshake_core::process_ledger",
                    process_uuid = %process_uuid,
                    error = %error,
                    "invalid embedded runtime descriptor is not safe to reclaim automatically; skipping independently"
                );
            }
        }
    }
    report.deferred_instances = report
        .deferred_instances
        .saturating_add(conflicts.len() as u64);
    for instance_id in conflicts {
        descriptors.remove(&instance_id);
    }

    for descriptor in descriptors.into_values() {
        if descriptor.host_scope_id != local_host_scope_id {
            report.deferred_instances = report.deferred_instances.saturating_add(1);
            tracing::warn!(
                target: "handshake_core::process_ledger",
                runtime_instance_id = %descriptor.instance_id,
                descriptor_host_scope_id = %descriptor.host_scope_id,
                local_host_scope_id,
                "foreign-host embedded runtime row cannot be judged through local loopback; skipping reclaim"
            );
            continue;
        }
        let _udp_claim = match try_claim_udp_lease(&descriptor) {
            UdpLeaseClaim::Claimed(socket) => socket,
            UdpLeaseClaim::Protected => {
                tracing::debug!(
                    target: "handshake_core::process_ledger",
                    runtime_instance_id = %descriptor.instance_id,
                    loopback_port = descriptor.loopback_port,
                    "embedded runtime UDP lease is still owned or its port was safely reused; skipping reclaim"
                );
                continue;
            }
            UdpLeaseClaim::Ambiguous(error) => {
                report.deferred_instances = report.deferred_instances.saturating_add(1);
                tracing::error!(
                    target: "handshake_core::process_ledger",
                    runtime_instance_id = %descriptor.instance_id,
                    loopback_port = descriptor.loopback_port,
                    error = %error,
                    "embedded runtime UDP lease could not be judged safely; skipping reclaim"
                );
                continue;
            }
        };

        let mut tx = pool.begin().await?;
        if let Err(error) = prepare_pidless_reclaim_transaction(
            &mut tx,
            &authority,
            None,
            ProcessLedgerAuthorityLockMode::RowExclusive,
        )
        .await
        {
            tx.rollback().await?;
            if is_pidless_reclaim_process_error_timeout(&error) {
                report.deferred_instances = report.deferred_instances.saturating_add(1);
                continue;
            }
            return Err(error);
        }
        let acquired: bool = sqlx::query_scalar("SELECT pg_catalog.pg_try_advisory_xact_lock($1)")
            .bind(embedded_runtime_mutex_key(&descriptor))
            .fetch_one(&mut *tx)
            .await?;
        if !acquired {
            tx.rollback().await?;
            report.deferred_instances = report.deferred_instances.saturating_add(1);
            continue;
        }

        let conflict_sql = format!(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM ONLY {} AS conflict
                WHERE conflict.parent_session_id IS NULL
                  AND conflict.os_pid IS NULL
                  AND conflict.stopped_at IS NULL
                  AND conflict.exit_code IS NULL
                  AND conflict.stop_reason IS NULL
                  AND conflict.engine_kind IN ('llamacpp', 'candle')
                  AND conflict.started_at < $1
                  AND conflict.owner_runtime_instance_id = $2::uuid
                  AND (
                      conflict.owner_lease_schema_id IS DISTINCT FROM $3
                      OR conflict.owner_host_scope_id IS DISTINCT FROM $4
                      OR conflict.owner_lease_protocol IS DISTINCT FROM $5
                      OR conflict.owner_lease_address IS DISTINCT FROM $6
                      OR conflict.owner_lease_port IS DISTINCT FROM $7
                  )
            )
            "#,
            authority.qualified_table
        );
        let descriptor_conflict = match sqlx::query_scalar::<_, bool>(&conflict_sql)
            .bind(started_before)
            .bind(descriptor.instance_id.to_string())
            .bind(EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID)
            .bind(&descriptor.host_scope_id)
            .bind(&descriptor.lease_protocol)
            .bind(descriptor.loopback_address.to_string())
            .bind(i32::from(descriptor.loopback_port))
            .fetch_one(&mut *tx)
            .await
        {
            Ok(conflict) => conflict,
            Err(error) if is_pidless_reclaim_timeout(&error) => {
                let sqlstate = postgres_sqlstate(&error).unwrap_or_else(|| "unknown".to_string());
                tracing::warn!(
                    target: "handshake_core::process_ledger",
                    runtime_instance_id = %descriptor.instance_id,
                    postgres_sqlstate = %sqlstate,
                    reclaim_outcome = "descriptor_conflict_check_timeout_skipped",
                    "pid-less embedded orphan conflict check exceeded its bounded PostgreSQL deadline; leaving the matching START open for a later sweep"
                );
                tx.rollback().await?;
                report.deferred_instances = report.deferred_instances.saturating_add(1);
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        if descriptor_conflict {
            tracing::error!(
                target: "handshake_core::process_ledger",
                runtime_instance_id = %descriptor.instance_id,
                "conflicting embedded runtime descriptors share one instance id; skipping that instance"
            );
            tx.rollback().await?;
            report.deferred_instances = report.deferred_instances.saturating_add(1);
            continue;
        }

        let update_sql = format!(
            r#"
            UPDATE ONLY {}
            SET stopped_at = pg_catalog.clock_timestamp(),
                exit_code = -1,
                stop_reason = 'orphan_reclaim_pidless_embedded_boot'
            WHERE parent_session_id IS NULL
              AND os_pid IS NULL
              AND stopped_at IS NULL
              AND exit_code IS NULL
              AND stop_reason IS NULL
              AND engine_kind IN ('llamacpp', 'candle')
              AND started_at < $1
              AND owner_lease_schema_id = $2
              AND owner_runtime_instance_id = $3::uuid
              AND owner_host_scope_id = $4
              AND owner_lease_protocol = $5
              AND owner_lease_address = $6
              AND owner_lease_port = $7
            "#,
            authority.qualified_table
        );
        let result = match sqlx::query(&update_sql)
            .bind(started_before)
            .bind(EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID)
            .bind(descriptor.instance_id.to_string())
            .bind(&descriptor.host_scope_id)
            .bind(&descriptor.lease_protocol)
            .bind(descriptor.loopback_address.to_string())
            .bind(i32::from(descriptor.loopback_port))
            .execute(&mut *tx)
            .await
        {
            Ok(result) => result,
            Err(error) if is_pidless_reclaim_timeout(&error) => {
                let sqlstate = postgres_sqlstate(&error).unwrap_or_else(|| "unknown".to_string());
                tracing::warn!(
                    target: "handshake_core::process_ledger",
                    runtime_instance_id = %descriptor.instance_id,
                    postgres_sqlstate = %sqlstate,
                    reclaim_outcome = "descriptor_update_timeout_skipped",
                    "pid-less embedded orphan update exceeded its bounded PostgreSQL deadline; leaving the matching START open for a later sweep"
                );
                tx.rollback().await?;
                report.deferred_instances = report.deferred_instances.saturating_add(1);
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        force_all_constraints_immediate(&mut tx).await?;
        assert_process_ledger_authority_relation(&mut tx, &authority).await?;
        let final_readback_sql = format!(
            r#"
            SELECT
                NOT EXISTS (
                    SELECT 1
                    FROM ONLY {}
                    WHERE parent_session_id IS NULL
                      AND os_pid IS NULL
                      AND stopped_at IS NULL
                      AND exit_code IS NULL
                      AND stop_reason IS NULL
                      AND engine_kind IN ('llamacpp', 'candle')
                      AND started_at < $1
                      AND owner_lease_schema_id = $2
                      AND owner_runtime_instance_id = $3::uuid
                      AND owner_host_scope_id = $4
                      AND owner_lease_protocol = $5
                      AND owner_lease_address = $6
                      AND owner_lease_port = $7
                )
                AND (
                    SELECT pg_catalog.count(*)
                    FROM ONLY {}
                    WHERE parent_session_id IS NULL
                      AND os_pid IS NULL
                      AND stopped_at IS NOT NULL
                      AND exit_code = -1
                      AND stop_reason = 'orphan_reclaim_pidless_embedded_boot'
                      AND engine_kind IN ('llamacpp', 'candle')
                      AND started_at < $1
                      AND owner_lease_schema_id = $2
                      AND owner_runtime_instance_id = $3::uuid
                      AND owner_host_scope_id = $4
                      AND owner_lease_protocol = $5
                      AND owner_lease_address = $6
                      AND owner_lease_port = $7
                ) >= $8
            "#,
            authority.qualified_table, authority.qualified_table
        );
        let rows_affected = result.rows_affected();
        let minimum_terminal_rows = i64::try_from(rows_affected).map_err(|_| {
            ProcessLedgerError::Store(
                "pid-less reclaim rows_affected exceeded PostgreSQL bigint readback range"
                    .to_string(),
            )
        })?;
        let final_readback_valid: bool = sqlx::query_scalar(&final_readback_sql)
            .bind(started_before)
            .bind(EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID)
            .bind(descriptor.instance_id.to_string())
            .bind(&descriptor.host_scope_id)
            .bind(&descriptor.lease_protocol)
            .bind(descriptor.loopback_address.to_string())
            .bind(i32::from(descriptor.loopback_port))
            .bind(minimum_terminal_rows)
            .fetch_one(&mut *tx)
            .await?;
        if !final_readback_valid {
            return Err(ProcessLedgerError::Store(format!(
                "pid-less reclaim final lifecycle readback rejected runtime instance {}",
                descriptor.instance_id
            )));
        }
        require_synchronous_commit(&mut tx, "pid-less process-ledger terminal mutation commit")
            .await?;
        tx.commit().await?;
        report.closed_rows = report.closed_rows.saturating_add(rows_affected);
    }
    Ok(report)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReclaimTrigger {
    Close,
    Failure,
    Restart,
    Stale,
    OperatorCancel,
}

const RECLAIM_CLAIM_TTL: Duration = Duration::from_secs(30);
const RECLAIM_CLAIM_RENEW_INTERVAL: Duration = Duration::from_secs(10);
const RECLAIM_KILL_TIMEOUT: Duration = Duration::from_secs(30);
const RECLAIM_STOP_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const RECLAIM_IN_PROGRESS_RECOVERY_LIMIT: usize = 64;

#[derive(Default)]
struct ProcessKillFence {
    result: std::sync::Mutex<Option<Result<(), KillError>>>,
    completed: tokio::sync::Notify,
}

static PROCESS_KILL_FENCES: OnceLock<std::sync::Mutex<HashMap<Uuid, Arc<ProcessKillFence>>>> =
    OnceLock::new();

fn acquire_process_kill_fence(process_uuid: Uuid) -> (Arc<ProcessKillFence>, bool) {
    let fences = PROCESS_KILL_FENCES.get_or_init(Default::default);
    let mut fences = match fences.lock() {
        Ok(fences) => fences,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(existing) = fences.get(&process_uuid) {
        return (Arc::clone(existing), false);
    }
    let fence = Arc::new(ProcessKillFence::default());
    fences.insert(process_uuid, Arc::clone(&fence));
    (fence, true)
}

fn clear_process_kill_fence(process_uuid: Uuid, completed: &Arc<ProcessKillFence>) {
    let Some(fences) = PROCESS_KILL_FENCES.get() else {
        return;
    };
    let mut fences = match fences.lock() {
        Ok(fences) => fences,
        Err(poisoned) => poisoned.into_inner(),
    };
    if fences
        .get(&process_uuid)
        .is_some_and(|current| Arc::ptr_eq(current, completed))
    {
        fences.remove(&process_uuid);
    }
}

fn clear_completed_process_kill_fence(process_uuid: Uuid) {
    let Some(fences) = PROCESS_KILL_FENCES.get() else {
        return;
    };
    let completed = {
        let fences = match fences.lock() {
            Ok(fences) => fences,
            Err(poisoned) => poisoned.into_inner(),
        };
        fences.get(&process_uuid).cloned()
    };
    let Some(completed) = completed else {
        return;
    };
    let is_complete = match completed.result.lock() {
        Ok(result) => result.is_some(),
        Err(poisoned) => poisoned.into_inner().is_some(),
    };
    if is_complete {
        clear_process_kill_fence(process_uuid, &completed);
    }
}

/// Fenced ownership of one open lifecycle row. Both fields participate in
/// release, renewal, pending-stop, and final STOP transitions; a stale
/// claimant can therefore neither erase nor finalize a newer claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimClaim {
    pub claimant_uuid: Uuid,
    pub kill_operation_uuid: Uuid,
    pub generation: u64,
    pub claimed_at_unix_ms: i64,
    pub lease_expires_at_unix_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReclaimKillOperationStatus {
    NotStarted,
    InProgress,
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimKillOperation {
    pub process_uuid: Uuid,
    pub kill_operation_uuid: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "candidate", rename_all = "snake_case")]
pub enum ReclaimKillOperationCandidate {
    Operation {
        operation: ReclaimKillOperation,
    },
    Malformed {
        process_identity: String,
        kill_operation_identity: Option<String>,
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimKillOperationSweep {
    pub operations: Vec<ReclaimKillOperationSweepEntry>,
    pub reclaim_report: Option<ReclaimReport>,
    pub reclaim_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimKillOperationSweepEntry {
    pub candidate: ReclaimKillOperationCandidate,
    pub outcome: ReclaimKillOperationSweepOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ReclaimKillOperationSweepOutcome {
    StateAdvanced {
        status: ReclaimKillOperationStatus,
    },
    StateOpen {
        status: ReclaimKillOperationStatus,
    },
    StatusQueryFailed {
        error: String,
    },
    StateTransitionFailed {
        status: ReclaimKillOperationStatus,
        error: String,
    },
    MalformedRecoveryRow {
        error: String,
    },
}

impl ReclaimKillOperationStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

impl ReclaimTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::Failure => "failure",
            Self::Restart => "restart",
            Self::Stale => "stale",
            Self::OperatorCancel => "operator_cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimableProcess {
    pub process_uuid: Uuid,
    pub os_pid: Option<u32>,
    pub parent_session_id: String,
    pub parent_process_id: Option<Uuid>,
    pub sandbox_adapter_id: Option<String>,
    pub sandbox_internal_id: Option<String>,
    pub engine_kind: ProcessEngineKind,
    pub started_at: DateTime<Utc>,
    pub model_artifact_sha256: Option<String>,
    pub work_profile_id: Option<String>,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub role_id: Option<String>,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub runtime_owner: Option<ProcessRuntimeOwner>,
    pub sandbox_capabilities_snapshot: serde_json::Value,
    pub metadata_jsonb: serde_json::Value,
    pub reclaim_claim: ReclaimClaim,
    /// A prior claimant already proved the kill. Recovery must persist STOP
    /// from the durable row and must never invoke the sandbox kill again.
    pub kill_succeeded_pending_stop: bool,
}

impl ReclaimableProcess {
    fn sync_reclaim_claim_metadata(&mut self) -> Result<(), ProcessLedgerError> {
        let claim = serde_json::to_value(&self.reclaim_claim).map_err(|error| {
            ProcessLedgerError::Store(format!(
                "failed to serialize reclaim claim for process_uuid {}: {error}",
                self.process_uuid
            ))
        })?;
        if let Some(metadata) = self.metadata_jsonb.as_object_mut() {
            metadata.insert("reclaim_claim".to_string(), claim);
            Ok(())
        } else {
            Err(ProcessLedgerError::Store(format!(
                "reclaim metadata for process_uuid {} is not a JSON object",
                self.process_uuid
            )))
        }
    }

    pub fn reclaim_stop(&self, exit_code: i32) -> ProcessStop {
        let mut metadata_jsonb = self.metadata_jsonb.clone();
        if let Some(metadata) = metadata_jsonb.as_object_mut() {
            metadata.insert(
                "reclaim_pending_stop".to_string(),
                serde_json::json!({
                    "exit_code": exit_code,
                    "stop_reason": "reclaim",
                    "claimant_uuid": self.reclaim_claim.claimant_uuid,
                    "kill_operation_uuid": self.reclaim_claim.kill_operation_uuid,
                    "generation": self.reclaim_claim.generation,
                }),
            );
            metadata.insert(
                "reclaim_last_kill_operation".to_string(),
                serde_json::json!({
                    "kill_operation_uuid": self.reclaim_claim.kill_operation_uuid,
                    "status": "succeeded",
                }),
            );
        }
        ProcessStop {
            process_uuid: self.process_uuid,
            os_pid: self.os_pid,
            parent_session_id: Some(self.parent_session_id.clone()),
            parent_process_id: self.parent_process_id,
            sandbox_adapter_id: self.sandbox_adapter_id.clone(),
            sandbox_internal_id: self.sandbox_internal_id.clone(),
            engine_kind: self.engine_kind,
            started_at: self.started_at,
            stopped_at: Utc::now(),
            exit_code: Some(exit_code),
            stop_reason: Some("reclaim".to_string()),
            model_artifact_sha256: self.model_artifact_sha256.clone(),
            work_profile_id: self.work_profile_id.clone(),
            owner_role: self.owner_role.clone(),
            owner_wp: self.owner_wp.clone(),
            role_id: self.role_id.clone(),
            wp_id: self.wp_id.clone(),
            mt_id: self.mt_id.clone(),
            runtime_owner: self.runtime_owner.clone(),
            sandbox_capabilities_snapshot: self.sandbox_capabilities_snapshot.clone(),
            metadata_jsonb,
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("{message}")]
pub struct KillError {
    message: String,
}

impl KillError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum KillOutcome {
    Killed,
    /// The kill completed, but store acknowledgement did not arrive within the
    /// bounded wait. The queued writer row remains retained for retry and the
    /// fenced PostgreSQL row remains recoverable without another kill.
    KilledPendingStop {
        error: String,
    },
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimedProcess {
    pub process_uuid: Uuid,
    pub engine_kind: ProcessEngineKind,
    pub sandbox_adapter_id: Option<String>,
    pub kill_result: KillOutcome,
    pub stop_event_kind: Option<LedgerEventKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimReport {
    pub session_id: String,
    pub trigger: ReclaimTrigger,
    pub processes_reclaimed: Vec<ReclaimedProcess>,
    pub total_duration_ms: u128,
}

#[async_trait]
pub trait ReclaimProcessStore: Send + Sync + 'static {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError>;

    /// Claim one exact process without allowing a lane-level teardown fallback
    /// to kill healthy sibling lanes that share the same coordinator session.
    /// Stores may override this with a single-row query; the conservative
    /// default atomically claims the session set and immediately releases every
    /// non-target claim before returning the requested row.
    async fn active_process_for_session(
        &self,
        session_id: &str,
        process_uuid: Uuid,
    ) -> Result<Option<ReclaimableProcess>, ProcessLedgerError> {
        let claimed = self.active_processes_for_session(session_id).await?;
        let mut target = None;
        let mut release_error = None;
        for process in claimed {
            if process.process_uuid == process_uuid {
                target = Some(process);
            } else if !process.kill_succeeded_pending_stop {
                if let Err(error) = self
                    .release_reclaim_claim(process.process_uuid, &process.reclaim_claim)
                    .await
                {
                    release_error.get_or_insert(error);
                }
            }
        }
        if let Some(error) = release_error {
            if let Some(target) = target.as_ref() {
                if !target.kill_succeeded_pending_stop {
                    let _ = self
                        .release_reclaim_claim(target.process_uuid, &target.reclaim_claim)
                        .await;
                }
            }
            return Err(error);
        }
        Ok(target)
    }

    async fn renew_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError>;

    async fn mark_reclaim_kill_succeeded(
        &self,
        stop: &ProcessStop,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError>;

    async fn mark_reclaim_kill_started(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError>;

    async fn release_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError>;

    async fn resolve_reclaim_kill_operation(
        &self,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
        status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError>;

    async fn in_progress_kill_operations_for_session(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError>;
}

#[async_trait]
pub trait SandboxKill: Send + Sync + 'static {
    /// Execute or coalesce one stable kill operation. Implementations must use
    /// `kill_operation_uuid` as their idempotency key across retries.
    async fn kill(&self, process_uuid: Uuid, kill_operation_uuid: Uuid) -> Result<(), KillError>;

    /// Query the adapter's authoritative idempotency record for crash recovery.
    async fn kill_operation_status(
        &self,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError>;
}

/// Production process killer for ledger rows whose owning adapter exposes an
/// OS process identity. The ledger's immutable executable hash and launch
/// process-generation identity are checked before termination, so a reused PID
/// cannot redirect stale reclaim to another process generation.
#[derive(Clone)]
pub struct ProductionSandboxKill {
    pool: PgPool,
    sandbox_registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
}

impl ProductionSandboxKill {
    pub fn new(pool: PgPool, _runtime: tokio::runtime::Handle) -> Self {
        let adapter_id =
            crate::sandbox::AdapterId::new(crate::sandbox::HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID);
        let mut registry = crate::sandbox::SandboxAdapterRegistry::new(adapter_id);
        registry.register(Arc::new(
            crate::sandbox::HandshakeNativeSandboxAdapter::new(),
        ));
        Self::with_registry(pool, Arc::new(registry))
    }

    pub fn with_registry(
        pool: PgPool,
        sandbox_registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
    ) -> Self {
        Self {
            pool,
            sandbox_registry,
        }
    }

    async fn identity(&self, process_uuid: Uuid) -> Result<ProductionKillIdentity, KillError> {
        tokio::time::timeout(
            Duration::from_secs(5),
            load_production_kill_identity(&self.pool, process_uuid),
        )
        .await
        .map_err(|_| {
            KillError::new(format!(
                "production reclaim identity lookup timed out for process {process_uuid}"
            ))
        })?
        .map_err(|error| KillError::new(error.to_string()))
    }

    fn owning_adapter(
        &self,
        identity: &ProductionKillIdentity,
    ) -> Result<Arc<dyn crate::sandbox::SandboxAdapter>, KillError> {
        self.sandbox_registry
            .get(&identity.detached.handle.adapter_id)
            .ok_or_else(|| {
                KillError::new(format!(
                    "process {} owning sandbox adapter {} is not registered",
                    identity.detached.process_uuid, identity.detached.handle.adapter_id
                ))
            })
    }
}

#[derive(Debug, Clone)]
struct ProductionKillIdentity {
    stopped: bool,
    detached: crate::sandbox::DetachedProcessIdentity,
    kill_operation_uuid: Option<Uuid>,
}

async fn load_production_kill_identity(
    pool: &PgPool,
    process_uuid: Uuid,
) -> Result<ProductionKillIdentity, ProcessLedgerError> {
    let authority = resolve_process_ledger_authority_relation(pool).await?;
    let sql = format!(
        r#"
        SELECT os_pid, started_at, stopped_at IS NOT NULL AS stopped,
               sandbox_adapter_id, sandbox_internal_id,
               metadata_jsonb::text AS metadata_jsonb
        FROM ONLY {}
        WHERE process_uuid = $1
        "#,
        authority.qualified_table
    );
    let row = sqlx::query(&sql)
        .bind(process_uuid)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| {
            ProcessLedgerError::Store(format!(
                "production reclaim identity missing for process {process_uuid}"
            ))
        })?;
    let metadata = json_text_column(&row, "metadata_jsonb")?;
    let executable_sha256 = metadata
        .get("effective_executable_sha256")
        .or_else(|| metadata.get("executable_sha256"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let os_creation_time_100ns = metadata
        .get("os_creation_time_100ns")
        .and_then(Value::as_u64);
    let kill_operation_uuid = metadata
        .get("reclaim_last_kill_operation")
        .and_then(|value| value.get("kill_operation_uuid"))
        .and_then(Value::as_str)
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|error| {
            ProcessLedgerError::Store(format!(
                "invalid production reclaim operation identity: {error}"
            ))
        })?;
    let os_pid = row
        .try_get::<Option<i64>, _>("os_pid")
        .map_err(ProcessLedgerError::from)?
        .map(pg_pid_to_u32)
        .transpose()?;
    let adapter_id = row
        .try_get::<Option<String>, _>("sandbox_adapter_id")?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            ProcessLedgerError::Store(format!(
                "production reclaim identity for process {process_uuid} has no owning sandbox adapter"
            ))
        })?;
    let handle_id = metadata
        .get("sandbox_handle_id")
        .and_then(Value::as_str)
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|error| {
            ProcessLedgerError::Store(format!(
                "invalid production sandbox handle identity for process {process_uuid}: {error}"
            ))
        })?
        .unwrap_or(process_uuid);
    let started_at = row.try_get::<DateTime<Utc>, _>("started_at")?;
    let sandbox_internal_id = row
        .try_get::<Option<String>, _>("sandbox_internal_id")?
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| process_uuid.to_string());
    Ok(ProductionKillIdentity {
        stopped: row.get("stopped"),
        detached: crate::sandbox::DetachedProcessIdentity {
            process_uuid,
            handle: crate::sandbox::ProcessHandle {
                id: handle_id,
                adapter_id: crate::sandbox::AdapterId::new(adapter_id),
                pid: os_pid,
                sandbox_internal_id,
                spawned_at_utc: started_at,
            },
            executable_sha256,
            os_creation_time_100ns,
        },
        kill_operation_uuid,
    })
}

#[async_trait]
impl SandboxKill for ProductionSandboxKill {
    async fn kill(&self, process_uuid: Uuid, kill_operation_uuid: Uuid) -> Result<(), KillError> {
        let identity = self.identity(process_uuid).await?;
        if identity.stopped {
            return Ok(());
        }
        if identity.kill_operation_uuid != Some(kill_operation_uuid) {
            return Err(KillError::new(format!(
                "process {process_uuid} kill operation does not match the durable reclaim fence"
            )));
        }
        let adapter = self.owning_adapter(&identity)?;
        adapter
            .reclaim_detached(&identity.detached, crate::sandbox::Signal::Kill)
            .await
            .map_err(|error| KillError::new(error.to_string()))
    }

    async fn kill_operation_status(
        &self,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError> {
        let identity = tokio::time::timeout(
            Duration::from_secs(5),
            load_production_kill_identity(&self.pool, process_uuid),
        )
        .await
        .map_err(|_| {
            KillError::new(format!(
                "production reclaim identity lookup timed out for process {process_uuid}"
            ))
        })?
        .map_err(|error| KillError::new(error.to_string()))?;
        if identity.stopped {
            return Ok(ReclaimKillOperationStatus::Succeeded);
        }
        if identity.kill_operation_uuid != Some(kill_operation_uuid) {
            return Ok(ReclaimKillOperationStatus::NotStarted);
        }
        let adapter = self.owning_adapter(&identity)?;
        let status = tokio::time::timeout(
            Duration::from_secs(10),
            adapter.detached_status(&identity.detached),
        )
        .await
        .map_err(|_| {
            KillError::new(format!(
                "owning adapter status timed out for process {process_uuid}"
            ))
        })?
        .map_err(|error| KillError::new(error.to_string()))?;
        Ok(match status {
            crate::sandbox::ProcessStatus::Running => ReclaimKillOperationStatus::InProgress,
            crate::sandbox::ProcessStatus::Exited { .. }
            | crate::sandbox::ProcessStatus::Killed { .. }
            | crate::sandbox::ProcessStatus::Orphaned => ReclaimKillOperationStatus::Succeeded,
            crate::sandbox::ProcessStatus::FailedToStart { .. } => {
                ReclaimKillOperationStatus::Failed
            }
        })
    }
}

#[async_trait]
pub trait ReclaimStopReservation: Send + 'static {
    async fn persist(
        self: Box<Self>,
        stop: ProcessStop,
        timeout: Duration,
    ) -> Result<(), ProcessLedgerError>;
}

pub trait ReclaimStopWriter: Send + Sync + 'static {
    fn reserve_reclaim_stop(&self) -> Result<Box<dyn ReclaimStopReservation>, ProcessLedgerError>;
}

pub struct Reclaim {
    store: Arc<dyn ReclaimProcessStore>,
    sandbox_kill: Arc<dyn SandboxKill>,
    stop_writer: Arc<dyn ReclaimStopWriter>,
    claim_renew_interval: Duration,
    kill_timeout: Duration,
    stop_ack_timeout: Duration,
}

impl Reclaim {
    pub fn new<S, K, W>(store: Arc<S>, sandbox_kill: Arc<K>, stop_writer: Arc<W>) -> Self
    where
        S: ReclaimProcessStore,
        K: SandboxKill,
        W: ReclaimStopWriter,
    {
        Self {
            store,
            sandbox_kill,
            stop_writer,
            claim_renew_interval: RECLAIM_CLAIM_RENEW_INTERVAL,
            kill_timeout: RECLAIM_KILL_TIMEOUT,
            stop_ack_timeout: RECLAIM_STOP_ACK_TIMEOUT,
        }
    }

    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub fn with_reclaim_timings_for_test(
        mut self,
        claim_renew_interval: Duration,
        stop_ack_timeout: Duration,
    ) -> Self {
        self.claim_renew_interval = claim_renew_interval;
        self.stop_ack_timeout = stop_ack_timeout;
        self
    }

    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub fn with_kill_timeout_for_test(mut self, kill_timeout: Duration) -> Self {
        self.kill_timeout = kill_timeout;
        self
    }

    pub async fn run(
        &self,
        session_id: &str,
        trigger: ReclaimTrigger,
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let active = self.store.active_processes_for_session(session_id).await?;
        self.run_claimed(session_id, trigger, started, active).await
    }

    pub async fn run_process(
        &self,
        session_id: &str,
        process_uuid: Uuid,
        trigger: ReclaimTrigger,
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let active = self
            .store
            .active_process_for_session(session_id, process_uuid)
            .await?
            .into_iter()
            .collect();
        self.run_claimed(session_id, trigger, started, active).await
    }

    async fn run_claimed(
        &self,
        session_id: &str,
        trigger: ReclaimTrigger,
        started: std::time::Instant,
        active: Vec<ReclaimableProcess>,
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let mut reclaimed = Vec::with_capacity(active.len());
        let mut active = active.into_iter();

        while let Some(mut process) = active.next() {
            let reservation = match self.stop_writer.reserve_reclaim_stop() {
                Ok(reservation) => reservation,
                Err(error) => {
                    self.release_unprocessed_claims_after_abort(
                        std::iter::once(&process).chain(active.as_slice().iter()),
                        "STOP reservation rejection",
                    )
                    .await;
                    return Err(error);
                }
            };

            let (kill_result, stop_event_kind) = if process.kill_succeeded_pending_stop {
                let stop = process.reclaim_stop(-1);
                match reservation.persist(stop, self.stop_ack_timeout).await {
                    Ok(()) => {
                        clear_completed_process_kill_fence(process.process_uuid);
                        (KillOutcome::Killed, Some(LedgerEventKind::Stop))
                    }
                    Err(error) => (
                        KillOutcome::KilledPendingStop {
                            error: error.to_string(),
                        },
                        None,
                    ),
                }
            } else {
                if let Err(error) = self
                    .store
                    .mark_reclaim_kill_started(process.process_uuid, &process.reclaim_claim)
                    .await
                {
                    drop(reservation);
                    self.release_unprocessed_claims_after_abort(
                        std::iter::once(&process).chain(active.as_slice().iter()),
                        "kill-start fence rejection",
                    )
                    .await;
                    return Err(error);
                }
                let (kill, renewal_error, renewed_claim, kill_fence) =
                    self.kill_with_claim_renewal(&process).await?;
                process.reclaim_claim = renewed_claim;
                process.sync_reclaim_claim_metadata()?;
                match kill {
                    Ok(()) => {
                        let stop = process.reclaim_stop(-1);
                        let pending_mark_error = self
                            .store
                            .mark_reclaim_kill_succeeded(&stop, &process.reclaim_claim)
                            .await
                            .err();
                        let stop_persisted = reservation.persist(stop, self.stop_ack_timeout).await;
                        if renewal_error.is_none()
                            && pending_mark_error.is_none()
                            && stop_persisted.is_ok()
                        {
                            clear_process_kill_fence(process.process_uuid, &kill_fence);
                            (KillOutcome::Killed, Some(LedgerEventKind::Stop))
                        } else {
                            let mut errors = Vec::new();
                            if let Some(error) = renewal_error {
                                errors.push(format!("claim renewal failed: {error}"));
                            }
                            if let Some(error) = pending_mark_error {
                                errors.push(format!("pending-stop marker failed: {error}"));
                            }
                            if let Err(error) = stop_persisted {
                                errors.push(format!("STOP durability failed: {error}"));
                            } else {
                                errors.push(
                                    "STOP was durable but reclaim ownership continuity was not proven"
                                        .to_string(),
                                );
                            }
                            (
                                KillOutcome::KilledPendingStop {
                                    error: errors.join("; "),
                                },
                                None,
                            )
                        }
                    }
                    Err(error) => {
                        drop(reservation);
                        self.store
                            .release_reclaim_claim(process.process_uuid, &process.reclaim_claim)
                            .await?;
                        clear_process_kill_fence(process.process_uuid, &kill_fence);
                        (
                            KillOutcome::Failed {
                                error: error.message().to_string(),
                            },
                            None,
                        )
                    }
                }
            };
            reclaimed.push(ReclaimedProcess {
                process_uuid: process.process_uuid,
                engine_kind: process.engine_kind,
                sandbox_adapter_id: process.sandbox_adapter_id,
                kill_result,
                stop_event_kind,
            });
        }

        Ok(ReclaimReport {
            session_id: session_id.to_string(),
            trigger,
            processes_reclaimed: reclaimed,
            total_duration_ms: started.elapsed().as_millis(),
        })
    }

    /// Reconcile a crash-left kill operation from the owning adapter's
    /// authoritative idempotency record. Unknown/in-progress evidence leaves
    /// PostgreSQL unchanged and truthfully open; terminal/not-started evidence
    /// advances the shared recovery state without trusting a caller-supplied
    /// success flag.
    pub async fn reconcile_kill_operation(
        &self,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, ProcessLedgerError> {
        let status = self
            .sandbox_kill
            .kill_operation_status(process_uuid, kill_operation_uuid)
            .await
            .map_err(|error| {
                ProcessLedgerError::Store(format!(
                    "kill-operation status query failed for process {process_uuid} operation {kill_operation_uuid}: {error}"
                ))
            })?;
        if matches!(
            status,
            ReclaimKillOperationStatus::Succeeded
                | ReclaimKillOperationStatus::Failed
                | ReclaimKillOperationStatus::NotStarted
        ) {
            self.store
                .resolve_reclaim_kill_operation(process_uuid, kill_operation_uuid, status)
                .await?;
        }
        Ok(status)
    }

    pub async fn reconcile_in_progress_for_session(
        &self,
        session_id: &str,
    ) -> Result<ReclaimKillOperationSweep, ProcessLedgerError> {
        let recoverable = self
            .store
            .in_progress_kill_operations_for_session(session_id, RECLAIM_IN_PROGRESS_RECOVERY_LIMIT)
            .await?;
        let mut operations = Vec::with_capacity(recoverable.len());
        let mut state_advanced = false;
        for candidate in recoverable {
            let operation = match candidate {
                ReclaimKillOperationCandidate::Operation { operation } => operation,
                ReclaimKillOperationCandidate::Malformed {
                    process_identity,
                    kill_operation_identity,
                    error,
                } => {
                    operations.push(ReclaimKillOperationSweepEntry {
                        candidate: ReclaimKillOperationCandidate::Malformed {
                            process_identity,
                            kill_operation_identity,
                            error: error.clone(),
                        },
                        outcome: ReclaimKillOperationSweepOutcome::MalformedRecoveryRow { error },
                    });
                    continue;
                }
            };
            let status = match self
                .sandbox_kill
                .kill_operation_status(operation.process_uuid, operation.kill_operation_uuid)
                .await
            {
                Ok(status) => status,
                Err(error) => {
                    operations.push(ReclaimKillOperationSweepEntry {
                        candidate: ReclaimKillOperationCandidate::Operation { operation },
                        outcome: ReclaimKillOperationSweepOutcome::StatusQueryFailed {
                            error: error.message().to_string(),
                        },
                    });
                    continue;
                }
            };
            if matches!(
                status,
                ReclaimKillOperationStatus::Succeeded
                    | ReclaimKillOperationStatus::Failed
                    | ReclaimKillOperationStatus::NotStarted
            ) {
                match self
                    .store
                    .resolve_reclaim_kill_operation(
                        operation.process_uuid,
                        operation.kill_operation_uuid,
                        status,
                    )
                    .await
                {
                    Ok(()) => {
                        state_advanced = true;
                        operations.push(ReclaimKillOperationSweepEntry {
                            candidate: ReclaimKillOperationCandidate::Operation { operation },
                            outcome: ReclaimKillOperationSweepOutcome::StateAdvanced { status },
                        });
                    }
                    Err(error) => operations.push(ReclaimKillOperationSweepEntry {
                        candidate: ReclaimKillOperationCandidate::Operation { operation },
                        outcome: ReclaimKillOperationSweepOutcome::StateTransitionFailed {
                            status,
                            error: error.to_string(),
                        },
                    }),
                }
            } else {
                operations.push(ReclaimKillOperationSweepEntry {
                    candidate: ReclaimKillOperationCandidate::Operation { operation },
                    outcome: ReclaimKillOperationSweepOutcome::StateOpen { status },
                });
            }
        }
        let (reclaim_report, reclaim_error) = if state_advanced {
            match self.run(session_id, ReclaimTrigger::Stale).await {
                Ok(report) => (Some(report), None),
                Err(error) => (None, Some(error.to_string())),
            }
        } else {
            (None, None)
        };
        Ok(ReclaimKillOperationSweep {
            operations,
            reclaim_report,
            reclaim_error,
        })
    }

    async fn release_unprocessed_claims_after_abort<'a>(
        &self,
        processes: impl Iterator<Item = &'a ReclaimableProcess>,
        context: &'static str,
    ) {
        for process in processes {
            if process.kill_succeeded_pending_stop {
                continue;
            }
            if let Err(error) = self
                .store
                .release_reclaim_claim(process.process_uuid, &process.reclaim_claim)
                .await
            {
                tracing::error!(
                    process_uuid = %process.process_uuid,
                    error = %error,
                    context,
                    "failed to release an unprocessed reclaim claim after abort"
                );
            }
        }
    }

    async fn kill_with_claim_renewal(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<
        (
            Result<(), KillError>,
            Option<ProcessLedgerError>,
            ReclaimClaim,
            Arc<ProcessKillFence>,
        ),
        ProcessLedgerError,
    > {
        let process_uuid = process.process_uuid;
        let (kill_fence, owns_kill) = acquire_process_kill_fence(process_uuid);
        let mut claim = process.reclaim_claim.clone();
        let mut renewal_error = None;
        let renew_every = if self.claim_renew_interval.is_zero() {
            Duration::from_millis(1)
        } else {
            self.claim_renew_interval.min(RECLAIM_CLAIM_TTL / 2)
        };
        let mut renewal = time::interval(renew_every);
        renewal.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // `interval` ticks immediately; the claim was just committed, so wait
        // one full interval before the first renewal.
        renewal.tick().await;
        let kill_timeout = if self.kill_timeout.is_zero() {
            Duration::from_millis(1)
        } else {
            self.kill_timeout
        };
        let kill_deadline = time::sleep(kill_timeout);
        tokio::pin!(kill_deadline);

        if owns_kill {
            let killer = Arc::clone(&self.sandbox_kill);
            let kill_operation_uuid = process.reclaim_claim.kill_operation_uuid;
            let mut kill_task =
                tokio::spawn(async move { killer.kill(process_uuid, kill_operation_uuid).await });
            loop {
                tokio::select! {
                    joined = &mut kill_task => {
                    let result = joined.unwrap_or_else(|error| {
                        Err(KillError::new(format!(
                            "reclaim kill task for process {process_uuid} failed to join: {error}"
                        )))
                    });
                    match kill_fence.result.lock() {
                        Ok(mut published) => *published = Some(result.clone()),
                        Err(poisoned) => *poisoned.into_inner() = Some(result.clone()),
                    }
                    kill_fence.completed.notify_waiters();
                    return Ok((result, renewal_error, claim, kill_fence));
                    }
                    _ = renewal.tick() => {
                        match self.store.renew_reclaim_claim(process_uuid, &claim).await {
                            Ok(renewed) => {
                                claim = renewed;
                                renewal_error = None;
                            }
                            Err(error) => renewal_error = Some(error),
                        }
                    }
                    _ = &mut kill_deadline => {
                        kill_task.abort();
                        let _ = (&mut kill_task).await;
                        let result = Err(KillError::new(format!(
                            "reclaim kill operation for process {process_uuid} exceeded {}ms",
                            kill_timeout.as_millis()
                        )));
                        match kill_fence.result.lock() {
                            Ok(mut published) => *published = Some(result.clone()),
                            Err(poisoned) => *poisoned.into_inner() = Some(result.clone()),
                        }
                        kill_fence.completed.notify_waiters();
                        return Ok((result, renewal_error, claim, kill_fence));
                    }
                }
            }
        } else {
            loop {
                let completed = kill_fence.completed.notified();
                tokio::pin!(completed);
                if let Some(result) = match kill_fence.result.lock() {
                    Ok(result) => result.clone(),
                    Err(poisoned) => poisoned.into_inner().clone(),
                } {
                    return Ok((result, renewal_error, claim, Arc::clone(&kill_fence)));
                }
                tokio::select! {
                    _ = &mut completed => {}
                    _ = renewal.tick() => {
                        match self.store.renew_reclaim_claim(process_uuid, &claim).await {
                            Ok(renewed) => {
                                claim = renewed;
                                renewal_error = None;
                            }
                            Err(error) => renewal_error = Some(error),
                        }
                    }
                    _ = &mut kill_deadline => {
                        return Ok((
                            Err(KillError::new(format!(
                                "coalesced reclaim wait for process {process_uuid} exceeded {}ms",
                                kill_timeout.as_millis()
                            ))),
                            renewal_error,
                            claim,
                            Arc::clone(&kill_fence),
                        ));
                    }
                }
            }
        }
    }
}

pub fn reclaim_handle<S, K, W>(store: Arc<S>, sandbox_kill: Arc<K>, stop_writer: Arc<W>) -> Reclaim
where
    S: ReclaimProcessStore,
    K: SandboxKill,
    W: ReclaimStopWriter,
{
    Reclaim::new(store, sandbox_kill, stop_writer)
}

struct WriterReclaimStopReservation {
    reserved: ReservedProcessStop,
}

#[async_trait]
impl ReclaimStopReservation for WriterReclaimStopReservation {
    async fn persist(
        self: Box<Self>,
        stop: ProcessStop,
        timeout: Duration,
    ) -> Result<(), ProcessLedgerError> {
        self.reserved
            .commit_with_durable_ack(stop)?
            .wait(timeout)
            .await
    }
}

impl ReclaimStopWriter for ProcessLedgerWriter {
    fn reserve_reclaim_stop(&self) -> Result<Box<dyn ReclaimStopReservation>, ProcessLedgerError> {
        Ok(Box::new(WriterReclaimStopReservation {
            reserved: self.try_reserve_reclaim_stop()?,
        }))
    }
}

impl ReclaimStopWriter for super::LedgerBatcher {
    fn reserve_reclaim_stop(&self) -> Result<Box<dyn ReclaimStopReservation>, ProcessLedgerError> {
        Ok(Box::new(WriterReclaimStopReservation {
            reserved: self.try_reserve_reclaim_stop()?,
        }))
    }
}

impl PostgresProcessLedgerStore {
    /// Resolve a crash-left `reclaim_kill_in_progress` operation from verified
    /// sandbox evidence. `Succeeded` becomes retryable pending STOP without a
    /// kill; `Failed`/`NotStarted` release the claim so the same stable
    /// operation id can be retried idempotently.
    async fn apply_reclaim_kill_operation_status(
        &self,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
        status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        let authority = resolve_process_ledger_authority_relation(self.pool()).await?;
        let sql = format!(
            r#"
            UPDATE ONLY {}
            SET stop_reason = CASE
                    WHEN $3 = 'succeeded' THEN 'kill_succeeded_pending_stop'
                    ELSE NULL
                END,
                metadata_jsonb = CASE
                    WHEN $3 = 'succeeded' THEN jsonb_set(
                        metadata_jsonb,
                        '{{reclaim_last_kill_operation,status}}',
                        to_jsonb($3::text),
                        false
                    )
                    ELSE jsonb_set(
                        metadata_jsonb - 'reclaim_claim',
                        '{{reclaim_last_kill_operation,status}}',
                        to_jsonb($3::text),
                        false
                    )
                END
            WHERE process_uuid = $1
              AND stopped_at IS NULL
              AND stop_reason = 'reclaim_kill_in_progress'
              AND metadata_jsonb->'reclaim_last_kill_operation'->>'kill_operation_uuid' = $2
            "#,
            authority.qualified_table
        );
        let mut tx = self.pool().begin().await?;
        pin_transaction_search_path(&mut tx, &authority.schema).await?;
        lock_process_ledger_authority_relation(
            &mut tx,
            &authority,
            ProcessLedgerAuthorityLockMode::RowExclusive,
        )
        .await?;
        require_postgres_crash_durability(&mut tx, "reclaim kill-operation resolution").await?;
        let result = sqlx::query(&sql)
            .bind(process_uuid)
            .bind(kill_operation_uuid.to_string())
            .bind(status.as_str())
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() != 1 {
            return Err(ProcessLedgerError::Store(format!(
                "reclaim kill-operation resolution did not match process_uuid {process_uuid} operation {kill_operation_uuid}"
            )));
        }
        assert_process_ledger_authority_relation(&mut tx, &authority).await?;
        require_synchronous_commit(&mut tx, "reclaim kill-operation resolution commit").await?;
        tx.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl ReclaimProcessStore for PostgresProcessLedgerStore {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        // MT-008: the open-row claim is atomic in the
        // UPDATE...RETURNING. We map the returned rows BEFORE committing so that a
        // row-decode failure rolls the claim back rather than leaving rows marked
        // claimed but never returned to the caller (which would orphan them:
        // claimed yet never killed). Commit only after every row decodes cleanly.
        let authority = resolve_process_ledger_authority_relation(self.pool()).await?;
        let mut tx = self.pool().begin().await?;
        prepare_pidless_reclaim_transaction(
            &mut tx,
            &authority,
            None,
            ProcessLedgerAuthorityLockMode::RowExclusive,
        )
        .await?;
        let claimant_uuid = Uuid::now_v7();
        let rows = sqlx::query(POSTGRES_ACTIVE_RECLAIM_QUERY_SQL)
            .bind(session_id)
            .bind(claimant_uuid.to_string())
            .fetch_all(&mut *tx)
            .await?;

        let claimed: Result<Vec<ReclaimableProcess>, ProcessLedgerError> = rows
            .into_iter()
            .map(|row| {
                let process_uuid_raw: String = row.get("process_uuid");
                let engine_kind_raw: String = row.get("engine_kind");
                let metadata_jsonb = json_text_column(&row, "metadata_jsonb")?;
                let reclaim_claim = reclaim_claim_from_metadata(&metadata_jsonb)?;
                let stop_reason: String = row.get("stop_reason");
                Ok(ReclaimableProcess {
                    process_uuid: Uuid::parse_str(&process_uuid_raw).map_err(|error| {
                        ProcessLedgerError::Store(format!(
                            "invalid process_uuid in reclaim query: {error}"
                        ))
                    })?,
                    os_pid: row
                        .try_get::<Option<i64>, _>("os_pid")
                        .map_err(ProcessLedgerError::from)?
                        .map(pg_pid_to_u32)
                        .transpose()?,
                    parent_session_id: row.get("parent_session_id"),
                    parent_process_id: row
                        .try_get::<Option<String>, _>("parent_process_id")
                        .map_err(ProcessLedgerError::from)?
                        .map(|raw| {
                            Uuid::parse_str(&raw).map_err(|error| {
                                ProcessLedgerError::Store(format!(
                                    "invalid parent_process_id in reclaim query: {error}"
                                ))
                            })
                        })
                        .transpose()?,
                    sandbox_adapter_id: row.get("sandbox_adapter_id"),
                    sandbox_internal_id: row.get("sandbox_internal_id"),
                    engine_kind: ProcessEngineKind::try_from(engine_kind_raw.as_str())
                        .map_err(ProcessLedgerError::Store)?,
                    started_at: row.get("started_at"),
                    model_artifact_sha256: row.get("model_artifact_sha256"),
                    work_profile_id: row.get("work_profile_id"),
                    owner_role: row.get("owner_role"),
                    owner_wp: row.get("owner_wp"),
                    role_id: row.get("role_id"),
                    wp_id: row.get("wp_id"),
                    mt_id: row.get("mt_id"),
                    runtime_owner: process_runtime_owner_from_row(&row)?,
                    sandbox_capabilities_snapshot: json_text_column(
                        &row,
                        "sandbox_capabilities_snapshot",
                    )?,
                    metadata_jsonb,
                    reclaim_claim,
                    kill_succeeded_pending_stop: stop_reason == "kill_succeeded_pending_stop",
                })
            })
            .collect();

        match claimed {
            Ok(processes) => {
                force_all_constraints_immediate(&mut tx).await?;
                assert_process_ledger_authority_relation(&mut tx, &authority).await?;
                let process_ids: Vec<Uuid> = processes
                    .iter()
                    .map(|process| process.process_uuid)
                    .collect();
                let expected_rows = i64::try_from(process_ids.len()).map_err(|_| {
                    ProcessLedgerError::Store(
                        "session reclaim claim count exceeded PostgreSQL bigint readback range"
                            .to_string(),
                    )
                })?;
                let final_readback_sql = format!(
                    r#"
                    SELECT pg_catalog.count(*) = $2
                    FROM ONLY {}
                    WHERE process_uuid = ANY($1)
                      AND stopped_at IS NULL
                      AND stop_reason IN ('reclaim_claimed', 'kill_succeeded_pending_stop')
                      AND metadata_jsonb->'reclaim_claim'->>'claimant_uuid' = $3
                    "#,
                    authority.qualified_table
                );
                let final_readback_valid: bool = sqlx::query_scalar(&final_readback_sql)
                    .bind(&process_ids)
                    .bind(expected_rows)
                    .bind(claimant_uuid.to_string())
                    .fetch_one(&mut *tx)
                    .await?;
                if !final_readback_valid {
                    return Err(ProcessLedgerError::Store(
                        "session reclaim final lifecycle readback did not match the atomic claim"
                            .to_string(),
                    ));
                }
                require_synchronous_commit(&mut tx, "session process reclaim claim commit").await?;
                tx.commit().await?;
                Ok(processes)
            }
            Err(error) => {
                // Roll the claim back so the rows stay reclaimable; surface the
                // decode error to the caller.
                let _ = tx.rollback().await;
                Err(error)
            }
        }
    }

    async fn mark_reclaim_kill_started(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        let authority = resolve_process_ledger_authority_relation(self.pool()).await?;
        let sql = format!(
            r#"
            UPDATE ONLY {}
            SET stop_reason = 'reclaim_kill_in_progress',
                metadata_jsonb = jsonb_set(
                    metadata_jsonb,
                    '{{reclaim_last_kill_operation}}',
                    jsonb_build_object(
                        'kill_operation_uuid', metadata_jsonb->'reclaim_claim'->>'kill_operation_uuid',
                        'status', 'in_progress',
                        'recorded_at_unix_ms', (extract(epoch FROM clock_timestamp()) * 1000)::bigint
                    ),
                    true
                )
            WHERE process_uuid = $1
              AND stopped_at IS NULL
              AND stop_reason = 'reclaim_claimed'
              AND metadata_jsonb->'reclaim_claim'->>'claimant_uuid' = $2
              AND (metadata_jsonb->'reclaim_claim'->>'generation')::bigint = $3
              AND metadata_jsonb->'reclaim_claim'->>'kill_operation_uuid' = $4
            "#,
            authority.qualified_table
        );
        let mut tx = self.pool().begin().await?;
        pin_transaction_search_path(&mut tx, &authority.schema).await?;
        lock_process_ledger_authority_relation(
            &mut tx,
            &authority,
            ProcessLedgerAuthorityLockMode::RowExclusive,
        )
        .await?;
        require_postgres_crash_durability(&mut tx, "reclaim kill-in-progress fence").await?;
        let result = sqlx::query(&sql)
            .bind(process_uuid)
            .bind(claim.claimant_uuid.to_string())
            .bind(claim.generation as i64)
            .bind(claim.kill_operation_uuid.to_string())
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() != 1 {
            return Err(ProcessLedgerError::Store(format!(
                "reclaim kill-start fence lost ownership for process_uuid {process_uuid}"
            )));
        }
        assert_process_ledger_authority_relation(&mut tx, &authority).await?;
        require_synchronous_commit(&mut tx, "reclaim kill-in-progress fence commit").await?;
        tx.commit().await?;
        Ok(())
    }

    async fn renew_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        let authority = resolve_process_ledger_authority_relation(self.pool()).await?;
        let sql = format!(
            r#"
            UPDATE ONLY {}
            SET metadata_jsonb = jsonb_set(
                    jsonb_set(
                        metadata_jsonb,
                        '{{reclaim_claim,claimed_at_unix_ms}}',
                        to_jsonb((extract(epoch FROM clock_timestamp()) * 1000)::bigint),
                        false
                    ),
                    '{{reclaim_claim,lease_expires_at_unix_ms}}',
                    to_jsonb((extract(epoch FROM clock_timestamp()) * 1000)::bigint + 30000::bigint),
                    false
                )
            WHERE process_uuid = $1
              AND stopped_at IS NULL
              AND stop_reason IN ('reclaim_claimed', 'reclaim_kill_in_progress', 'kill_succeeded_pending_stop')
              AND metadata_jsonb->'reclaim_claim'->>'claimant_uuid' = $2
              AND (metadata_jsonb->'reclaim_claim'->>'generation')::bigint = $3
              AND metadata_jsonb->'reclaim_claim'->>'kill_operation_uuid' = $4
            RETURNING
                (metadata_jsonb->'reclaim_claim'->>'claimed_at_unix_ms')::bigint AS claimed_at_unix_ms,
                (metadata_jsonb->'reclaim_claim'->>'lease_expires_at_unix_ms')::bigint AS lease_expires_at_unix_ms
            "#,
            authority.qualified_table
        );
        let row = sqlx::query(&sql)
            .bind(process_uuid)
            .bind(claim.claimant_uuid.to_string())
            .bind(i64::try_from(claim.generation).map_err(|_| {
                ProcessLedgerError::Store(
                    "reclaim claim generation exceeds PostgreSQL bigint".into(),
                )
            })?)
            .bind(claim.kill_operation_uuid.to_string())
            .fetch_optional(self.pool())
            .await?
            .ok_or_else(|| {
                ProcessLedgerError::Store(format!(
                    "reclaim claim ownership lost while renewing process {process_uuid}"
                ))
            })?;
        Ok(ReclaimClaim {
            claimant_uuid: claim.claimant_uuid,
            kill_operation_uuid: claim.kill_operation_uuid,
            generation: claim.generation,
            claimed_at_unix_ms: row.get("claimed_at_unix_ms"),
            lease_expires_at_unix_ms: row.get("lease_expires_at_unix_ms"),
        })
    }

    async fn mark_reclaim_kill_succeeded(
        &self,
        stop: &ProcessStop,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        let authority = resolve_process_ledger_authority_relation(self.pool()).await?;
        let mut tx = self.pool().begin().await?;
        prepare_pidless_reclaim_transaction(
            &mut tx,
            &authority,
            None,
            ProcessLedgerAuthorityLockMode::RowExclusive,
        )
        .await?;
        let sql = format!(
            r#"
            UPDATE ONLY {}
            SET stop_reason = 'kill_succeeded_pending_stop',
                metadata_jsonb = jsonb_set(
                    jsonb_set(
                        metadata_jsonb,
                        '{{reclaim_pending_stop}}',
                        $4::jsonb->'reclaim_pending_stop',
                        true
                    ),
                    '{{reclaim_last_kill_operation}}',
                    $4::jsonb->'reclaim_last_kill_operation',
                    true
                )
            WHERE process_uuid = $1
              AND stopped_at IS NULL
              AND stop_reason IN ('reclaim_claimed', 'reclaim_kill_in_progress')
              AND metadata_jsonb->'reclaim_claim'->>'claimant_uuid' = $2
              AND (metadata_jsonb->'reclaim_claim'->>'generation')::bigint = $3
              AND metadata_jsonb->'reclaim_claim'->>'kill_operation_uuid' = $5
            "#,
            authority.qualified_table
        );
        let result = sqlx::query(&sql)
            .bind(stop.process_uuid)
            .bind(claim.claimant_uuid.to_string())
            .bind(i64::try_from(claim.generation).map_err(|_| {
                ProcessLedgerError::Store(
                    "reclaim claim generation exceeds PostgreSQL bigint".into(),
                )
            })?)
            .bind(stop.metadata_jsonb.clone())
            .bind(claim.kill_operation_uuid.to_string())
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() != 1 {
            return Err(ProcessLedgerError::Store(format!(
                "reclaim claim ownership lost before pending STOP for process {}",
                stop.process_uuid
            )));
        }
        force_all_constraints_immediate(&mut tx).await?;
        assert_process_ledger_authority_relation(&mut tx, &authority).await?;
        require_synchronous_commit(&mut tx, "reclaim kill-succeeded pending STOP commit").await?;
        tx.commit().await?;
        Ok(())
    }

    async fn release_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        let authority = resolve_process_ledger_authority_relation(self.pool()).await?;
        let sql = format!(
            r#"
            UPDATE ONLY {}
            SET stop_reason = NULL,
                metadata_jsonb = jsonb_set(
                    COALESCE(metadata_jsonb, '{{}}'::jsonb) - 'reclaim_claim',
                    '{{reclaim_last_kill_operation}}',
                    jsonb_build_object(
                        'kill_operation_uuid', metadata_jsonb->'reclaim_claim'->>'kill_operation_uuid',
                        'status', CASE
                            WHEN stop_reason = 'reclaim_kill_in_progress' THEN 'failed'
                            ELSE 'not_started'
                        END,
                        'recorded_at_unix_ms', (extract(epoch FROM clock_timestamp()) * 1000)::bigint
                    ),
                    true
                )
            WHERE process_uuid = $1
              AND stopped_at IS NULL
              AND stop_reason IN ('reclaim_claimed', 'reclaim_kill_in_progress')
              AND metadata_jsonb->'reclaim_claim'->>'claimant_uuid' = $2
              AND (metadata_jsonb->'reclaim_claim'->>'generation')::bigint = $3
              AND metadata_jsonb->'reclaim_claim'->>'kill_operation_uuid' = $4
            "#,
            authority.qualified_table
        );
        let mut tx = self.pool().begin().await?;
        pin_transaction_search_path(&mut tx, &authority.schema).await?;
        lock_process_ledger_authority_relation(
            &mut tx,
            &authority,
            ProcessLedgerAuthorityLockMode::RowExclusive,
        )
        .await?;
        require_postgres_crash_durability(&mut tx, "reclaim claim release").await?;
        let result = sqlx::query(&sql)
            .bind(process_uuid)
            .bind(claim.claimant_uuid.to_string())
            .bind(i64::try_from(claim.generation).map_err(|_| {
                ProcessLedgerError::Store(
                    "reclaim claim generation exceeds PostgreSQL bigint".into(),
                )
            })?)
            .bind(claim.kill_operation_uuid.to_string())
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() != 1 {
            return Err(ProcessLedgerError::Store(format!(
                "failed to release open reclaim claim for process {process_uuid}"
            )));
        }
        assert_process_ledger_authority_relation(&mut tx, &authority).await?;
        require_synchronous_commit(&mut tx, "reclaim claim release commit").await?;
        tx.commit().await?;
        Ok(())
    }

    async fn resolve_reclaim_kill_operation(
        &self,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
        status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        if matches!(
            status,
            ReclaimKillOperationStatus::InProgress | ReclaimKillOperationStatus::Unknown
        ) {
            return Err(ProcessLedgerError::InvalidConfig(
                "non-terminal kill-operation evidence must remain open and cannot mutate recovery state"
                    .to_string(),
            ));
        }
        self.apply_reclaim_kill_operation_status(process_uuid, kill_operation_uuid, status)
            .await
    }

    async fn in_progress_kill_operations_for_session(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        if limit == 0 || limit > RECLAIM_IN_PROGRESS_RECOVERY_LIMIT {
            return Err(ProcessLedgerError::InvalidConfig(format!(
                "in-progress reclaim recovery limit must be 1..={RECLAIM_IN_PROGRESS_RECOVERY_LIMIT}"
            )));
        }
        let authority = resolve_process_ledger_authority_relation(self.pool()).await?;
        let sql = format!(
            r#"
            SELECT process_uuid::text,
                   metadata_jsonb->'reclaim_last_kill_operation'->>'kill_operation_uuid'
                       AS kill_operation_uuid
            FROM ONLY {}
            WHERE parent_session_id = $1
              AND stopped_at IS NULL
              AND stop_reason = 'reclaim_kill_in_progress'
            ORDER BY started_at, process_uuid
            LIMIT $2
            "#,
            authority.qualified_table
        );
        let rows = sqlx::query(&sql)
            .bind(session_id)
            .bind(limit as i64)
            .fetch_all(self.pool())
            .await?;
        rows.into_iter()
            .map(|row| {
                let process_identity: String = row.try_get("process_uuid").map_err(|error| {
                    ProcessLedgerError::Store(format!(
                        "invalid in-progress process_uuid column: {error}"
                    ))
                })?;
                let kill_operation_uuid: Option<String> =
                    row.try_get("kill_operation_uuid").map_err(|error| {
                        ProcessLedgerError::Store(format!(
                            "invalid in-progress kill_operation_uuid column: {error}"
                        ))
                    })?;
                let process_uuid = match Uuid::parse_str(&process_identity) {
                    Ok(process_uuid) => process_uuid,
                    Err(error) => {
                        return Ok(ReclaimKillOperationCandidate::Malformed {
                            process_identity,
                            kill_operation_identity: kill_operation_uuid,
                            error: format!("invalid in-progress process_uuid: {error}"),
                        });
                    }
                };
                let Some(kill_operation_identity) = kill_operation_uuid else {
                    return Ok(ReclaimKillOperationCandidate::Malformed {
                        process_identity,
                        kill_operation_identity: None,
                        error: format!(
                            "in-progress process {process_uuid} is missing kill_operation_uuid"
                        ),
                    });
                };
                let kill_operation_uuid = match Uuid::parse_str(&kill_operation_identity) {
                    Ok(kill_operation_uuid) => kill_operation_uuid,
                    Err(error) => {
                        return Ok(ReclaimKillOperationCandidate::Malformed {
                            process_identity,
                            kill_operation_identity: Some(kill_operation_identity),
                            error: format!("invalid in-progress kill_operation_uuid: {error}"),
                        });
                    }
                };
                Ok(ReclaimKillOperationCandidate::Operation {
                    operation: ReclaimKillOperation {
                        process_uuid,
                        kill_operation_uuid,
                    },
                })
            })
            .collect()
    }
}

fn reclaim_claim_from_metadata(
    metadata: &serde_json::Value,
) -> Result<ReclaimClaim, ProcessLedgerError> {
    let claim = metadata.get("reclaim_claim").ok_or_else(|| {
        ProcessLedgerError::Store("claimed lifecycle row is missing reclaim_claim metadata".into())
    })?;
    let claimant_uuid = claim
        .get("claimant_uuid")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProcessLedgerError::Store("reclaim claim is missing claimant_uuid".into())
        })?;
    Ok(ReclaimClaim {
        claimant_uuid: Uuid::parse_str(claimant_uuid).map_err(|error| {
            ProcessLedgerError::Store(format!("invalid reclaim claimant_uuid: {error}"))
        })?,
        kill_operation_uuid: claim
            .get("kill_operation_uuid")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProcessLedgerError::Store("reclaim claim is missing kill_operation_uuid".into())
            })
            .and_then(|value| {
                Uuid::parse_str(value).map_err(|error| {
                    ProcessLedgerError::Store(format!(
                        "invalid reclaim kill_operation_uuid: {error}"
                    ))
                })
            })?,
        generation: claim
            .get("generation")
            .and_then(Value::as_u64)
            .ok_or_else(|| ProcessLedgerError::Store("invalid reclaim generation".into()))?,
        claimed_at_unix_ms: claim
            .get("claimed_at_unix_ms")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                ProcessLedgerError::Store("invalid reclaim claimed_at_unix_ms".into())
            })?,
        lease_expires_at_unix_ms: claim
            .get("lease_expires_at_unix_ms")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                ProcessLedgerError::Store("invalid reclaim lease_expires_at_unix_ms".into())
            })?,
    })
}

fn json_text_column(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<serde_json::Value, ProcessLedgerError> {
    let raw = row
        .try_get::<Option<String>, _>(column)
        .map_err(ProcessLedgerError::from)?
        .unwrap_or_else(|| "{}".to_string());
    serde_json::from_str(&raw).map_err(|error| {
        ProcessLedgerError::Store(format!("invalid JSONB column {column}: {error}"))
    })
}

fn process_runtime_owner_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<ProcessRuntimeOwner>, ProcessLedgerError> {
    let instance_id = row.try_get::<Option<String>, _>("owner_runtime_instance_id")?;
    let host_scope_id = row.try_get::<Option<String>, _>("owner_host_scope_id")?;
    let lease_schema_id = row.try_get::<Option<String>, _>("owner_lease_schema_id")?;
    let lease_protocol = row.try_get::<Option<String>, _>("owner_lease_protocol")?;
    let lease_address = row.try_get::<Option<String>, _>("owner_lease_address")?;
    let lease_port = row.try_get::<Option<i32>, _>("owner_lease_port")?;
    match (
        instance_id,
        host_scope_id,
        lease_schema_id,
        lease_protocol,
        lease_address,
        lease_port,
    ) {
        (None, None, None, None, None, None) => Ok(None),
        (
            Some(instance_id),
            Some(host_scope_id),
            Some(lease_schema_id),
            Some(lease_protocol),
            Some(lease_address),
            Some(lease_port),
        ) => {
            let runtime_instance_id = Uuid::parse_str(&instance_id).map_err(|error| {
                ProcessLedgerError::Store(format!(
                    "invalid owner_runtime_instance_id in reclaim query: {error}"
                ))
            })?;
            let lease_port = u16::try_from(lease_port).map_err(|_| {
                ProcessLedgerError::Store(
                    "owner_lease_port in reclaim query is outside 1..=65535".to_string(),
                )
            })?;
            if lease_port == 0 {
                return Err(ProcessLedgerError::Store(
                    "owner_lease_port in reclaim query must not be zero".to_string(),
                ));
            }
            Ok(Some(ProcessRuntimeOwner {
                runtime_instance_id,
                host_scope_id,
                lease_schema_id,
                lease_protocol,
                lease_address,
                lease_port,
            }))
        }
        _ => Err(ProcessLedgerError::Store(
            "partial typed runtime-owner identity in reclaim query".to_string(),
        )),
    }
}

fn pg_pid_to_u32(value: i64) -> Result<u32, ProcessLedgerError> {
    u32::try_from(value)
        .map_err(|_| ProcessLedgerError::Store(format!("invalid os_pid in reclaim query: {value}")))
}

#[async_trait]
pub trait StaleSessionSource: Send + Sync + 'static {
    async fn stale_sessions(&self, ttl: Duration) -> Result<Vec<String>, ProcessLedgerError>;

    async fn restart_sessions(&self) -> Result<Vec<String>, ProcessLedgerError> {
        Ok(Vec::new())
    }
}

/// PostgreSQL-authoritative stale-session source for model lanes that still
/// own open process rows. It uses each canonical lane record's terminal state,
/// explicit `reclaim_after_utc`, and heartbeat; visible UI/session caches are
/// never treated as full backend state.
#[derive(Clone)]
pub struct PostgresModelLaneStaleSessionSource {
    pool: PgPool,
    runtime_instance: EmbeddedRuntimeInstanceDescriptor,
}

impl PostgresModelLaneStaleSessionSource {
    pub fn new(pool: PgPool, runtime_instance: EmbeddedRuntimeInstanceDescriptor) -> Self {
        Self {
            pool,
            runtime_instance,
        }
    }
}

#[async_trait]
impl StaleSessionSource for PostgresModelLaneStaleSessionSource {
    async fn restart_sessions(&self) -> Result<Vec<String>, ProcessLedgerError> {
        let authority = resolve_process_ledger_authority_relation(&self.pool).await?;
        let sql = format!(
            r#"
            SELECT DISTINCT
                lifecycle.parent_session_id,
                (lifecycle.parent_session_id IS NOT NULL
                 AND lifecycle.sandbox_adapter_id IS NOT NULL) AS reclaim_candidate,
                lifecycle.owner_runtime_instance_id::text AS owner_runtime_instance_id,
                lifecycle.owner_host_scope_id,
                lifecycle.owner_lease_schema_id,
                lifecycle.owner_lease_protocol,
                lifecycle.owner_lease_address,
                lifecycle.owner_lease_port
            FROM ONLY {} AS lifecycle
            WHERE lifecycle.stopped_at IS NULL
              AND (
                  (lifecycle.parent_session_id IS NOT NULL
                   AND lifecycle.sandbox_adapter_id IS NOT NULL)
                  OR lifecycle.owner_runtime_instance_id IS NOT NULL
              )
            ORDER BY lifecycle.parent_session_id NULLS LAST, owner_runtime_instance_id
            "#,
            authority.qualified_table
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        let mut parsed_rows = Vec::with_capacity(rows.len());
        let mut canonical_descriptors = HashMap::<Uuid, ProcessRuntimeOwner>::new();
        let mut conflicting_instance_ids = BTreeSet::<Uuid>::new();
        for row in rows {
            let session_id: Option<String> = row.try_get("parent_session_id")?;
            let reclaim_candidate: bool = row.try_get("reclaim_candidate")?;
            let owner = process_runtime_owner_from_row(&row)?;
            if let Some(owner) = &owner {
                match canonical_descriptors.get(&owner.runtime_instance_id) {
                    Some(canonical) if canonical != owner => {
                        conflicting_instance_ids.insert(owner.runtime_instance_id);
                    }
                    None => {
                        canonical_descriptors.insert(owner.runtime_instance_id, owner.clone());
                    }
                    Some(_) => {}
                }
            }
            if reclaim_candidate {
                let session_id = session_id.ok_or_else(|| {
                    ProcessLedgerError::Store(
                        "restart reclaim candidate has no parent_session_id".to_string(),
                    )
                })?;
                parsed_rows.push((session_id, owner));
            }
        }
        let mut session_safe = BTreeMap::<String, bool>::new();
        let mut descriptor_state = HashMap::<ProcessRuntimeOwner, bool>::new();
        for (session_id, owner) in parsed_rows {
            let safely_dead = match owner {
                Some(owner) if conflicting_instance_ids.contains(&owner.runtime_instance_id) => {
                    tracing::error!(
                        runtime_instance_id = %owner.runtime_instance_id,
                        session_id,
                        "conflicting typed runtime-owner descriptors veto restart reclaim; contradictory open rows remain durable reconciliation evidence"
                    );
                    false
                }
                Some(owner)
                    if owner.host_scope_id == self.runtime_instance.host_scope_id
                        && owner.runtime_instance_id != self.runtime_instance.instance_id
                        && owner.lease_schema_id == EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID
                        && owner.lease_protocol == EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL =>
                {
                    if let Some(dead) = descriptor_state.get(&owner) {
                        *dead
                    } else {
                        let address = owner.lease_address.parse::<IpAddr>().ok();
                        let dead = address.filter(IpAddr::is_loopback).is_some_and(|address| {
                            let descriptor = EmbeddedRuntimeInstanceDescriptor {
                                instance_id: owner.runtime_instance_id,
                                host_scope_id: owner.host_scope_id.clone(),
                                lease_protocol: owner.lease_protocol.clone(),
                                loopback_address: address,
                                loopback_port: owner.lease_port,
                            };
                            matches!(try_claim_udp_lease(&descriptor), UdpLeaseClaim::Claimed(_))
                        });
                        descriptor_state.insert(owner, dead);
                        dead
                    }
                }
                _ => false,
            };
            session_safe
                .entry(session_id)
                .and_modify(|safe| *safe &= safely_dead)
                .or_insert(safely_dead);
        }
        Ok(session_safe
            .into_iter()
            .filter_map(|(session_id, safe)| safe.then_some(session_id))
            .collect())
    }

    async fn stale_sessions(&self, ttl: Duration) -> Result<Vec<String>, ProcessLedgerError> {
        let authority = resolve_process_ledger_authority_relation(&self.pool).await?;
        let sql = format!(
            r#"
            SELECT lifecycle.parent_session_id, lanes.record_json::text AS record_json
            FROM ONLY {} AS lifecycle
            LEFT JOIN model_lanes AS lanes
              ON lanes.record_json->>'coordinator_session_id' = lifecycle.parent_session_id
             AND lanes.record_json->>'process_ownership_ref'
                   = 'process-ledger://' || lifecycle.process_uuid::text
            WHERE lifecycle.stopped_at IS NULL
              AND lifecycle.sandbox_adapter_id IS NOT NULL
              AND lifecycle.owner_runtime_instance_id = $1::uuid
              AND lifecycle.owner_host_scope_id = $2
            ORDER BY lifecycle.parent_session_id, lanes.event_ledger_seq DESC
            "#,
            authority.qualified_table
        );
        let rows = sqlx::query(&sql)
            .bind(self.runtime_instance.instance_id.to_string())
            .bind(&self.runtime_instance.host_scope_id)
            .fetch_all(&self.pool)
            .await?;
        let now = Utc::now();
        let ttl = chrono::Duration::from_std(ttl).map_err(|error| {
            ProcessLedgerError::InvalidConfig(format!("invalid stale-session TTL: {error}"))
        })?;
        // The task returned by this source reclaims a complete coordinator
        // session. Therefore one stale lane is insufficient authority: every
        // open sandbox-owned lifecycle row in that session must have exact
        // process-ownership evidence and must independently be reclaimable.
        // LEFT JOIN keeps unlinked open rows in the decision and makes them a
        // fail-closed veto instead of silently excluding them.
        let mut session_reclaimable = BTreeMap::<String, bool>::new();
        for row in rows {
            let session_id: String = row.get("parent_session_id");
            let row_reclaimable = match row.try_get::<Option<String>, _>("record_json")? {
                Some(raw) => {
                    let record: Value = serde_json::from_str(&raw).map_err(|error| {
                        ProcessLedgerError::Store(format!(
                            "model lane stale-session record is invalid JSON: {error}"
                        ))
                    })?;
                    let status = record.get("status").and_then(Value::as_str).unwrap_or("");
                    let terminal =
                        matches!(status, "completed" | "failed" | "cancelled" | "reclaimable");
                    let reclaim_due = parse_optional_lane_time(&record, "reclaim_after_utc")?
                        .is_some_and(|deadline| deadline <= now);
                    let heartbeat_stale = parse_optional_lane_time(&record, "heartbeat_at_utc")?
                        .is_some_and(|heartbeat| heartbeat < now - ttl);
                    terminal || reclaim_due || heartbeat_stale
                }
                None => false,
            };
            session_reclaimable
                .entry(session_id)
                .and_modify(|all_reclaimable| *all_reclaimable &= row_reclaimable)
                .or_insert(row_reclaimable);
        }
        Ok(session_reclaimable
            .into_iter()
            .filter_map(|(session_id, all_reclaimable)| all_reclaimable.then_some(session_id))
            .collect())
    }
}

fn parse_optional_lane_time(
    record: &Value,
    field: &str,
) -> Result<Option<DateTime<Utc>>, ProcessLedgerError> {
    record
        .get(field)
        .and_then(Value::as_str)
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|time| time.with_timezone(&Utc))
                .map_err(|error| {
                    ProcessLedgerError::Store(format!(
                        "model lane {field} is invalid RFC3339: {error}"
                    ))
                })
        })
        .transpose()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StalenessReclaimConfig {
    pub ttl: Duration,
    pub scan_interval: Duration,
}

impl StalenessReclaimConfig {
    pub fn normalized(self) -> Self {
        Self {
            ttl: if self.ttl.is_zero() {
                Duration::from_secs(300)
            } else {
                self.ttl
            },
            scan_interval: if self.scan_interval.is_zero() {
                Duration::from_secs(30)
            } else {
                self.scan_interval
            },
        }
    }
}

impl Default for StalenessReclaimConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(300),
            scan_interval: Duration::from_secs(30),
        }
    }
}

/// Durable evidence produced by one composed boot restart-reconcile pass.
#[derive(Debug, Default, Clone)]
pub struct RestartOrphanBootReconcileReport {
    /// Sessions surfaced by [`StaleSessionSource::restart_sessions`] whose open
    /// process rows were reconciled this pass.
    pub sessions_reconciled: usize,
    /// Total process rows a Restart-triggered reclaim acted on across every
    /// surfaced session (killed, killed-pending-stop, or fenced-closed).
    pub processes_reclaimed: usize,
    /// The per-session Restart reclaim reports, in surfaced order.
    pub reclaim_reports: Vec<ReclaimReport>,
}

/// Run the production boot restart-reconcile pass: reclaim every restart-orphan
/// session the PostgreSQL-authoritative [`StaleSessionSource`] surfaces.
///
/// This is the exact composition [`ProcessReclaimRuntime`](crate::process_ledger::ProcessReclaimRuntime)
/// runs at boot, factored into a named callable so an integration test can drive
/// the real production path — `restart_sessions` ->
/// `reconcile_in_progress_for_session` -> `run(ReclaimTrigger::Restart)` —
/// instead of re-implementing it inline. A generic spawned-process START row
/// (for example an Official-CLI bridge child) whose owning runtime instance is
/// provably dead is therefore killed via the composed [`SandboxKill`] and given
/// a durable STOP by the same code product boot executes, so a terminated or
/// unreaped official-CLI process cannot remain OPEN without a production owner.
///
/// Error semantics are fail-closed and match the prior inline boot block: the
/// first surfacing-scan, in-progress-reconcile, or reclaim error aborts the pass
/// and propagates so the caller (boot) can fail closed instead of continuing as
/// if reconciliation completed.
pub async fn reconcile_restart_orphans_at_boot(
    reclaim: &Reclaim,
    stale_source: &dyn StaleSessionSource,
) -> Result<RestartOrphanBootReconcileReport, ProcessLedgerError> {
    let mut report = RestartOrphanBootReconcileReport::default();
    for session_id in stale_source.restart_sessions().await? {
        reclaim
            .reconcile_in_progress_for_session(&session_id)
            .await?;
        let reclaim_report = reclaim.run(&session_id, ReclaimTrigger::Restart).await?;
        report.sessions_reconciled += 1;
        report.processes_reclaimed += reclaim_report.processes_reclaimed.len();
        report.reclaim_reports.push(reclaim_report);
    }
    Ok(report)
}

pub fn spawn_staleness_reclaim_task(
    reclaim: Arc<Reclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
) -> JoinHandle<()> {
    let config = config.normalized();
    tokio::spawn(async move {
        let mut interval = time::interval(config.scan_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let session_ids = match stale_source.stale_sessions(config.ttl).await {
                Ok(session_ids) => session_ids,
                Err(error) => {
                    tracing::error!(error = %error, "process-ledger stale-session scan failed");
                    continue;
                }
            };
            for session_id in session_ids {
                if let Err(error) = reclaim.reconcile_in_progress_for_session(&session_id).await {
                    tracing::error!(session_id, error = %error, "process-ledger in-progress kill reconciliation failed");
                    continue;
                }
                if let Err(error) = reclaim.run(&session_id, ReclaimTrigger::Stale).await {
                    tracing::error!(session_id, error = %error, "process-ledger stale-session reclaim failed");
                }
            }
        }
    })
}

#[derive(Clone)]
pub struct ManagedStalenessReclaimTask {
    inner: Arc<ManagedStalenessReclaimTaskInner>,
}

struct ManagedStalenessReclaimTaskInner {
    shutdown: watch::Sender<bool>,
    join: std::sync::Mutex<Option<JoinHandle<()>>>,
}

impl ManagedStalenessReclaimTask {
    pub async fn shutdown_and_join(&self, timeout: Duration) -> bool {
        let _ = self.inner.shutdown.send(true);
        let join = self
            .inner
            .join
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        let Some(mut join) = join else {
            return true;
        };
        match time::timeout(timeout, &mut join).await {
            Ok(Ok(())) => true,
            Ok(Err(error)) if error.is_cancelled() => true,
            Ok(Err(error)) => {
                tracing::error!(error = %error, "managed process-reclaim task failed to join");
                false
            }
            Err(_) => {
                join.abort();
                let _ = join.await;
                false
            }
        }
    }

    pub fn abort_and_join_blocking(&self, timeout: Duration) -> bool {
        let _ = self.inner.shutdown.send(true);
        let join = self
            .inner
            .join
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        let Some(join) = join else {
            return true;
        };
        join.abort();
        let (completed_tx, completed_rx) = std::sync::mpsc::sync_channel(1);
        let helper = std::thread::Builder::new()
            .name("handshake-reclaim-drop-join".to_string())
            .spawn(move || {
                let _ = futures::executor::block_on(join);
                let _ = completed_tx.send(());
            });
        let Ok(_helper) = helper else {
            return false;
        };
        completed_rx.recv_timeout(timeout).is_ok()
    }
}

pub fn spawn_managed_staleness_reclaim_task(
    reclaim: Arc<Reclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
) -> ManagedStalenessReclaimTask {
    spawn_managed_staleness_reclaim_task_internal(reclaim, stale_source, config, true)
}

pub(crate) fn spawn_managed_staleness_reclaim_task_after_boot(
    reclaim: Arc<Reclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
) -> ManagedStalenessReclaimTask {
    spawn_managed_staleness_reclaim_task_internal(reclaim, stale_source, config, false)
}

fn spawn_managed_staleness_reclaim_task_internal(
    reclaim: Arc<Reclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
    run_restart_pass: bool,
) -> ManagedStalenessReclaimTask {
    let config = config.normalized();
    let (shutdown, mut shutdown_rx) = watch::channel(false);
    let join = tokio::spawn(async move {
        if run_restart_pass {
            match stale_source.restart_sessions().await {
                Ok(session_ids) => {
                    for session_id in session_ids {
                        if *shutdown_rx.borrow() {
                            return;
                        }
                        if let Err(error) =
                            reclaim.reconcile_in_progress_for_session(&session_id).await
                        {
                            tracing::error!(session_id, error = %error, "process-ledger restart kill reconciliation failed");
                            continue;
                        }
                        if let Err(error) = reclaim.run(&session_id, ReclaimTrigger::Restart).await
                        {
                            tracing::error!(session_id, error = %error, "process-ledger restart reclaim failed");
                        }
                    }
                }
                Err(error) => {
                    tracing::error!(error = %error, "process-ledger restart-session scan failed");
                }
            }
        }

        let mut interval = time::interval(config.scan_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        return;
                    }
                }
                _ = interval.tick() => {
                    let session_ids = match stale_source.stale_sessions(config.ttl).await {
                        Ok(session_ids) => session_ids,
                        Err(error) => {
                            tracing::error!(error = %error, "process-ledger stale-session scan failed");
                            continue;
                        }
                    };
                    for session_id in session_ids {
                        if *shutdown_rx.borrow() {
                            return;
                        }
                        if let Err(error) = reclaim.reconcile_in_progress_for_session(&session_id).await {
                            tracing::error!(session_id, error = %error, "process-ledger in-progress kill reconciliation failed");
                            continue;
                        }
                        if let Err(error) = reclaim.run(&session_id, ReclaimTrigger::Stale).await {
                            tracing::error!(session_id, error = %error, "process-ledger stale-session reclaim failed");
                        }
                    }
                }
            }
        }
    });
    ManagedStalenessReclaimTask {
        inner: Arc::new(ManagedStalenessReclaimTaskInner {
            shutdown,
            join: std::sync::Mutex::new(Some(join)),
        }),
    }
}

impl Drop for ManagedStalenessReclaimTaskInner {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        let join = self
            .join
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        let Some(join) = join else {
            return;
        };
        join.abort();
        let (completed_tx, completed_rx) = std::sync::mpsc::sync_channel(1);
        let helper = std::thread::Builder::new()
            .name("handshake-reclaim-final-drop-join".to_string())
            .spawn(move || {
                let _ = futures::executor::block_on(join);
                let _ = completed_tx.send(());
            });
        if helper.is_ok() && completed_rx.recv_timeout(Duration::from_secs(2)).is_err() {
            tracing::error!(
                "managed process-reclaim task did not terminate within the bounded final-drop deadline"
            );
        }
    }
}
