//! WP-KERNEL-012 MT-037 (E6 — backend reuse wiring): the `backend` submodule that holds the typed
//! native Rust clients consolidating the EXISTING handshake_core HTTP surfaces the editors bind. The
//! shared HTTP client + base-URL resolution + the canonical document identity-header constants live in
//! the WP-011 [`crate::backend_client`] module (NOT forked here); these submodules build typed,
//! per-domain request/response clients on top of that shared transport.
//!
//! MT-037 delivers [`knowledge_documents`]: the consolidated client for the full
//! `/knowledge/documents/*` route family. It is reachable as
//! `handshake_native::backend::knowledge_documents`.
//!
//! MT-038 delivers [`loom`]: the consolidated, unified client namespace for the
//! `/workspaces/:ws/loom/*` route family. It RE-EXPORTS the existing WP-011/MT-021..032 egui-thread
//! builder clients (so `handshake_native::backend::loom::<X>` resolves to the SAME type without forking
//! them) and ADDS a stateless async [`loom::LoomClient`] for the genuinely-missing + editor-consumed
//! routes (block CRUD, edges, daily journal, knowledge-bridge, transclusion, folder membership, wiki
//! extras, markdown import, visual-debug). It is reachable as `handshake_native::backend::loom`.

pub mod knowledge_documents;
pub mod loom;
