use std::{error::Error, path::Path, path::PathBuf};

const SPEC_LINK_TABLE: &[(&str, &[&str])] = include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../src/typeck/spec_link_table.in"
));

const REQUIRED_CODES: &[&str] = &[
    "TCK001", "TCK002", "TCK003", "TCK010", "TCK011", "TCK012", "TCK013", "TCK014", "TCK015",
    "TCK022", "TCK030", "TCK032", "TCK035", "TCK037", "TCK070", "TCK071", "TCK072", "TCK080",
    "TCK090", "TCK091", "TCK092", "TCK093", "TCK094", "TCK095", "TCK096", "TCK097", "TCK098",
    "TCK099",
];

const MM_CODES: &[&str] = &["MM0001", "MM0002", "MM0003", "MM0101", "MM0102"];

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut enforce_mm = false;
    for arg in args {
        match arg.as_str() {
            "--enforce-mm" => enforce_mm = true,
            "--help" | "-h" => {
                println!(
                    "cargo xtask task-status [--enforce-mm]\n\n\
                     Without flags the command verifies core spec-linked diagnostics.\n\
                     Pass --enforce-mm to include MM-series concurrency diagnostics once they land."
                );
                return Ok(());
            }
            other => {
                return Err(format!("unknown task-status flag `{other}`").into());
            }
        }
    }

    let mut required_codes: Vec<&str> = REQUIRED_CODES.to_vec();
    if enforce_mm {
        required_codes.extend_from_slice(MM_CODES);
    }

    let root = repo_root();
    let missing_codes = missing_required_codes(&required_codes);
    let missing_docs = missing_doc_paths(&root);

    if !missing_codes.is_empty() {
        eprintln!(
            "spec link entries missing for codes: {}",
            missing_codes.join(", ")
        );
    }
    if !missing_docs.is_empty() {
        eprintln!("spec link targets missing files:");
        for doc in &missing_docs {
            eprintln!("  - {doc}");
        }
    }

    if missing_codes.is_empty() && missing_docs.is_empty() {
        println!(
            "{} spec-linked diagnostics verified across {} documents (MM enforcement: {}).",
            SPEC_LINK_TABLE.len(),
            count_unique_docs(),
            if enforce_mm { "enabled" } else { "disabled" }
        );
        Ok(())
    } else {
        Err("task-status checks failed".into())
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask in workspace")
        .to_path_buf()
}

fn missing_required_codes(required: &[&str]) -> Vec<String> {
    required
        .iter()
        .filter(|code| SPEC_LINK_TABLE.iter().all(|(entry, _)| entry != *code))
        .map(|code| code.to_string())
        .collect()
}

fn missing_doc_paths(root: &Path) -> Vec<String> {
    let mut missing = Vec::new();
    for (_, docs) in SPEC_LINK_TABLE {
        for doc in *docs {
            let path = doc.split('#').next().unwrap_or(doc);
            if !root.join(path).exists() {
                missing.push(path.to_string());
            }
        }
    }
    missing.sort();
    missing.dedup();
    missing
}

fn count_unique_docs() -> usize {
    use std::collections::HashSet;
    let mut docs = HashSet::new();
    for (_, entries) in SPEC_LINK_TABLE {
        for doc in *entries {
            docs.insert(doc.split('#').next().unwrap_or(doc));
        }
    }
    docs.len()
}
