use super::fixtures::{needs_ctor_class, parameterless_ctor, simple_class, usage_class};
use super::{ArenaDiagnosticCase, ArenaDiagnosticFixture, Expectation, run_cases};
use crate::frontend::ast::{
    GenericConstraint, GenericConstraintKind, GenericParam, GenericParams, InterfaceDecl, Item,
    Module, TypeExpr, Visibility,
};
use crate::frontend::parser::parse_module;
use crate::mir::TypeLayoutTable;
use crate::typeck::arena::TypeCheckResult;

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::parsed(
        "struct_constraint_rejects_reference_types",
        r#"
namespace Demo;

public interface IMarker { }

public struct Wrapper<T>
    where T : struct
{
}

public struct Usage
{
    public Wrapper<string> Field;
}
"#,
        Expectation::contains(&["must be a value type due to `struct` constraint"]),
    ),
    ArenaDiagnosticCase::parsed(
        "class_constraint_rejects_value_types",
        r#"
namespace Demo;

public class Wrapper<T>
    where T : class
{
}

public class Usage
{
    public Wrapper<int> Field;
}
"#,
        Expectation::contains(&["must be a reference type due to `class` constraint"]),
    ),
    ArenaDiagnosticCase::parsed(
        "notnull_constraint_rejects_nullable_arguments",
        r#"
namespace Demo;

public class Wrapper<T>
    where T : notnull
{
}

public class Usage
{
    public Wrapper<string?> Field;
}
"#,
        Expectation::contains(&["cannot be nullable because of `notnull` constraint"]),
    ),
    ArenaDiagnosticCase::parsed(
        "covariant_parameter_cannot_appear_in_input_position",
        r#"
namespace Demo;

public interface IProducer<out T>
{
    public T Produce();
    public void Consume(T value);
}
"#,
        Expectation::contains(&["declares `T` as covariant (`out`)"]),
    ),
    ArenaDiagnosticCase::parsed(
        "contravariant_parameter_cannot_appear_in_output_position",
        r#"
namespace Demo;

public interface IConsumer<in T>
{
    public void Consume(T value);
    public T Convert();
}
"#,
        Expectation::contains(&["declares `T` as contravariant (`in`)"]),
    ),
    ArenaDiagnosticCase::custom(
        "conflicting_struct_and_class_constraints_report_error",
        conflicting_struct_and_class_constraints_report_error,
        Expectation::contains(&["cannot combine `struct` and `class`"]),
    ),
    ArenaDiagnosticCase::custom(
        "new_constraint_conflicts_with_struct_constraint",
        new_constraint_conflicts_with_struct_constraint,
        Expectation::contains(&["cannot combine `new()` with `struct`"]),
    ),
    ArenaDiagnosticCase::custom(
        "new_constraint_requires_public_default_constructor",
        new_constraint_requires_public_default_constructor,
        Expectation::contains(&["must provide a public parameterless constructor"]),
    ),
    ArenaDiagnosticCase::custom(
        "new_constraint_rejects_non_public_constructor",
        new_constraint_rejects_non_public_constructor,
        Expectation::contains(&["Hidden", "public parameterless constructor"]),
    ),
    ArenaDiagnosticCase::custom(
        "interface_constraint_requires_implementation",
        interface_constraint_requires_implementation,
        Expectation::contains(&["TCK022"]),
    ),
];

fn conflicting_struct_and_class_constraints_report_error(
    fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let mut param = GenericParam::type_param("T", None);
    {
        let constraints = &mut param.as_type_mut().expect("type parameter").constraints;
        constraints.push(GenericConstraint::new(GenericConstraintKind::Struct, None));
        constraints.push(GenericConstraint::new(GenericConstraintKind::Class, None));
    }

    let module = Module::with_namespace_items(
        Some("Demo".into()),
        None,
        Vec::new(),
        Vec::new(),
        vec![Item::Class(simple_class(
            "Conflict",
            Vec::new(),
            Some(GenericParams {
                span: None,
                params: vec![param],
            }),
        ))],
    );

    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

fn new_constraint_conflicts_with_struct_constraint(
    fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let mut param = GenericParam::type_param("T", None);
    {
        let constraints = &mut param.as_type_mut().expect("type parameter").constraints;
        constraints.push(GenericConstraint::new(GenericConstraintKind::Struct, None));
        constraints.push(GenericConstraint::new(
            GenericConstraintKind::DefaultConstructor,
            None,
        ));
    }

    let module = Module::with_namespace_items(
        Some("Demo".into()),
        None,
        Vec::new(),
        Vec::new(),
        vec![Item::Class(simple_class(
            "Conflict",
            Vec::new(),
            Some(GenericParams {
                span: None,
                params: vec![param],
            }),
        ))],
    );

    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

fn new_constraint_requires_public_default_constructor(
    fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let needs_ctor = needs_ctor_class();
    let usage = usage_class(TypeExpr::simple("IMissing"));
    let module = Module::with_namespace_items(
        Some("Demo".into()),
        None,
        Vec::new(),
        Vec::new(),
        vec![
            Item::Interface(InterfaceDecl {
                visibility: Visibility::Public,
                name: "IMissing".into(),
                bases: Vec::new(),
                members: Vec::new(),
                thread_safe_override: None,
                shareable_override: None,
                copy_override: None,
                doc: None,
                attributes: Vec::new(),
                generics: None,
            }),
            Item::Class(needs_ctor),
            Item::Class(usage),
        ],
    );

    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

fn new_constraint_rejects_non_public_constructor(
    fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let module = Module::with_namespace_items(
        Some("Demo".into()),
        None,
        Vec::new(),
        Vec::new(),
        vec![
            Item::Class(needs_ctor_class()),
            Item::Class(simple_class(
                "Hidden",
                vec![parameterless_ctor(Visibility::Private)],
                None,
            )),
            Item::Class(usage_class(TypeExpr::simple("Hidden"))),
        ],
    );

    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

fn interface_constraint_requires_implementation(
    fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let parsed = parse_module(
        r#"
namespace Demo;

public interface IFoo { void Do(); }

public class Bar { }

public class NeedsFoo<T>
    where T : IFoo
{
    public void Use(T value) { value.Do(); }
}

public class Usage
{
    public NeedsFoo<Bar> Field;
}
"#,
    )
    .expect("parse module");
    fixture.check_module(&parsed.module, &[], &TypeLayoutTable::default())
}

#[test]
fn constraint_diagnostics() {
    run_cases("constraints", CASES);
}
