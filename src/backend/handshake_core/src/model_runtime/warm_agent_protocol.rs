use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::sandbox::SnapshotRef;

pub const WARM_AGENT_PROTOCOL_ID: &str = "hsk.warm_agent";
pub const WARM_AGENT_PROTOCOL_VERSION: u16 = 1;
pub const WARM_AGENT_MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WarmAgentProtocolError {
    #[error("warm-agent frame exceeds max size: {actual} > {max}")]
    FrameTooLarge { actual: usize, max: usize },
    #[error("warm-agent frame JSON error: {0}")]
    Json(String),
    #[error("warm-agent protocol mismatch")]
    ProtocolMismatch { expected: String, actual: String },
    #[error("warm-agent version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u16, actual: u16 },
    #[error("warm-agent model hash mismatch")]
    ModelHashMismatch { expected: String, actual: String },
    #[error("warm-agent model guest path mismatch")]
    ModelGuestPathMismatch { expected: String, actual: String },
    #[error("warm-agent unexpected frame: expected {expected}, got {actual}")]
    UnexpectedFrame {
        expected: &'static str,
        actual: &'static str,
    },
    #[error("warm-agent request id mismatch for {frame_type}")]
    RequestIdMismatch {
        frame_type: &'static str,
        expected: String,
        actual: String,
    },
}

impl From<serde_json::Error> for WarmAgentProtocolError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarmAgentGenerateRequest {
    pub request_id: String,
    pub model_id: String,
    pub model_guest_path: String,
    pub model_artifact_sha256: String,
    pub prompt: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum WarmAgentHostFrame {
    Load {
        request_id: String,
        model_guest_path: String,
        model_artifact_sha256: String,
    },
    Generate {
        request: WarmAgentGenerateRequest,
    },
    Cancel {
        request_id: String,
    },
    Ping {
        request_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum WarmAgentGuestFrame {
    Ready {
        protocol_id: String,
        protocol_version: u16,
        agent_id: String,
        ready_nonce: String,
        loaded_model_sha256: Option<String>,
        #[serde(default)]
        loaded_model_guest_path: Option<String>,
    },
    Token {
        request_id: String,
        token_id: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token_index: Option<u32>,
        text: String,
    },
    Complete {
        request_id: String,
        finish_reason: String,
    },
    Error {
        request_id: Option<String>,
        code: String,
        message: String,
    },
    Heartbeat {
        request_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarmVmSnapshotManifest {
    pub protocol_id: String,
    pub protocol_version: u16,
    pub worktree_id: String,
    pub model_artifact_sha256: String,
    pub model_guest_path: String,
    pub ready_nonce: String,
    pub snapshot: SnapshotRef,
}

impl WarmVmSnapshotManifest {
    pub fn new(
        worktree_id: impl Into<String>,
        model_artifact_sha256: impl Into<String>,
        model_guest_path: impl Into<String>,
        ready_nonce: impl Into<String>,
        snapshot: SnapshotRef,
    ) -> Self {
        Self {
            protocol_id: WARM_AGENT_PROTOCOL_ID.to_string(),
            protocol_version: WARM_AGENT_PROTOCOL_VERSION,
            worktree_id: worktree_id.into(),
            model_artifact_sha256: model_artifact_sha256.into(),
            model_guest_path: model_guest_path.into(),
            ready_nonce: ready_nonce.into(),
            snapshot,
        }
    }

    pub fn validate_for_restore(
        &self,
        expected_model_artifact_sha256: &str,
        expected_model_guest_path: &str,
    ) -> Result<(), WarmAgentProtocolError> {
        if self.protocol_id != WARM_AGENT_PROTOCOL_ID {
            return Err(WarmAgentProtocolError::ProtocolMismatch {
                expected: WARM_AGENT_PROTOCOL_ID.to_string(),
                actual: self.protocol_id.clone(),
            });
        }
        if self.protocol_version != WARM_AGENT_PROTOCOL_VERSION {
            return Err(WarmAgentProtocolError::VersionMismatch {
                expected: WARM_AGENT_PROTOCOL_VERSION,
                actual: self.protocol_version,
            });
        }
        if self.model_artifact_sha256 != expected_model_artifact_sha256 {
            return Err(WarmAgentProtocolError::ModelHashMismatch {
                expected: expected_model_artifact_sha256.to_string(),
                actual: self.model_artifact_sha256.clone(),
            });
        }
        if self.model_guest_path != expected_model_guest_path {
            return Err(WarmAgentProtocolError::ModelGuestPathMismatch {
                expected: expected_model_guest_path.to_string(),
                actual: self.model_guest_path.clone(),
            });
        }
        Ok(())
    }
}

pub fn encode_warm_agent_frame<T: Serialize>(frame: &T) -> Result<String, WarmAgentProtocolError> {
    let mut encoded = serde_json::to_string(frame)?;
    if encoded.len() > WARM_AGENT_MAX_FRAME_BYTES {
        return Err(WarmAgentProtocolError::FrameTooLarge {
            actual: encoded.len(),
            max: WARM_AGENT_MAX_FRAME_BYTES,
        });
    }
    encoded.push('\n');
    Ok(encoded)
}

pub fn decode_warm_agent_frame<T: DeserializeOwned>(
    line: &str,
) -> Result<T, WarmAgentProtocolError> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if trimmed.len() > WARM_AGENT_MAX_FRAME_BYTES {
        return Err(WarmAgentProtocolError::FrameTooLarge {
            actual: trimmed.len(),
            max: WARM_AGENT_MAX_FRAME_BYTES,
        });
    }
    Ok(serde_json::from_str(trimmed)?)
}

pub fn validate_ready_frame(frame: &WarmAgentGuestFrame) -> Result<(), WarmAgentProtocolError> {
    match frame {
        WarmAgentGuestFrame::Ready {
            protocol_id,
            protocol_version,
            ..
        } => {
            if protocol_id != WARM_AGENT_PROTOCOL_ID {
                return Err(WarmAgentProtocolError::ProtocolMismatch {
                    expected: WARM_AGENT_PROTOCOL_ID.to_string(),
                    actual: protocol_id.clone(),
                });
            }
            if *protocol_version != WARM_AGENT_PROTOCOL_VERSION {
                return Err(WarmAgentProtocolError::VersionMismatch {
                    expected: WARM_AGENT_PROTOCOL_VERSION,
                    actual: *protocol_version,
                });
            }
            Ok(())
        }
        other => Err(WarmAgentProtocolError::UnexpectedFrame {
            expected: "ready",
            actual: warm_agent_guest_frame_type(other),
        }),
    }
}

pub fn warm_agent_guest_frame_type(frame: &WarmAgentGuestFrame) -> &'static str {
    match frame {
        WarmAgentGuestFrame::Ready { .. } => "ready",
        WarmAgentGuestFrame::Token { .. } => "token",
        WarmAgentGuestFrame::Complete { .. } => "complete",
        WarmAgentGuestFrame::Error { .. } => "error",
        WarmAgentGuestFrame::Heartbeat { .. } => "heartbeat",
    }
}

pub fn validate_guest_frame_request_id(
    frame: &WarmAgentGuestFrame,
    expected_request_id: &str,
) -> Result<(), WarmAgentProtocolError> {
    let actual_request_id = match frame {
        WarmAgentGuestFrame::Token { request_id, .. }
        | WarmAgentGuestFrame::Complete { request_id, .. } => Some(request_id.as_str()),
        WarmAgentGuestFrame::Error { request_id, .. }
        | WarmAgentGuestFrame::Heartbeat { request_id } => request_id.as_deref(),
        WarmAgentGuestFrame::Ready { .. } => None,
    };

    match actual_request_id {
        Some(actual) if actual != expected_request_id => {
            Err(WarmAgentProtocolError::RequestIdMismatch {
                frame_type: warm_agent_guest_frame_type(frame),
                expected: expected_request_id.to_string(),
                actual: actual.to_string(),
            })
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::AdapterId;

    #[test]
    fn host_frame_encodes_as_jsonl_and_round_trips() {
        let frame = WarmAgentHostFrame::Generate {
            request: WarmAgentGenerateRequest {
                request_id: "req-1".to_string(),
                model_id: "model-1".to_string(),
                model_guest_path: "/models/model.gguf".to_string(),
                model_artifact_sha256: "abc123".to_string(),
                prompt: "hello".to_string(),
                max_tokens: 16,
            },
        };

        let encoded = encode_warm_agent_frame(&frame).expect("encode frame");
        assert!(encoded.ends_with('\n'));
        let decoded: WarmAgentHostFrame = decode_warm_agent_frame(&encoded).expect("decode frame");
        assert_eq!(decoded, frame);
    }

    #[test]
    fn oversized_frame_fails_closed_before_decode() {
        let huge = "x".repeat(WARM_AGENT_MAX_FRAME_BYTES + 1);
        let err = decode_warm_agent_frame::<WarmAgentGuestFrame>(&huge)
            .expect_err("oversized frame must fail");
        assert!(matches!(err, WarmAgentProtocolError::FrameTooLarge { .. }));
    }

    #[test]
    fn strict_decode_rejects_unknown_host_fields() {
        let json = r#"{"type":"load","request_id":"req-1","model_guest_path":"/models/model.gguf","model_artifact_sha256":"sha","extra":"nope"}"#;
        let err = decode_warm_agent_frame::<WarmAgentHostFrame>(json)
            .expect_err("unknown fields must fail closed");

        assert!(matches!(err, WarmAgentProtocolError::Json(_)));
    }

    #[test]
    fn strict_decode_rejects_missing_required_guest_fields() {
        let json = r#"{"type":"token","token_id":1,"text":"hello"}"#;
        let err = decode_warm_agent_frame::<WarmAgentGuestFrame>(json)
            .expect_err("missing request_id must fail closed");

        assert!(matches!(err, WarmAgentProtocolError::Json(_)));
    }

    #[test]
    fn ready_frame_validates_protocol_and_version() {
        let ready = WarmAgentGuestFrame::Ready {
            protocol_id: WARM_AGENT_PROTOCOL_ID.to_string(),
            protocol_version: WARM_AGENT_PROTOCOL_VERSION,
            agent_id: "agent-1".to_string(),
            ready_nonce: "nonce-1".to_string(),
            loaded_model_sha256: Some("sha".to_string()),
            loaded_model_guest_path: Some("/models/model.gguf".to_string()),
        };
        validate_ready_frame(&ready).expect("ready frame accepted");

        let wrong = WarmAgentGuestFrame::Ready {
            protocol_id: "other".to_string(),
            protocol_version: WARM_AGENT_PROTOCOL_VERSION,
            agent_id: "agent-1".to_string(),
            ready_nonce: "nonce-1".to_string(),
            loaded_model_sha256: None,
            loaded_model_guest_path: None,
        };
        assert!(matches!(
            validate_ready_frame(&wrong),
            Err(WarmAgentProtocolError::ProtocolMismatch { .. })
        ));
    }

    #[test]
    fn ready_validation_rejects_non_ready_without_payload_leak() {
        let token = WarmAgentGuestFrame::Token {
            request_id: "req-1".to_string(),
            token_id: 1,
            token_index: None,
            text: "sensitive-token-text".to_string(),
        };

        let err = validate_ready_frame(&token).expect_err("token is not ready");
        assert!(matches!(
            err,
            WarmAgentProtocolError::UnexpectedFrame {
                expected: "ready",
                actual: "token"
            }
        ));
        assert!(!err.to_string().contains("sensitive-token-text"));
    }

    #[test]
    fn request_id_helper_detects_mismatch_without_payload_leak() {
        let token = WarmAgentGuestFrame::Token {
            request_id: "other-request".to_string(),
            token_id: 7,
            token_index: Some(1),
            text: "sensitive-token-text".to_string(),
        };

        let err = validate_guest_frame_request_id(&token, "req-1")
            .expect_err("mismatched request_id must fail");
        assert!(matches!(
            err,
            WarmAgentProtocolError::RequestIdMismatch {
                frame_type: "token",
                ..
            }
        ));
        let rendered = err.to_string();
        assert!(!rendered.contains("other-request"));
        assert!(!rendered.contains("sensitive-token-text"));

        let unscoped_error = WarmAgentGuestFrame::Error {
            request_id: None,
            code: "agent_error".to_string(),
            message: "guest-owned diagnostic".to_string(),
        };
        validate_guest_frame_request_id(&unscoped_error, "req-1")
            .expect("unscoped error frames remain accepted");
    }

    #[test]
    fn warm_snapshot_manifest_rejects_stale_model_hash() {
        let snapshot = SnapshotRef::new(AdapterId::new("cloud_hypervisor"), "/snap");
        let manifest =
            WarmVmSnapshotManifest::new("wt-a", "sha-old", "/models/model.gguf", "nonce", snapshot);
        assert!(manifest
            .validate_for_restore("sha-old", "/models/model.gguf")
            .is_ok());
        assert!(matches!(
            manifest.validate_for_restore("sha-new", "/models/model.gguf"),
            Err(WarmAgentProtocolError::ModelHashMismatch { .. })
        ));
    }

    #[test]
    fn warm_snapshot_manifest_rejects_stale_guest_path() {
        let snapshot = SnapshotRef::new(AdapterId::new("cloud_hypervisor"), "/snap");
        let manifest =
            WarmVmSnapshotManifest::new("wt-a", "sha", "/models/model.gguf", "nonce", snapshot);
        assert!(matches!(
            manifest.validate_for_restore("sha", "/models/other.gguf"),
            Err(WarmAgentProtocolError::ModelGuestPathMismatch { .. })
        ));
    }
}
