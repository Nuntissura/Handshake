use std::path::PathBuf;

use crate::capabilities::CapabilityRegistry;
use regex::Regex;

use super::{config::TerminalConfig, TerminalError, TerminalRequest};

/// Guard responsible for capability and workspace scoping checks.
pub trait TerminalGuard: Send + Sync {
    fn check_capability(
        &self,
        req: &TerminalRequest,
        registry: &CapabilityRegistry,
    ) -> Result<(), TerminalError>;

    fn check_session_isolation(
        &self,
        req: &TerminalRequest,
        session: &super::session::TerminalSession,
        registry: &CapabilityRegistry,
    ) -> Result<(), TerminalError>;

    fn validate_cwd(
        &self,
        req: &TerminalRequest,
        cfg: &TerminalConfig,
    ) -> Result<PathBuf, TerminalError>;

    fn pre_exec(
        &self,
        _req: &mut TerminalRequest,
        _cfg: &TerminalConfig,
    ) -> Result<(), TerminalError> {
        Ok(())
    }
}

pub struct DefaultTerminalGuard;

impl TerminalGuard for DefaultTerminalGuard {
    fn check_capability(
        &self,
        req: &TerminalRequest,
        registry: &CapabilityRegistry,
    ) -> Result<(), TerminalError> {
        #[allow(clippy::manual_unwrap_or)]
        let capability = match req.requested_capability.as_deref() {
            Some(value) => value,
            None => "terminal.exec",
        };
        if capability.trim().is_empty() {
            return Err(TerminalError::CapabilityDenied(
                "HSK-TERM-002: requested capability missing".to_string(),
            ));
        }

        if self.capability_allowed(capability, req, registry)? {
            Ok(())
        } else {
            Err(TerminalError::CapabilityDenied(
                "HSK-TERM-002: capability denied".to_string(),
            ))
        }
    }

    fn check_session_isolation(
        &self,
        req: &TerminalRequest,
        session: &super::session::TerminalSession,
        registry: &CapabilityRegistry,
    ) -> Result<(), TerminalError> {
        let is_ai_context = req.job_context.job_id.is_some() || req.job_context.model_id.is_some();
        if !is_ai_context {
            return Ok(());
        }

        if matches!(
            session.session_type,
            super::session::TerminalSessionType::HumanDev
        ) {
            let allowed = self.capability_allowed("terminal.attach_human", req, registry)?;
            if !allowed {
                return Err(TerminalError::IsolationViolation(
                    "HSK-TERM-009: AI cannot attach to human terminal without terminal.attach_human"
                        .to_string(),
                ));
            }

            if !session.human_consent_obtained {
                return Err(TerminalError::IsolationViolation(
                    "HSK-TERM-009: AI cannot attach to human terminal without logged consent"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }

    fn validate_cwd(
        &self,
        req: &TerminalRequest,
        cfg: &TerminalConfig,
    ) -> Result<PathBuf, TerminalError> {
        let root = cfg
            .workspace_root
            .canonicalize()
            .map_err(|e| TerminalError::CwdViolation(format!("HSK-TERM-003: {}", e)))?;

        if let Some(cwd) = &req.cwd {
            if cwd.is_absolute() {
                return Err(TerminalError::CwdViolation(
                    "HSK-TERM-003: cwd must be workspace-relative".to_string(),
                ));
            }
        }

        let target = if let Some(cwd) = &req.cwd {
            root.join(cwd)
        } else {
            root.clone()
        };

        let resolved = target
            .canonicalize()
            .map_err(|e| TerminalError::CwdViolation(format!("HSK-TERM-003: {}", e)))?;

        if !resolved.starts_with(&root) {
            return Err(TerminalError::CwdViolation(
                "HSK-TERM-003: cwd escapes workspace root".to_string(),
            ));
        }

        if !cfg.allowed_cwd_roots.is_empty() {
            let mut allowed = false;
            for allowed_root in cfg.allowed_cwd_roots.iter() {
                if allowed_root.is_absolute() {
                    return Err(TerminalError::CwdViolation(
                        "HSK-TERM-003: allowed cwd roots must be workspace-relative".to_string(),
                    ));
                }
                let allowed_path = root
                    .join(allowed_root)
                    .canonicalize()
                    .map_err(|e| TerminalError::CwdViolation(format!("HSK-TERM-003: {}", e)))?;
                if resolved.starts_with(&allowed_path) {
                    allowed = true;
                    break;
                }
            }

            if !allowed {
                return Err(TerminalError::CwdViolation(
                    "HSK-TERM-003: cwd not in allowed roots".to_string(),
                ));
            }
        }

        Ok(resolved)
    }

    fn pre_exec(
        &self,
        req: &mut TerminalRequest,
        cfg: &TerminalConfig,
    ) -> Result<(), TerminalError> {
        let command_line = if req.args.is_empty() {
            req.command.clone()
        } else {
            format!("{} {}", req.command, req.args.join(" "))
        };

        if matches_any(&cfg.denied_command_patterns, &command_line)? {
            return Err(TerminalError::CapabilityDenied(
                "HSK-TERM-002: command denied by policy".to_string(),
            ));
        }

        if !cfg.allowed_command_patterns.is_empty()
            && !matches_any(&cfg.allowed_command_patterns, &command_line)?
        {
            return Err(TerminalError::CapabilityDenied(
                "HSK-TERM-002: command not allowed by policy".to_string(),
            ));
        }

        Ok(())
    }
}

impl DefaultTerminalGuard {
    fn capability_allowed(
        &self,
        capability: &str,
        req: &TerminalRequest,
        registry: &CapabilityRegistry,
    ) -> Result<bool, TerminalError> {
        if let Some(profile_id) = &req.job_context.capability_profile_id {
            let allowed = registry
                .profile_can(profile_id, capability)
                .map_err(|e| TerminalError::CapabilityDenied(format!("HSK-TERM-002: {}", e)))?;
            return Ok(allowed);
        }

        if req.granted_capabilities.is_empty() {
            return Err(TerminalError::CapabilityDenied(
                "HSK-TERM-002: no capabilities granted".to_string(),
            ));
        }

        registry
            .enforce_can_perform(capability, &req.granted_capabilities)
            .map_err(|e| TerminalError::CapabilityDenied(format!("HSK-TERM-002: {}", e)))
    }
}

fn matches_any(patterns: &[String], command_line: &str) -> Result<bool, TerminalError> {
    for pattern in patterns {
        let regex = Regex::new(pattern).map_err(|e| {
            TerminalError::CapabilityDenied(format!("HSK-TERM-002: invalid command pattern: {}", e))
        })?;
        if regex.is_match(command_line) {
            return Ok(true);
        }
    }
    Ok(false)
}
