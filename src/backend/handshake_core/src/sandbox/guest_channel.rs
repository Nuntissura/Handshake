use bytes::Bytes;

use super::SandboxAdapterError;

pub const GUEST_CHANNEL_PROTOCOL_ID: &str = "hsk.guest_channel";
pub const GUEST_CHANNEL_PROTOCOL_VERSION: u16 = 1;
pub const GUEST_CHANNEL_MAX_FRAME_BYTES: usize = 1024 * 1024;

pub const GUEST_CHANNEL_READY_PREFIX: &str = "HSK-AGENT-READY";
pub const GUEST_CHANNEL_EXEC_PREFIX: &str = "HSK-EXEC";
pub const GUEST_CHANNEL_EXEC_DONE_PREFIX: &str = "HSK-EXEC-DONE";
pub const GUEST_CHANNEL_AGENT_ERROR_PREFIX: &str = "HSK-AGENT-ERROR";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestChannelExecResult {
    pub request_id: String,
    pub exit_code: i32,
    pub stdout: Bytes,
    pub stderr: Bytes,
}

pub fn encode_guest_channel_blob(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    if bytes.is_empty() {
        "-".to_string()
    } else {
        BASE64.encode(bytes)
    }
}

pub fn decode_guest_channel_blob(value: &str) -> Result<Bytes, SandboxAdapterError> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    if value.is_empty() {
        return Err(guest_channel_error("guest channel blob field is missing"));
    }
    if value == "-" {
        return Ok(Bytes::new());
    }
    BASE64
        .decode(value.as_bytes())
        .map(Bytes::from)
        .map_err(|error| {
            guest_channel_error(format!("guest channel base64 decode failed: {error}"))
        })
}

pub fn encode_guest_channel_exec_request(
    request_id: &str,
    command_line: &str,
    stdin: Option<&Bytes>,
) -> Result<String, SandboxAdapterError> {
    let cmd_b64 = encode_guest_channel_blob(command_line.as_bytes());
    let stdin_b64 = stdin
        .map(|bytes| encode_guest_channel_blob(bytes.as_ref()))
        .unwrap_or_else(|| "-".to_string());
    let frame = format!("{GUEST_CHANNEL_EXEC_PREFIX} {request_id} {cmd_b64} {stdin_b64}\n");
    if frame.len() > GUEST_CHANNEL_MAX_FRAME_BYTES {
        return Err(guest_channel_error(format!(
            "guest channel frame exceeds max size: {} > {}",
            frame.len(),
            GUEST_CHANNEL_MAX_FRAME_BYTES
        )));
    }
    Ok(frame)
}

pub fn parse_guest_channel_exec_result(
    line: &str,
    expected_request_id: &str,
) -> Result<GuestChannelExecResult, SandboxAdapterError> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if trimmed.len() > GUEST_CHANNEL_MAX_FRAME_BYTES {
        return Err(guest_channel_error(format!(
            "guest channel response exceeds max size: {} > {}",
            trimmed.len(),
            GUEST_CHANNEL_MAX_FRAME_BYTES
        )));
    }
    let mut parts = trimmed.splitn(5, ' ');
    let prefix = parts.next().unwrap_or_default();
    match prefix {
        GUEST_CHANNEL_EXEC_DONE_PREFIX => {
            let request_id = parts.next().ok_or_else(|| {
                guest_channel_error("guest channel exec response missing request id")
            })?;
            if request_id != expected_request_id {
                return Err(guest_channel_error(format!(
                        "guest channel response request id mismatch: expected {expected_request_id}, got {request_id}"
                )));
            }
            let exit_code = parts
                .next()
                .ok_or_else(|| {
                    guest_channel_error("guest channel exec response missing exit code")
                })?
                .parse::<i32>()
                .map_err(|error| {
                    guest_channel_error(format!("guest channel exit code parse failed: {error}"))
                })?;
            let stdout_b64 = parts
                .next()
                .ok_or_else(|| guest_channel_error("guest channel exec response missing stdout"))?;
            let stderr_b64 = parts
                .next()
                .ok_or_else(|| guest_channel_error("guest channel exec response missing stderr"))?;
            Ok(GuestChannelExecResult {
                request_id: request_id.to_string(),
                exit_code,
                stdout: decode_guest_channel_blob(stdout_b64)?,
                stderr: decode_guest_channel_blob(stderr_b64)?,
            })
        }
        GUEST_CHANNEL_AGENT_ERROR_PREFIX => {
            let request_id = parts.next().ok_or_else(|| {
                guest_channel_error("guest channel agent error missing request id")
            })?;
            if request_id != expected_request_id {
                return Err(guest_channel_error(format!(
                        "guest channel error request id mismatch: expected {expected_request_id}, got {request_id}"
                )));
            }
            let code = parts
                .next()
                .ok_or_else(|| guest_channel_error("guest channel agent error missing code"))?;
            let message_b64 = parts
                .next()
                .ok_or_else(|| guest_channel_error("guest channel agent error missing message"))?;
            let message_bytes = decode_guest_channel_blob(message_b64)?;
            let message = String::from_utf8_lossy(&message_bytes).into_owned();
            Err(guest_channel_error(format!(
                "guest channel agent error {code}: {message}"
            )))
        }
        other => Err(guest_channel_error(format!(
            "unexpected guest channel response prefix `{other}`"
        ))),
    }
}

fn guest_channel_error(reason: impl Into<String>) -> SandboxAdapterError {
    SandboxAdapterError::SpawnFailed {
        adapter_id: super::AdapterId::new("guest_channel"),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_channel_exec_request_uses_bounded_line_protocol() {
        let frame =
            encode_guest_channel_exec_request("req-1", "echo hello", None).expect("encode request");
        assert!(frame.starts_with("HSK-EXEC req-1 "));
        assert!(frame.ends_with('\n'));
        assert!(frame.contains("ZWNobyBoZWxsbw=="));
        assert!(frame.ends_with(" -\n"));
    }

    #[test]
    fn guest_channel_exec_done_decodes_stdout_and_stderr() {
        let line = format!(
            "HSK-EXEC-DONE req-1 7 {} {}",
            encode_guest_channel_blob(b"out"),
            encode_guest_channel_blob(b"err")
        );
        let parsed = parse_guest_channel_exec_result(&line, "req-1").expect("parse done");
        assert_eq!(parsed.exit_code, 7);
        assert_eq!(parsed.stdout, Bytes::from_static(b"out"));
        assert_eq!(parsed.stderr, Bytes::from_static(b"err"));
    }

    #[test]
    fn guest_channel_rejects_wrong_request_id() {
        let line = format!(
            "HSK-EXEC-DONE other 0 {} -",
            encode_guest_channel_blob(b"out")
        );
        let err = parse_guest_channel_exec_result(&line, "req-1")
            .expect_err("wrong request id must fail");
        assert!(format!("{err}").contains("request id mismatch"));
    }

    #[test]
    fn guest_channel_rejects_truncated_exec_done() {
        let line = format!(
            "HSK-EXEC-DONE req-1 0 {}",
            encode_guest_channel_blob(b"out")
        );
        let err = parse_guest_channel_exec_result(&line, "req-1")
            .expect_err("truncated response must fail");
        assert!(format!("{err}").contains("missing stderr"));
    }

    #[test]
    fn guest_channel_rejects_empty_blob_field() {
        let line = "HSK-EXEC-DONE req-1 0  -";
        let err =
            parse_guest_channel_exec_result(line, "req-1").expect_err("empty blob field must fail");
        assert!(format!("{err}").contains("blob field is missing"));
    }
}
