//! MT-136 fail-closed proofs for the embedded knowledge schema registry.
//!
//! The compiled seed hash protects the executable seed bytes. These probes
//! additionally prove that a real persisted store cannot reopen when its live
//! registry has the expected row count but divergent metadata, or when the
//! registry is only partially populated.

use std::collections::BTreeMap;

use surrealdb::types::Value;

use super::{
    mt136_proof_harness::{embedded_proof_backend, EmbeddedProofBackend},
    SurrealStorage, SurrealStorageConfig,
};
use crate::storage::{Database, StorageError, StorageResult};

enum MutationStep {
    Query(&'static str),
    InjectedFailure,
}

struct CaseOutcome {
    mutation_result: StorageResult<()>,
    first_shutdown_result: StorageResult<()>,
    verification_result: Option<StorageResult<()>>,
    cleanup_result: std::io::Result<()>,
    data_dir: std::path::PathBuf,
}

impl CaseOutcome {
    fn into_result(self) -> StorageResult<()> {
        let mut failures = Vec::new();
        if let Err(error) = self.mutation_result {
            failures.push(format!("mutation failed: {error}"));
        }
        if let Err(error) = self.first_shutdown_result {
            failures.push(format!("first shutdown failed: {error}"));
        }
        match self.verification_result {
            Some(Err(error)) => failures.push(format!("reopen verification failed: {error}")),
            Some(Ok(())) => {}
            None if failures.is_empty() => {
                failures.push("reopen verification was not executed".to_owned())
            }
            None => {}
        }
        if let Err(error) = self.cleanup_result {
            failures.push(format!(
                "cleanup failed for {}: {error}",
                self.data_dir.display()
            ));
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(StorageError::Database(failures.join("; ")))
        }
    }
}

async fn verify_reopen(data_dir: &std::path::Path, case: &str) -> StorageResult<()> {
    let config = SurrealStorageConfig::for_data_dir(data_dir)
        .map_err(|error| StorageError::Database(error.to_string()))?;
    let reopened = SurrealStorage::open(config)
        .await
        .map_err(|error| StorageError::Database(error.to_string()))?;
    let database = super::SurrealDatabase::new(reopened.clone());
    let migration_result = database.run_migrations().await;
    drop(database);
    let shutdown_result = reopened
        .shutdown()
        .await
        .map_err(|error| StorageError::Database(error.to_string()));
    drop(reopened);
    let verification_result = migration_result.err().ok_or_else(|| {
        StorageError::Database(format!("{case} registry unexpectedly passed verification"))
    });
    let verification_result = verification_result.and_then(|error| {
        let message = error.to_string();
        if message.contains("HANDSHAKE_SURREAL_KNOWLEDGE_REGISTRY_DIVERGENT") {
            Ok(())
        } else {
            Err(StorageError::Database(format!(
                "{case} registry failed for the wrong reason: {message}"
            )))
        }
    });
    match (verification_result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(verification_error), Err(shutdown_error)) => Err(StorageError::Database(format!(
            "{verification_error}; additionally reopened shutdown failed: {shutdown_error}"
        ))),
    }
}

async fn execute_case(
    backend: EmbeddedProofBackend,
    mutation: MutationStep,
    case: &str,
) -> CaseOutcome {
    let data_dir = backend.data_dir.clone();
    let database = backend.database.clone();
    let storage = backend.storage.clone();
    let mutation_result = match mutation {
        MutationStep::Query(statement) => storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values::<Value, _>(statement, BTreeMap::<String, Value>::new())
                        .await
                        .map(|_| ())
                })
            })
            .await
            .map_err(|error| StorageError::Database(error.to_string())),
        MutationStep::InjectedFailure => Err(StorageError::Database(
            "MT136_FORCED_MUTATION_FAILURE".to_owned(),
        )),
    };

    drop(database);
    let first_shutdown_result = storage
        .shutdown()
        .await
        .map_err(|error| StorageError::Database(error.to_string()));
    drop(storage);
    let verification_result = if mutation_result.is_ok() && first_shutdown_result.is_ok() {
        Some(verify_reopen(&data_dir, case).await)
    } else {
        None
    };
    let cleanup_result = std::fs::remove_dir_all(&data_dir);
    drop(backend);
    CaseOutcome {
        mutation_result,
        first_shutdown_result,
        verification_result,
        cleanup_result,
        data_dir,
    }
}

async fn reopen_must_fail_after(statement: &'static str, case: &str) -> StorageResult<()> {
    let backend = embedded_proof_backend().await?;
    execute_case(backend, MutationStep::Query(statement), case)
        .await
        .into_result()
}

async fn forced_failure_must_cleanup() -> StorageResult<()> {
    let backend = embedded_proof_backend().await?;
    let outcome = execute_case(backend, MutationStep::InjectedFailure, "forced-failure").await;
    let mutation_is_expected = outcome
        .mutation_result
        .as_ref()
        .err()
        .is_some_and(|error| error.to_string().contains("MT136_FORCED_MUTATION_FAILURE"));
    if !mutation_is_expected {
        return Err(StorageError::Database(
            "forced registry proof did not exercise the injected mutation failure".to_owned(),
        ));
    }
    if let Err(error) = outcome.first_shutdown_result {
        return Err(StorageError::Database(format!(
            "forced registry proof first shutdown failed: {error}"
        )));
    }
    if outcome.verification_result.is_some() {
        return Err(StorageError::Database(
            "forced registry proof unexpectedly attempted reopen verification".to_owned(),
        ));
    }
    if let Err(error) = outcome.cleanup_result {
        return Err(StorageError::Database(format!(
            "forced registry proof cleanup failed for {}: {error}",
            outcome.data_dir.display()
        )));
    }
    if outcome.data_dir.try_exists().map_err(|error| {
        StorageError::Database(format!(
            "could not inspect forced registry proof path {}: {error}",
            outcome.data_dir.display()
        ))
    })? {
        return Err(StorageError::Database(format!(
            "forced registry proof failure leaked {}",
            outcome.data_dir.display()
        )));
    }
    Ok(())
}

pub(crate) async fn run_all() -> StorageResult<()> {
    forced_failure_must_cleanup().await?;
    reopen_must_fail_after(
        "UPDATE knowledge_schema_registry:schema_registry SET schema_source = 'tampered.surql' RETURN AFTER;",
        "same-count-divergent",
    )
    .await?;
    reopen_must_fail_after(
        "DELETE knowledge_schema_registry:schema_registry RETURN BEFORE;",
        "partial",
    )
    .await
}
