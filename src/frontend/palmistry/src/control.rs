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
use std::path::PathBuf;

use interprocess::local_socket::traits::Listener as _;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Name, ToNsName};
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
    /// Register a child process for passive no-progress stall detection (MT-106). The socket carries
    /// only the control fact: which child pid/session and which passive liveness source to observe. The
    /// watch itself happens on Palmistry's polling thread, never by asking the child over this socket.
    RegisterChild {
        /// OS pid of the child process being watched.
        pid: u32,
        /// Opaque launcher-supplied child-session token so reused pids do not collide.
        child_session_id: u64,
        /// Passive source kind.
        liveness_kind: ControlChildLivenessKind,
        /// Local reference to the passive liveness source. For `FileCounter`, this is a local file path
        /// whose contents are a numeric counter.
        liveness_ref: PathBuf,
        /// No-progress threshold in milliseconds. `0` means Palmistry's default threshold.
        threshold_ms: u64,
    },
    /// Stop watching a previously registered child process.
    DeregisterChild {
        /// OS pid of the child process.
        pid: u32,
        /// Opaque launcher-supplied child-session token.
        child_session_id: u64,
    },
    /// The ONE deliberate exit command. On receipt Palmistry shuts down promptly + cleanly and records
    /// NO crash (the §6.13 clean-shutdown rule). This is the ONLY message that ends the watcher's life
    /// short of a bounded post-parent-death finalize.
    Shutdown,
    /// Fresh signed identity challenge; public material only, never a key provision message.
    ProvenanceChallengeV1 {
        identity: WatcherSigningIdentity,
        challenge: [u8; 32],
        transport_public_key: [u8; 32],
    },
}

/// Passive liveness source kind for [`ControlMessage::RegisterChild`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlChildLivenessKind {
    FileCounter,
}

/// A reply Palmistry sends back on the same connection. Kept minimal: the control channel is
/// fire-and-mostly-forget; only `Ping` expects a `Pong`, and `Shutdown`/`Hello` get an `Ack`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlReply {
    /// Reply to [`ControlMessage::Ping`].
    Pong,
    ProvenanceReadyV1 {
        identity: WatcherSigningIdentity,
        challenge: [u8; 32],
        transport_public_key: [u8; 32],
        public_verifier: [u8; 32],
        signature: Vec<u8>,
    },
    /// Generic acknowledgement (Hello / CrashContext / Shutdown received).
    Ack,
    /// The message decoded, but the requested side effect failed. The code is closed and typed so the
    /// control channel does not carry paths, command lines, or free-text diagnostics.
    Error { code: ControlErrorCode },
}

/// Closed error codes for control-message side-effect failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlErrorCode {
    SigningUnavailable,
    SigningIdentityMismatch,
    ChildRegisterFailed,
    ChildNotRegistered,
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
        ControlMessage::ProvenanceChallengeV1 { .. } => (
            ControlReply::Error {
                code: ControlErrorCode::SigningUnavailable,
            },
            ControlOutcome::Continue,
        ),
        ControlMessage::Ping => (ControlReply::Pong, ControlOutcome::Continue),
        ControlMessage::HandshakeHello { .. } => (ControlReply::Ack, ControlOutcome::Continue),
        ControlMessage::CrashContext { .. } => (ControlReply::Ack, ControlOutcome::Continue),
        ControlMessage::RegisterChild { .. } => (ControlReply::Ack, ControlOutcome::Continue),
        ControlMessage::DeregisterChild { .. } => (ControlReply::Ack, ControlOutcome::Continue),
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

#[derive(Debug)]
pub enum ControlConnectionError {
    Listener(io::Error),
    Peer(io::Error),
}

#[cfg(windows)]
enum ControlPipeReadiness {
    Data,
    Empty,
    Closed,
}

#[cfg(windows)]
fn control_pipe_write_limit(stream: &interprocess::local_socket::Stream) -> io::Result<usize> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Pipes::{GetNamedPipeInfo, PIPE_SERVER_END, PIPE_TYPE_MESSAGE};
    let interprocess::local_socket::Stream::NamedPipe(pipe) = stream;
    let (mut flags, mut output, mut input) = (0, 0, 0);
    if unsafe { GetNamedPipeInfo(pipe.inner().as_raw_handle(), &mut flags, &mut output, &mut input, std::ptr::null_mut()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if flags & PIPE_TYPE_MESSAGE != 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "expected byte control pipe"));
    }
    // FilePipeLocalInformation: client writes use InboundQuota; server writes use OutboundQuota.
    let capacity = if flags & PIPE_SERVER_END != 0 { output } else { input };
    if capacity == 0 {
        return Err(io::Error::new(io::ErrorKind::Unsupported, "control pipe reported no bounded write capacity"));
    }
    Ok(capacity as usize)
}

#[cfg(windows)]
fn control_pipe_readiness(stream: &interprocess::local_socket::Stream) -> io::Result<ControlPipeReadiness> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_BROKEN_PIPE, ERROR_PIPE_NOT_CONNECTED};
    use windows_sys::Win32::System::Pipes::PeekNamedPipe;
    let interprocess::local_socket::Stream::NamedPipe(pipe) = stream;
    let mut available = 0;
    // PIPE_NOWAIT empty reads are ERROR_NO_DATA, which interprocess maps to EOF.
    // Peek preserves the distinction between an empty live pipe and a closed peer.
    let ready = unsafe {
        PeekNamedPipe(
            pipe.inner().as_raw_handle(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut available,
            std::ptr::null_mut(),
        )
    };
    if ready != 0 {
        return Ok(if available == 0 { ControlPipeReadiness::Empty } else { ControlPipeReadiness::Data });
    }
    match unsafe { GetLastError() } {
        ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED => Ok(ControlPipeReadiness::Closed),
        code => Err(io::Error::from_raw_os_error(code as i32)),
    }
}

fn wait_for_signed_peer_close(
    stream: &mut interprocess::local_socket::Stream,
    deadline: std::time::Instant,
) -> io::Result<()> {
    use std::io::Read;
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "signed shutdown peer-close deadline"));
        }
        #[cfg(windows)]
        match control_pipe_readiness(stream)? {
            ControlPipeReadiness::Closed => return Ok(()),
            ControlPipeReadiness::Empty => {
                std::thread::sleep(std::time::Duration::from_millis(2).min(
                    deadline.saturating_duration_since(std::time::Instant::now()),
                ));
                continue;
            }
            ControlPipeReadiness::Data => {}
        }
        let mut byte = [0];
        match stream.read(&mut byte) {
            Ok(0) => return Ok(()),
            Ok(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected signed shutdown trailing data")),
            Err(error) if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => {
                std::thread::sleep(std::time::Duration::from_millis(2).min(
                    deadline.saturating_duration_since(std::time::Instant::now()),
                ));
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(all(test, windows))]
struct PipeCensus {
    actor: &'static str,
    phase: &'static str,
    planned_bytes: Option<usize>,
    pipe: serde_json::Value,
    written: usize,
    read: usize,
    read_calls: usize,
    zero_reads: usize,
    empty_polls: usize,
    write_pairs: std::collections::BTreeMap<String, usize>,
    errors: std::collections::BTreeMap<String, usize>,
}

#[cfg(all(test, windows))]
impl PipeCensus {
    fn new(actor: &'static str, phase: &'static str, planned_bytes: Option<usize>, stream: &interprocess::local_socket::Stream) -> Self {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::Pipes::{GetNamedPipeHandleStateW, GetNamedPipeInfo};
        let interprocess::local_socket::Stream::NamedPipe(pipe) = stream;
        let (mut flags, mut output_capacity, mut input_capacity, mut max_instances, mut state) = (0, 0, 0, 0, 0);
        let handle = pipe.inner().as_raw_handle();
        let info_ok = unsafe { GetNamedPipeInfo(handle, &mut flags, &mut output_capacity, &mut input_capacity, &mut max_instances) };
        let info_error = if info_ok == 0 { io::Error::last_os_error().raw_os_error() } else { None };
        let state_ok = unsafe { GetNamedPipeHandleStateW(handle, &mut state, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), 0) };
        let state_error = if state_ok == 0 { io::Error::last_os_error().raw_os_error() } else { None };
        Self {
            actor, phase, planned_bytes,
            pipe: serde_json::json!({
                "info_ok": info_ok != 0, "info_error": info_error,
                "state_ok": state_ok != 0, "state_error": state_error,
                "flags": flags, "state": state, "server_end": flags & 1 != 0,
                "message_type": flags & 4 != 0, "message_read_mode": state & 2 != 0,
                "nonblocking": state & 1 != 0, "input_capacity": input_capacity,
                "output_capacity": output_capacity, "max_instances": max_instances,
            }),
            written: 0, read: 0, read_calls: 0, zero_reads: 0, empty_polls: 0,
            write_pairs: Default::default(), errors: Default::default(),
        }
    }

    fn note_write(&mut self, requested: usize, result: &io::Result<usize>) {
        match result {
            Ok(count) => {
                self.written += count;
                *self.write_pairs.entry(format!("{requested}:{count}")).or_default() += 1;
            }
            Err(error) => *self.errors.entry(format!("write:{:?}", error.kind())).or_default() += 1,
        }
    }

    fn note_read(&mut self, result: &io::Result<usize>) {
        self.read_calls += 1;
        match result {
            Ok(count) => {
                self.read += count;
                self.zero_reads += usize::from(*count == 0);
            }
            Err(error) => *self.errors.entry(format!("read:{:?}", error.kind())).or_default() += 1,
        }
    }
}

#[cfg(all(test, windows))]
impl Drop for PipeCensus {
    fn drop(&mut self) {
        eprintln!("PIPE_CENSUS {}", serde_json::json!({
            "actor": self.actor, "phase": self.phase, "planned_bytes": self.planned_bytes,
            "pipe": self.pipe, "written_bytes": self.written, "read_bytes": self.read,
            "read_calls": self.read_calls, "zero_reads": self.zero_reads,
            "empty_polls": self.empty_polls, "write_pairs": self.write_pairs, "errors": self.errors,
        }));
    }
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

    /// Additive signed mode. Ordinary launches use the unchanged legacy control implementation.
    /// Each signed-mode frame and reply shares one deadline and owns its socket synchronously.
    pub fn serve_connection_with_signer(
        &self,
        on_message: &mut dyn FnMut(&ControlMessage) -> Result<(), ControlErrorCode>,
        signer: Option<&WatcherSigningBootstrap>,
    ) -> Result<ControlOutcome, ControlConnectionError> {
        use interprocess::local_socket::traits::Stream as _;
        use std::io::Read;
        let Some(signer) = signer else {
            return self
                .serve_connection(on_message)
                .map_err(ControlConnectionError::Listener);
        };
        struct OwnedConnection(interprocess::local_socket::Stream);
        impl Drop for OwnedConnection {
            fn drop(&mut self) {
                #[cfg(windows)]
                if let interprocess::local_socket::Stream::NamedPipe(stream) = &self.0 {
                    stream.inner().assume_flushed();
                }
            }
        }
        let mut conn = OwnedConnection(
            self.listener
                .accept()
                .map_err(ControlConnectionError::Listener)?,
        );
        let mut serve_peer = || -> io::Result<ControlOutcome> {
            #[cfg(all(test, windows))]
            drop(PipeCensus::new("server", "accepted_before_nonblocking", None, &conn.0));
            conn.0.set_nonblocking(true)?;
            #[cfg(windows)]
            let write_limit = control_pipe_write_limit(&conn.0)?;
            let mut frame = Vec::with_capacity(1024);
            loop {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
                frame.clear();
                #[cfg(all(test, windows))]
                let mut read_census = PipeCensus::new("server", "frame_read", None, &conn.0);
                loop {
                    if std::time::Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "signed control frame deadline",
                        ));
                    }
                    #[cfg(windows)]
                    match control_pipe_readiness(&conn.0)? {
                        ControlPipeReadiness::Data => {}
                        ControlPipeReadiness::Empty => {
                            #[cfg(all(test, windows))]
                            { read_census.empty_polls += 1; }
                            std::thread::sleep(std::time::Duration::from_millis(2).min(
                                deadline.saturating_duration_since(std::time::Instant::now()),
                            ));
                            continue;
                        }
                        ControlPipeReadiness::Closed if frame.is_empty() => return Ok(ControlOutcome::Continue),
                        ControlPipeReadiness::Closed => return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof, "partial signed control frame",
                        )),
                    }
                    let mut byte = [0u8; 1];
                    let read_result = conn.0.read(&mut byte);
                    #[cfg(all(test, windows))]
                    read_census.note_read(&read_result);
                    match read_result {
                        Ok(0) if frame.is_empty() => return Ok(ControlOutcome::Continue),
                        Ok(0) => {
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "partial signed control frame",
                            ))
                        }
                        Ok(_) if byte[0] == b'\n' => break,
                        Ok(_) => {
                            frame.push(byte[0]);
                            if frame.len() > 16384 {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "signed control frame too large",
                                ));
                            }
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                            ) =>
                        {
                            std::thread::sleep(std::time::Duration::from_millis(2).min(
                                deadline.saturating_duration_since(std::time::Instant::now()),
                            ));
                        }
                        Err(error) => return Err(error),
                    }
                }
                let msg: ControlMessage =
                    serde_json::from_slice(&frame).map_err(io::Error::other)?;
                let (reply, outcome) = match &msg {
                    ControlMessage::ProvenanceChallengeV1 {
                        identity,
                        challenge,
                        transport_public_key,
                    } => {
                        let reply = match signer.sign_challenge(
                            identity,
                            *challenge,
                            *transport_public_key,
                        ) {
                            Ok(signature) => ControlReply::ProvenanceReadyV1 {
                                identity: signer.identity().clone(),
                                challenge: *challenge,
                                transport_public_key: *transport_public_key,
                                public_verifier: signer.public_verifier(),
                                signature,
                            },
                            Err(_) => ControlReply::Error {
                                code: ControlErrorCode::SigningIdentityMismatch,
                            },
                        };
                        (reply, ControlOutcome::Continue)
                    }
                    _ => {
                        let (reply, outcome) = handle_message(&msg);
                        match on_message(&msg) {
                            Ok(()) => (reply, outcome),
                            Err(code) => (ControlReply::Error { code }, ControlOutcome::Continue),
                        }
                    }
                };
                let mut output = serde_json::to_vec(&reply).map_err(io::Error::other)?;
                output.push(b'\n');
                #[cfg(all(test, windows))]
                let mut write_census = PipeCensus::new("server", match &reply {
                    ControlReply::ProvenanceReadyV1 { .. } => "ProvenanceReadyV1",
                    ControlReply::Pong => "Pong", ControlReply::Ack => "Ack", ControlReply::Error { .. } => "Error",
                }, Some(output.len()), &conn.0);
                let mut written = 0;
                while written < output.len() {
                    if std::time::Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "signed control reply deadline",
                        ));
                    }
                    let remaining = &output[written..];
                    #[cfg(windows)]
                    let offered = &remaining[..remaining.len().min(write_limit)];
                    #[cfg(not(windows))]
                    let offered = remaining;
                    let write_result = conn.0.write(offered);
                    #[cfg(all(test, windows))]
                    write_census.note_write(offered.len(), &write_result);
                    match write_result {
                        Ok(0) => {
                            #[cfg(windows)]
                            if !matches!(control_pipe_readiness(&conn.0)?, ControlPipeReadiness::Closed) {
                                std::thread::sleep(std::time::Duration::from_millis(2).min(
                                    deadline.saturating_duration_since(std::time::Instant::now()),
                                ));
                                continue;
                            }
                            return Err(io::Error::new(
                                io::ErrorKind::WriteZero,
                                "signed control reply closed",
                            ))
                        }
                        Ok(count) => written += count,
                        Err(error)
                            if matches!(
                                error.kind(),
                                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                            ) =>
                        {
                            std::thread::sleep(std::time::Duration::from_millis(2).min(
                                deadline.saturating_duration_since(std::time::Instant::now()),
                            ));
                        }
                        Err(error) => return Err(error),
                    }
                }
                // The resolved Stream's flush is a no-op; release is owned, including Windows no-limbo drop.
                if outcome == ControlOutcome::Shutdown {
                    // Native verifies this ACK before closing the same owned connection. Keep the
                    // response owner alive until that close; EOF alone is not a receipt assertion.
                    wait_for_signed_peer_close(&mut conn.0, deadline)?;
                    return Ok(outcome);
                }
            }
        };
        serve_peer().map_err(ControlConnectionError::Peer)
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
        on_message: &mut dyn FnMut(&ControlMessage) -> Result<(), ControlErrorCode>,
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
            let side_effect = on_message(&msg);
            let (reply, outcome) = handle_message(&msg);
            let reply = match side_effect {
                Ok(()) => reply,
                Err(code) => ControlReply::Error { code },
            };
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
            ControlMessage::RegisterChild {
                pid: 9,
                child_session_id: 44,
                liveness_kind: ControlChildLivenessKind::FileCounter,
                liveness_ref: PathBuf::from("child.progress"),
                threshold_ms: 5000,
            },
            ControlMessage::DeregisterChild {
                pid: 9,
                child_session_id: 44,
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
        assert_eq!(
            outcome,
            ControlOutcome::Continue,
            "Ping must NOT exit (AC-009-3)"
        );
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
        let (_, o3) = handle_message(&ControlMessage::RegisterChild {
            pid: 9,
            child_session_id: 44,
            liveness_kind: ControlChildLivenessKind::FileCounter,
            liveness_ref: PathBuf::from("child.progress"),
            threshold_ms: 5000,
        });
        assert_eq!(o3, ControlOutcome::Continue);
    }

    #[test]
    fn shutdown_resolves_to_shutdown_outcome() {
        let (reply, outcome) = handle_message(&ControlMessage::Shutdown);
        assert_eq!(reply, ControlReply::Ack);
        assert_eq!(outcome, ControlOutcome::Shutdown);
    }

    #[test]
    fn error_reply_shape_is_typed() {
        let line = serde_json::to_string(&ControlReply::Error {
            code: ControlErrorCode::ChildRegisterFailed,
        })
        .unwrap();
        assert_eq!(line, r#"{"type":"Error","code":"ChildRegisterFailed"}"#);
        let back: ControlReply = serde_json::from_str(&line).unwrap();
        assert_eq!(
            back,
            ControlReply::Error {
                code: ControlErrorCode::ChildRegisterFailed
            }
        );
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
        assert_eq!(
            hello,
            r#"{"type":"HandshakeHello","parent_pid":7,"session_id":"z"}"#
        );
        let register = serde_json::to_string(&ControlMessage::RegisterChild {
            pid: 9,
            child_session_id: 44,
            liveness_kind: ControlChildLivenessKind::FileCounter,
            liveness_ref: PathBuf::from("child.progress"),
            threshold_ms: 5000,
        })
        .unwrap();
        assert_eq!(
            register,
            r#"{"type":"RegisterChild","pid":9,"child_session_id":44,"liveness_kind":"FileCounter","liveness_ref":"child.progress","threshold_ms":5000}"#
        );
    }
}

pub use palmistry_control_protocol::{watcher_challenge_bytes, WatcherSigningIdentity};

fn valid_runtime_identity(identity: &WatcherSigningIdentity) -> bool {
    crate::cli::SigningRuntimeIdentity {
        runtime_instance_schema_id: identity.runtime_instance_schema_id.clone(),
        runtime_instance_id: identity.runtime_instance_id,
        runtime_host_scope_id: identity.runtime_host_scope_id.clone(),
        runtime_lease_protocol: identity.runtime_lease_protocol.clone(),
        runtime_lease_address: identity.runtime_lease_address,
        runtime_lease_port: identity.runtime_lease_port,
        parent_os_creation_time_100ns: identity.parent_os_creation_time_100ns,
    }
    .is_valid_for_session(identity.session_id)
}

/// Source-compatible signing identity only. No backend-issued native HMAC key is held here.
/// Deliberately has no Debug/Serialize implementation.
pub struct WatcherSigningBootstrap {
    identity: WatcherSigningIdentity,
    key: ed25519_dalek::SigningKey,
}
impl WatcherSigningBootstrap {
    pub fn from_private_seed(
        identity: WatcherSigningIdentity,
        seed: zeroize::Zeroizing<Vec<u8>>,
    ) -> io::Result<Self> {
        if seed.len() != 32
            || identity.protocol_version != 1
            || identity.session_id.is_nil()
            || identity.process_instance_id.is_nil()
            || identity.process_instance_id == identity.session_id
            || identity.parent_pid == 0
            || identity.watcher_pid == 0
            || identity.ring_version != 2
            || identity.record_size != 120
            || identity.launch_nonce == [0; 16]
            || !valid_runtime_identity(&identity)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid watcher signing bootstrap shape",
            ));
        }
        let exact =
            zeroize::Zeroizing::new(<[u8; 32]>::try_from(seed.as_slice()).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "signing seed must be exactly 32 bytes",
                )
            })?);
        let key = ed25519_dalek::SigningKey::from_bytes(&exact);
        Ok(Self { identity, key })
    }
    pub fn identity(&self) -> &WatcherSigningIdentity {
        &self.identity
    }
    pub fn public_verifier(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }
    /// Sign fresh public challenge plus exact current identity. This does not attest durable START.
    pub fn sign_challenge(
        &self,
        identity: &WatcherSigningIdentity,
        challenge: [u8; 32],
        transport_public_key: [u8; 32],
    ) -> io::Result<Vec<u8>> {
        use ed25519_dalek::Signer;
        if identity != &self.identity || challenge == [0; 32] || transport_public_key == [0; 32] {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "watcher provenance identity mismatch",
            ));
        }
        let bytes = watcher_challenge_bytes(identity, challenge, transport_public_key)?;
        Ok(self.key.sign(&bytes).to_bytes().to_vec())
    }
}

/// Read one inherited PRIVATE pipe to EOF. Exact 32 bytes only, bounded time and zeroized buffers.
/// The seed never arrives via CLI/environment/control JSON/logging.
pub fn read_private_signing_seed(
    timeout: std::time::Duration,
) -> io::Result<zeroize::Zeroizing<Vec<u8>>> {
    let deadline = std::time::Instant::now() + timeout;
    let mut bytes = zeroize::Zeroizing::new(Vec::with_capacity(33));
    let mut scratch = zeroize::Zeroizing::new([0u8; 33]);
    #[cfg(windows)]
    let handle = {
        use windows_sys::Win32::Storage::FileSystem::{GetFileType, FILE_TYPE_PIPE};
        use windows_sys::Win32::System::Console::{GetStdHandle, STD_INPUT_HANDLE};
        // SAFETY: query our inherited stdin handle only; no transfer or foreign process handle.
        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if handle.is_null() || unsafe { GetFileType(handle) } != FILE_TYPE_PIPE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "private signing bootstrap requires inherited pipe",
            ));
        }
        handle
    };
    #[cfg(unix)]
    {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: valid out-pointer and our process's inherited stdin fd only.
        if unsafe { libc::fstat(libc::STDIN_FILENO, stat.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let stat = unsafe { stat.assume_init() };
        if stat.st_mode & libc::S_IFMT != libc::S_IFIFO {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "private signing bootstrap requires inherited pipe",
            ));
        }
    }
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "private signing bootstrap deadline elapsed",
            ));
        }
        let count: usize;
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::{GetLastError, ERROR_BROKEN_PIPE};
            use windows_sys::Win32::Storage::FileSystem::ReadFile;
            use windows_sys::Win32::System::Pipes::PeekNamedPipe;
            let mut available = 0;
            // SAFETY: borrowed inherited pipe handle, no read unless available bytes are reported.
            if unsafe {
                PeekNamedPipe(
                    handle,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut available,
                    std::ptr::null_mut(),
                )
            } == 0
            {
                let error = unsafe { GetLastError() };
                if error == ERROR_BROKEN_PIPE {
                    break;
                }
                return Err(io::Error::from_raw_os_error(error as i32));
            }
            if available == 0 {
                std::thread::sleep(
                    std::time::Duration::from_millis(5)
                        .min(deadline.saturating_duration_since(std::time::Instant::now())),
                );
                continue;
            }
            let mut read = 0;
            let want = available.min((33 - bytes.len()) as u32);
            if unsafe {
                ReadFile(
                    handle,
                    scratch.as_mut_ptr(),
                    want,
                    &mut read,
                    std::ptr::null_mut(),
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            count = read as usize;
        }
        #[cfg(unix)]
        {
            let mut poll = libc::pollfd {
                fd: libc::STDIN_FILENO,
                events: libc::POLLIN,
                revents: 0,
            };
            let wait = deadline
                .saturating_duration_since(std::time::Instant::now())
                .as_millis()
                .min(20) as i32;
            // SAFETY: one valid fd poll record; bounded timeout before a pipe read.
            let result = unsafe { libc::poll(&mut poll, 1, wait) };
            if result < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
            if result == 0 {
                continue;
            }
            if poll.revents & (libc::POLLIN | libc::POLLHUP) == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "private signing pipe became unavailable",
                ));
            }
            let read = unsafe {
                libc::read(
                    libc::STDIN_FILENO,
                    scratch.as_mut_ptr().cast(),
                    33 - bytes.len(),
                )
            };
            if read < 0 {
                return Err(io::Error::last_os_error());
            }
            count = read as usize;
        }
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&scratch[..count]);
        zeroize::Zeroize::zeroize(&mut *scratch);
        if bytes.len() > 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "private signing seed exceeds 32 bytes",
            ));
        }
    }
    if bytes.len() != 32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "private signing seed must be exactly 32 bytes",
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod signing_bootstrap_tests {
    use super::*;
    fn identity() -> WatcherSigningIdentity {
        WatcherSigningIdentity {
            protocol_version: 1,
            session_id: uuid::Uuid::from_u128(1),
            process_instance_id: uuid::Uuid::from_u128(2),
            parent_pid: 3,
            watcher_pid: 4,
            ring_version: 2,
            record_size: 120,
            launch_nonce: [5; 16],
            runtime_instance_schema_id: crate::cli::RUNTIME_INSTANCE_SCHEMA_ID.into(),
            runtime_instance_id: uuid::Uuid::from_u128(1),
            runtime_host_scope_id: "component-host".into(),
            runtime_lease_protocol: crate::cli::RUNTIME_LEASE_PROTOCOL.into(),
            runtime_lease_address: std::net::Ipv4Addr::LOCALHOST.into(),
            runtime_lease_port: 12345,
            parent_os_creation_time_100ns: 42,
        }
    }

    #[cfg(windows)]
    fn signed_pipe(label: &str) -> (ControlServer, WatcherSigningBootstrap) {
        let name = format!("palmistry-{label}-{}", uuid::Uuid::now_v7());
        let server = ControlServer::bind(&name).unwrap();
        server.listener.set_nonblocking(interprocess::local_socket::ListenerNonblockingMode::Accept).unwrap();
        let signer = WatcherSigningBootstrap::from_private_seed(
            identity(), zeroize::Zeroizing::new(vec![7; 32]),
        ).unwrap();
        (server, signer)
    }

    #[cfg(windows)]
    type SignedPeerCompletion = (ControlServer, WatcherSigningBootstrap, Result<ControlOutcome, ControlConnectionError>);

    #[cfg(windows)]
    fn signed_peer(
        server: ControlServer, signer: WatcherSigningBootstrap, deadline: std::time::Instant,
        deny_shutdown: bool,
    ) -> std::thread::JoinHandle<SignedPeerCompletion> {
        std::thread::spawn(move || {
          let result = loop {
            if std::time::Instant::now() >= deadline {
                break Err(ControlConnectionError::Listener(io::ErrorKind::TimedOut.into()));
            }
            match server.serve_connection_with_signer(&mut |msg| {
                if deny_shutdown && matches!(msg, ControlMessage::Shutdown) {
                    Err(ControlErrorCode::SigningUnavailable)
                } else { Ok(()) }
            }, Some(&signer)) {
                Err(ControlConnectionError::Listener(error)) if error.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(2).min(
                        deadline.saturating_duration_since(std::time::Instant::now()),
                    ));
                }
                result => break result,
            }
          };
          (server, signer, result)
        })
    }

    #[cfg(windows)]
    fn finish_signed_peer(
        worker: std::thread::JoinHandle<SignedPeerCompletion>,
        deadline: std::time::Instant,
    ) -> SignedPeerCompletion {
        while !worker.is_finished() {
            assert!(std::time::Instant::now() < deadline, "signed peer completion deadline");
            std::thread::sleep(std::time::Duration::from_millis(2).min(
                deadline.saturating_duration_since(std::time::Instant::now()),
            ));
        }
        worker.join().unwrap()
    }

    #[cfg(windows)]
    struct SignedPipeClient(interprocess::local_socket::Stream);

    #[cfg(windows)]
    impl SignedPipeClient {
        fn connect(name: &str, deadline: std::time::Instant) -> io::Result<Self> {
            use interprocess::local_socket::traits::Stream as _;
            use interprocess::os::windows::named_pipe::{local_socket::Stream, pipe_mode::Bytes, DuplexPipeStream};
            let path = format!(r"\\.\pipe\{name}");
            loop {
                if std::time::Instant::now() >= deadline { return Err(io::ErrorKind::TimedOut.into()); }
                match DuplexPipeStream::<Bytes>::connect_by_path_with_wait_mode(
                    path.as_str(), interprocess::ConnectWaitMode::Timeout(std::time::Duration::ZERO),
                ) {
                    Ok(pipe) => {
                        let stream = interprocess::local_socket::Stream::from(Stream::from(pipe));
                        stream.set_nonblocking(true)?;
                        return Ok(Self(stream));
                    }
                    Err(_) => std::thread::sleep(std::time::Duration::from_millis(2).min(
                        deadline.saturating_duration_since(std::time::Instant::now()),
                    )),
                }
            }
        }

        fn write(&mut self, mut bytes: &[u8], deadline: std::time::Instant) -> io::Result<()> {
            let mut census = PipeCensus::new("client", "request_write", Some(bytes.len()), &self.0);
            let write_limit = control_pipe_write_limit(&self.0)?;
            while !bytes.is_empty() {
                if std::time::Instant::now() >= deadline { return Err(io::ErrorKind::TimedOut.into()); }
                let offered = &bytes[..bytes.len().min(write_limit)];
                let result = self.0.write(offered);
                census.note_write(offered.len(), &result);
                match result {
                    Ok(0) => {
                        if matches!(control_pipe_readiness(&self.0)?, ControlPipeReadiness::Closed) {
                            return Err(io::ErrorKind::WriteZero.into());
                        }
                        std::thread::sleep(std::time::Duration::from_millis(2).min(
                            deadline.saturating_duration_since(std::time::Instant::now()),
                        ));
                    }
                    Ok(count) => bytes = &bytes[count..],
                    Err(error) if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => {
                        std::thread::sleep(std::time::Duration::from_millis(2).min(
                            deadline.saturating_duration_since(std::time::Instant::now()),
                        ));
                    }
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        }

        fn read_reply(&mut self, deadline: std::time::Instant) -> io::Result<ControlReply> {
            use std::io::Read;
            let mut census = PipeCensus::new("client", "reply_read", None, &self.0);
            let mut frame = Vec::new();
            loop {
                if std::time::Instant::now() >= deadline { return Err(io::ErrorKind::TimedOut.into()); }
                match control_pipe_readiness(&self.0)? {
                    ControlPipeReadiness::Empty => {
                        census.empty_polls += 1;
                        std::thread::sleep(std::time::Duration::from_millis(2).min(
                            deadline.saturating_duration_since(std::time::Instant::now()),
                        ));
                        continue;
                    }
                    ControlPipeReadiness::Closed => return Err(io::ErrorKind::UnexpectedEof.into()),
                    ControlPipeReadiness::Data => {}
                }
                let mut byte = [0];
                let result = self.0.read(&mut byte);
                census.note_read(&result);
                match result? {
                    0 => return Err(io::ErrorKind::UnexpectedEof.into()),
                    _ if byte[0] == b'\n' => return serde_json::from_slice(&frame).map_err(io::Error::other),
                    _ => frame.push(byte[0]),
                }
                if frame.len() > 16384 { return Err(io::ErrorKind::InvalidData.into()); }
            }
        }
    }

    #[cfg(windows)]
    impl Drop for SignedPipeClient {
        fn drop(&mut self) {
            let interprocess::local_socket::Stream::NamedPipe(pipe) = &self.0;
            pipe.inner().assume_flushed();
        }
    }

    #[cfg(windows)]
    #[test]
    fn signed_pipe_accepts_delayed_fragmented_frames_and_shutdown() {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let (server, signer) = signed_pipe("delayed");
        let name = server.socket_name().to_owned();
        let worker = signed_peer(server, signer, deadline, false);
        let mut stream = SignedPipeClient::connect(&name, deadline).unwrap();
        let challenge = ControlMessage::ProvenanceChallengeV1 {
            identity: identity(), challenge: [8; 32], transport_public_key: [9; 32],
        };
        let frame = encode_message(&challenge).unwrap();
        let wire_roundtrip = decode_message(frame.trim_end_matches(['\n', '\r']))
            .map(|decoded| decoded == challenge);
        std::thread::sleep(std::time::Duration::from_millis(50));
        stream.write(&frame.as_bytes()[..frame.len() / 2], deadline).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        stream.write(&frame.as_bytes()[frame.len() / 2..], deadline).unwrap();
        let reply = match stream.read_reply(deadline) {
            Ok(reply) => reply,
            Err(error) => {
                drop(stream);
                let (_, _, peer_result) = finish_signed_peer(worker, deadline);
                panic!("signed challenge read failed: {error:?}; actual peer: {peer_result:?}; whole ControlMessage roundtrip: {wire_roundtrip:?}");
            }
        };
        assert_eq!(wire_roundtrip.unwrap(), true, "whole producer/consumer message must roundtrip");
        match reply {
            ControlReply::ProvenanceReadyV1 { identity: actual, challenge, transport_public_key, .. } => {
                assert_eq!(actual, identity());
                assert_eq!(challenge, [8; 32]);
                assert_eq!(transport_public_key, [9; 32]);
            }
            reply => panic!("expected signed ready, got {reply:?}"),
        }
        let unfragmented_message = ControlMessage::ProvenanceChallengeV1 {
            identity: identity(), challenge: [10; 32], transport_public_key: [9; 32],
        };
        let mut unfragmented = encode_message(&unfragmented_message).unwrap();
        let capacity = control_pipe_write_limit(&stream.0).unwrap();
        const MAX_FRAME_BYTES: usize = 16384;
        const MAX_WIRE_BYTES: usize = MAX_FRAME_BYTES + 1;
        if unfragmented.len() <= capacity && capacity < MAX_WIRE_BYTES {
            assert_eq!(unfragmented.pop(), Some('\n'));
            unfragmented.extend(std::iter::repeat_n(' ', capacity - unfragmented.len()));
            unfragmented.push('\n');
        }
        assert!(unfragmented.len() <= MAX_WIRE_BYTES);
        assert!(unfragmented.len() > capacity || capacity >= MAX_WIRE_BYTES);
        assert_eq!(decode_message(&unfragmented).unwrap(), unfragmented_message);
        eprintln!("PIPE_COVERAGE {}", serde_json::json!({
            "unfragmented_request_bytes": unfragmented.len(), "client_write_capacity": capacity,
            "oversize_request_covered": unfragmented.len() > capacity,
            "all_allowed_frames_fit_capacity": capacity >= MAX_WIRE_BYTES,
            "max_frame_body_bytes": MAX_FRAME_BYTES,
        }));
        stream.write(unfragmented.as_bytes(), deadline).unwrap();
        match stream.read_reply(deadline).unwrap() {
            ControlReply::ProvenanceReadyV1 { identity: actual, challenge, transport_public_key, .. } => {
                assert_eq!(actual, identity());
                assert_eq!(challenge, [10; 32]);
                assert_eq!(transport_public_key, [9; 32]);
            }
            reply => panic!("expected signed reply to unfragmented request, got {reply:?}"),
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        stream.write(b"{", deadline).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        stream.write(b"\"type\":\"Shutdown\"}\n", deadline).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(!worker.is_finished(), "shutdown cannot retire while the ACK peer remains open");
        assert_eq!(stream.read_reply(deadline).unwrap(), ControlReply::Ack);
        drop(stream);
        assert_eq!(finish_signed_peer(worker, deadline).2.unwrap(), ControlOutcome::Shutdown);
    }

    #[cfg(windows)]
    #[test]
    fn signed_pipe_failures_keep_listener_available_until_a_drained_shutdown() {
        let (mut server, mut signer) = signed_pipe("reuse");
        for case in ["eof", "malformed", "partial-timeout", "denied", "held-ack", "trailing", "shutdown"] {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            let name = server.socket_name().to_owned();
            let worker = signed_peer(server, signer, deadline, case == "denied");
            let mut stream = SignedPipeClient::connect(&name, deadline).unwrap();
            match case {
                "eof" => {
                    stream.write(b"{\"type\":\"Ping\"}\n", deadline).unwrap();
                    assert_eq!(stream.read_reply(deadline).unwrap(), ControlReply::Pong);
                }
                "malformed" => stream.write(b"{malformed}\n", deadline).unwrap(),
                "partial-timeout" => stream.write(b"{", deadline).unwrap(),
                _ => {
                    stream.write(b"{\"type\":\"Shutdown\"}\n", deadline).unwrap();
                    let reply = stream.read_reply(deadline).unwrap();
                    if case == "denied" {
                        assert_eq!(reply, ControlReply::Error { code: ControlErrorCode::SigningUnavailable });
                    } else { assert_eq!(reply, ControlReply::Ack); }
                    if case == "trailing" { stream.write(b"x", deadline).unwrap(); }
                }
            }
            let keep_peer_open = matches!(case, "malformed" | "partial-timeout" | "held-ack" | "trailing");
            let mut peer = Some(stream);
            if !keep_peer_open { drop(peer.take()); }
            let (returned_server, returned_signer, result) = finish_signed_peer(worker, deadline);
            drop(peer);
            server = returned_server;
            signer = returned_signer;
            match (case, result) {
                ("eof" | "denied", Ok(ControlOutcome::Continue)) => {}
                ("partial-timeout" | "held-ack", Err(ControlConnectionError::Peer(error))) => assert_eq!(error.kind(), io::ErrorKind::TimedOut),
                ("malformed", Err(ControlConnectionError::Peer(_))) => {}
                ("trailing", Err(ControlConnectionError::Peer(error))) => assert_eq!(error.kind(), io::ErrorKind::InvalidData),
                ("shutdown", Ok(ControlOutcome::Shutdown)) => {}
                (case, result) => panic!("unexpected signed connection result ({case}): {result:?}"),
            }
        }
    }

    #[test]
    fn exact_seed_and_signed_original_identity_fail_closed() {
        let mut invalid = identity();
        invalid.runtime_instance_id = invalid.process_instance_id;
        assert!(WatcherSigningBootstrap::from_private_seed(
            invalid,
            zeroize::Zeroizing::new(vec![7; 32]),
        )
        .is_err());
        let mut invalid = identity();
        invalid.parent_os_creation_time_100ns = 0;
        assert!(WatcherSigningBootstrap::from_private_seed(
            invalid,
            zeroize::Zeroizing::new(vec![7; 32]),
        )
        .is_err());
        for size in [31, 33] {
            assert!(WatcherSigningBootstrap::from_private_seed(
                identity(),
                zeroize::Zeroizing::new(vec![7; size])
            )
            .is_err());
        }
        let signer = WatcherSigningBootstrap::from_private_seed(
            identity(),
            zeroize::Zeroizing::new(vec![7; 32]),
        )
        .unwrap();
        let signature = signer
            .sign_challenge(&identity(), [8; 32], [9; 32])
            .unwrap();
        assert_eq!(signature.len(), 64);
        let verifier = ed25519_dalek::VerifyingKey::from_bytes(&signer.public_verifier()).unwrap();
        let signature = ed25519_dalek::Signature::from_slice(&signature).unwrap();
        verifier
            .verify_strict(
                &watcher_challenge_bytes(&identity(), [8; 32], [9; 32]).unwrap(),
                &signature,
            )
            .unwrap();
        assert!(verifier
            .verify_strict(
                &watcher_challenge_bytes(&identity(), [10; 32], [9; 32]).unwrap(),
                &signature
            )
            .is_err());
        let mut foreign = identity();
        foreign.launch_nonce[0] ^= 1;
        assert!(signer.sign_challenge(&foreign, [8; 32], [9; 32]).is_err());
        foreign = identity();
        foreign.watcher_pid += 1;
        assert!(signer.sign_challenge(&foreign, [8; 32], [9; 32]).is_err());
        foreign = identity();
        foreign.parent_os_creation_time_100ns += 1;
        assert!(signer.sign_challenge(&foreign, [8; 32], [9; 32]).is_err());
        foreign = identity();
        foreign.runtime_lease_port += 1;
        assert!(signer.sign_challenge(&foreign, [8; 32], [9; 32]).is_err());
        let mut wire = serde_json::to_value(identity()).unwrap();
        wire["unknown_signed_field"] = serde_json::json!(1);
        assert!(serde_json::from_value::<WatcherSigningIdentity>(wire).is_err());
        let mut wire = serde_json::to_value(identity()).unwrap();
        wire.as_object_mut()
            .unwrap()
            .remove("runtime_instance_schema_id");
        assert!(serde_json::from_value::<WatcherSigningIdentity>(wire).is_err());
        assert!(signer
            .sign_challenge(&identity(), [0; 32], [9; 32])
            .is_err());
        assert!(signer
            .sign_challenge(&identity(), [8; 32], [0; 32])
            .is_err());
        let (reply, outcome) = handle_message(&ControlMessage::ProvenanceChallengeV1 {
            identity: identity(),
            challenge: [8; 32],
            transport_public_key: [9; 32],
        });
        assert_eq!(
            reply,
            ControlReply::Error {
                code: ControlErrorCode::SigningUnavailable
            }
        );
        assert_eq!(outcome, ControlOutcome::Continue);
    }
}
