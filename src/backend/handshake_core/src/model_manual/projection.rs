use super::types::{
    CommandReference, CommandStatus, Manual, ManualFeatureGroup, ManualSafetyConstraint,
    ManualWorkflow,
};

pub const MODEL_MANUAL_PROJECTION_UPDATED_AT: &str = "2026-05-18T00:00:00Z";

pub fn render_model_manual_markdown(manual: &Manual) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("file_id: model-manual\n");
    out.push_str("file_kind: ModelManual\n");
    out.push_str(&format!(
        "updated_at: \"{}\"\n",
        yaml_escape(MODEL_MANUAL_PROJECTION_UPDATED_AT)
    ));
    out.push_str(&format!(
        "manual_version: \"{}\"\n",
        yaml_escape(manual.version)
    ));
    out.push_str("---\n\n");

    out.push_str("# ModelManual\n\n");
    out.push_str(
        "This legacy ModelManual projection is a compatibility artifact. UserManual is canonical.\n\n",
    );

    for group in manual.feature_groups {
        render_feature_group(&mut out, manual.version, group);
    }
    for command in manual.command_reference {
        render_command_reference(&mut out, manual.version, command);
    }
    for constraint in manual.safety_constraints {
        render_safety_constraint(&mut out, manual.version, constraint);
    }
    for workflow in manual.workflows {
        render_workflow(&mut out, manual.version, workflow);
    }

    out
}

fn render_feature_group(out: &mut String, version: &str, group: &ManualFeatureGroup) {
    open_topic(
        out,
        &format!("feature-{}", kebab_id(group.id)),
        "current",
        version,
        group.title,
    );
    out.push_str(&format!("## Feature Group: {}\n\n", group.title));
    out.push_str(&format!("ID: `{}`\n\n", group.id));
    out.push_str(group.description);
    out.push_str("\n\nCommands:\n");
    for command in group.commands {
        out.push_str(&format!("- `{command}`\n"));
    }
    close_topic(out);
}

fn render_command_reference(out: &mut String, version: &str, command: &CommandReference) {
    open_topic(
        out,
        &format!("command-{}", kebab_id(command.id)),
        status_label(command.status),
        version,
        command.name,
    );
    out.push_str(&format!("## Command: {}\n\n", command.name));
    out.push_str(&format!("ID: `{}`\n\n", command.id));
    out.push_str(&format!("Status: `{}`\n\n", status_label(command.status)));
    if let Some(channel) = command.ipc_channel {
        out.push_str(&format!("IPC channel: `{channel}`\n\n"));
    }
    if let Some(tauri_command) = command.tauri_command {
        out.push_str(&format!("Tauri command: `{tauri_command}`\n\n"));
    }
    if let Some(cli_flag) = command.cli_flag {
        out.push_str(&format!("CLI flag: `{cli_flag}`\n\n"));
    }
    out.push_str(&format!("Description: {}\n\n", command.description));
    out.push_str(&format!("Expected input: {}\n\n", command.expected_input));
    out.push_str(&format!("Expected output: {}\n\n", command.expected_output));
    render_string_list(out, "Schema fields", command.schema_fields);
    render_string_list(out, "Common errors", command.common_errors);
    render_string_list(out, "Recovery steps", command.recovery_steps);
    close_topic(out);
}

fn render_safety_constraint(out: &mut String, version: &str, constraint: &ManualSafetyConstraint) {
    open_topic(
        out,
        &format!("safety-{}", kebab_id(constraint.id)),
        "current",
        version,
        constraint.id,
    );
    out.push_str(&format!("## Safety Constraint: {}\n\n", constraint.id));
    out.push_str(&format!("Constraint: {}\n\n", constraint.constraint_text));
    out.push_str(&format!(
        "Enforcement point: {}\n",
        constraint.enforcement_point
    ));
    close_topic(out);
}

fn render_workflow(out: &mut String, version: &str, workflow: &ManualWorkflow) {
    open_topic(
        out,
        &format!("workflow-{}", kebab_id(workflow.id)),
        "current",
        version,
        workflow.title,
    );
    out.push_str(&format!("## Workflow: {}\n\n", workflow.title));
    out.push_str(&format!("ID: `{}`\n\n", workflow.id));
    render_string_list(out, "Prerequisites", workflow.prerequisites);
    render_string_list(out, "Steps", workflow.steps);
    out.push_str(&format!(
        "Expected outcome: {}\n\n",
        workflow.expected_outcome
    ));
    render_string_list(out, "Failure modes", workflow.failure_modes);
    close_topic(out);
}

fn render_string_list(out: &mut String, label: &str, values: &[&str]) {
    out.push_str(&format!("{label}:\n"));
    if values.is_empty() {
        out.push_str("- None\n\n");
        return;
    }
    for value in values {
        out.push_str(&format!("- {value}\n"));
    }
    out.push('\n');
}

fn open_topic(out: &mut String, id: &str, status: &str, version: &str, summary: &str) {
    out.push_str(&format!(
        "<topic id=\"{}\" status=\"{}\" version=\"{}\" summary=\"{}\">\n\n",
        attr_escape(id),
        attr_escape(status),
        attr_escape(version),
        attr_escape(summary)
    ));
}

fn close_topic(out: &mut String) {
    out.push_str("\n</topic>\n\n");
}

fn status_label(status: CommandStatus) -> &'static str {
    match status {
        CommandStatus::Wired => "wired",
        CommandStatus::Planned => "planned",
    }
}

fn kebab_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'A'..='Z' => ch.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' => ch,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn yaml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn attr_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
