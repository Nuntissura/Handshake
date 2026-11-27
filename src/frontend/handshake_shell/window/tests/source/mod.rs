//! This crate's own source as code: comments and the content of literals blanked out, so naming a call is not making
//! one.

use std::fs;
use std::path::{Path, PathBuf};

pub struct Source {
    /// Relative to the crate, with forward slashes.
    pub path: String,
    pub code: String,
}

/// Every Rust file below a folder of the crate.
pub fn below(folder: &str) -> Vec<Source> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect(&root.join(folder), &mut files);
    files.sort();
    files
        .into_iter()
        .map(|file| Source {
            path: file
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            code: code_only(&fs::read_to_string(&file).unwrap()),
        })
        .collect()
}

fn collect(folder: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn part_of_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The lines of `code` in which `word` stands on its own, counted from 1. A boundary is checked only at an edge of
/// `word` that is itself a word character: `operator_root(` is found before `ring`, and `ProgramConfig::new(` before
/// an argument (MT-214 E9-F4).
pub fn lines_with(code: &str, word: &str) -> Vec<usize> {
    let starts = word.chars().next().is_some_and(part_of_word);
    let ends = word.chars().next_back().is_some_and(part_of_word);
    code.lines()
        .enumerate()
        .filter(|(_, line)| {
            line.match_indices(word).any(|(at, _)| {
                let before = line[..at].chars().next_back();
                let after = line[at + word.len()..].chars().next();
                !(starts && before.is_some_and(part_of_word)) && !(ends && after.is_some_and(part_of_word))
            })
        })
        .map(|(number, _)| number + 1)
        .collect()
}

/// The lines of `code` in which a name starts with `prefix`, whatever follows it (`ImmEscape` finds `ImmEscapeW` and
/// `ImmEscapeA`), counted from 1. What comes before must not be part of a word (MT-214 E9-F3).
#[allow(dead_code)] // used by the input crate's scan only; this file is included by several test binaries
pub fn lines_with_prefix(code: &str, prefix: &str) -> Vec<usize> {
    code.lines()
        .enumerate()
        .filter(|(_, line)| {
            line.match_indices(prefix)
                .any(|(at, _)| !line[..at].chars().next_back().is_some_and(part_of_word))
        })
        .map(|(number, _)| number + 1)
        .collect()
}

/// `source` with comments and the content of string and character literals replaced by spaces. Lines are kept.
pub fn code_only(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut code = String::with_capacity(source.len());
    let blank = |code: &mut String, c: char| code.push(if c == '\n' { '\n' } else { ' ' });
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        match (c, next) {
            ('/', Some('/')) => {
                while at < chars.len() && chars[at] != '\n' {
                    code.push(' ');
                    at += 1;
                }
            }
            ('/', Some('*')) => {
                let mut depth = 0;
                while at < chars.len() {
                    let pair = (chars[at], chars.get(at + 1).copied());
                    match pair {
                        ('/', Some('*')) => depth += 1,
                        ('*', Some('/')) => depth -= 1,
                        _ => {
                            blank(&mut code, chars[at]);
                            at += 1;
                            continue;
                        }
                    }
                    code.push_str("  ");
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                }
            }
            ('r', Some('"' | '#'))
                if !chars[..at]
                    .last()
                    .is_some_and(|c| c.is_alphanumeric() || *c == '_') =>
            {
                let hashes = chars[at + 1..].iter().take_while(|c| **c == '#').count();
                if chars.get(at + 1 + hashes) != Some(&'"') {
                    code.push(c);
                    at += 1;
                    continue;
                }
                let end: Vec<char> = std::iter::once('"')
                    .chain(std::iter::repeat_n('#', hashes))
                    .collect();
                for _ in 0..hashes + 2 {
                    code.push(' ');
                    at += 1;
                }
                while at < chars.len() && !chars[at..].starts_with(&end) {
                    blank(&mut code, chars[at]);
                    at += 1;
                }
                for _ in 0..end.len() {
                    code.push(' ');
                    at += 1;
                }
            }
            ('"', _) => {
                code.push('"');
                at += 1;
                while at < chars.len() && chars[at] != '"' {
                    if chars[at] == '\\' {
                        blank(&mut code, chars[at]);
                        at += 1;
                    }
                    if let Some(c) = chars.get(at) {
                        blank(&mut code, *c);
                    }
                    at += 1;
                }
                code.push('"');
                at += 1;
            }
            // A character literal; a lifetime has no closing quote there.
            ('\'', Some('\\')) => {
                // The quote, the backslash and what it escapes, which may be a quote.
                code.push_str("   ");
                at += 3;
                while at < chars.len() && chars[at] != '\'' {
                    blank(&mut code, chars[at]);
                    at += 1;
                }
                code.push(' ');
                at += 1;
            }
            ('\'', Some(_)) if chars.get(at + 2) == Some(&'\'') => {
                code.push_str("   ");
                at += 3;
            }
            _ => {
                code.push(c);
                at += 1;
            }
        }
    }
    code
}
