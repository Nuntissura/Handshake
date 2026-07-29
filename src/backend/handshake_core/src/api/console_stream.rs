//! WP-1 live orchestration debug console — Server-Sent Events route.
//!
//! `GET /wp1/diagnostics/console/stream` streams the NON-AUTHORITATIVE WP-1
//! orchestration console ([`crate::console_stream`]) as `text/event-stream`. It
//! makes the swarm-lane/message diagnostics (MT-008), operator-chat launch
//! activity (MT-012), and cloud-access/CLI-bridge login (MT-015) observable and
//! provable HEADLESSLY via a live text stream — instead of the fragile
//! "screenshot a pop-out window" path.
//!
//! On connect the endpoint replays the bounded recent history, then follows the
//! live tail. A slow reader that lags the broadcast ring receives a
//! `console_lagged` event (the count skipped) rather than a silent gap; a
//! periodic keep-alive comment holds the connection open through idle periods.
//!
//! This is a DISPLAY/STREAM surface only. The durable authority for every event
//! is PostgreSQL/EventLedger + the Flight Recorder; dropping, lagging, or closing
//! this stream never affects durable state.

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures::stream::{Stream, StreamExt};
use tokio::sync::broadcast::error::RecvError;

use crate::console_stream::{ConsoleBroadcast, ConsoleEntry, CONSOLE_ENTRY_SCHEMA_ID};

/// How many recent-history entries to replay to a subscriber on connect.
const REPLAY_LIMIT: usize = crate::console_stream::DEFAULT_HISTORY_CAPACITY;

/// Keep-alive comment interval so idle connections (no live events) stay open
/// through proxies and the native client's read loop.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

pub fn routes(hub: ConsoleBroadcast) -> Router {
    Router::new()
        .route("/wp1/diagnostics/console/stream", get(console_stream))
        .with_state(hub)
}

async fn console_stream(
    State(hub): State<ConsoleBroadcast>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Subscribe FIRST so the live tail captures every entry from this moment on,
    // THEN snapshot the bounded recent history for connect-time replay. Any small
    // overlap at the boundary is deduped downstream by the monotonic `seq`.
    let rx = hub.subscribe();
    let replay = hub.recent(REPLAY_LIMIT);

    let replay_stream = futures::stream::iter(
        replay
            .into_iter()
            .map(|entry| Ok::<Event, Infallible>(entry_to_event(&entry))),
    );

    let live_stream = futures::stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(entry) => Some((Ok::<Event, Infallible>(entry_to_event(&entry)), rx)),
            // A slow reader lagged the ring: surface the drop count explicitly so
            // the gap is OBSERVABLE, then keep following.
            Err(RecvError::Lagged(skipped)) => {
                let event = Event::default()
                    .event("console_lagged")
                    .data(skipped.to_string());
                Some((Ok(event), rx))
            }
            // The hub was dropped (process shutdown): end the stream cleanly.
            Err(RecvError::Closed) => None,
        }
    });

    let stream = replay_stream.chain(live_stream);

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(KEEP_ALIVE_INTERVAL)
            .text("keep-alive"),
    )
}

/// Render one console entry as an SSE event: the schema id is the event name, the
/// monotonic `seq` is the SSE `id`, and the JSON entry is the data. Serialization
/// of this plain struct cannot fail; a comment fallback keeps the stream alive if
/// it somehow did.
fn entry_to_event(entry: &ConsoleEntry) -> Event {
    Event::default()
        .event(CONSOLE_ENTRY_SCHEMA_ID)
        .id(entry.seq.to_string())
        .json_data(entry)
        .unwrap_or_else(|_| Event::default().comment("console entry serialization failed"))
}
