use regex::Regex;
use std::error::Error;
use std::path::Path;
use walkdir::WalkDir;

pub fn run() -> Result<(), Box<dyn Error>> {
    let root = Path::new("packages");
    let testcase_re = Regex::new(
        r"(?m)^(\s*(?:@[^\n]+\s+)*)?(async\s+)?testcase\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(",
    )?;
    let name_re = Regex::new(r"^Given_.*_When_.*_Then_.*$")?;

    let mut errors = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("cl") {
            continue;
        }
        let text = std::fs::read_to_string(entry.path())?;
        let matches: Vec<_> = testcase_re.captures_iter(&text).collect();
        for (idx, caps) in matches.iter().enumerate() {
            let name = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            if !name_re.is_match(name) {
                errors.push(format!(
                    "{}: testcase name must use Given/When/Then pattern (found `{name}`)",
                    entry.path().display()
                ));
            }
            let start = caps.get(0).map(|m| m.end()).unwrap_or(0);
            let end = matches
                .get(idx + 1)
                .and_then(|next| next.get(0))
                .map(|m| m.start())
                .unwrap_or_else(|| text.len());
            let body = &text[start..end];
            let assert_count = body.matches("Assert.").count();
            if assert_count != 1 {
                errors.push(format!(
                    "{}: testcase `{name}` must contain exactly one Assert.* call (found {assert_count})",
                    entry.path().display()
                ));
            }
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    errors.sort();
    errors.dedup();
    let mut message = String::from("testcase authoring violations detected:");
    for error in errors {
        message.push_str("\n  - ");
        message.push_str(&error);
    }
    Err(message.into())
}
