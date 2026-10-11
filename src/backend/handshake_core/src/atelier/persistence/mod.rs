use crate::storage::surreal::SurrealStorage;

pub mod foundation;

pub use foundation::FoundationAtelierPersistence;

/// Typed persistence boundary for the Atelier domain.
///
/// Each subtrait owns a disjoint source group so implementations can land in
/// parallel without a generic string/JSON dispatcher. There is deliberately no
/// default, in-memory, compatibility, or fail-open implementation. Only the
/// foundation group is declared on this tree; further groups are added as
/// their embedded-Surreal providers land.
pub trait AtelierPersistence: FoundationAtelierPersistence + Send + Sync + 'static {}

impl<T> AtelierPersistence for T where T: FoundationAtelierPersistence + Send + Sync + 'static {}

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
