//! WP-KERNEL-012 MT-103 — foreground-safe model navigation driver.
//!
//! This module is a thin sequence wrapper over the existing MCP/AccessKit steering primitives. It
//! composes `dispatch_request` for the JSON-RPC tool surface and `ActionChannel` for the focus-only
//! step that has no public JSON-RPC method today. It never calls OS foreground or input APIs.

use crate::accessibility::UiTreeSnapshot;
use crate::mcp::{
    dispatch_request, ActionChannel, McpRequest, SessionToken, UiAction, ERR_UNAUTHORIZED,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationStep {
    OpenPane {
        target: String,
        expected_pane: String,
    },
    Click {
        target: String,
    },
    SetValue {
        target: String,
        value: String,
    },
    Focus {
        target: String,
    },
}

impl NavigationStep {
    pub fn open_pane(target: impl Into<String>, expected_pane: impl Into<String>) -> Self {
        Self::OpenPane {
            target: target.into(),
            expected_pane: expected_pane.into(),
        }
    }

    pub fn click(target: impl Into<String>) -> Self {
        Self::Click {
            target: target.into(),
        }
    }

    pub fn set_value(target: impl Into<String>, value: impl Into<String>) -> Self {
        Self::SetValue {
            target: target.into(),
            value: value.into(),
        }
    }

    pub fn focus(target: impl Into<String>) -> Self {
        Self::Focus {
            target: target.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationReceipt {
    pub index: usize,
    pub target: String,
    pub action: String,
    pub node_id: u64,
    pub text_payload: Option<String>,
    pub expected_pane: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationError {
    Unauthorized,
    Tool {
        index: usize,
        target: String,
        code: i64,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationSequence {
    steps: Vec<NavigationStep>,
}

impl NavigationSequence {
    pub fn new(steps: Vec<NavigationStep>) -> Self {
        Self { steps }
    }

    pub fn steps(&self) -> &[NavigationStep] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn dispatch_step(
        &self,
        index: usize,
        token: &SessionToken,
        presented_token: &str,
        snapshot: &UiTreeSnapshot,
        channel: &mut ActionChannel,
    ) -> Result<NavigationReceipt, NavigationError> {
        // One-step dispatch lets model workflows prove that each action was resolved from the
        // current AccessKit/MCP tree after the previous action was applied.
        let Some(step) = self.steps.get(index) else {
            return Err(NavigationError::Tool {
                index,
                target: "<sequence-index>".to_owned(),
                code: crate::mcp::ERR_INVALID_PARAMS,
                message: format!("navigation step index {index} out of range"),
            });
        };
        match step {
            NavigationStep::OpenPane {
                target,
                expected_pane,
            } => {
                let mut receipt = dispatch_tool(
                    index,
                    "click_widget",
                    target,
                    None,
                    token,
                    presented_token,
                    snapshot,
                    channel,
                )?;
                receipt.expected_pane = Some(expected_pane.clone());
                Ok(receipt)
            }
            NavigationStep::Click { target } => dispatch_tool(
                index,
                "click_widget",
                target,
                None,
                token,
                presented_token,
                snapshot,
                channel,
            ),
            NavigationStep::SetValue { target, value } => dispatch_tool(
                index,
                "set_value",
                target,
                Some(value),
                token,
                presented_token,
                snapshot,
                channel,
            ),
            NavigationStep::Focus { target } => {
                dispatch_focus(index, target, token, presented_token, snapshot, channel)
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_tool(
    index: usize,
    method: &str,
    target: &str,
    value: Option<&str>,
    token: &SessionToken,
    presented_token: &str,
    snapshot: &UiTreeSnapshot,
    channel: &mut ActionChannel,
) -> Result<NavigationReceipt, NavigationError> {
    let params = match value {
        Some(value) => serde_json::json!({ "target": target, "value": value }),
        None => serde_json::json!({ "target": target }),
    };
    let response = dispatch_request(
        &McpRequest {
            id: serde_json::json!(index),
            method: method.to_owned(),
            params,
            session_token: presented_token.to_owned(),
        },
        token,
        snapshot,
        channel,
        || {
            Err(crate::mcp::ScreenshotError(
                "navigation sequence does not capture screenshots".to_owned(),
            ))
        },
    );

    match response.result_ref() {
        Ok(result) => Ok(NavigationReceipt {
            index,
            target: target.to_owned(),
            action: result
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_owned(),
            node_id: result.get("node_id").and_then(|v| v.as_u64()).unwrap_or(0),
            text_payload: value.map(str::to_owned),
            expected_pane: None,
        }),
        Err(error) if error.code == ERR_UNAUTHORIZED => Err(NavigationError::Unauthorized),
        Err(error) => Err(NavigationError::Tool {
            index,
            target: target.to_owned(),
            code: error.code,
            message: error.message.clone(),
        }),
    }
}

fn dispatch_focus(
    index: usize,
    target: &str,
    token: &SessionToken,
    presented_token: &str,
    snapshot: &UiTreeSnapshot,
    channel: &mut ActionChannel,
) -> Result<NavigationReceipt, NavigationError> {
    if !token.matches(presented_token) {
        return Err(NavigationError::Unauthorized);
    }
    let outcome = channel
        .enqueue(snapshot, target, UiAction::Focus)
        .map_err(|error| NavigationError::Tool {
            index,
            target: target.to_owned(),
            code: crate::mcp::McpError::from(error.clone()).code,
            message: error.to_string(),
        })?;
    Ok(NavigationReceipt {
        index,
        target: target.to_owned(),
        action: format!("{:?}", outcome.request.action),
        node_id: outcome.request.target.0,
        text_payload: outcome.text_payload,
        expected_pane: None,
    })
}
