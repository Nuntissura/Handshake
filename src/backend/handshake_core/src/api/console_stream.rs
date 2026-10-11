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
//! live tail. A scoped reader that lags the shared broadcast ring silently
//! resumes from later authorized entries: reporting the shared skipped count
//! would leak the volume of foreign-scope events. A periodic keep-alive comment
//! holds the connection open through idle periods.
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

use crate::api::account_scope::RequestAccountScope;
use crate::console_stream::{ConsoleBroadcast, ConsoleEntry, CONSOLE_ENTRY_SCHEMA_ID};
use crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;

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
    scope: RequestAccountScope,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Receiver creation and history capture share the publication mutex, making
    // replay a strict prefix and live delivery a strict suffix.
    let exact_scope = scope.into_exact();
    let (rx, replay) = hub.subscribe_with_recent(REPLAY_LIMIT);
    let replay_scope = exact_scope.clone();

    let authorized_replay = replay
        .into_iter()
        .filter(move |entry| entry_is_authorized(entry, &replay_scope))
        .collect::<Vec<_>>();
    let next_visible_seq = authorized_replay.len() as u64;
    let replay_stream = futures::stream::iter(authorized_replay.into_iter().enumerate().map(
        |(visible_seq, entry)| Ok::<Event, Infallible>(entry_to_event(entry, visible_seq as u64)),
    ));

    let live_stream = futures::stream::unfold(
        (rx, exact_scope, next_visible_seq),
        |(mut rx, exact_scope, mut next_visible_seq)| async move {
            loop {
                match rx.recv().await {
                    Ok(entry) if entry_is_authorized(&entry, &exact_scope) => {
                        let event = entry_to_event(entry, next_visible_seq);
                        next_visible_seq = next_visible_seq.wrapping_add(1);
                        return Some((
                            Ok::<Event, Infallible>(event),
                            (rx, exact_scope, next_visible_seq),
                        ));
                    }
                    Ok(_) => continue,
                    // The shared ring can lag because another account is noisy. Do
                    // not reveal that account's skipped-event count to this reader.
                    Err(RecvError::Lagged(_)) => continue,
                    // The hub was dropped (process shutdown): end the stream cleanly.
                    Err(RecvError::Closed) => return None,
                }
            }
        },
    );

    let stream = replay_stream.chain(live_stream);

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(KEEP_ALIVE_INTERVAL)
            .text("keep-alive"),
    )
}

fn entry_is_authorized(entry: &ConsoleEntry, exact: &ExactResourceScopeAttribution) -> bool {
    entry.resource_scope.as_ref() == Some(exact)
}

/// Render one console entry as an SSE event: the schema id is the event name, the
/// monotonic `seq` is the SSE `id`, and the JSON entry is the data. Serialization
/// of this plain struct cannot fail; a comment fallback keeps the stream alive if
/// it somehow did.
fn entry_to_event(mut entry: ConsoleEntry, visible_seq: u64) -> Event {
    entry.seq = visible_seq;
    Event::default()
        .event(CONSOLE_ENTRY_SCHEMA_ID)
        .id(visible_seq.to_string())
        .json_data(&entry)
        .unwrap_or_else(|_| Event::default().comment("console entry serialization failed"))
}
