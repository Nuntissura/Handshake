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
            conn.0.set_nonblocking(true)?;
            let mut frame = Vec::with_capacity(1024);
            loop {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
                frame.clear();
                loop {
                    if std::time::Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "signed control frame deadline",
                        ));
                    }
                    let mut byte = [0u8; 1];
                    match conn.0.read(&mut byte) {
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
                        (
                            match on_message(&msg) {
                                Ok(()) => reply,
                                Err(code) => ControlReply::Error { code },
                            },
                            outcome,
                        )
                    }
                };
                let mut output = serde_json::to_vec(&reply).map_err(io::Error::other)?;
                output.push(b'\n');
                let mut written = 0;
                while written < output.len() {
                    if std::time::Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "signed control reply deadline",
                        ));
                    }
                    match conn.0.write(&output[written..]) {
                        Ok(0) => {
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
