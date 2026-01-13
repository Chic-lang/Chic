use super::common::RequireExt;
use super::*;

#[test]
fn class_new_invokes_runtime_allocator_and_passes_self() {
    let source = r#"
namespace Demo;

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

public Point Build()
{
    return new Point(1, 2);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let build_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Build"))
        .require("missing Build function");

    let mut alloc_local: Option<LocalId> = None;
    let mut ctor_args: Option<Vec<Operand>> = None;

    for block in &build_func.body.blocks {
        if let Some(Terminator::Call {
            func,
            destination,
            args,
            ..
        }) = &block.terminator
        {
            match func {
                Operand::Const(ConstOperand {
                    value: ConstValue::Symbol(name),
                    ..
                }) if name == "chic_rt_object_new" => {
                    let place = destination
                        .as_ref()
                        .expect("allocator should have a destination");
                    alloc_local = Some(place.local);
                }
                Operand::Const(ConstOperand {
                    value: ConstValue::Symbol(name),
                    ..
                }) if name.starts_with("Demo::Point::init") => {
                    ctor_args = Some(args.clone());
                }
                Operand::Pending(pending) if pending.repr == "Point" => {
                    ctor_args = Some(args.clone());
                }
                _ => {}
            }
        }
    }

    let ctor_args = ctor_args.expect("constructor call missing");
    let alloc_local = alloc_local.expect("runtime allocator call missing");
    assert!(
        ctor_args.len() >= 1,
        "constructor should receive at least the self argument"
    );
    match &ctor_args[0] {
        Operand::Copy(place) | Operand::Move(place) => {
            assert_eq!(
                place.local, alloc_local,
                "constructor self argument should reuse allocator result"
            );
        }
        Operand::Borrow(borrow) => {
            assert_eq!(
                borrow.place.local, alloc_local,
                "constructor self argument should reuse allocator result"
            );
            assert!(
                borrow.place.projection.is_empty(),
                "constructor self operand should not include projections"
            );
        }
        other => panic!("expected constructor self operand, got {other:?}"),
    }
}

#[test]
fn struct_new_uses_stack_storage_for_self_argument() {
    let source = r#"
namespace Demo;

public struct Pair
{
    public int X;
    public int Y;

    public init(int x, int y)
    {
        self.X = x;
        self.Y = y;
    }
}

public Pair Build()
{
    return new Pair(3, 4);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let build_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Build"))
        .require("missing Build function");

    // Ensure no runtime allocator call is emitted.
    for block in &build_func.body.blocks {
        if let Some(Terminator::Call { func, .. }) = &block.terminator {
            if let Operand::Const(ConstOperand {
                value: ConstValue::Symbol(name),
                ..
            }) = func
            {
                assert_ne!(
                    name, "chic_rt_object_new",
                    "struct `new` should not invoke the runtime allocator"
                );
            }
        }
    }

    let constructor_call = build_func
        .body
        .blocks
        .iter()
        .flat_map(|block| block.terminator.as_ref())
        .find(|term| {
            matches!(
                term,
                Terminator::Call {
                    func: Operand::Const(ConstOperand {
                        value: ConstValue::Symbol(name),
                        ..
                    }),
                    ..
                } if name.starts_with("Demo::Pair::init")
            ) || matches!(
                term,
                Terminator::Call {
                    func: Operand::Pending(pending),
                    ..
                } if pending.repr == "Pair"
            )
        })
        .expect("missing constructor call");

    if let Terminator::Call { args, .. } = constructor_call {
        assert!(
            args.len() >= 1,
            "constructor should receive the synthesized self operand"
        );
        match &args[0] {
            Operand::Copy(place) | Operand::Move(place) => {
                assert!(
                    place.projection.is_empty(),
                    "stack destination should not include projections"
                );
            }
            Operand::Borrow(borrow) => {
                assert!(
                    borrow.place.projection.is_empty(),
                    "stack destination should not include projections"
                );
            }
            other => panic!("expected stack self operand, found {other:?}"),
        }
    }
}

#[test]
fn object_initializer_emits_field_assignments_in_source_order() {
    let source = r#"
namespace Demo;

public class Point
{
    public int X;
    public int Y;
}

public class Factory
{
    public Point Build()
    {
        return new Point { X = 1, Y = 2 };
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let build_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Factory::Build"))
        .require("missing Factory::Build function");

    let mut assigned_fields = Vec::new();
    for block in &build_func.body.blocks {
        for statement in &block.statements {
            if let MirStatementKind::Assign { place, .. } = &statement.kind {
                if let Some(ProjectionElem::FieldNamed(name)) = place.projection.last() {
                    if !name.starts_with('$') {
                        assigned_fields.push(name.clone());
                    }
                }
            }
        }
    }

    assert_eq!(
        assigned_fields,
        ["X".to_string(), "Y".to_string()],
        "object initializer should assign fields in source order"
    );
}

#[test]
fn record_object_initializer_allows_readonly_fields() {
    let source = r#"
namespace Demo;

public record struct Point
{
    public int X;
    public int Y;
}

public class Factory
{
    public Point Build()
    {
        return new Point { X = 1, Y = 2 };
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let build_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Factory::Build"))
        .require("missing Factory::Build function");

    let assigned_fields: Vec<String> = build_func
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter_map(|statement| {
            if let MirStatementKind::Assign { place, .. } = &statement.kind {
                place.projection.last().and_then(|elem| match elem {
                    ProjectionElem::FieldNamed(name) if !name.starts_with('$') => {
                        Some(name.clone())
                    }
                    _ => None,
                })
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        assigned_fields,
        ["X".to_string(), "Y".to_string()],
        "record object initializer should assign readonly fields"
    );
}

#[test]
fn array_new_zero_inits_and_sets_length_after_elements() {
    let source = r#"
namespace Demo;

public int Build()
{
    var values = new int[3] { 1, 2, 3 };
    return values[0] + values[1] + values[2];
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let build_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Build"))
        .require("missing Build function");

    let mut zero_pos: Option<(usize, usize)> = None;
    let mut len_pos: Option<(usize, usize)> = None;
    let mut element_positions = Vec::new();

    for (block_idx, block) in build_func.body.blocks.iter().enumerate() {
        for (stmt_idx, stmt) in block.statements.iter().enumerate() {
            match &stmt.kind {
                MirStatementKind::ZeroInitRaw { .. } => {
                    zero_pos.get_or_insert((block_idx, stmt_idx));
                }
                MirStatementKind::Assign { place, .. } => {
                    if place.projection.iter().any(
                        |proj| matches!(proj, ProjectionElem::FieldNamed(name) if name == "len"),
                    ) {
                        len_pos.get_or_insert((block_idx, stmt_idx));
                    }
                    if place
                        .projection
                        .iter()
                        .any(|proj| matches!(proj, ProjectionElem::Index(_)))
                    {
                        element_positions.push((block_idx, stmt_idx));
                    }
                }
                _ => {}
            }
        }
    }

    let zero_pos = zero_pos.expect("array zero-initialisation missing");
    let len_pos = len_pos.expect("array length assignment missing");
    assert!(
        !element_positions.is_empty(),
        "expected element assignments in initializer"
    );

    let first_element = element_positions
        .iter()
        .min()
        .cloned()
        .expect("at least one element assignment");
    let last_element = element_positions
        .iter()
        .max()
        .cloned()
        .expect("at least one element assignment");

    let before = |a: (usize, usize), b: (usize, usize)| a.0 < b.0 || (a.0 == b.0 && a.1 < b.1);
    assert!(
        before(zero_pos, first_element),
        "zero-init should occur before element stores"
    );
    assert!(
        before(last_element, len_pos),
        "length assignment should occur after element stores"
    );
}

#[test]
fn collection_initializer_invokes_add_for_each_element() {
    let source = r#"
namespace Demo;

public class Bucket
{
    public int Count;

    public void Add(int value)
    {
        self.Count += value;
    }
}

public class Factory
{
    public Bucket Build(int first, int second)
    {
        return new Bucket { first, second };
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let build_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Factory::Build"))
        .require("missing Factory::Build function");

    let mut add_calls = 0usize;
    for block in &build_func.body.blocks {
        if let Some(Terminator::Call { func, .. }) = &block.terminator {
            match func {
                Operand::Const(ConstOperand {
                    value: ConstValue::Symbol(name),
                    ..
                }) if name.ends_with("::Bucket::Add") => {
                    add_calls += 1;
                }
                Operand::Pending(pending) if pending.repr.ends_with("Bucket::Add") => {
                    add_calls += 1;
                }
                _ => {}
            }
        }
    }

    assert_eq!(
        add_calls, 2,
        "collection initializer should call Add for each element"
    );
}
