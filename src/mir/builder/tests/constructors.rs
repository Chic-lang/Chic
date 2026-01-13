use super::common::RequireExt;
use super::*;
use crate::mir::ConstOperand;

#[test]
fn reports_uninitialised_fields_in_constructor() {
    let source = r#"
namespace Geometry;

public class Point
{
    public int X;
    public int Y;

    public init(int x)
    {
        self.X = x;
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(lowering.diagnostics.is_empty());

    let ctor = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.contains("Point::init"))
        .expect("constructor not lowered");
    assert!(matches!(ctor.kind, FunctionKind::Constructor));
}

#[test]
fn accepts_constructor_initialising_all_fields() {
    let source = r#"
namespace Geometry;

public class Point
{
    public int X;
    public int Y;

    public init(int x, int y)
    {
        self.X = x;
        self.Y = y;
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(lowering.diagnostics.is_empty());

    let ctor = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.contains("Point::init"))
        .expect("constructor not lowered");
    assert!(matches!(ctor.kind, FunctionKind::Constructor));
}

#[test]
fn convenience_constructor_can_delegate() {
    let source = r#"
namespace Geometry;

public class Point
{
    public int X;
    public int Y;

    public init(int x, int y)
    {
        self.X = x;
        self.Y = y;
    }

    public convenience init() : self(0, 0)
    {
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(lowering.diagnostics.is_empty());

    let constructors: Vec<_> = lowering
        .module
        .functions
        .iter()
        .filter(|func| matches!(func.kind, FunctionKind::Constructor))
        .collect();
    assert_eq!(constructors.len(), 2);
}

#[test]
fn reports_convenience_cycle_without_designated() {
    let source = r#"
namespace Geometry;

public class Widget
{
    public convenience init() : self() { }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| { diag.message.contains("requires a designated init") })
    );
}

#[test]
fn convenience_initializer_emits_delegate_call() {
    let source = r#"
namespace Geometry;

public class Point
{
    public int X;
    public int Y;

    public init(int x, int y)
    {
        self.X = x;
        self.Y = y;
    }

    public convenience init() : self(0, 1)
    {
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(lowering.diagnostics.is_empty());

    let ctor = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("Point::init#1"))
        .expect("delegate constructor missing");

    let entry = ctor
        .body
        .blocks
        .first()
        .expect("constructor entry block missing");
    let Terminator::Call {
        func,
        args,
        destination,
        arg_modes: _,
        ..
    } = entry
        .terminator
        .as_ref()
        .expect("constructor should start with delegation call")
    else {
        panic!("expected delegation call terminator");
    };
    assert!(destination.is_none());

    let pending = match func {
        Operand::Pending(pending) => pending,
        other => panic!("expected pending operand for initializer call, got {other:?}"),
    };
    assert_eq!(pending.repr, "Geometry::Point::init#self");

    assert_eq!(args.len(), 3);
    let self_index = ctor
        .body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| decl.name.as_deref() == Some("self"))
        .map(|(idx, _)| idx)
        .expect("self local missing");
    match &args[0] {
        Operand::Copy(place) | Operand::Move(place) => {
            assert_eq!(place.local.0, self_index);
        }
        other => panic!("expected self operand, got {other:?}"),
    }
    assert!(matches!(
        args[1],
        Operand::Const(ConstOperand {
            value: ConstValue::Int(0),
            ..
        })
    ));
    assert!(matches!(
        args[2],
        Operand::Const(ConstOperand {
            value: ConstValue::Int(1),
            ..
        })
    ));
}

#[test]
fn designated_initializer_emits_super_call() {
    let source = r#"
namespace Geometry;

public class Base
{
    public int Value;

    public init(int value)
    {
        self.Value = value;
    }
}

public class Derived : Base
{
    public int Extra;

    public init(int value) : super(value)
    {
        self.Extra = value;
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(lowering.diagnostics.is_empty());

    let ctor = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("Derived::init#0"))
        .expect("derived constructor missing");

    let entry = ctor
        .body
        .blocks
        .first()
        .expect("constructor entry block missing");
    let Terminator::Call {
        func,
        args,
        destination,
        arg_modes: _,
        ..
    } = entry
        .terminator
        .as_ref()
        .expect("constructor should call super")
    else {
        panic!("expected super call terminator");
    };
    assert!(destination.is_none());

    let pending = match func {
        Operand::Pending(pending) => pending,
        other => panic!("expected pending operand for super call, got {other:?}"),
    };
    assert_eq!(pending.repr, "Geometry::Derived::init#super");

    assert_eq!(args.len(), 2);
    let self_index = ctor
        .body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| decl.name.as_deref() == Some("self"))
        .map(|(idx, _)| idx)
        .expect("self local missing");
    match &args[0] {
        Operand::Copy(place) | Operand::Move(place) => {
            assert_eq!(place.local.0, self_index);
        }
        other => panic!("expected self operand, got {other:?}"),
    }

    let value_index = ctor
        .body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| decl.name.as_deref() == Some("value"))
        .map(|(idx, _)| idx)
        .expect("value parameter missing");
    match &args[1] {
        Operand::Copy(place) | Operand::Move(place) => {
            assert_eq!(place.local.0, value_index);
        }
        other => panic!("expected value argument, got {other:?}"),
    }
}
