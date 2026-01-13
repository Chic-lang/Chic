use super::common::RequireExt;
use super::*;

#[expect(
    clippy::too_many_lines,
    reason = "pattern layout validation relies on an end-to-end lowering fixture"
)]
#[test]
fn collects_type_layouts_for_structs_and_enums() {
    let source = r"
namespace Layouts;

public struct Point { public int X; public int Y; }

public class Wrapper
{
public Point Inner;
public long Total;
}

public enum Shape
{
Circle { public double Radius; },
Rect { public int W; public int H; }
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    let layouts = &lowering.module.type_layouts.types;

    let point_layout = layouts
        .get("Layouts::Point")
        .and_then(|layout| match layout {
            TypeLayout::Struct(layout) => Some(layout),
            _ => None,
        })
        .require("missing struct layout for Point");
    assert_eq!(point_layout.fields.len(), 2);
    assert_eq!(point_layout.fields[0].name, "X");
    assert_eq!(point_layout.fields[0].offset, Some(0));
    assert_eq!(point_layout.fields[1].name, "Y");
    assert_eq!(point_layout.fields[1].offset, Some(4));
    assert_eq!(point_layout.size, Some(8));

    let wrapper_layout = layouts
        .get("Layouts::Wrapper")
        .and_then(|layout| match layout {
            TypeLayout::Class(layout) => Some(layout),
            _ => None,
        })
        .require("missing class layout for Wrapper");
    let user_fields: Vec<_> = wrapper_layout
        .fields
        .iter()
        .filter(|field| !field.name.starts_with('$'))
        .collect();
    assert_eq!(user_fields.len(), 2);
    assert_eq!(user_fields[0].name, "Inner");
    assert!(user_fields[0].offset.is_some());
    assert!(wrapper_layout.size.is_some());

    let shape_layout = layouts
        .get("Layouts::Shape")
        .and_then(|layout| match layout {
            TypeLayout::Enum(layout) => Some(layout),
            _ => None,
        })
        .require("missing enum layout for Shape");
    assert_eq!(shape_layout.variants.len(), 2);
    assert_eq!(shape_layout.variants[0].name, "Circle");
    assert_eq!(shape_layout.variants[1].name, "Rect");
    assert!(shape_layout.size.is_some());
    assert!(shape_layout.align.is_some());
}

#[test]
fn repr_and_align_attributes_adjust_layout() {
    let source = r"
namespace Layouts;

@repr(c)
@repr(packed(1))
public struct Packed
{
    public byte A;
    public int B;
}

@align(32)
public struct Aligned
{
    public long Value;
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let layouts = &lowering.module.type_layouts.types;

    let packed_layout = layouts
        .get("Layouts::Packed")
        .and_then(|layout| match layout {
            TypeLayout::Struct(layout) => Some(layout),
            _ => None,
        })
        .require("missing layout for Packed");
    assert_eq!(
        packed_layout.fields[0].ty.canonical_name(),
        "byte",
        "expected bool field to canonicalize to byte storage"
    );
    assert_eq!(
        packed_layout.fields[1].ty.canonical_name(),
        "int",
        "expected int field to remain canonical"
    );
    assert_eq!(packed_layout.repr, TypeRepr::C);
    assert_eq!(packed_layout.packing, Some(1));
    assert_eq!(packed_layout.align, Some(1));
    assert_eq!(packed_layout.size, Some(5), "layout {:?}", packed_layout);
    assert_eq!(packed_layout.fields[0].offset, Some(0));
    assert_eq!(packed_layout.fields[1].offset, Some(1));

    let aligned_layout = layouts
        .get("Layouts::Aligned")
        .and_then(|layout| match layout {
            TypeLayout::Struct(layout) => Some(layout),
            _ => None,
        })
        .require("missing layout for Aligned");
    assert_eq!(aligned_layout.align, Some(32));
    assert_eq!(aligned_layout.size, Some(32));
    assert_eq!(aligned_layout.packing, None);
}

#[test]
fn collects_layouts_for_tuple_types() {
    let source = r"
namespace Tuples;

public (int, int) Keep((int, int) values)
{
    return values;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let layouts = &lowering.module.type_layouts.types;
    let tuple_layout = layouts
        .get("(int, int)")
        .and_then(|layout| match layout {
            TypeLayout::Struct(layout) => Some(layout),
            _ => None,
        })
        .require("tuple layout missing");
    assert_eq!(
        tuple_layout
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["Item1", "Item2"],
        "expected positional tuple field names"
    );
    assert_eq!(
        tuple_layout
            .fields
            .iter()
            .map(|field| field.offset)
            .collect::<Vec<_>>(),
        [Some(0), Some(4)],
        "tuple field offsets should be sequential"
    );
    assert_eq!(
        tuple_layout.size,
        Some(8),
        "tuple of two ints should occupy eight bytes"
    );
}

#[test]
fn struct_layout_attribute_merges_pack_and_align_hints() {
    let source = r"
import Std.Runtime.InteropServices;

namespace Layouts;

@StructLayout(LayoutKind.Sequential, Pack=2, Align=2)
public struct Styled
{
    public byte A;
    public long B;
}
";
    let parsed = parse_module(source).require("parse struct with StructLayout attribute");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let layouts = &lowering.module.type_layouts.types;
    let styled_layout = layouts
        .get("Layouts::Styled")
        .and_then(|layout| match layout {
            TypeLayout::Struct(layout) => Some(layout),
            _ => None,
        })
        .require("missing layout for Styled");
    assert_eq!(
        styled_layout.repr,
        TypeRepr::C,
        "`@StructLayout(LayoutKind.Sequential, ..)` should request repr(C)"
    );
    assert_eq!(
        styled_layout.packing,
        Some(2),
        "`Pack=2` hint should clamp packing"
    );
    assert_eq!(
        styled_layout.align,
        Some(2),
        "`Align=2` hint should drive computed alignment"
    );
    assert!(
        matches!(styled_layout.fields[0].offset, Some(0)),
        "first field should start at offset 0"
    );
    assert!(
        styled_layout.fields[1].offset.unwrap_or_default() >= 2,
        "second field offset should respect packing"
    );
}

#[test]
fn tuple_literal_lowers_to_tuple_aggregate() {
    let source = r"
namespace Tuples;

public void Make()
{
    var pair = (1, 2);
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Make"))
        .require("missing function lowering");
    let aggregate = func
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .find_map(|stmt| match &stmt.kind {
            MirStatementKind::Assign {
                value:
                    Rvalue::Aggregate {
                        kind: crate::mir::AggregateKind::Tuple,
                        fields,
                    },
                ..
            } => Some(fields.len()),
            _ => None,
        })
        .require("tuple aggregate assignment missing");
    assert_eq!(aggregate, 2, "tuple literal should produce two fields");
}

#[test]
fn lowering_evaluates_enum_discriminants() {
    let source = r"
namespace Flags;

public enum ExitCode
{
    Ok = 0,
    Warning = 7,
    Fatal,
}

@flags
public enum Permissions
{
    None = 0,
    Read = 1,
    Write = Read << 1,
    Execute,
    All = Read | Write | Execute,
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let layouts = &lowering.module.type_layouts.types;

    let exit_layout = layouts
        .get("Flags::ExitCode")
        .and_then(|layout| match layout {
            TypeLayout::Enum(layout) => Some(layout),
            _ => None,
        })
        .require("missing layout for ExitCode");
    assert_eq!(exit_layout.variants.len(), 3);
    assert_eq!(exit_layout.variants[0].discriminant, 0);
    assert_eq!(exit_layout.variants[1].discriminant, 7);
    assert_eq!(exit_layout.variants[2].discriminant, 8);
    assert!(!exit_layout.is_flags);

    let perm_layout = layouts
        .get("Flags::Permissions")
        .and_then(|layout| match layout {
            TypeLayout::Enum(layout) => Some(layout),
            _ => None,
        })
        .require("missing layout for Permissions");
    assert!(perm_layout.is_flags, "expected flags metadata");
    let discriminants: Vec<i128> = perm_layout
        .variants
        .iter()
        .map(|variant| variant.discriminant)
        .collect();
    assert_eq!(discriminants, vec![0, 1, 2, 4, 7]);
}

#[test]
fn lowering_reports_duplicate_enum_discriminants() {
    let source = r"
namespace Diagnostics;

public enum Status
{
    Ok = 0,
    AlsoOk = 0,
    Pending,
}

@flags
public enum BrokenFlags
{
    None = 0,
    Read = 1,
    Write = 1,
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("reuses discriminant value 0")),
        "expected duplicate discriminant diagnostic, found {:?}",
        lowering.diagnostics
    );
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("reuses discriminant value 1")),
        "expected duplicate flag discriminant diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn flags_enum_reports_payload_negative_and_multi_bit_errors() {
    let source = r"
namespace Diagnostics;

@flags
public enum Broken
{
    None = 0,
    Composite = 3,
    Read = 1,
    Write = 2,
    Data { public int Value; },
    Negative = -1,
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("multiple undefined flag bits")),
        "expected multi-bit flag diagnostic, found {:?}",
        lowering.diagnostics
    );
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("may not declare payload fields")),
        "expected payload diagnostic, found {:?}",
        lowering.diagnostics
    );
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("must use non-negative discriminants")),
        "expected negative discriminant diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn enum_layout_respects_underlying_type() {
    let source = r"
namespace Underlying;

public enum Small : byte
{
    A,
    B = 5,
}

public enum Signed : short
{
    Min = -1,
    Zero,
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let layouts = &lowering.module.type_layouts.types;
    let small = layouts
        .get("Underlying::Small")
        .and_then(|layout| match layout {
            TypeLayout::Enum(layout) => Some(layout),
            _ => None,
        })
        .require("missing layout for Underlying::Small");
    assert_eq!(small.size, Some(1));
    assert_eq!(small.align, Some(1));
    let small_values: Vec<i128> = small
        .variants
        .iter()
        .map(|variant| variant.discriminant)
        .collect();
    assert_eq!(small_values, vec![0, 5]);

    let signed = layouts
        .get("Underlying::Signed")
        .and_then(|layout| match layout {
            TypeLayout::Enum(layout) => Some(layout),
            _ => None,
        })
        .require("missing layout for Underlying::Signed");
    let signed_values: Vec<i128> = signed
        .variants
        .iter()
        .map(|variant| variant.discriminant)
        .collect();
    assert_eq!(signed_values, vec![-1, 0]);
}

#[test]
fn enum_underlying_type_validation_reports_errors() {
    let source = r"
namespace Invalid;

public enum Weird : float
{
    A,
}

public enum Overflow : byte
{
    A = 255,
    B,
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("Only integral numeric types")),
        "expected invalid underlying type diagnostic, found {:?}",
        lowering.diagnostics
    );
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("does not fit underlying type")),
        "expected underlying range diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn flag_enum_uses_underlying_width_for_bits() {
    let source = r"
namespace Flags;

@flags
public enum Wide : ushort
{
    None = 0,
    A = 1,
    B = 2,
    Max = 1 << 15,
    TooLarge = 1 << 16,
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("declared underlying type")),
        "expected bit-width diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "integration-style MIR lowering test requires full control-flow fixture"
)]
fn registers_union_layout_and_modes() {
    let source = r"
namespace Graphics;

public union Pixel
{
public struct Rgba
{
    public byte R;
    public byte G;
    public byte B;
    public byte A;
}

public readonly Gray Gray;
public Channels Channels;
}

public struct Gray { public ushort Luma; }
public struct Channels { public byte Value; }
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let layout = lowering
        .module
        .type_layouts
        .types
        .get("Graphics::Pixel")
        .and_then(|layout| match layout {
            TypeLayout::Union(layout) => Some(layout),
            _ => None,
        })
        .require("union layout");

    assert_eq!(layout.views.len(), 3);

    let rgba = layout
        .views
        .iter()
        .find(|view| view.name == "Rgba")
        .require("rgba view");
    assert_eq!(rgba.mode, UnionFieldMode::Value);

    let gray = layout
        .views
        .iter()
        .find(|view| view.name == "Gray")
        .require("gray view");
    assert_eq!(gray.mode, UnionFieldMode::Readonly);

    let channels = layout
        .views
        .iter()
        .find(|view| view.name == "Channels")
        .require("channels view");
    assert_eq!(channels.mode, UnionFieldMode::Value);
}

#[expect(
    clippy::too_many_lines,
    reason = "integration-style MIR lowering test requires full control-flow fixture"
)]
#[test]
fn lowers_union_field_projection_to_union_elem() {
    let source = r"
namespace Graphics;

public union Pixel
{
public struct Gray
{
    public ushort Luma;
}
public Gray Gray;
}

public ushort Read(in Pixel pixel)
{
return pixel.Gray.Luma;
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Read"))
        .require("Read function");
    let entry = &func.body.blocks[0];

    let assign = entry
        .statements
        .iter()
        .find(|stmt| match &stmt.kind {
            MirStatementKind::Assign { place, .. } => place.local == LocalId(0),
            _ => false,
        })
        .require("return assignment");

    match &assign.kind {
        MirStatementKind::Assign {
            value: Rvalue::Use(Operand::Copy(place)),
            ..
        } => {
            assert!(
                place.projection.iter().any(|elem| matches!(
                    elem,
                    ProjectionElem::UnionField { name, .. } if name == "Gray"
                )),
                "expected UnionField projection for Gray view"
            );
        }
        other => panic!("expected copy assignment, found {other:?}"),
    }
}

#[test]
fn flags_private_type_usage_outside_declaring_scope() {
    let source = r"
namespace Access;

private struct Hidden { public int Value; }

public struct Wrapper { public Hidden Value; }
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("field `Value` references inaccessible type `Access::Hidden`")),
        "expected private access diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn lowers_methods_with_combined_visibility_modifiers() {
    let source = r"
namespace Access;

public class Service
{
protected internal int Compute(int value)
{
    return value;
}

private protected int Cache(int value)
{
    return value;
}
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn global_allocator_attribute_records_type() {
    let source = r#"
namespace Alloc;

@global_allocator
public struct Heap { }
"#;

    let parsed = parse_module(source).require("parse");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected lowering diagnostics: {:?}",
        lowering.diagnostics
    );
    let allocator = lowering
        .module
        .attributes
        .global_allocator
        .as_ref()
        .expect("global allocator missing");
    assert_eq!(allocator.type_name, "Alloc::Heap");
    assert!(allocator.target.is_none());
}
