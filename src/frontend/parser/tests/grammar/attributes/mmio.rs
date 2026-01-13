use super::helpers::*;
use crate::frontend::ast::{Item, MmioAccess};

#[test]
fn mmio_struct_requires_base_argument() {
    let source = r#"
@mmio()
public struct Device
{
    @register(offset = 0x00, width = 32)
    public int Control;
}
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("@mmio") && msg.contains("base")),
        "expected missing base diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn mmio_duplicate_attribute_reports_error() {
    let (_, diagnostics) = collect_attributes_from_source("@mmio(base = 0)\n@mmio(base = 16)");
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("duplicate `@mmio` attribute")),
        "expected duplicate mmio diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn register_attribute_requires_mmio_struct() {
    let source = r"
public struct Widget
{
    @register(offset = 0, width = 16)
    public ushort Status;
}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("`@register` attribute")),
        "expected register misuse diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn mmio_struct_attributes_are_parsed() {
    let source = r#"
@mmio(base = 4096)
public struct Device
{
    public int Control;
}
"#;

    let (parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        diagnostics
    );
    let parse = parse.expect("expected parse result");
    let structure = match &parse.module.items[0] {
        Item::Struct(def) => def,
        other => panic!("expected struct, found {other:?}"),
    };

    let mmio = structure.mmio.as_ref().expect("expected mmio attribute");
    assert_eq!(mmio.base_address, 4096);
    assert!(mmio.size.is_none());
    assert!(mmio.address_space.is_none());
    assert!(mmio.requires_unsafe);
}

#[test]
fn register_attribute_parses_access_modes() {
    let source = r"
@mmio(base = 0)
public struct Device
{
    @register(offset = 16, width = 16, access = ro)
    public ushort Status;
}
";

    let (parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        diagnostics
    );
    let parse = parse.expect("expected parse result");
    let structure = match &parse.module.items[0] {
        Item::Struct(def) => def,
        other => panic!("expected struct, found {other:?}"),
    };
    let field = &structure.fields[0];
    let mmio = field.mmio.as_ref().expect("expected register metadata");
    assert_eq!(mmio.offset, 16);
    assert_eq!(mmio.width_bits, 16);
    assert_eq!(mmio.access, MmioAccess::ReadOnly);
}

#[test]
fn register_attribute_reports_invalid_access_mode() {
    let source = r"
@mmio(base = 0)
public struct Peripheral
{
    @register(offset = 0x0, access = invalid)
    public int Value;
}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("unsupported access mode `invalid`")),
        "expected invalid access diagnostic, found {diagnostics:?}"
    );
}
