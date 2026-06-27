//! The Palmistry CONTROL channel (MT-089, §6.13.3 + §6.13.4).
//!
//! Palmistry is controlled over a cross-platform local socket (interprocess 2.x: a Unix domain socket
//! on Unix, a Windows named pipe on Windows, behind one API). The protocol is newline-delimited JSON
//! [`ControlMessage`] values.
//!
//! # HARD design rule: CONTROL only, never liveness (RISK-009-3 / §6.13.4)
//!
//! This socket carries CONTROL commands ONLY (`Shutdown`, `Ping`, `CrashContext`, `HandshakeHello`).
//! Liveness is NEVER requested over it. A frozen Handshake could not answer a request-response liveness
//! probe, so a blocking liveness call here would make the watcher block / false-timeout exactly when it
//! is needed. Liveness is observed PASSIVELY from the shared ring (MT-090: `read_heartbeat`). The
//! `Ping`/`Pong` here is only a connectivity sanity check that explicitly DOES NOT drive any
//! freeze/crash decision and DOES NOT cause Palmistry to exit (AC-009-3).
//!
//! # Framing
//!
//! One JSON object per line (`\n`-terminated). This is the simplest robust framing for a control
//! channel: a `BufReader::read_line` on the server, a `writeln!` on the client. Each line is a complete
//! [`ControlMessage`]; partial lines are never acted on.

use std::io::{self, BufRead, BufReader, Write};

use interprocess::local_socket::traits::Listener as _;
use interprocess::local_socket::{
    GenericNamespaced, ListenerOptions, Name, ToNsName,
};
use serde::{Deserialize, Serialize};

/// A typed control message exchanged over the local socket. Tagged JSON so the wire form is explicit
/// and self-describing (`{"type":"Shutdown"}` etc.) and a future variant cannot be silently confused
/// with an existing one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlMessage {
    /// Handshake's startup handshake: announces the parent pid + session so Palmistry can ACK that it
    /// is watching the right process. (The watcher already has these from its CLI/env config; this is
    /// the over-socket confirmation Handshake sends after the spawn — MT-094 produces it.)
    HandshakeHello {
        /// The parent pid Handshake believes it is.
        parent_pid: u32,
        /// The session id.
        session_id: String,
    },
    /// Connectivity sanity check. NOT a liveness probe (see module docs). Replied to with [`ControlReply::Pong`].
    /// MUST NOT cause Palmistry to exit.
    Ping,
    /// The MT-092 hand-off: Handshake (or its in-process crash hook) passes a crash-context address +
    /// the faulting thread id so Palmistry can write a minidump for that thread. MT-092 fills the
    /// capture; here the message is defined + accepted so the protocol is stable. Carries NO project
    /// text — only numeric addresses/ids (the typed-allowlist stance of the whole substrate).
    CrashContext {
        /// Address (as a u64) of the shared crash-context blob Handshake exported.
        crash_context_addr: u64,
        /// OS thread id of the faulting thread.
        faulting_thread_id: u64,
    },
    /// The ONE deliberate exit command. On receipt Palmistry shuts down promptly + cleanly and records
    /// NO crash (the §6.13 clean-shutdown rule). This is the ONLY message that ends the watcher's life
    /// short of a bounded post-parent-death finalize.
    Shutdown,
}

/// A reply Palmistry sends back on the same connection. Kept minimal: the control channel is
/// fire-and-mostly-forget; only `Ping` expects a `Pong`, and `Shutdown`/`Hello` get an `Ack`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlReply {
    /// Reply to [`ControlMessage::Ping`].
    Pong,
    /// Generic acknowledgement (Hello / CrashContext / Shutdown received).
    Ack,
}

/// Build the interprocess `Name` for `socket_name` in the GENERIC NAMESPACED form (always supported on
/// every platform: an abstract-ish namespaced name — a `\\.\pipe\<name>` pipe on Windows, an abstract
/// or `/tmp`-backed socket on Unix). Centralized so the server and any client derive the identical
/// name.
pub fn control_name(socket_name: &str) -> io::Result<Name<'_>> {
    socket_name.to_ns_name::<GenericNamespaced>()
}

/// The decision a handler made about one received message — does the watcher keep running or begin its
/// shutdown? Returned by [`handle_message`] so the lifecycle owns the exit, not the socket layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlOutcome {
    /// Keep running (Ping / Hello / CrashContext).
    Continue,
    /// A `Shutdown` arrived: begin a clean exit.
    Shutdown,
}

/// Pure message handler: maps a received [`ControlMessage`] to the reply to send and the lifecycle
/// outcome. Side-effect free so it is unit-testable in isolation (the socket I/O is tested via the
/// real round-trip in `serve_once`-style tests). The CONTROL-only invariant is visible here: NOTHING
/// in this match consults or affects liveness, and only `Shutdown` returns `ControlOutcome::Shutdown`.
pub fn handle_message(msg: &ControlMessage) -> (ControlReply, ControlOutcome) {
    match msg {
        ControlMessage::Ping => (ControlReply::Pong, ControlOutcome::Continue),
        ControlMessage::HandshakeHello { .. } => (ControlReply::Ack, ControlOutcome::Continue),
        ControlMessage::CrashContext { .. } => (ControlReply::Ack, ControlOutcome::Continue),
        ControlMessage::Shutdown => (ControlReply::Ack, ControlOutcome::Shutdown),
    }
}

/// Serialize a [`ControlMessage`] to a single newline-terminated JSON line (the wire frame). The
/// symmetric counterpart of [`decode_message`]; the watcher binary only RECEIVES control messages (it
/// never sends them), so this helper is exercised by the protocol round-trip tests + by Handshake-side
/// senders (MT-094) rather than the watcher binary itself.
#[allow(dead_code)]
pub fn encode_message(msg: &ControlMessage) -> io::Result<String> {
    let mut line = serde_json::to_string(msg).map_err(io::Error::other)?;
    line.push('\n');
    Ok(line)
}

/// Parse one newline-stripped JSON line into a [`ControlMessage`].
pub fn decode_message(line: &str) -> io::Result<ControlMessage> {
    serde_json::from_str(line).map_err(io::Error::other)
}

/// The control-socket server. Owns the bound [`interprocess`] listener and drives the accept loop on a
/// dedicated thread (the lifecycle owns the thread; this type owns the protocol).
pub struct ControlServer {
    listener: interprocess::local_socket::Listener,
    socket_name: String,
}

impl ControlServer {
    /// Bind the control socket at `socket_name`. Fails if the name is taken / invalid — the caller
    /// treats a bind failure as a fatal startup error (a watcher with no control channel can never be
    /// cleanly shut down).
    pub fn bind(socket_name: &str) -> io::Result<Self> {
        let name = control_name(socket_name)?;
        let listener = ListenerOptions::new().name(name).create_sync()?;
        Ok(Self {
            listener,
            socket_name: socket_name.to_string(),
        })
    }

    /// The socket name this server is bound to (for logging).
    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    /// Accept exactly ONE connection, then read newline-delimited [`ControlMessage`]s from it until the
    /// peer disconnects OR a message resolves to [`ControlOutcome::Shutdown`]. Returns the outcome of
    /// the LAST message (or `Continue` if the peer simply disconnected). Each message gets its typed
    /// reply written back. `on_message` is invoked for every decoded message BEFORE the outcome is
    /// applied so the lifecycle can record side effects (e.g. note a CrashContext) without this layer
    /// owning that state.
    ///
    /// Blocking by design: this runs on the dedicated control thread. The lifecycle thread observes the
    /// shutdown via the shared flag the `on_message`/return path sets, never by blocking here.
    pub fn serve_connection(
        &self,
        on_message: &mut dyn FnMut(&ControlMessage),
    ) -> io::Result<ControlOutcome> {
        let conn = self.listener.accept()?;
        // interprocess Stream is read+write; wrap the read half in a BufReader for line framing. The
        // Stream is Clone-free, so we read and write through the same handle via a BufReader that holds
        // it and a re-borrow for writes is not possible — instead split by buffering reads and writing
        // to a second handle obtained by re-accept is wrong. interprocess Stream implements Read+Write
        // on &Stream? No — so we keep the BufReader owning the stream and write through get_mut().
        let mut reader = BufReader::new(conn);
        let mut last = ControlOutcome::Continue;
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                // EOF: peer closed the connection. NOT a shutdown trigger (a dropped control connection
                // must NOT kill the watcher — §6.13.3: it closes ONLY on an explicit Shutdown).
                return Ok(last);
            }
            let trimmed = line.trim_end_matches(['\n', '\r']);
            if trimmed.is_empty() {
                continue;
            }
            let msg = decode_message(trimmed)?;
            on_message(&msg);
            let (reply, outcome) = handle_message(&msg);
            let reply_line = serde_json::to_string(&reply).map_err(io::Error::other)?;
            {
                let w = reader.get_mut();
                w.write_all(reply_line.as_bytes())?;
                w.write_all(b"\n")?;
                w.flush()?;
            }
            last = outcome;
            if outcome == ControlOutcome::Shutdown {
                return Ok(ControlOutcome::Shutdown);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_round_trips_through_json() {
        let cases = [
            ControlMessage::Ping,
            ControlMessage::Shutdown,
            ControlMessage::HandshakeHello {
                parent_pid: 4242,
                session_id: "sess".into(),
            },
            ControlMessage::CrashContext {
                crash_context_addr: 0xDEAD_BEEF,
                faulting_thread_id: 17,
            },
        ];
        for msg in cases {
            let line = encode_message(&msg).unwrap();
            assert!(line.ends_with('\n'));
            let back = decode_message(line.trim_end()).unwrap();
            assert_eq!(back, msg);
        }
    }

    #[test]
    fn ping_does_not_shut_down() {
        let (reply, outcome) = handle_message(&ControlMessage::Ping);
        assert_eq!(reply, ControlReply::Pong);
        assert_eq!(outcome, ControlOutcome::Continue, "Ping must NOT exit (AC-009-3)");
    }

    #[test]
    fn hello_and_crash_context_do_not_shut_down() {
        let (_, o1) = handle_message(&ControlMessage::HandshakeHello {
            parent_pid: 1,
            session_id: "s".into(),
        });
        assert_eq!(o1, ControlOutcome::Continue);
        let (_, o2) = handle_message(&ControlMessage::CrashContext {
            crash_context_addr: 1,
            faulting_thread_id: 1,
        });
        assert_eq!(o2, ControlOutcome::Continue);
    }

    #[test]
    fn shutdown_resolves_to_shutdown_outcome() {
        let (reply, outcome) = handle_message(&ControlMessage::Shutdown);
        assert_eq!(reply, ControlReply::Ack);
        assert_eq!(outcome, ControlOutcome::Shutdown);
    }

    #[test]
    fn tagged_json_shape_is_explicit() {
        // The wire form must be self-describing so a future variant cannot be confused with an existing
        // one (a control-protocol safety property).
        let line = serde_json::to_string(&ControlMessage::Shutdown).unwrap();
        assert_eq!(line, r#"{"type":"Shutdown"}"#);
        let hello = serde_json::to_string(&ControlMessage::HandshakeHello {
            parent_pid: 7,
            session_id: "z".into(),
        })
        .unwrap();
        assert_eq!(hello, r#"{"type":"HandshakeHello","parent_pid":7,"session_id":"z"}"#);
    }
}
