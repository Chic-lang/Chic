use chic::frontend::metadata::reflection::{
    collect_reflection_tables, serialize_reflection_tables,
};
use chic::frontend::parser::parse_module;
use std::fs;
use std::path::{Path, PathBuf};

fn golden_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/reflection")
        .join(name)
}

#[test]
fn reflection_metadata_matches_golden_fixture() {
    let source = r#"
namespace Samples;

public struct Widget
{
    public int Id;
    public required string Name { get; set; }

    public init(int id)
    {
        Id = id;
    }

    public string Describe()
    {
        return Name;
    }
}

public interface INamed
{
    public string Name { get; }
}

public class EntryPoint
{
    public static void Run() { }
}
"#;

    let parse = parse_module(source).unwrap_or_else(|err| {
        panic!("parse failed: {:?}", err.diagnostics());
    });
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        parse.diagnostics
    );
    let module = parse.module;
    let tables = collect_reflection_tables(&module);
    let actual = serialize_reflection_tables(&tables).expect("serialize reflection tables");
    let golden = golden_path("basic.json");
    let expected = fs::read_to_string(&golden)
        .unwrap_or_else(|_| panic!("missing golden at {}", golden.display()));
    assert_eq!(
        actual.trim(),
        expected.trim(),
        "reflection metadata deviated"
    );
}
