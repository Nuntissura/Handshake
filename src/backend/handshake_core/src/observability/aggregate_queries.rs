//! MT-200 contract-path re-exports for multi-session aggregate queries.
//!
//! The folded X.4 span implementation owns the DTOs and in-memory query
//! primitive today. This module gives diagnostics callers the contract-owned
//! `observability::aggregate_queries` import path.

pub use crate::flight_recorder::spans::{
    ActivityRow, AggregateQueryError, AggregateQueryFixture, Limit, Offset,
    SessionAggregateQueries, SessionSummary, SessionTimeline, SessionTimelineEntry, SpanLatencyRow,
    SwarmSnapshot,
};
