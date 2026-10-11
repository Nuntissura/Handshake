use async_trait::async_trait;

/// Typed persistence surface for Atelier foundation identity, documents,
/// sheets, links, relationships, schema bootstrap, and event accounting.
///
/// Methods are added with their provider implementation so this trait never
/// advertises an operation backed only by a placeholder.
#[async_trait]
pub trait FoundationAtelierPersistence: Send + Sync {}
