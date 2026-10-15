use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetTemplateAst {
    pub template_id: String,
    pub template_version: Option<String>,
    pub template_hash: String,
    pub source_path: Option<String>,
    pub sections: Vec<SheetTemplateSection>,
    pub block_schemas: Vec<SheetBlockSchema>,
    pub block_instances: Vec<SheetBlockInstance>,
    pub fields: Vec<SheetTemplateField>,
    pub unmapped_lines: Vec<SheetUnmappedLine>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetTemplateSection {
    pub name: String,
    pub line_number: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetBlockSchema {
    pub name: String,
    pub line_number: usize,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetBlockInstance {
    pub instance_id: String,
    pub root_field_id: String,
    pub block_schema_name: String,
    pub ordinal: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub fields: Vec<SheetBlockInstanceField>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetBlockInstanceField {
    pub field_id: String,
    pub label: String,
    pub line_number: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub raw: String,
    pub template_descriptor: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetTemplateField {
    pub id: String,
    pub label: String,
    pub field_type: ParsedSheetFieldType,
    pub optional: bool,
    pub allowed_special_values: Vec<String>,
    pub section: Option<String>,
    pub block_schema_name: Option<String>,
    pub line_number: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub raw: String,
    pub template_descriptor: String,
    pub protected: bool,
    pub editable_roles: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetUnmappedLine {
    pub line_number: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub raw: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParsedSheetFieldType {
    String,
    Integer,
    Number,
    Paragraph,
    Descriptor,
    Rule,
    Score10 {
        allowed_special_values: Vec<String>,
    },
    List {
        item_type: String,
    },
    Block {
        block_schema_name: String,
    },
    BlockList {
        block_schema_name: String,
    },
    Enum {
        values: Vec<String>,
        allow_other_type: Option<String>,
        allowed_special_values: Vec<String>,
    },
    Union {
        variants: Vec<ParsedSheetFieldType>,
        allowed_special_values: Vec<String>,
    },
    Unknown {
        descriptor: String,
    },
}

pub(crate) struct FieldTypeInference {
    pub(crate) field_type: ParsedSheetFieldType,
    pub(crate) optional: bool,
    pub(crate) allowed_special_values: Vec<String>,
    pub(crate) protected: bool,
    pub(crate) editable_roles: Vec<String>,
}

pub fn descriptor_inner(descriptor: &str) -> String {
    descriptor
        .trim()
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
        .unwrap_or(descriptor.trim())
        .trim()
        .to_string()
}

pub fn split_descriptor_tokens(inner: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (idx, ch) in inner.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth -= 1,
            '|' if depth == 0 => {
                let token = inner[start..idx].trim();
                if !token.is_empty() {
                    tokens.push(token.to_string());
                }
                start = idx + 1;
            }
            _ => {}
        }
    }
    let token = inner[start..].trim();
    if !token.is_empty() {
        tokens.push(token.to_string());
    }
    tokens
}

fn normalize_other_type(token: &str) -> Option<String> {
    let raw = token.trim();
    if !raw.to_ascii_lowercase().starts_with("other:") {
        return None;
    }
    let other = descriptor_inner(raw[6..].trim()).to_ascii_lowercase();
    if is_primary_type_keyword(&other) {
        Some(other)
    } else {
        Some("descriptor".to_string())
    }
}

fn is_special_value(token: &str) -> bool {
    matches!(token, "optional" | "unset" | "unknown" | "none")
}

pub fn editable_roles_from_token(token: &str) -> Option<Vec<String>> {
    let trimmed = token.trim();
    let lower = trimmed.to_ascii_lowercase();
    let role_spec = lower
        .strip_prefix("editable:")
        .or_else(|| lower.strip_prefix("editable="))
        .or_else(|| lower.strip_prefix("role:"))
        .or_else(|| lower.strip_prefix("roles:"))?;
    let roles = role_spec
        .split([',', '+', ' '])
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    Some(roles)
}

fn is_primary_type_keyword(token: &str) -> bool {
    matches!(
        token,
        "string" | "integer" | "number" | "paragraph" | "descriptor" | "score_10" | "list" | "rule"
    )
}

fn primary_field_type(token: &str, allowed_special_values: Vec<String>) -> ParsedSheetFieldType {
    match token {
        "integer" | "int" => ParsedSheetFieldType::Integer,
        "number" | "float" => ParsedSheetFieldType::Number,
        "paragraph" | "text" => ParsedSheetFieldType::Paragraph,
        "descriptor" => ParsedSheetFieldType::Descriptor,
        "rule" => ParsedSheetFieldType::Rule,
        "score_10" => ParsedSheetFieldType::Score10 {
            allowed_special_values,
        },
        "list" => ParsedSheetFieldType::List {
            item_type: "string".to_string(),
        },
        _ => ParsedSheetFieldType::String,
    }
}

fn parse_single_type(token: &str, allowed_special_values: Vec<String>) -> ParsedSheetFieldType {
    let normalized = token.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "string" | "integer" | "int" | "number" | "float" | "paragraph" | "text" | "descriptor"
        | "score_10" | "list" | "rule" => primary_field_type(&normalized, allowed_special_values),
        value if value.starts_with("list of ") => {
            let block_schema_name = token.trim()[8..].trim().to_string();
            if block_schema_name.ends_with("_Block") {
                ParsedSheetFieldType::BlockList { block_schema_name }
            } else {
                ParsedSheetFieldType::List {
                    item_type: block_schema_name,
                }
            }
        }
        value if value.starts_with("block:") => ParsedSheetFieldType::Block {
            block_schema_name: token.trim()[6..].trim().to_string(),
        },
        value if value.ends_with("_block") => ParsedSheetFieldType::Block {
            block_schema_name: token.trim().to_string(),
        },
        _ => ParsedSheetFieldType::Unknown {
            descriptor: token.trim().to_string(),
        },
    }
}

fn is_optional_descriptor(descriptor: &str) -> bool {
    let lower = descriptor.to_ascii_lowercase();
    lower.contains("optional") || lower.contains("unset")
}

pub(crate) fn parse_field_type(descriptor: &str) -> FieldTypeInference {
    let optional = is_optional_descriptor(descriptor);
    let inner = descriptor_inner(descriptor);
    let raw_tokens = split_descriptor_tokens(&inner);
    let mut protected = false;
    let mut editable_roles = Vec::new();
    let mut tokens = Vec::new();
    for token in raw_tokens {
        let normalized = token.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "protected" | "read_only" | "readonly") {
            protected = true;
            continue;
        }
        if let Some(roles) = editable_roles_from_token(&token) {
            for role in roles {
                if !editable_roles.contains(&role) {
                    editable_roles.push(role);
                }
            }
            continue;
        }
        tokens.push(token);
    }

    if descriptor.trim().to_ascii_lowercase().starts_with("<rule>") {
        return FieldTypeInference {
            field_type: ParsedSheetFieldType::Rule,
            optional,
            allowed_special_values: Vec::new(),
            protected,
            editable_roles,
        };
    }

    if tokens.is_empty() {
        return FieldTypeInference {
            field_type: ParsedSheetFieldType::String,
            optional,
            allowed_special_values: Vec::new(),
            protected,
            editable_roles,
        };
    }
    if tokens.len() == 1 {
        let token = tokens[0].trim();
        let normalized = token.to_ascii_lowercase();
        if is_special_value(&normalized) {
            return FieldTypeInference {
                field_type: ParsedSheetFieldType::String,
                optional,
                allowed_special_values: vec![token.to_string()],
                protected,
                editable_roles,
            };
        }
        let field_type = match parse_single_type(token, Vec::new()) {
            ParsedSheetFieldType::Unknown { .. } => ParsedSheetFieldType::String,
            other => other,
        };
        return FieldTypeInference {
            field_type,
            optional,
            allowed_special_values: Vec::new(),
            protected,
            editable_roles,
        };
    }

    let mut allowed_special_values = Vec::new();
    let mut allow_other_type = None;
    let mut enum_values = Vec::new();
    let mut primary_variants: Vec<ParsedSheetFieldType> = Vec::new();
    let mut primary_keywords = HashSet::new();
    let mut block_type = None;

    for token in tokens {
        let normalized = token.trim().to_ascii_lowercase();
        if is_special_value(&normalized) {
            allowed_special_values.push(token.trim().to_string());
            continue;
        }
        if let Some(other_type) = normalize_other_type(token.trim()) {
            allow_other_type = Some(other_type);
            continue;
        }

        let parsed = parse_single_type(token.trim(), Vec::new());
        match parsed {
            ParsedSheetFieldType::Unknown { descriptor } => enum_values.push(descriptor),
            ParsedSheetFieldType::Block { block_schema_name } => {
                block_type = Some(ParsedSheetFieldType::Block { block_schema_name });
            }
            ParsedSheetFieldType::BlockList { block_schema_name } => {
                block_type = Some(ParsedSheetFieldType::BlockList { block_schema_name });
            }
            _ if is_primary_type_keyword(&normalized) => {
                if primary_keywords.insert(normalized.clone()) {
                    primary_variants.push(primary_field_type(&normalized, Vec::new()));
                }
            }
            other => {
                block_type = Some(other);
            }
        }
    }

    if let Some(field_type) = block_type {
        return FieldTypeInference {
            field_type,
            optional,
            allowed_special_values,
            protected,
            editable_roles,
        };
    }

    if primary_variants.len() > 1 {
        return FieldTypeInference {
            field_type: ParsedSheetFieldType::Union {
                variants: primary_variants,
                allowed_special_values: allowed_special_values.clone(),
            },
            optional,
            allowed_special_values,
            protected,
            editable_roles,
        };
    }

    if let Some(field_type) = primary_variants.into_iter().next() {
        let mut primary_allowed = allowed_special_values.clone();
        primary_allowed.extend(enum_values);
        let field_type = match field_type {
            ParsedSheetFieldType::Score10 { .. } => ParsedSheetFieldType::Score10 {
                allowed_special_values: primary_allowed.clone(),
            },
            ParsedSheetFieldType::List { .. } => ParsedSheetFieldType::List {
                item_type: "string".to_string(),
            },
            other => other,
        };
        return FieldTypeInference {
            field_type,
            optional,
            allowed_special_values: primary_allowed,
            protected,
            editable_roles,
        };
    }

    if !enum_values.is_empty() || allow_other_type.is_some() {
        return FieldTypeInference {
            field_type: ParsedSheetFieldType::Enum {
                values: enum_values,
                allow_other_type,
                allowed_special_values: allowed_special_values.clone(),
            },
            optional,
            allowed_special_values,
            protected,
            editable_roles,
        };
    }

    FieldTypeInference {
        field_type: ParsedSheetFieldType::String,
        optional,
        allowed_special_values,
        protected,
        editable_roles,
    }
}
