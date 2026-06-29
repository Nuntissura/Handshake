//! WP-KERNEL-012 MT-098: Runtime Chat pane mounted beside the native editor work surface.
//!
//! The current `handshake_core` native HTTP surface has no assistant chat send/receive route. The pane
//! therefore exposes a real input and send control, but its production client returns the typed
//! [`ChatSendError::EndpointMissing`] blocker instead of fabricating an assistant turn or misusing the
//! Flight Recorder runtime-chat ingestion endpoint as a chat backend.

mod panel;

pub use panel::{
    ChatPaneFactory, ChatRole, ChatSendError, ChatTurn, RuntimeChatClient, RuntimeChatPanel,
    RUNTIME_CHAT_INPUT_AUTHOR_ID, RUNTIME_CHAT_PANEL_AUTHOR_ID, RUNTIME_CHAT_SEND_AUTHOR_ID,
    RUNTIME_CHAT_STATUS_AUTHOR_ID,
};
