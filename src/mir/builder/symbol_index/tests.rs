use super::*;
use crate::frontend::ast::Visibility;
use crate::frontend::parser::parse_module;
use crate::mir::data::ConstValue;

#[test]
fn candidate_names_cover_namespaces() {
    let variants = candidate_function_names(Some("Root::Inner"), &["Foo"]);
    assert!(variants.contains(&"Foo".to_string()));
    assert!(variants.contains(&"Root::Inner::Foo".to_string()));
}

#[test]
fn update_const_value_updates_all_tables() {
    let source = r#"
namespace Samples;

public const int ANSWER = 1;
"#;
    let module = parse_module(source).unwrap().module;
    let mut index = SymbolIndex::build(&module);
    index.update_const_value("Samples::ANSWER", ConstValue::Int32(42));
    let symbol = index
        .const_symbol("Samples::ANSWER")
        .expect("const registered");
    assert!(matches!(symbol.value, Some(ConstValue::Int32(42))));
    let ns_symbol = index
        .namespace_const(Some("Samples"), "ANSWER")
        .expect("namespace constant");
    assert!(matches!(ns_symbol.value, Some(ConstValue::Int32(42))));
}

#[test]
fn registers_fields_and_properties() {
    let source = r#"
namespace Samples;

public struct Widget
{
    public int Value;
    public string Name { get; set; }
}
"#;
    let module = parse_module(source).unwrap().module;
    let index = SymbolIndex::build(&module);
    assert!(index.has_field("Samples::Widget", "Value"));
    let metadata = index
        .field_metadata("Samples::Widget", "Value")
        .expect("field metadata");
    assert_eq!(metadata.visibility, Visibility::Public);
    let property = index
        .property_symbols("Samples::Widget")
        .and_then(|props| props.get("Name"))
        .expect("property symbol");
    assert!(property.accessors.contains_key(&PropertyAccessorKind::Get));
    assert!(property.accessors.contains_key(&PropertyAccessorKind::Set));
}

#[test]
fn handles_complex_module() {
    let source = r#"
namespace Samples;

public struct Holder
{
    public int Field;
    public string Name { get; init; }
    public init(int field) { Field = field; }
}

public class Gadget
{
    public init() { }
    public static void Init() { }
    public int Run(int value) { return value; }
}

public enum Status { Ready, Busy }

public const int GLOBAL = 1;
"#;
    let module = parse_module(source).unwrap().module;
    let index = SymbolIndex::build(&module);
    assert!(index.contains_type("Samples::Gadget"));
    assert!(index.has_enum_variant("Samples::Status", "Ready"));
    assert_eq!(index.method_count("Samples::Gadget", "Run"), Some(1));
    let sig = index
        .function_signature("Samples::Gadget::Run")
        .expect("signature");
    assert_eq!(sig.params.len(), 1);
    let const_symbol = index.const_symbol("Samples::GLOBAL").expect("global const");
    assert!(const_symbol.value.is_none());
    let property = index
        .property_symbols("Samples::Holder")
        .and_then(|props| props.get("Name"))
        .expect("holder property");
    assert!(property.accessors.contains_key(&PropertyAccessorKind::Get));
}

#[test]
fn registers_union_members() {
    let module = parse_module(
        r#"
namespace Samples;

public union Register
{
    public int Raw;
    public short Low;
}
"#,
    )
    .unwrap()
    .module;
    let index = SymbolIndex::build(&module);
    assert!(index.contains_type("Samples::Register"));
    assert!(index.has_field("Samples::Register", "Raw"));
    assert!(index.has_field("Samples::Register::Bits", "Low") == false);
}

#[test]
fn registers_interface_members() {
    let module = parse_module(
        r#"
namespace Samples;

public interface IRunnable
{
    public int Run(int value);
}
"#,
    )
    .unwrap()
    .module;
    let index = SymbolIndex::build(&module);
    assert!(index.contains_type("Samples::IRunnable"));
    assert_eq!(index.method_count("Samples::IRunnable", "Run"), Some(1));
}
