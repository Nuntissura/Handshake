//! WP-KERNEL-012 MT-098: Runtime Chat pane mounted beside the native editor work surface.
//!
//! The current `handshake_core` native HTTP surface has no assistant chat send/receive route. The pane
//! therefore exposes a real input and send control whose production client probes `POST /chat` through
//! the local HTTP transport and maps handshake_core's real 404 fallback to the typed
//! [`ChatSendError::EndpointMissing`] blocker. It never fabricates an assistant turn or misuses Flight
//! Recorder runtime-chat event ingestion as a chat backend.

mod panel;

pub use panel::{
    runtime_chat_turn_body_author_id, runtime_chat_turn_role_author_id, ChatPaneFactory, ChatRole,
    ChatSendError, ChatTurn, RuntimeChatClient, RuntimeChatPanel, RUNTIME_CHAT_INPUT_AUTHOR_ID,
    RUNTIME_CHAT_PANEL_AUTHOR_ID, RUNTIME_CHAT_SEND_AUTHOR_ID, RUNTIME_CHAT_STATUS_AUTHOR_ID,
};
