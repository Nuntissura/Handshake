use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use super::TerminalRequest;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TerminalSessionType {
    HumanDev,
    AiJob,
    PluginTool,
}

impl TerminalSessionType {
    pub fn derive(
        explicit: Option<Self>,
        job_id: Option<&String>,
        model_id: Option<&String>,
    ) -> Self {
        if let Some(session_type) = explicit {
            return session_type;
        }

        if job_id.is_some() || model_id.is_some() {
            TerminalSessionType::AiJob
        } else {
            TerminalSessionType::HumanDev
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TerminalSessionType::HumanDev => "HUMAN_DEV",
            TerminalSessionType::AiJob => "AI_JOB",
            TerminalSessionType::PluginTool => "PLUGIN_TOOL",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalSession {
    pub session_type: TerminalSessionType,
    pub session_id: Option<String>,
    pub job_id: Option<String>,
    pub wsids: Vec<String>,
    pub capability_set: Vec<String>,
    pub human_consent_obtained: bool,
}

impl TerminalSession {
    pub fn from_request(req: &TerminalRequest) -> Self {
        let mut job_context = req.job_context.clone();
        job_context.normalize();

        let capability_set = req
            .granted_capabilities
            .iter()
            .map(|c| c.nfc().collect::<String>())
            .collect();

        let session_id = job_context
            .session_id
            .or_else(|| Some(Uuid::new_v4().to_string()));

        TerminalSession {
            session_type: req.session_type,
            session_id,
            job_id: job_context.job_id,
            wsids: job_context.wsids,
            capability_set,
            human_consent_obtained: req.human_consent_obtained,
        }
    }
}
