use serde::Serialize;

#[allow(dead_code, unused_imports)]
#[path = "../../../src/backend/handshake_core/src/model_manual/mod.rs"]
mod core_model_manual;

use core_model_manual::{
    model_manual, CommandReference, CommandStatus, Manual, ManualFeatureGroup,
    ManualSafetyConstraint, ManualWorkflow,
};

pub const MODEL_MANUAL_GET_IPC_CHANNEL: &str = "kernel.model_manual.get";
pub const MODEL_MANUAL_LIST_COMMANDS_IPC_CHANNEL: &str = "kernel.model_manual.list_commands";
pub const MODEL_MANUAL_SEARCH_IPC_CHANNEL: &str = "kernel.model_manual.search";

#[derive(Debug, Clone, Serialize)]
pub struct ManualJson {
    pub version: String,
    pub feature_groups: Vec<ManualFeatureGroupJson>,
    pub command_reference: Vec<CommandReferenceJson>,
    pub safety_constraints: Vec<ManualSafetyConstraintJson>,
    pub workflows: Vec<ManualWorkflowJson>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManualFeatureGroupJson {
    pub id: String,
    pub title: String,
    pub description: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandReferenceJson {
    pub id: String,
    pub name: String,
    pub status: String,
    pub ipc_channel: Option<String>,
    pub tauri_command: Option<String>,
    pub schema_fields: Vec<String>,
    pub cli_flag: Option<String>,
    pub description: String,
    pub expected_input: String,
    pub expected_output: String,
    pub common_errors: Vec<String>,
    pub recovery_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandReferenceSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub ipc_channel: Option<String>,
    pub tauri_command: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManualSafetyConstraintJson {
    pub id: String,
    pub constraint_text: String,
    pub enforcement_point: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManualWorkflowJson {
    pub id: String,
    pub title: String,
    pub prerequisites: Vec<String>,
    pub steps: Vec<String>,
    pub expected_outcome: String,
    pub failure_modes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManualSearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub result_kind: String,
    pub command_id: Option<String>,
    pub workflow_id: Option<String>,
    pub safety_constraint_id: Option<String>,
    pub feature_group_id: Option<String>,
    pub title: String,
    pub excerpt: String,
}

#[tauri::command]
pub async fn model_manual_get() -> ManualJson {
    let _ = MODEL_MANUAL_GET_IPC_CHANNEL;
    manual_to_json(model_manual())
}

#[tauri::command]
pub async fn model_manual_list_commands() -> Vec<CommandReferenceSummary> {
    let _ = MODEL_MANUAL_LIST_COMMANDS_IPC_CHANNEL;
    model_manual()
        .command_reference
        .iter()
        .map(command_summary)
        .collect()
}

#[tauri::command]
pub async fn model_manual_search(query: String) -> ManualSearchResponse {
    let _ = MODEL_MANUAL_SEARCH_IPC_CHANNEL;
    let trimmed = query.trim().to_string();
    let needle = trimmed.to_ascii_lowercase();
    if needle.is_empty() {
        return ManualSearchResponse {
            query: trimmed,
            results: Vec::new(),
        };
    }

    let manual = model_manual();
    let mut results = Vec::new();
    for command in manual.command_reference {
        if command_matches(command, &needle) {
            results.push(SearchResult {
                result_kind: "command".to_string(),
                command_id: Some(command.id.to_string()),
                workflow_id: None,
                safety_constraint_id: None,
                feature_group_id: None,
                title: command.name.to_string(),
                excerpt: command.description.to_string(),
            });
        }
    }
    for workflow in manual.workflows {
        if workflow_matches(workflow, &needle) {
            results.push(SearchResult {
                result_kind: "workflow".to_string(),
                command_id: None,
                workflow_id: Some(workflow.id.to_string()),
                safety_constraint_id: None,
                feature_group_id: None,
                title: workflow.title.to_string(),
                excerpt: workflow.expected_outcome.to_string(),
            });
        }
    }
    for constraint in manual.safety_constraints {
        if safety_constraint_matches(constraint, &needle) {
            results.push(SearchResult {
                result_kind: "safety_constraint".to_string(),
                command_id: None,
                workflow_id: None,
                safety_constraint_id: Some(constraint.id.to_string()),
                feature_group_id: None,
                title: constraint.id.to_string(),
                excerpt: constraint.constraint_text.to_string(),
            });
        }
    }
    for group in manual.feature_groups {
        if feature_group_matches(group, &needle) {
            results.push(SearchResult {
                result_kind: "feature_group".to_string(),
                command_id: None,
                workflow_id: None,
                safety_constraint_id: None,
                feature_group_id: Some(group.id.to_string()),
                title: group.title.to_string(),
                excerpt: group.description.to_string(),
            });
        }
    }

    ManualSearchResponse {
        query: trimmed,
        results,
    }
}

fn manual_to_json(manual: &Manual) -> ManualJson {
    ManualJson {
        version: manual.version.to_string(),
        feature_groups: manual
            .feature_groups
            .iter()
            .map(feature_group_to_json)
            .collect(),
        command_reference: manual
            .command_reference
            .iter()
            .map(command_to_json)
            .collect(),
        safety_constraints: manual
            .safety_constraints
            .iter()
            .map(safety_constraint_to_json)
            .collect(),
        workflows: manual.workflows.iter().map(workflow_to_json).collect(),
    }
}

fn feature_group_to_json(group: &ManualFeatureGroup) -> ManualFeatureGroupJson {
    ManualFeatureGroupJson {
        id: group.id.to_string(),
        title: group.title.to_string(),
        description: group.description.to_string(),
        commands: group
            .commands
            .iter()
            .map(|command| command.to_string())
            .collect(),
    }
}

fn command_to_json(command: &CommandReference) -> CommandReferenceJson {
    CommandReferenceJson {
        id: command.id.to_string(),
        name: command.name.to_string(),
        status: status_label(command.status).to_string(),
        ipc_channel: command.ipc_channel.map(str::to_string),
        tauri_command: command.tauri_command.map(str::to_string),
        schema_fields: command
            .schema_fields
            .iter()
            .map(|field| field.to_string())
            .collect(),
        cli_flag: command.cli_flag.map(str::to_string),
        description: command.description.to_string(),
        expected_input: command.expected_input.to_string(),
        expected_output: command.expected_output.to_string(),
        common_errors: command
            .common_errors
            .iter()
            .map(|error| error.to_string())
            .collect(),
        recovery_steps: command
            .recovery_steps
            .iter()
            .map(|step| step.to_string())
            .collect(),
    }
}

fn command_summary(command: &CommandReference) -> CommandReferenceSummary {
    CommandReferenceSummary {
        id: command.id.to_string(),
        name: command.name.to_string(),
        status: status_label(command.status).to_string(),
        ipc_channel: command.ipc_channel.map(str::to_string),
        tauri_command: command.tauri_command.map(str::to_string),
    }
}

fn safety_constraint_to_json(constraint: &ManualSafetyConstraint) -> ManualSafetyConstraintJson {
    ManualSafetyConstraintJson {
        id: constraint.id.to_string(),
        constraint_text: constraint.constraint_text.to_string(),
        enforcement_point: constraint.enforcement_point.to_string(),
    }
}

fn workflow_to_json(workflow: &ManualWorkflow) -> ManualWorkflowJson {
    ManualWorkflowJson {
        id: workflow.id.to_string(),
        title: workflow.title.to_string(),
        prerequisites: workflow
            .prerequisites
            .iter()
            .map(|entry| entry.to_string())
            .collect(),
        steps: workflow
            .steps
            .iter()
            .map(|entry| entry.to_string())
            .collect(),
        expected_outcome: workflow.expected_outcome.to_string(),
        failure_modes: workflow
            .failure_modes
            .iter()
            .map(|entry| entry.to_string())
            .collect(),
    }
}

fn status_label(status: CommandStatus) -> &'static str {
    match status {
        CommandStatus::Wired => "wired",
        CommandStatus::Planned => "planned",
    }
}

fn command_matches(command: &CommandReference, needle: &str) -> bool {
    text_matches(needle, command.id)
        || text_matches(needle, command.name)
        || command
            .ipc_channel
            .is_some_and(|value| text_matches(needle, value))
        || command
            .tauri_command
            .is_some_and(|value| text_matches(needle, value))
        || command
            .cli_flag
            .is_some_and(|value| text_matches(needle, value))
        || text_matches(needle, command.description)
        || text_matches(needle, command.expected_input)
        || text_matches(needle, command.expected_output)
        || command
            .schema_fields
            .iter()
            .any(|value| text_matches(needle, value))
        || command
            .common_errors
            .iter()
            .any(|value| text_matches(needle, value))
        || command
            .recovery_steps
            .iter()
            .any(|value| text_matches(needle, value))
}

fn workflow_matches(workflow: &ManualWorkflow, needle: &str) -> bool {
    text_matches(needle, workflow.id)
        || text_matches(needle, workflow.title)
        || text_matches(needle, workflow.expected_outcome)
        || workflow
            .prerequisites
            .iter()
            .any(|value| text_matches(needle, value))
        || workflow
            .steps
            .iter()
            .any(|value| text_matches(needle, value))
        || workflow
            .failure_modes
            .iter()
            .any(|value| text_matches(needle, value))
}

fn safety_constraint_matches(constraint: &ManualSafetyConstraint, needle: &str) -> bool {
    text_matches(needle, constraint.id)
        || text_matches(needle, constraint.constraint_text)
        || text_matches(needle, constraint.enforcement_point)
}

fn feature_group_matches(group: &ManualFeatureGroup, needle: &str) -> bool {
    text_matches(needle, group.id)
        || text_matches(needle, group.title)
        || text_matches(needle, group.description)
        || group
            .commands
            .iter()
            .any(|value| text_matches(needle, value))
}

fn text_matches(needle: &str, haystack: &str) -> bool {
    haystack.to_ascii_lowercase().contains(needle)
}
