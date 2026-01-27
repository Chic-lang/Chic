use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViolationKind {
    Namespace,
    Using,
}

impl ViolationKind {
    fn code(self) -> &'static str {
        "AN0005"
    }

    fn message(self) -> &'static str {
        match self {
            ViolationKind::Namespace => "namespace declarations must use the `Std` root",
            ViolationKind::Using => "using directives must import from the `Std` namespace",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Violation {
    path: PathBuf,
    line: usize,
    kind: ViolationKind,
    snippet: String,
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives inside workspace")
        .to_path_buf();

    let mut violations = Vec::new();
    for entry in WalkDir::new(&repo_root)
        .into_iter()
        .filter_entry(|entry| !is_ignored(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .map_or(true, |ext| ext != "ch")
        {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        violations.extend(find_violations(entry.path(), &contents));
    }

    if violations.is_empty() {
        println!("AN0005: namespace/usings already rooted in `Std`.");
        return Ok(());
    }

    eprintln!("AN0005: detected legacy `stdlib`/`StdLib` usage:");
    for violation in &violations {
        eprintln!(
            "  - {}:{} {}: {}",
            display(&repo_root, &violation.path),
            violation.line,
            violation.kind.code(),
            violation.kind.message()
        );
        eprintln!("      {}", violation.snippet);
    }

    Err("AN0005 namespace enforcement failed".into())
}

fn is_ignored(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(".git")
                | Some("target")
                | Some("coverage")
                | Some("generated")
                | Some("profiling")
                | Some("tmp")
                | Some("build")
        )
    })
}

fn find_violations(path: &Path, contents: &str) -> Vec<Violation> {
    contents
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| match detect_violation(line) {
            Some(kind) => Some(Violation {
                path: path.to_path_buf(),
                line: idx + 1,
                kind,
                snippet: line.trim().to_string(),
            }),
            None => None,
        })
        .collect()
}

fn detect_violation(line: &str) -> Option<ViolationKind> {
    let normalized = line.to_ascii_lowercase();
    if normalized.contains("namespace stdlib") {
        return Some(ViolationKind::Namespace);
    }
    if normalized.contains("using static stdlib") {
        return Some(ViolationKind::Using);
    }
    if normalized.contains("using stdlib") {
        return Some(ViolationKind::Using);
    }
    None
}

fn display(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => relative.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_namespace_stdlib() {
        let path = Path::new("sample.ch");
        let violations = find_violations(path, "namespace Std;\nnamespace stdlib.Helpers;");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 2);
        assert!(violations[0].snippet.contains("namespace stdlib.Helpers"));
        assert!(matches!(violations[0].kind, ViolationKind::Namespace));
    }

    #[test]
    fn detects_using_stdlib() {
        let path = Path::new("sample.ch");
        let violations = find_violations(path, "using static Std.Core;\nusing StdLib.Diagnostics;");
        assert_eq!(violations.len(), 1);
        assert!(matches!(violations[0].kind, ViolationKind::Using));
    }

    #[test]
    fn ignores_valid_namespaces() {
        let path = Path::new("sample.ch");
        let violations =
            find_violations(path, "namespace Std.Async;\nimport Std.Collections.Vector;");
        assert!(violations.is_empty());
    }
}
