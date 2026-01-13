use regex::Regex;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn run() -> Result<(), Box<dyn Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("failed to locate workspace root")?
        .to_path_buf();
    let parser_root = repo_root.join("src/frontend/parser");

    let checks = vec![
        Check::new(
            "peek_n",
            r"\.\s*peek_n\s*\(",
            &["src/frontend/parser/core/cursor.rs"],
        ),
        Check::new("peek_keyword_n", r"\.\s*peek_keyword_n\s*\(", &[]),
        Check::new("peek_punctuation_n", r"\.\s*peek_punctuation_n\s*\(", &[]),
        Check::new(
            "try_type_expr_from",
            r"\.\s*try_type_expr_from\s*\(",
            &["src/frontend/parser/types.rs"],
        ),
    ];

    let mut violations = Vec::new();
    for entry in WalkDir::new(&parser_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        if entry.path().extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let rel_path = entry
            .path()
            .strip_prefix(&repo_root)
            .unwrap_or(entry.path());
        let rel_path_str = normalized_path(rel_path);
        let source = fs::read_to_string(entry.path())?;
        let lines: Vec<&str> = source.lines().collect();

        for check in &checks {
            if check.should_skip(&rel_path_str) {
                continue;
            }
            for mat in check.regex.find_iter(&source) {
                let line_idx = line_index(&source, mat.start());
                let snippet = lines
                    .get(line_idx)
                    .copied()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                match find_ll1_comment(&lines, line_idx) {
                    Some(reason) if reason.trim().is_empty() => {
                        violations.push(Violation {
                            path: rel_path_str.clone(),
                            line: line_idx + 1,
                            detail: format!(
                                "`// LL1_ALLOW:` before `{}` must describe the exemption",
                                check.name
                            ),
                            snippet,
                        });
                    }
                    Some(_) => continue,
                    None => {
                        violations.push(Violation {
                            path: rel_path_str.clone(),
                            line: line_idx + 1,
                            detail: format!(
                                "missing `// LL1_ALLOW:` comment for `{}` lookahead",
                                check.name
                            ),
                            snippet,
                        });
                    }
                }
            }
        }
    }

    if violations.is_empty() {
        println!("LL(1) guardrails clean");
        return Ok(());
    }

    eprintln!("LL(1) guardrail violations:");
    for violation in &violations {
        eprintln!(
            "- {}:{} {}",
            violation.path, violation.line, violation.detail
        );
        if !violation.snippet.is_empty() {
            eprintln!("    {}", violation.snippet);
        }
    }
    Err("LL(1) guardrail lint failed".into())
}

struct Violation {
    path: String,
    line: usize,
    detail: String,
    snippet: String,
}

struct Check {
    name: &'static str,
    regex: Regex,
    skip: &'static [&'static str],
}

impl Check {
    fn new(name: &'static str, pattern: &str, skip: &'static [&'static str]) -> Self {
        Self {
            name,
            regex: Regex::new(pattern).expect("invalid regex"),
            skip,
        }
    }

    fn should_skip(&self, path: &str) -> bool {
        self.skip.iter().any(|candidate| path == *candidate)
    }
}

fn normalized_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn line_index(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx].bytes().filter(|b| *b == b'\n').count()
}

fn find_ll1_comment(lines: &[&str], line_idx: usize) -> Option<String> {
    if let Some(reason) = extract_reason(lines.get(line_idx).copied().unwrap_or("")) {
        return Some(reason);
    }

    let mut idx = line_idx;
    while idx > 0 {
        let candidate = lines.get(idx - 1).copied().unwrap_or("").trim();
        if candidate.is_empty() {
            idx -= 1;
            continue;
        }
        if let Some(reason) = extract_reason(candidate) {
            return Some(reason);
        }
        return None;
    }
    None
}

fn extract_reason(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("//") {
        let rest = rest.trim();
        if let Some(reason) = rest.strip_prefix("LL1_ALLOW:") {
            return Some(reason.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{extract_reason, find_ll1_comment};

    #[test]
    fn extracts_reason_from_comment() {
        assert_eq!(
            extract_reason("// LL1_ALLOW: contextual keyword").as_deref(),
            Some("contextual keyword")
        );
        assert!(extract_reason("// something else").is_none());
    }

    #[test]
    fn finds_comment_above_code() {
        let src = vec!["// LL1_ALLOW: reason", "if self.peek_n(1) { ... }"];
        assert_eq!(find_ll1_comment(&src, 1).as_deref(), Some("reason"));
    }

    #[test]
    fn rejects_missing_comment() {
        let src = vec!["// unrelated", "if self.peek_n(1) { ... }"];
        assert!(find_ll1_comment(&src, 1).is_none());
    }
}
