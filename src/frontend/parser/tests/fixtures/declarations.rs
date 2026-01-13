use crate::frontend::ast::{Item, Module, StatementKind, UnionMember};

use super::{function_body, parse_ok};

pub(crate) const GEOMETRY_SOURCE: &str = r"
namespace Geometry;

public double Add(double x, double y)
{
    return x + y;
}

public struct Point
{
    public double X;
    public double Y;
}

public enum Shape
{
    Circle { public double Radius; },
    Rect { public double Width; public double Height; }
}
";

pub(crate) const PIXEL_SOURCE: &str = r"
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
public struct Channels { public int Value; }
";

#[must_use]
pub(crate) fn parse_geometry_module() -> Module {
    let parse = parse_ok(GEOMETRY_SOURCE);
    assert!(
        parse.diagnostics.is_empty(),
        "expected no diagnostics: {:?}",
        parse.diagnostics
    );
    parse.module
}

pub(crate) fn assert_geometry_namespace(module: &Module) {
    assert_eq!(module.namespace.as_deref(), Some("Geometry"));
    assert_eq!(module.items.len(), 3);
}

pub(crate) fn assert_add_function(item: &Item) {
    let Item::Function(func) = item else {
        panic!("expected Item::Function, found {item:?}");
    };

    assert_eq!(func.name, "Add");
    assert_eq!(func.signature.parameters.len(), 2);
    assert_eq!(func.signature.parameters[0].name, "x");
    assert_eq!(func.signature.parameters[0].ty.name, "double");

    let body = function_body(func);
    assert_eq!(body.statements.len(), 1);
    let StatementKind::Return {
        expression: Some(expr),
    } = &body.statements[0].kind
    else {
        panic!(
            "expected return statement, found {:?}",
            body.statements[0].kind
        );
    };
    assert_eq!(expr.text.trim(), "x + y");
}

pub(crate) fn assert_point_struct(item: &Item) {
    let Item::Struct(def) = item else {
        panic!("expected Item::Struct, found {item:?}");
    };
    assert_eq!(def.name, "Point");
    assert_eq!(def.fields.len(), 2);
}

pub(crate) fn assert_pixel_union(module: &Module) {
    assert!(matches!(module.items[0], Item::Union(_)));
    let union = match &module.items[0] {
        Item::Union(u) => u,
        other => panic!("expected union, found {other:?}"),
    };
    assert_eq!(union.name, "Pixel");
    assert_eq!(union.members.len(), 3);

    let UnionMember::View(view) = &union.members[0] else {
        panic!("expected union view, found {:?}", union.members[0]);
    };
    assert_eq!(view.name, "Rgba");
    assert_eq!(view.fields.len(), 4);
    assert!(!view.is_readonly);

    let UnionMember::Field(gray) = &union.members[1] else {
        panic!("expected union field, found {:?}", union.members[1]);
    };
    assert_eq!(gray.name, "Gray");
    assert!(gray.is_readonly);

    let UnionMember::Field(channels) = &union.members[2] else {
        panic!("expected union field, found {:?}", union.members[2]);
    };
    assert_eq!(channels.name, "Channels");
    assert!(!channels.is_readonly);
}
