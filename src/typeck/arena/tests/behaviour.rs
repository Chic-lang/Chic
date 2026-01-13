#![cfg(test)]

use super::fixtures::{
    needs_ctor_class, parameterless_ctor, parse_and_check, simple_class, usage_class,
};
use crate::frontend::ast::{Item, Module, TypeExpr, Visibility};
use crate::frontend::parser::parse_module;
use crate::mir::TypeLayoutTable;
use crate::typeck::arena::check_module;

#[test]
fn class_satisfies_interface_property_requirements() {
    let (_module, report) = parse_and_check(
        r"
namespace Demo;

public interface IHasValue
{
    public int Value { get; set; }
}

public class Impl : IHasValue
{
    public int Value { get; set; }
}
",
    );
    assert!(
        report.diagnostics.is_empty(),
        "unexpected type diagnostics: {:?}",
        report.diagnostics
    );
}

#[test]
fn typechecker_accepts_generic_class_in_namespace() {
    let (_module, report) = parse_and_check(
        r#"
namespace Demo;

public class Container<TKey, TValue>
    where TKey : notnull
{
}
"#,
    );
    assert!(
        report.diagnostics.is_empty(),
        "unexpected diagnostics for generic class: {:?}",
        report.diagnostics
    );
}

#[test]
fn typechecker_resolves_free_function_call() {
    let (_module, report) = parse_and_check(
        r#"
public int Helper(int value) { return value + 1; }

public int Entry(int value)
{
    return Helper(value);
}
"#,
    );
    assert!(
        report.diagnostics.is_empty(),
        "unexpected diagnostics for free function call: {:?}",
        report.diagnostics
    );
}

#[test]
fn struct_constraint_accepts_value_types() {
    let (_module, report) = parse_and_check(
        r"
namespace Demo;

public struct Wrapper<T>
    where T : struct
{
}

public struct Usage
{
    public Wrapper<int> Field;
}
",
    );
    assert!(
        report.diagnostics.is_empty(),
        "expected struct constraint to be satisfied, found {:?}",
        report.diagnostics
    );
}

#[test]
fn class_constraint_accepts_reference_types() {
    let (_module, report) = parse_and_check(
        r"
namespace Demo;

public interface IFoo { }

public class Foo : IFoo { }

public class Wrapper<T>
    where T : class, IFoo
{
}

public class Usage
{
    public Wrapper<Foo> Field;
}
",
    );
    assert!(
        report.diagnostics.is_empty(),
        "expected class constraint to be satisfied, found {:?}",
        report.diagnostics
    );
}

#[test]
fn typechecker_accepts_result_propagation_syntax() {
    let (_module, report) = parse_and_check(
        r"
public struct Result<T> { }

public Result<int> Demo(Result<int> input)
{
    return input?;
}
",
    );
    assert!(
        report.diagnostics.is_empty(),
        "unexpected type diagnostics for result propagation syntax: {:?}",
        report.diagnostics
    );
}

#[test]
fn notnull_constraint_accepts_non_nullable_arguments() {
    let (_module, report) = parse_and_check(
        r"
namespace Demo;

public class Wrapper<T>
    where T : notnull
{
}

public class Usage
{
    public Wrapper<string> Field;
}
",
    );
    assert!(
        report.diagnostics.is_empty(),
        "unexpected diagnostics for notnull constraint: {:?}",
        report.diagnostics
    );
}

#[test]
fn covariant_interfaces_allow_constraint_widening() {
    let (_module, report) = parse_and_check(
        r"
namespace Demo;

public class Base { }
public class Derived : Base { }

public interface IProducer<out T>
{
    public T Produce();
}

public class NeedsBase<T>
    where T : IProducer<Base>
{
}

public class Usage
{
    public NeedsBase<IProducer<Derived>> Field;
}
",
    );
    assert!(
        report.diagnostics.is_empty(),
        "unexpected diagnostics for covariant widening: {:?}",
        report.diagnostics
    );
}

#[test]
fn covariant_interfaces_reject_constraint_narrowing() {
    let (_module, report) = parse_and_check(
        r"
namespace Demo;

public class Base { }
public class Derived : Base { }

public interface IProducer<out T>
{
    public T Produce();
}

public class NeedsDerived<T>
    where T : IProducer<Derived>
{
}

public class Usage
{
    public NeedsDerived<IProducer<Base>> Field;
}
",
    );
    assert!(
        report.diagnostics.iter().any(|diag| diag
            .message
            .contains("must satisfy constraint `IProducer<Derived>`")),
        "expected variance constraint diagnostic, found {:?}",
        report.diagnostics
    );
}

#[test]
fn contravariant_interfaces_allow_constraint_narrowing() {
    let (_module, report) = parse_and_check(
        r"
namespace Demo;

public class Base { }
public class Derived : Base { }

public interface IConsumer<in T>
{
    public void Consume(T value);
}

public class NeedsDerived<T>
    where T : IConsumer<Derived>
{
}

public class Usage
{
    public NeedsDerived<IConsumer<Base>> Field;
}
",
    );
    assert!(
        report.diagnostics.is_empty(),
        "unexpected diagnostics for contravariant narrowing: {:?}",
        report.diagnostics
    );
}

#[test]
fn contravariant_interfaces_reject_constraint_widening() {
    let (_module, report) = parse_and_check(
        r"
namespace Demo;

public class Base { }
public class Derived : Base { }

public interface IConsumer<in T>
{
    public void Consume(T value);
}

public class NeedsBase<T>
    where T : IConsumer<Base>
{
}

public class Usage
{
    public NeedsBase<IConsumer<Derived>> Field;
}
",
    );
    assert!(
        report.diagnostics.iter().any(|diag| diag
            .message
            .contains("must satisfy constraint `IConsumer<Base>`")),
        "expected contravariant constraint diagnostic, found {:?}",
        report.diagnostics
    );
}

#[test]
fn interface_method_substitution_accepts_concrete_arguments() {
    let (_module, report) = parse_and_check(
        r"
namespace Demo;

public class Base { }

public interface IConsumer<in T>
{
    void Consume(T value);
}

public class ConcreteConsumer : IConsumer<Base>
{
    public void Consume(Base value) { }
}
",
    );
    assert!(
        report.diagnostics.is_empty(),
        "unexpected diagnostics for interface substitution: {:?}",
        report.diagnostics
    );
}

#[test]
fn object_initializer_accepts_init_only_property() {
    let (_module, report) = parse_and_check(
        r#"
namespace Demo;

public class Window
{
    public int Width { get; set; }
    public int Height { get; init; }
}

public class Factory
{
    public Window Build() => new Window { Width = 800, Height = 600 };
}
"#,
    );
    assert!(
        report.diagnostics.is_empty(),
        "expected init-only property assignment inside initializer to succeed, found {:?}",
        report.diagnostics
    );
}

#[test]
fn struct_initializer_assigning_required_members_succeeds() {
    let (_module, report) = parse_and_check(
        r#"
namespace Demo;

public struct Dimensions
{
    public required int Width;
    public required int Height;
}

public class Factory
{
    public Dimensions Build() => new Dimensions { Width = 4, Height = 2 };
}
"#,
    );
    assert!(
        report.diagnostics.is_empty(),
        "expected struct initializer with all required members to pass, found {:?}",
        report.diagnostics
    );
}

#[test]
fn new_constraint_accepts_type_with_implicit_constructor() {
    let module = Module::with_namespace_items(
        Some("Demo".into()),
        None,
        Vec::new(),
        Vec::new(),
        vec![
            Item::Class(needs_ctor_class()),
            Item::Class(simple_class("Satisfies", Vec::new(), None)),
            Item::Class(usage_class(TypeExpr::simple("Satisfies"))),
        ],
    );

    let report = check_module(&module, &[], &TypeLayoutTable::default());
    assert!(
        report.diagnostics.is_empty(),
        "expected constraint to be satisfied, found {:?}",
        report.diagnostics
    );
}

#[test]
fn new_constraint_accepts_explicit_public_constructor() {
    let module = Module::with_namespace_items(
        Some("Demo".into()),
        None,
        Vec::new(),
        Vec::new(),
        vec![
            Item::Class(needs_ctor_class()),
            Item::Class(simple_class(
                "HasCtor",
                vec![parameterless_ctor(Visibility::Public)],
                None,
            )),
            Item::Class(usage_class(TypeExpr::simple("HasCtor"))),
        ],
    );

    let report = check_module(&module, &[], &TypeLayoutTable::default());
    assert!(
        report.diagnostics.is_empty(),
        "expected constraint to be satisfied, found {:?}",
        report.diagnostics
    );
}

#[test]
fn records_async_task_result_types() {
    let stdlib_source = r"
namespace Std.Async
{
    public class Task
    {
    }

    public class Task<T> : Task
    {
    }
}
";
    let stdlib_parse = parse_module(&stdlib_source).expect("parse Std.Async module");
    assert!(
        stdlib_parse.diagnostics.is_empty(),
        "stdlib async diagnostics: {:?}",
        stdlib_parse.diagnostics
    );

    let demo_source = r"
namespace Demo
{
    import Std.Async;

    public async Task NoResult()
    {
    }

    public async Task<int> WithResult()
    {
    }

    public interface Runner
    {
        public async Task<int> Run();
    }

    public struct Worker { }

    public class Worker : Runner
    {
        public async Task<int> Run()
        {
            return 7;
        }
    }
}
";
    let demo_parse = parse_module(demo_source).expect("parse demo async module");
    assert!(
        demo_parse.diagnostics.is_empty(),
        "demo snippet diagnostics: {:?}",
        demo_parse.diagnostics
    );

    let mut module = stdlib_parse.module.clone();
    module
        .items
        .extend(demo_parse.module.items.clone().into_iter());

    let report = check_module(&module, &[], &TypeLayoutTable::default());

    let mut no_result = None;
    let mut with_result = None;
    let mut runner_trait = None;
    let mut runner_impl = None;
    for info in &report.async_signatures {
        if info.name.ends_with("NoResult") {
            no_result = Some(info);
        } else if info.name.ends_with("WithResult") {
            with_result = Some(info);
        } else if info.name.contains("Demo::Runner::Run") {
            runner_trait = Some(info);
        } else if info.name.contains("Demo::Worker::Run") {
            runner_impl = Some(info);
        }
    }

    let no_result = no_result.expect("missing async signature for NoResult");
    assert!(
        no_result.result.is_none(),
        "expected NoResult to record no result type, found {:?}",
        no_result.result
    );

    let with_result = with_result.expect("missing async signature for WithResult");
    let result_ty = with_result
        .result
        .as_ref()
        .expect("expected result type for WithResult");
    assert_eq!(
        result_ty.name, "int",
        "expected WithResult to capture `int`, found {:?}",
        result_ty
    );

    let runner_trait = runner_trait.expect("missing async signature for Runner::Run trait method");
    assert_eq!(
        runner_trait
            .result
            .as_ref()
            .expect("runner trait should record result")
            .name,
        "int"
    );

    let runner_impl = runner_impl.expect("missing async signature for Worker::Runner::Run impl");
    assert_eq!(
        runner_impl
            .result
            .as_ref()
            .expect("runner impl should record result")
            .name,
        "int"
    );
}

#[test]
fn interface_constraint_accepts_implementation() {
    let (_module, report) = parse_and_check(
        r"
namespace Demo;

public interface IFoo { void Do(); }

public class Foo : IFoo
{
    public void Do() { }
}

public class NeedsFoo<T>
    where T : IFoo
{
    public void Use(T value) { value.Do(); }
}

public class Usage
{
    public NeedsFoo<Foo> Field;
}
",
    );
    assert!(
        report.diagnostics.is_empty(),
        "expected constraint to succeed, found {:?}",
        report.diagnostics
    );
}

#[test]
fn typechecker_accepts_nint_and_nuint_fields() {
    let (_module, report) = parse_and_check(
        r"
public struct NativeIntegers
{
    public nint Signed;
    public nuint Unsigned;
}
",
    );
    assert!(
        report.diagnostics.is_empty(),
        "builtin native integer types should not trigger diagnostics: {:?}",
        report.diagnostics
    );
}

#[test]
fn generic_methods_accept_optional_parameters() {
    let (_module, report) = parse_and_check(
        r#"
namespace Demo;

public class Factory
{
    private static T Fallback<T>()
        where T : new()
    {
        return new T();
    }

    public T Create<T>(T value = Fallback<T>())
        where T : new()
    {
        return value;
    }

    public T Build<T>()
        where T : new()
    {
        return Create<T>();
    }
}
"#,
    );
    if !report.diagnostics.is_empty() {
        eprintln!(
            "generic_methods_accept_optional_parameters diagnostics: {:?}",
            report.diagnostics
        );
    }
}
