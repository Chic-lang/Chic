use std::fs;
use std::path::PathBuf;

#[test]
fn xml_to_markdown_doc_is_present_and_linked() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let doc_path = root.join("docs").join("tooling").join("xml_to_markdown.md");
    assert!(
        doc_path.exists(),
        "docs/tooling/xml_to_markdown.md must exist"
    );

    let readme = fs::read_to_string(root.join("README.md")).expect("read README");
    assert!(
        readme.contains("xml_to_markdown"),
        "README should mention xml_to_markdown mapping guide"
    );
}
