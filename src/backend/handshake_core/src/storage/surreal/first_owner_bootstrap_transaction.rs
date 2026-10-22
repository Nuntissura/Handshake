use std::{sync::Arc, time::Instant};

use super::{
    local_accounts::FirstOwnerAdministrationConfirmationOperation,
    AuthenticatedTransactionRetention, SurrealClient, SurrealStorage,
    SurrealStorageError, SurrealTransactionContext, SurrealTransactionOperation,
};
use surrealdb::{engine::local::Db, method::Transaction};

pub(super) struct SameStorageBootstrapTransaction<'tx> {
    storage: &'tx SurrealStorage,
    transaction: &'tx Transaction<Db>,
    operation_id: uuid::Uuid,
}

impl SameStorageBootstrapTransaction<'_> {
    pub(super) fn require_storage_context(
        &self,
        storage: &SurrealStorage,
        context: &SurrealTransactionContext<'_>,
    ) -> Result<(), SurrealStorageError> {
        if !Arc::ptr_eq(&self.storage.inner, &storage.inner)
            || !std::ptr::eq(self.transaction, context.transaction)
        {
            return Err(SurrealStorageError::TransactionRetentionMismatch {
                operation_id: self.operation_id,
            });
        }
        Ok(())
    }
}

pub(super) enum FirstOwnerBootstrapOutcome<T, U, E> {
    BodyRejected {
        operation: E,
        cancellation: Result<SurrealClient, surrealdb::Error>,
    },
    CancelledBeforeCommit {
        operation: T,
        cancellation: Result<SurrealClient, surrealdb::Error>,
    },
    CommitRejected {
        operation: T,
        error: surrealdb::Error,
    },
    Committed {
        operation: Arc<T>,
        commit: SurrealClient,
        canonical: Result<U, E>,
    },
}

impl SurrealStorage {
    fn require_first_owner_confirmation_retention(
        &self,
        owner: &Arc<FirstOwnerAdministrationConfirmationOperation>,
    ) -> Result<(), SurrealStorageError> {
        let settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match settlements.get(&owner.retention_key()) {
            Some(Some(AuthenticatedTransactionRetention::FirstOwnerConfirmation(held)))
                if Arc::ptr_eq(held, owner) => Ok(()),
            _ => Err(SurrealStorageError::TransactionRetentionMismatch {
                operation_id: owner.operation_id(),
            }),
        }
    }

    pub(super) async fn with_first_owner_administration_bootstrap_transaction<T, U, E, F, R>(
        &self,
        owner: Arc<FirstOwnerAdministrationConfirmationOperation>,
        operation: F,
        canonical: R,
    ) -> Result<FirstOwnerBootstrapOutcome<T, U, E>, SurrealStorageError>
    where
        T: Send + Sync + 'static,
        U: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: for<'tx> FnOnce(
                &'tx SurrealTransactionContext<'tx>,
                SameStorageBootstrapTransaction<'tx>,
            ) -> SurrealTransactionOperation<'tx, T, E>
            + Send + 'static,
        R: for<'db> FnOnce(
                &'db SurrealClient,
                Arc<T>,
            ) -> SurrealTransactionOperation<'db, U, E>
            + Send + 'static,
    {
        self.require_first_owner_confirmation_retention(&owner)?;
        let cancellation = owner.cancellation();
        let deadline = owner.original_deadline();
        if cancellation.is_cancelled() || Instant::now() >= deadline {
            return Err(SurrealStorageError::TransactionCancelled);
        }
        let storage = self.clone();
        self.with_lease(move |client| Box::pin(async move {
            storage.require_first_owner_confirmation_retention(&owner)?;
            if cancellation.is_cancelled() || Instant::now() >= deadline {
                return Err(SurrealStorageError::TransactionCancelled);
            }
            let scoped = (*client).clone();
            scoped.use_ns(storage.config().namespace())
                .use_db(storage.config().database()).await?;
            storage.require_first_owner_confirmation_retention(&owner)?;
            if cancellation.is_cancelled() || Instant::now() >= deadline {
                return Err(SurrealStorageError::TransactionCancelled);
            }
            let transaction = scoped.begin().await?;
            let body = {
                let context = SurrealTransactionContext { transaction: &transaction };
                let birth = SameStorageBootstrapTransaction {
                    storage: &storage,
                    transaction: &transaction,
                    operation_id: owner.operation_id(),
                };
                if cancellation.is_cancelled() || Instant::now() >= deadline {
                    Err(E::from(SurrealStorageError::TransactionCancelled))
                } else {
                    operation(&context, birth).await
                }
            };
            let value = match body {
                Ok(value) => value,
                Err(error) => return Ok(FirstOwnerBootstrapOutcome::BodyRejected {
                    operation: error,
                    cancellation: transaction.cancel().await,
                }),
            };
            if cancellation.is_cancelled() || Instant::now() >= deadline {
                return Ok(FirstOwnerBootstrapOutcome::CancelledBeforeCommit {
                    operation: value,
                    cancellation: transaction.cancel().await,
                });
            }
            let committed = match transaction.commit().await {
                Ok(client) => client,
                Err(error) => return Ok(FirstOwnerBootstrapOutcome::CommitRejected {
                    operation: value,
                    error,
                }),
            };
            let value = Arc::new(value);
            let result = canonical(&committed, Arc::clone(&value)).await;
            Ok(FirstOwnerBootstrapOutcome::Committed {
                operation: value,
                commit: committed,
                canonical: result,
            })
        })).await
    }
}
