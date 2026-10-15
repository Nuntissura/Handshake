pub(crate) fn single_line_value(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

pub(crate) fn count_template_field_lines(raw_text: &str) -> usize {
    raw_text.lines().filter(|line| field_id_line(line)).count()
}

pub(crate) fn count_template_section_lines(raw_text: &str) -> usize {
    raw_text
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.contains(':')
                && line
                    .chars()
                    .next()
                    .map(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
                    .unwrap_or(false)
        })
        .count()
}

fn field_id_line(line: &str) -> bool {
    let Some(colon) = line.find(':') else {
        return false;
    };
    let before_colon = line[..colon].trim();
    let separator = [" \u{2014} ", " \u{2013} ", " - "]
        .iter()
        .find_map(|separator| before_colon.find(separator));
    let Some(separator) = separator else {
        return false;
    };
    let field_id = before_colon[..separator].trim();
    let mut saw_dash = false;
    !field_id.is_empty()
        && field_id.chars().all(|ch| {
            if ch == '-' {
                saw_dash = true;
                true
            } else {
                ch.is_ascii_uppercase() || ch.is_ascii_digit()
            }
        })
        && saw_dash
}
