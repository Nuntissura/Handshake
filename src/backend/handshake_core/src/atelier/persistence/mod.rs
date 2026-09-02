use crate::storage::surreal::SurrealStorage;

pub mod foundation;
pub mod media;
pub mod operations;
pub mod workflows;

pub use foundation::FoundationAtelierPersistence;
pub use media::MediaAtelierPersistence;
pub use operations::OperationsAtelierPersistence;
pub use workflows::WorkflowAtelierPersistence;

/// Complete typed persistence boundary for the Atelier domain.
///
/// Each subtrait owns a disjoint source group so implementations can land in
/// parallel without a generic string/JSON dispatcher. There is deliberately no
/// default, in-memory, compatibility, or fail-open implementation.
pub trait AtelierPersistence:
    FoundationAtelierPersistence
    + MediaAtelierPersistence
    + WorkflowAtelierPersistence
    + OperationsAtelierPersistence
    + Send
    + Sync
    + 'static
{
}

impl<T> AtelierPersistence for T where
    T: FoundationAtelierPersistence
        + MediaAtelierPersistence
        + WorkflowAtelierPersistence
        + OperationsAtelierPersistence
        + Send
        + Sync
        + 'static
{
}

/// Embedded-Surreal Atelier provider over the application-owned handle.
///
/// Construction never opens or selects a database, ensuring all group
/// implementations use the composition root's exact namespace/database.
#[derive(Clone)]
pub struct SurrealAtelierPersistence {
    storage: SurrealStorage,
}

impl SurrealAtelierPersistence {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub(crate) fn storage(&self) -> &SurrealStorage {
        &self.storage
    }
}
