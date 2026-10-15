use std::collections::HashSet;

pub fn normalize_tag(text: &str) -> String {
    text.trim().to_ascii_lowercase()
}

pub fn text_snippet(text: &str, query: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.len() <= 220 {
        return text;
    }
    let lower = text.to_ascii_lowercase();
    let needle = query.trim().to_ascii_lowercase();
    let start = if needle.is_empty() {
        0
    } else {
        lower
            .find(&needle)
            .map(|idx| idx.saturating_sub(60))
            .unwrap_or(0)
    };
    let end = (start + 220).min(text.len());
    let mut start = start;
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = end;
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    text[start..end].to_owned()
}

pub fn normalized_search_tags(tags: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        let normalized = normalize_tag(tag);
        if !normalized.is_empty() && !out.iter().any(|existing| existing == &normalized) {
            out.push(normalized);
        }
    }
    out.sort();
    out
}

pub fn merged_tag_sets(sets: &[&[String]]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for set in sets {
        for tag in set.iter() {
            let normalized = normalize_tag(tag);
            if !normalized.is_empty() && !out.contains(&normalized) {
                out.push(normalized);
            }
        }
    }
    out.sort();
    out
}

pub fn contains_all_tags(actual: &[String], required: &[String]) -> bool {
    required
        .iter()
        .all(|required| actual.iter().any(|tag| tag == required))
}

pub fn ckc_query_terms(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for term in query
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_alphanumeric())
        .map(str::trim)
        .filter(|term| !term.is_empty())
    {
        if !terms.iter().any(|existing| existing == term) {
            terms.push(term.to_owned());
        }
    }
    terms
}

pub fn ckc_candidate_matches_query_terms(title: &str, search_text: &str, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let haystack = format!("{}\n{}", title, search_text).to_ascii_lowercase();
    terms.iter().all(|term| haystack.contains(term))
}

fn ckc_trigrams(value: &str) -> HashSet<String> {
    value
        .to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .flat_map(|word| {
            let padded = format!("  {word} ").chars().collect::<Vec<_>>();
            padded
                .windows(3)
                .map(|window| window.iter().collect::<String>())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Jaccard similarity over trigram sets, the `similarity()` definition of
/// pg_trgm (the reference `atelier_trgm_similarity` SQL function).
fn ckc_trigram_similarity(left: &str, right: &str) -> f64 {
    let left = ckc_trigrams(left);
    let right = ckc_trigrams(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let common = left.intersection(&right).count();
    common as f64 / (left.len() + right.len() - common) as f64
}

/// The reference `fuzzy_score`: the greatest of trigram similarity against the
/// search text, trigram similarity against the title, and 1.0 for a literal
/// case-insensitive containment in either.
pub fn ckc_fuzzy_score(title: &str, search_text: &str, query: &str) -> f64 {
    if query.is_empty() {
        return 0.0;
    }
    let lower_query = query.to_ascii_lowercase();
    let mut score = ckc_trigram_similarity(search_text, query)
        .max(ckc_trigram_similarity(title, query));
    if search_text.to_ascii_lowercase().contains(&lower_query)
        || title.to_ascii_lowercase().contains(&lower_query)
    {
        score = score.max(1.0);
    }
    score
}

pub fn join_search_text(parts: &[Option<&str>]) -> String {
    parts
        .iter()
        .filter_map(|part| part.map(str::trim).filter(|part| !part.is_empty()))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn tags_text(tags: &[String]) -> Option<String> {
    if tags.is_empty() {
        None
    } else {
        serde_json::to_string(tags).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_normalization() {
        assert_eq!(normalize_tag("  BlondE "), "blonde");
        assert_eq!(normalize_tag("Red Hair"), "red hair");
    }
}
