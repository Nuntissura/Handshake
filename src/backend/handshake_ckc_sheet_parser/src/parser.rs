use sha2::{Digest, Sha256};

use crate::types::{
    parse_field_type, ParsedSheetFieldType, SheetBlockInstance, SheetBlockInstanceField,
    SheetBlockSchema, SheetTemplateAst, SheetTemplateField, SheetTemplateSection, SheetUnmappedLine,
};

pub fn sha256_hex(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn first_template_version(text: &str) -> Option<String> {
    let Some(line) = text.lines().map(str::trim).find(|line| !line.is_empty()) else {
        return Some("unknown".to_string());
    };
    let lower = line.to_ascii_lowercase();
    let Some(start) = lower.find("(v") else {
        return Some("unknown".to_string());
    };
    let version_start = start + 2;
    let Some(end) = line[version_start..].find(')') else {
        return Some("unknown".to_string());
    };
    let version = &line[version_start..version_start + end];
    if version.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
        && version.chars().any(|ch| ch.is_ascii_digit())
    {
        Some(version.to_string())
    } else {
        Some("unknown".to_string())
    }
}

fn source_lines(text: &str) -> Vec<(usize, usize, usize, String)> {
    let mut lines = Vec::new();
    let mut byte_start = 0usize;
    for (idx, raw_line) in text.split_inclusive('\n').enumerate() {
        let byte_end = byte_start + raw_line.len();
        let content = raw_line.trim_end_matches(&['\r', '\n'][..]).to_string();
        lines.push((idx + 1, byte_start, byte_start + content.len(), content));
        byte_start = byte_end;
    }
    if !text.is_empty() && !text.ends_with('\n') && lines.is_empty() {
        lines.push((1, 0, text.len(), text.to_string()));
    }
    lines
}

fn trimmed_span(byte_start: usize, byte_end: usize, line: &str) -> (usize, usize, String) {
    let trimmed = line.trim();
    let leading = line.find(trimmed).unwrap_or(0);
    let trailing = line.len().saturating_sub(leading + trimmed.len());
    (
        byte_start + leading,
        byte_end.saturating_sub(trailing),
        trimmed.to_string(),
    )
}

fn is_block_schema_header(line: &str) -> bool {
    line.ends_with("_Block")
        && line
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_section_header(line: &str) -> bool {
    let mut chars = line.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_uppercase() || first.is_ascii_digit())
        && !line.contains(':')
        && !is_block_schema_header(line)
        && line.chars().all(|ch| {
            ch.is_ascii_uppercase()
                || ch.is_ascii_digit()
                || ch.is_ascii_whitespace()
                || ch == '_'
                || ch == '-'
                || ch == '&'
                || ch == '/'
                || ch == '('
                || ch == ')'
                || ch == ','
                || ch == '.'
                || ch == ';'
                || ch == '\u{2014}'
                || ch == '\u{2013}'
        })
}

fn field_id_end(before_colon: &str) -> Option<usize> {
    let mut idx = 0usize;
    for segment_idx in 0..3 {
        let segment_start = idx;
        while let Some(ch) = before_colon[idx..].chars().next() {
            if ch.is_ascii_uppercase() || ch.is_ascii_digit() {
                idx += ch.len_utf8();
            } else {
                break;
            }
        }
        if idx == segment_start {
            return None;
        }
        if segment_idx < 2 {
            if before_colon[idx..].starts_with('-') {
                idx += 1;
            } else {
                return None;
            }
        }
    }
    Some(idx)
}

pub fn split_field_line(line: &str) -> Option<(String, String, String)> {
    let colon = line.find(':')?;
    let before_colon = line[..colon].trim();
    let descriptor = line[colon + 1..].trim().to_string();
    let id_end = field_id_end(before_colon)?;
    let id = before_colon[..id_end].trim();
    let after_id = before_colon[id_end..].trim_start();
    let separator = after_id.chars().next()?;
    if !matches!(separator, '\u{2014}' | '\u{2013}' | '-') {
        return None;
    }
    let label = after_id[separator.len_utf8()..].trim();
    if !label.is_empty() {
        Some((id.to_string(), label.to_string(), descriptor))
    } else {
        None
    }
}

/// The upper-cased Field ID of one CKC sheet line, if the line is a field line.
pub fn sheet_field_id_from_line(line: &str) -> Option<String> {
    split_field_line(line.trim()).map(|(field_id, _, _)| field_id.to_ascii_uppercase())
}

/// True when a line has the `ID — Label:` shape even if the label is empty, so
/// a safe-subset export can drop a malformed field line instead of leaking it.
pub fn sheet_line_looks_like_field(line: &str) -> bool {
    let Some(colon) = line.find(':') else {
        return false;
    };
    let before_colon = line[..colon].trim();
    let Some(id_end) = field_id_end(before_colon) else {
        return false;
    };
    before_colon[id_end..]
        .chars()
        .any(|ch| matches!(ch, '\u{2014}' | '\u{2013}' | '-'))
}

fn validate_template_descriptor(
    field_id: &str,
    line_number: usize,
    descriptor: &str,
) -> Result<(), String> {
    let trimmed = descriptor.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "structured_parse_error: line_number={line_number} field_id={field_id} reason=empty_descriptor"
        ));
    }
    if !trimmed.starts_with('<') {
        return Err(format!(
            "structured_parse_error: line_number={line_number} field_id={field_id} reason=descriptor_must_start_with_angle"
        ));
    }
    if !trimmed.contains('>') {
        return Err(format!(
            "structured_parse_error: line_number={line_number} field_id={field_id} reason=unclosed_descriptor"
        ));
    }
    if trimmed.matches('<').count() != trimmed.matches('>').count() {
        return Err(format!(
            "structured_parse_error: line_number={line_number} field_id={field_id} reason=unbalanced_descriptor_angles"
        ));
    }
    Ok(())
}

pub struct ParsedBlockInstanceFieldLine {
    pub root_field_id: String,
    pub instance_id: String,
    pub ordinal: usize,
    pub field_id: String,
    label: String,
    descriptor: String,
}

fn block_list_field_id_from_root_path(root_path: &str) -> Option<&str> {
    let terminal = root_path.rsplit('.').next()?.trim();
    let id_end = field_id_end(terminal)?;
    if id_end == terminal.len() {
        Some(terminal)
    } else {
        None
    }
}

fn parent_instance_id_from_root_path(root_path: &str) -> Option<&str> {
    let parent_end = root_path.rfind('.')?;
    Some(root_path[..parent_end].trim())
}

pub fn split_block_instance_field_line(line: &str) -> Option<ParsedBlockInstanceFieldLine> {
    let colon = line.find(':')?;
    let before_colon = line[..colon].trim();
    let descriptor = line[colon + 1..].trim().to_string();
    let close = before_colon.rfind(']')?;
    let open = before_colon[..close].rfind('[')?;
    let root_field_id = before_colon[..open].trim();
    block_list_field_id_from_root_path(root_field_id)?;
    let ordinal = before_colon[open + 1..close].parse::<usize>().ok()?;
    let member = before_colon[close + 1..]
        .trim_start()
        .strip_prefix('.')?
        .trim_start();
    let id_end = field_id_end(member)?;
    let field_id = member[..id_end].trim();
    let after_id = member[id_end..].trim_start();
    let separator = after_id.chars().next()?;
    if !matches!(separator, '\u{2014}' | '\u{2013}' | '-') {
        return None;
    }
    let label = after_id[separator.len_utf8()..].trim();
    if label.is_empty() {
        return None;
    }
    let instance_id = format!("{root_field_id}[{ordinal}]");
    Some(ParsedBlockInstanceFieldLine {
        root_field_id: root_field_id.to_string(),
        instance_id,
        ordinal,
        field_id: field_id.to_string(),
        label: label.to_string(),
        descriptor,
    })
}

fn resolve_block_list_schema_for_instance_root(
    fields: &[SheetTemplateField],
    block_instances: &[SheetBlockInstance],
    root_field_id: &str,
) -> Option<String> {
    let list_field_id = block_list_field_id_from_root_path(root_field_id)?;
    let parent_schema = parent_instance_id_from_root_path(root_field_id).and_then(|parent_id| {
        block_instances
            .iter()
            .find(|instance| instance.instance_id == parent_id)
            .map(|instance| instance.block_schema_name.as_str())
    });
    fields.iter().rev().find_map(|field| {
        if field.id != list_field_id {
            return None;
        }
        if parent_schema.is_some_and(|schema| field.block_schema_name.as_deref() != Some(schema)) {
            return None;
        }
        match &field.field_type {
            ParsedSheetFieldType::BlockList { block_schema_name } => {
                Some(block_schema_name.clone())
            }
            _ => None,
        }
    })
}

pub fn parse_sheet_template_ast(
    raw_text: &str,
    template_id: &str,
    source_path: Option<&str>,
) -> Result<SheetTemplateAst, String> {
    let template_version = first_template_version(raw_text);
    let template_hash = sha256_hex(raw_text);
    let mut sections = Vec::new();
    let mut block_schemas: Vec<SheetBlockSchema> = Vec::new();
    let mut block_instances: Vec<SheetBlockInstance> = Vec::new();
    let mut fields: Vec<SheetTemplateField> = Vec::new();
    let mut unmapped_lines = Vec::new();
    let mut current_section: Option<String> = None;
    let mut current_block_schema: Option<String> = None;
    let mut header_consumed = false;

    for (line_number, byte_start, byte_end, line) in source_lines(raw_text) {
        let (_, _, trimmed) = trimmed_span(byte_start, byte_end, &line);
        if trimmed.is_empty() {
            unmapped_lines.push(SheetUnmappedLine {
                line_number,
                byte_start,
                byte_end,
                raw: line,
            });
            continue;
        }
        if !header_consumed && template_version.is_some() && trimmed.contains("(v") {
            header_consumed = true;
            continue;
        }
        header_consumed = true;

        if is_block_schema_header(&trimmed) {
            current_block_schema = Some(trimmed.clone());
            block_schemas.push(SheetBlockSchema {
                name: trimmed,
                line_number,
                fields: Vec::new(),
            });
            continue;
        }

        if is_section_header(&trimmed) {
            current_section = Some(trimmed.clone());
            current_block_schema = None;
            sections.push(SheetTemplateSection {
                name: trimmed,
                line_number,
            });
            continue;
        }

        if let Some(instance_field) = split_block_instance_field_line(&trimmed) {
            let block_schema_name = resolve_block_list_schema_for_instance_root(
                &fields,
                &block_instances,
                &instance_field.root_field_id,
            );
            if let Some(block_schema_name) = block_schema_name {
                let belongs_to_schema = block_schemas
                    .iter()
                    .find(|schema| schema.name == block_schema_name)
                    .map(|schema| schema.fields.contains(&instance_field.field_id))
                    .unwrap_or(false);
                if belongs_to_schema {
                    validate_template_descriptor(
                        &instance_field.field_id,
                        line_number,
                        &instance_field.descriptor,
                    )?;
                    let field = SheetBlockInstanceField {
                        field_id: instance_field.field_id,
                        label: instance_field.label,
                        line_number,
                        byte_start,
                        byte_end,
                        raw: line,
                        template_descriptor: instance_field.descriptor,
                    };
                    if let Some(instance) = block_instances
                        .iter_mut()
                        .find(|instance| instance.instance_id == instance_field.instance_id)
                    {
                        instance.line_end = line_number;
                        instance.fields.push(field);
                    } else {
                        block_instances.push(SheetBlockInstance {
                            instance_id: instance_field.instance_id,
                            root_field_id: instance_field.root_field_id,
                            block_schema_name,
                            ordinal: instance_field.ordinal,
                            line_start: line_number,
                            line_end: line_number,
                            fields: vec![field],
                        });
                    }
                    continue;
                }
            }
        }

        if let Some((id, label, descriptor)) = split_field_line(&trimmed) {
            validate_template_descriptor(&id, line_number, &descriptor)?;
            let inferred = parse_field_type(&descriptor);
            if let Some(block_schema_name) = &current_block_schema {
                if let Some(schema) = block_schemas
                    .iter_mut()
                    .find(|schema| &schema.name == block_schema_name)
                {
                    schema.fields.push(id.clone());
                }
            }
            fields.push(SheetTemplateField {
                id,
                label,
                field_type: inferred.field_type,
                optional: inferred.optional,
                allowed_special_values: inferred.allowed_special_values,
                section: current_section.clone(),
                block_schema_name: current_block_schema.clone(),
                line_number,
                byte_start,
                byte_end,
                raw: line,
                template_descriptor: descriptor,
                protected: inferred.protected,
                editable_roles: inferred.editable_roles,
            });
            continue;
        }

        unmapped_lines.push(SheetUnmappedLine {
            line_number,
            byte_start,
            byte_end,
            raw: line,
        });
    }

    Ok(SheetTemplateAst {
        template_id: template_id.to_string(),
        template_version,
        template_hash,
        source_path: source_path.map(ToOwned::to_owned),
        sections,
        block_schemas,
        block_instances,
        fields,
        unmapped_lines,
    })
}
