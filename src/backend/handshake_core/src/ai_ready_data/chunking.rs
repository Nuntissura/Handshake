use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::ai_ready_data::AiReadyDataError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeLanguage {
    Rust,
    TypeScript,
    JavaScript,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub strategy_id: String,
    pub bronze_ref: String,
    pub silver_id: String,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub token_count: u32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: u32,
    pub line_end: u32,
    pub content_hash: String,
    pub text: String,
}

pub fn estimate_tokens(text: &str) -> u32 {
    (text.len().saturating_add(3) / 4) as u32
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn deterministic_uuid_for_str(value: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();

    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub(crate) fn compute_silver_id(
    bronze_ref: &str,
    strategy_id: &str,
    chunk_index: u32,
    byte_start: usize,
    byte_end: usize,
    content_hash: &str,
    processing_pipeline_version: &str,
    embedding_model_id: &str,
    embedding_model_version: &str,
) -> String {
    let raw = format!(
        "bronze_ref={bronze_ref}\nstrategy_id={strategy_id}\nchunk_index={chunk_index}\nbyte_start={byte_start}\nbyte_end={byte_end}\ncontent_hash={content_hash}\nprocessing_pipeline_version={processing_pipeline_version}\nembedding_model_id={embedding_model_id}\nembedding_model_version={embedding_model_version}\n"
    );
    format!("slv_{}", deterministic_uuid_for_str(&raw))
}

fn compute_line_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for (idx, ch) in text.char_indices() {
        if ch == '\n' {
            offsets.push(idx + 1);
        }
    }
    offsets
}

fn byte_range_to_line_range(
    line_offsets: &[usize],
    byte_start: usize,
    byte_end: usize,
) -> (u32, u32) {
    fn line_for_byte(line_offsets: &[usize], byte: usize) -> u32 {
        match line_offsets.binary_search(&byte) {
            Ok(idx) => idx as u32,
            Err(idx) => idx.saturating_sub(1) as u32,
        }
    }

    let start = line_for_byte(line_offsets, byte_start);
    let end = line_for_byte(line_offsets, byte_end.saturating_sub(1));
    (start + 1, end + 1)
}

fn safe_slice(source: &str, byte_start: usize, byte_end: usize) -> Result<&str, AiReadyDataError> {
    source
        .get(byte_start..byte_end)
        .ok_or(AiReadyDataError::Chunking("invalid utf8 slice boundaries"))
}

pub fn chunk_code_treesitter(
    bronze_ref: &str,
    source: &str,
    language: CodeLanguage,
    processing_pipeline_version: &str,
    embedding_model_id: &str,
    embedding_model_version: &str,
) -> Result<Vec<Chunk>, AiReadyDataError> {
    let strategy_id = "code_ast_treesitter_v1".to_string();
    let line_offsets = compute_line_offsets(source);

    let mut parser = tree_sitter::Parser::new();
    let ts_lang = match language {
        CodeLanguage::Rust => tree_sitter_rust::LANGUAGE,
        CodeLanguage::JavaScript => tree_sitter_javascript::LANGUAGE,
        CodeLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
    };
    parser
        .set_language(&ts_lang.into())
        .map_err(|_| AiReadyDataError::Chunking("tree-sitter language init failed"))?;

    let tree = parser
        .parse(source, None)
        .ok_or(AiReadyDataError::Chunking(
            "tree-sitter parse returned None",
        ))?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(AiReadyDataError::Chunking("tree-sitter parse error"));
    }

    let mut cursor = root.walk();
    let mut nodes = Vec::new();
    for node in root.named_children(&mut cursor) {
        let kind = node.kind();
        let selected = match language {
            CodeLanguage::Rust => matches!(
                kind,
                "function_item"
                    | "struct_item"
                    | "enum_item"
                    | "impl_item"
                    | "trait_item"
                    | "mod_item"
            ),
            CodeLanguage::JavaScript | CodeLanguage::TypeScript => matches!(
                kind,
                "function_declaration"
                    | "class_declaration"
                    | "interface_declaration"
                    | "type_alias_declaration"
                    | "enum_declaration"
                    | "export_statement"
            ),
        };
        if selected {
            nodes.push(node);
        }
    }

    if nodes.is_empty() {
        let content_hash = sha256_hex(source.as_bytes());
        let silver_id = compute_silver_id(
            bronze_ref,
            &strategy_id,
            0,
            0,
            source.len(),
            &content_hash,
            processing_pipeline_version,
            embedding_model_id,
            embedding_model_version,
        );
        let token_count = estimate_tokens(source);
        let (line_start, line_end) = byte_range_to_line_range(&line_offsets, 0, source.len());
        return Ok(vec![Chunk {
            strategy_id,
            bronze_ref: bronze_ref.to_string(),
            silver_id,
            chunk_index: 0,
            total_chunks: 1,
            token_count,
            byte_start: 0,
            byte_end: source.len(),
            line_start,
            line_end,
            content_hash,
            text: source.to_string(),
        }]);
    }

    nodes.sort_by_key(|n| n.start_byte());

    // Enforce "imports with first chunk" by extending the first chunk start to 0.
    let mut ranges: Vec<(usize, usize)> = nodes
        .iter()
        .map(|n| (n.start_byte(), n.end_byte()))
        .collect();
    if let Some(first) = ranges.first_mut() {
        first.0 = 0;
    }

    let total_chunks = ranges.len() as u32;
    let mut out = Vec::with_capacity(ranges.len());
    for (idx, (byte_start, byte_end)) in ranges.into_iter().enumerate() {
        if byte_end <= byte_start || byte_end > source.len() {
            return Err(AiReadyDataError::Chunking("invalid chunk byte range"));
        }

        let text = safe_slice(source, byte_start, byte_end)?.to_string();
        let content_hash = sha256_hex(text.as_bytes());
        let silver_id = compute_silver_id(
            bronze_ref,
            &strategy_id,
            idx as u32,
            byte_start,
            byte_end,
            &content_hash,
            processing_pipeline_version,
            embedding_model_id,
            embedding_model_version,
        );
        let token_count = estimate_tokens(&text);
        let (line_start, line_end) = byte_range_to_line_range(&line_offsets, byte_start, byte_end);

        out.push(Chunk {
            strategy_id: strategy_id.clone(),
            bronze_ref: bronze_ref.to_string(),
            silver_id,
            chunk_index: idx as u32,
            total_chunks,
            token_count,
            byte_start,
            byte_end,
            line_start,
            line_end,
            content_hash,
            text,
        });
    }

    Ok(out)
}

pub fn chunk_document_header_recursive(
    bronze_ref: &str,
    source: &str,
    processing_pipeline_version: &str,
    embedding_model_id: &str,
    embedding_model_version: &str,
) -> Result<Vec<Chunk>, AiReadyDataError> {
    let strategy_id = "document_header_recursive_v1".to_string();
    let line_offsets = compute_line_offsets(source);

    let mut headers: Vec<(usize, usize)> = Vec::new(); // (byte_offset, level)
    let mut in_code_fence = false;
    let mut line_start = 0usize;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
        }
        if !in_code_fence {
            let mut level = 0usize;
            for ch in trimmed.chars() {
                if ch == '#' {
                    level += 1;
                } else {
                    break;
                }
            }
            if level > 0 && trimmed[level..].starts_with(' ') {
                headers.push((line_start, level));
            }
        }

        line_start = line_start.saturating_add(line.len());
    }

    let mut sections: Vec<(usize, usize)> = Vec::new();
    if headers.is_empty() {
        sections.push((0, source.len()));
    } else {
        for (idx, (start, _level)) in headers.iter().enumerate() {
            let end = headers
                .get(idx + 1)
                .map(|(s, _)| *s)
                .unwrap_or(source.len());
            sections.push((*start, end));
        }
        if sections.first().map(|(s, _)| *s).unwrap_or(0) > 0 {
            // Prefix before first header becomes section 0.
            let first_start = sections.first().map(|(s, _)| *s).unwrap_or(0);
            sections.insert(0, (0, first_start));
        }
    }

    // Split oversized sections on blank lines (outside code fences), deterministically.
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for (sec_start, sec_end) in sections {
        let sec_text = safe_slice(source, sec_start, sec_end)?;
        let mut cursor = 0usize;
        let mut local_in_code = false;
        let mut last_break = 0usize;
        for line in sec_text.split_inclusive('\n') {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                local_in_code = !local_in_code;
            }
            cursor = cursor.saturating_add(line.len());
            if !local_in_code && line.trim().is_empty() {
                let abs_break = sec_start.saturating_add(cursor);
                ranges.push((sec_start.saturating_add(last_break), abs_break));
                last_break = cursor;
            }
        }
        if last_break < sec_text.len() {
            ranges.push((sec_start.saturating_add(last_break), sec_end));
        }
    }

    let mut kept_ranges: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
    for (byte_start, byte_end) in ranges {
        if byte_end <= byte_start || byte_end > source.len() {
            continue;
        }
        let text = safe_slice(source, byte_start, byte_end)?;
        if text.trim().is_empty() {
            continue;
        }
        kept_ranges.push((byte_start, byte_end));
    }

    let total_chunks = kept_ranges.len() as u32;
    let mut out = Vec::with_capacity(kept_ranges.len());
    for (idx, (byte_start, byte_end)) in kept_ranges.into_iter().enumerate() {
        let text = safe_slice(source, byte_start, byte_end)?.to_string();
        let content_hash = sha256_hex(text.as_bytes());
        let silver_id = compute_silver_id(
            bronze_ref,
            &strategy_id,
            idx as u32,
            byte_start,
            byte_end,
            &content_hash,
            processing_pipeline_version,
            embedding_model_id,
            embedding_model_version,
        );
        let token_count = estimate_tokens(&text);
        let (line_start, line_end) = byte_range_to_line_range(&line_offsets, byte_start, byte_end);
        out.push(Chunk {
            strategy_id: strategy_id.clone(),
            bronze_ref: bronze_ref.to_string(),
            silver_id,
            chunk_index: idx as u32,
            total_chunks,
            token_count,
            byte_start,
            byte_end,
            line_start,
            line_end,
            content_hash,
            text,
        });
    }

    Ok(out)
}
