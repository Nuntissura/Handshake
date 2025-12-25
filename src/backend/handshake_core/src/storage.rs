#![allow(dead_code)]

use sqlx::SqlitePool;
use std::sync::Arc;

/// Database trait abstraction to hide concrete pools behind a single API surface.
pub trait Database: Send + Sync {
    fn sqlite_pool(&self) -> &SqlitePool;
}

/// Sqlite implementation of the Database trait.
pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn into_arc(self) -> Arc<dyn Database> {
        Arc::new(self)
    }
}

impl Database for SqliteDatabase {
    fn sqlite_pool(&self) -> &SqlitePool {
        &self.pool
    }
}
