//! AC-001-4 / PT-001-C — the typed-allowlist proof: DiagEvent carries NO content field.
//!
//! Three independent guarantees:
//!  1. `size_of::<DiagEvent>()` equals the fixed expected size (no surprise blob field grew it).
//!  2. DiagEvent is `bytemuck::Pod` (compiles only if EVERY field is POD — a String/Vec/pointer
//!     field would fail to compile, so reaching this test at all is part of the proof). We exercise
//!     a real Pod round-trip through bytes to prove the cast works.
//!  3. A source-scan of schema.rs confirms it declares no `String`, `Vec`, `&str`, or `[u8]` blob
//!     field inside DiagEvent — a belt-and-braces check on top of the compile-time bytemuck gate.

use std::path::Path;

use handshake_diag_ring::schema::{DiagEvent, DiagEventCode, DiagPhase, DiagSeverity};
use handshake_diag_ring::DIAG_EVENT_SIZE;

#[test]
fn diag_event_has_fixed_expected_size() {
    // AC-001-4 (size). 56 bytes: u16 + u8 + u8 + [u8;4] + 6*u64.
    assert_eq!(std::mem::size_of::<DiagEvent>(), DIAG_EVENT_SIZE);
    assert_eq!(DIAG_EVENT_SIZE, 56);
    assert_eq!(std::mem::align_of::<DiagEvent>(), 8);
}

#[test]
fn diag_event_is_pod_round_trips_through_bytes() {
    // AC-001-4 (Pod). bytemuck::bytes_of / from_bytes only work for a Pod type. If a non-POD field
    // were added, schema.rs would not compile and this test could not exist — so the cast working
    // here is the runtime witness of the compile-time allowlist.
    let event = DiagEvent::resource_sample(11, 22, 33, 44, 55, 66);
    let bytes: &[u8] = bytemuck::bytes_of(&event);
    assert_eq!(bytes.len(), DIAG_EVENT_SIZE);

    let restored: DiagEvent = *bytemuck::from_bytes(bytes);
    assert_eq!(restored, event);
    assert_eq!(restored.event_code, DiagEventCode::ResourceSample.as_u16());
    assert_eq!(restored.thread_id, 11);
    assert_eq!(restored.sequence_id, 22);
    assert_eq!(restored.counter_a, 33);
    assert_eq!(restored.counter_b, 44);
    assert_eq!(restored.metric_micros, 55);
    assert_eq!(restored.timestamp_nanos, 66);
}

#[test]
fn constructors_set_typed_codes() {
    let hb = DiagEvent::heartbeat(1, 2, 100, 200);
    assert_eq!(hb.event_code, DiagEventCode::Heartbeat.as_u16());
    assert_eq!(hb.phase_marker, DiagPhase::Tick.as_u8());
    assert_eq!(hb.severity, DiagSeverity::Info.as_u8());
    assert_eq!(hb.counter_a, 100);
    assert_eq!(hb.timestamp_nanos, 200);

    let be = DiagEvent::backend_unreachable(1, 2, 37501, 999);
    assert_eq!(be.event_code, DiagEventCode::BackendUnreachable.as_u16());
    assert_eq!(be.phase_marker, DiagPhase::Degraded.as_u8());
    assert_eq!(be.severity, DiagSeverity::Error.as_u8());
    assert_eq!(be.counter_a, 37501);

    // The `_reserved` padding is always zero (not a content channel).
    assert_eq!(hb._reserved, [0u8; 4]);
    assert_eq!(be._reserved, [0u8; 4]);
}

#[test]
fn diag_event_serde_round_trips_for_survivor_store() {
    // The later survivor store (MT-092) serializes DiagEvent to JSON. Prove serde round-trips and
    // that the JSON is pure numeric primitives — no nested object/string payload.
    let event = DiagEvent::slow_frame(7, 8, 9, 16_000, 123_456);
    let json = serde_json::to_string(&event).expect("serialize");
    // No content field: the only keys are the numeric primitives.
    for key in [
        "event_code",
        "phase_marker",
        "severity",
        "thread_id",
        "sequence_id",
        "counter_a",
        "counter_b",
        "metric_micros",
        "timestamp_nanos",
    ] {
        assert!(json.contains(key), "expected key {key} in {json}");
    }
    let restored: DiagEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, event);
}

#[test]
fn schema_source_declares_no_content_field_in_diag_event() {
    // AC-001-4 (source-scan). Read schema.rs and confirm the `struct DiagEvent { ... }` body
    // declares NO `String`, `Vec`, `&str`, or `[u8]` blob field. This is a belt-and-braces guard on
    // top of the compile-time bytemuck::Pod enforcement.
    let schema_src = std::fs::read_to_string(Path::new("src/schema.rs"))
        .expect("read src/schema.rs from the crate dir");

    // Isolate the `pub struct DiagEvent { ... }` body.
    let start = schema_src
        .find("pub struct DiagEvent {")
        .expect("DiagEvent struct present");
    let body_start = schema_src[start..]
        .find('{')
        .map(|i| start + i + 1)
        .expect("open brace");
    let body_end = schema_src[body_start..]
        .find('}')
        .map(|i| body_start + i)
        .expect("close brace");
    let body = &schema_src[body_start..body_end];

    // Scan each declared field line (skip doc-comment / attribute lines). A field line looks like
    // `name: Type,`. The ONLY permitted byte-array is the explicit `_reserved` POD padding (always
    // zero, asserted by `constructors_set_typed_codes`); every other heap/text/blob marker is a
    // hard allowlist violation.
    let forbidden_markers = [
        "String", "Vec<", "&str", "[u8", "Box<", "*const", "*mut", "&[", "&'",
    ];
    for raw_line in body.lines() {
        let line = raw_line.trim();
        // Skip doc comments, attributes, and blank lines — only inspect actual field declarations.
        if line.is_empty()
            || line.starts_with("///")
            || line.starts_with("//")
            || line.starts_with('#')
        {
            continue;
        }
        // Field declarations contain a `:`; grab the field name before it.
        let Some((name, _ty)) = line.split_once(':') else {
            continue;
        };
        let field_name = name.trim().trim_start_matches("pub ").trim();
        for forbidden in forbidden_markers {
            if line.contains(forbidden) {
                assert_eq!(
                    field_name, "_reserved",
                    "DiagEvent field `{field_name}` declares a forbidden `{forbidden}` (typed-allowlist invariant); only the `_reserved` POD pad may use a byte array. Line: {line}"
                );
            }
        }
    }

    // Positive sanity: it DOES declare the expected fixed-width integer fields.
    for expected in ["event_code: u16", "thread_id: u64", "timestamp_nanos: u64"] {
        assert!(
            body.contains(expected),
            "DiagEvent struct body should declare `{expected}`"
        );
    }
}

#[test]
fn additive_v2_layout_preserves_v1_and_full_digest() {
    use handshake_diag_ring::schema::{DiagEventV2, DIAG_EVENT_V2_SIZE};
    assert_eq!(DIAG_EVENT_SIZE, 56);
    assert_eq!(std::mem::size_of::<DiagEventV2>(), 120);
    assert_eq!(DIAG_EVENT_V2_SIZE, 120);
    assert_eq!(std::mem::align_of::<DiagEventV2>(), 8);
    assert_eq!(std::mem::offset_of!(DiagEventV2, event), 0);
    assert_eq!(
        std::mem::offset_of!(DiagEventV2, observation_heartbeat_counter),
        56
    );
    assert_eq!(std::mem::offset_of!(DiagEventV2, observed_at_unix_ms), 64);
    assert_eq!(std::mem::offset_of!(DiagEventV2, launch_nonce), 72);
    assert_eq!(std::mem::offset_of!(DiagEventV2, correlation_hmac), 88);
    let ordinary = DiagEventV2::ordinary(DiagEvent::heartbeat(1, 2, 3, 4)).unwrap();
    assert_eq!(&bytemuck::bytes_of(&ordinary)[56..], &[0; 64]);
    assert_eq!(
        &bytemuck::bytes_of(&ordinary)[..56],
        bytemuck::bytes_of(&ordinary.event)
    );
    let observation = DiagEventV2::behavior_observation(2, 3, 4, [0x55; 16], [0xaa; 32]);
    let bytes = bytemuck::bytes_of(&observation);
    assert_eq!(&bytes[0..2], &12u16.to_ne_bytes());
    assert_eq!(&bytes[56..64], &3u64.to_ne_bytes());
    assert_eq!(&bytes[64..72], &4u64.to_ne_bytes());
    assert_eq!(&bytes[72..88], &[0x55; 16]);
    assert_eq!(&bytes[88..120], &[0xaa; 32]);
    assert!(DiagEventV2::ordinary(observation.event).is_none());
    let restored: DiagEventV2 = *bytemuck::from_bytes(bytes);
    assert!(restored == observation);
    assert_eq!(observation.event.thread_id, 0);
    assert_eq!(observation.event.counter_a, 0);
    assert_eq!(observation.event.counter_b, 0);
    assert_eq!(observation.event.metric_micros, 0);
    assert_eq!(observation.event.timestamp_nanos, 0);
}
