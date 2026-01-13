use crate::frontend::ast::{
    AutoTraitConstraint, GenericConstraintKind, Item, Variance, Visibility,
};
use crate::frontend::diagnostics::Severity;
use crate::frontend::parser::parse_module;
use crate::frontend::parser::tests::fixtures::*;

#[test]
fn parses_where_clause_with_multiple_constraints() {
    let source = r"
public interface IFoo { }

public class Wrapper<T, U>
    where T : struct, IFoo, new()
    where U : class, IFoo
{
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Class(class) = &parse.module.items[1] else {
        panic!("expected class wrapper, found {:?}", parse.module.items[1]);
    };

    let generics = class
        .generics
        .as_ref()
        .expect("expected wrapper to record generics");
    assert_eq!(generics.params.len(), 2);

    let t_param = &generics.params[0];
    assert_eq!(t_param.name, "T");
    let t_constraints = &t_param
        .as_type()
        .expect("T should be a type parameter")
        .constraints;
    assert!(
        t_constraints
            .iter()
            .any(|c| matches!(c.kind, GenericConstraintKind::Struct))
    );
    assert!(t_constraints.iter().any(|c| {
        matches!(
            &c.kind,
            GenericConstraintKind::Type(ty) if ty.name == "IFoo"
        )
    }));
    assert!(
        t_constraints
            .iter()
            .any(|c| { matches!(c.kind, GenericConstraintKind::DefaultConstructor) })
    );

    let u_param = &generics.params[1];
    assert_eq!(u_param.name, "U");
    let u_constraints = &u_param
        .as_type()
        .expect("U should be a type parameter")
        .constraints;
    assert!(
        u_constraints
            .iter()
            .any(|c| matches!(c.kind, GenericConstraintKind::Class))
    );
    assert!(u_constraints.iter().any(|c| {
        matches!(
            &c.kind,
            GenericConstraintKind::Type(ty) if ty.name == "IFoo"
        )
    }));
}

#[test]
fn parses_const_generic_parameters_and_predicates() {
    let source = r"
public class Buffer<const N: int, const M: int>
    where N : const(N > 0), const(N < 1024)
    where M : const(M % 2 == 0)
{
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Class(class) = &parse.module.items[0] else {
        panic!("expected class item, found {:?}", parse.module.items[0]);
    };

    let generics = class.generics.as_ref().expect("expected generics");
    assert_eq!(generics.params.len(), 2);

    let n_param = &generics.params[0];
    assert_eq!(n_param.name, "N");
    let n_data = n_param
        .as_const()
        .expect("N should be recorded as const parameter");
    assert_eq!(n_data.ty.name, "int");
    assert_eq!(n_data.constraints.len(), 2);
    assert_eq!(n_data.constraints[0].expr.text, "N > 0");
    assert_eq!(n_data.constraints[1].expr.text, "N < 1024");

    let m_param = &generics.params[1];
    assert_eq!(m_param.name, "M");
    let m_data = m_param
        .as_const()
        .expect("M should be recorded as const parameter");
    assert_eq!(m_data.ty.name, "int");
    assert_eq!(m_data.constraints.len(), 1);
    assert_eq!(m_data.constraints[0].expr.text, "M % 2 == 0");
}

#[test]
fn reports_const_predicate_on_type_parameter() {
    let source = r"
public class Bad<T>
    where T : const(T > 0)
{
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| { diag.message.contains("is not a const generic parameter") }),
        "expected diagnostic about const predicate on type parameter, got {:?}",
        diagnostics
    );
}

#[test]
fn parses_generic_class_with_attributes_and_visibility() {
    let source = r"
namespace Sample;

@data
internal class Repository<TKey, TValue>
    where TKey : notnull
    where TValue : class
{
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    assert_eq!(
        parse.module.namespace.as_deref(),
        Some("Sample"),
        "file-scoped namespace should be recorded"
    );
    let Item::Class(class) = &parse.module.items[0] else {
        panic!(
            "expected class item at module scope, found {:?}",
            parse.module.items[0]
        );
    };

    assert_eq!(class.visibility, Visibility::Internal);
    assert_eq!(
        class.attributes.len(),
        1,
        "expected attribute to be recorded"
    );

    let generics = class
        .generics
        .as_ref()
        .expect("expected Repository to record generics");
    assert_eq!(generics.params.len(), 2);
    assert!(
        generics.span.is_some(),
        "expected generic parameter span to be recorded"
    );

    let key_param = &generics.params[0];
    assert_eq!(key_param.name, "TKey");
    assert!(
        key_param
            .as_type()
            .expect("TKey is a type parameter")
            .constraints
            .iter()
            .any(|c| matches!(c.kind, GenericConstraintKind::NotNull)),
        "expected `notnull` constraint on TKey"
    );

    let value_param = &generics.params[1];
    assert_eq!(value_param.name, "TValue");
    assert!(
        value_param
            .as_type()
            .expect("TValue is a type parameter")
            .constraints
            .iter()
            .any(|c| matches!(c.kind, GenericConstraintKind::Class)),
        "expected `class` constraint on TValue"
    );
}

#[test]
fn parses_generic_class_inside_nested_namespaces() {
    let source = r"
namespace Demo
{
    namespace Storage
    {
        public class Box<T>
        {
        }
    }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let outer = match &parse.module.items[0] {
        Item::Namespace(ns) => ns,
        other => panic!("expected namespace item, found {other:?}"),
    };
    let inner = match &outer.items[0] {
        Item::Namespace(ns) => ns,
        other => panic!("expected nested namespace, found {other:?}"),
    };
    let Item::Class(class) = &inner.items[0] else {
        panic!("expected class item, found {:?}", inner.items[0]);
    };
    assert!(
        class.generics.is_some(),
        "expected Box to record generic parameters"
    );
}

#[test]
fn parses_method_level_generic_constraints() {
    let source = r"
public class Processor
{
    public void Handle<TItem, TAllocator>(ref Span<TItem> span)
        where TItem : struct, IEquatable<TItem>
        where TAllocator : IAllocator, new()
    {
    }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Class(class) = &parse.module.items[0] else {
        panic!("expected class item, found {:?}", parse.module.items[0]);
    };
    let method = class
        .members
        .iter()
        .find_map(|member| match member {
            crate::frontend::ast::ClassMember::Method(func) => Some(func),
            _ => None,
        })
        .expect("expected to locate Handle method");
    let generics = method
        .generics
        .as_ref()
        .expect("method should record generic parameters");
    assert_eq!(generics.params.len(), 2);
    assert!(
        generics.params[0]
            .as_type()
            .expect("Handle generics should be type parameters")
            .constraints
            .iter()
            .any(|c| matches!(c.kind, GenericConstraintKind::Struct)),
        "expected method to retain struct constraint"
    );
}

#[test]
fn reports_where_clause_with_unknown_parameter() {
    let source = r"
public interface IFoo { }

public class Wrapper<T>
    where U : IFoo
{
}
";
    let err = parse_module(source).expect_err("expected parse failure");
    assert!(
        err.diagnostics().iter().any(|diag| {
            diag.message
                .contains("constraint references unknown type parameter `U`")
                && diag.severity == Severity::Error
        }),
        "expected diagnostic about unknown type parameter, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn parses_variance_on_interface_generics() {
    let source = r"
public interface ITransformer<in TInput, out TOutput>
{
    TOutput Convert(TInput value);
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Interface(iface) = &parse.module.items[0] else {
        panic!("expected interface item, found {:?}", parse.module.items[0]);
    };
    let generics = iface
        .generics
        .as_ref()
        .expect("expected interface generics to be recorded");
    assert_eq!(generics.params.len(), 2);

    let input_param = generics.params[0]
        .as_type()
        .expect("TInput should be a type parameter");
    assert_eq!(
        input_param.variance,
        Variance::Contravariant,
        "expected `in` parameter to record contravariant variance"
    );

    let output_param = generics.params[1]
        .as_type()
        .expect("TOutput should be a type parameter");
    assert_eq!(
        output_param.variance,
        Variance::Covariant,
        "expected `out` parameter to record covariant variance"
    );
}

#[test]
fn rejects_variance_on_class_generics() {
    let source = r"
public class Invalid<out TValue>
{
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag.message.contains(
            "variance modifiers are only supported on interface or delegate type parameters"
        )),
        "expected diagnostic about invalid class variance; got {:?}",
        diagnostics
    );
}

#[test]
fn rejects_variance_on_const_generic_parameter() {
    let source = r"
public interface IPool<in const N: int>
{
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("variance modifiers are not allowed on const generic parameters")),
        "expected diagnostic about const variance; got {:?}",
        diagnostics
    );
}

#[test]
fn parses_auto_trait_constraints() {
    let source = r"
namespace Demo;

public class Holder<T>
    where T : @thread_safe, @shareable
{
}
";
    let parsed = parse_module(source).unwrap_or_else(|err| {
        panic!("parse failed: {:?}", err.diagnostics());
    });
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let Item::Class(class) = &parsed.module.items[0] else {
        panic!("expected class item");
    };
    let generics = class.generics.as_ref().expect("missing generics");
    let param = generics.params[0]
        .as_type()
        .expect("expected type parameter");
    assert_eq!(param.constraints.len(), 2);
    assert!(matches!(
        param.constraints[0].kind,
        GenericConstraintKind::AutoTrait(AutoTraitConstraint::ThreadSafe)
    ));
    assert!(matches!(
        param.constraints[1].kind,
        GenericConstraintKind::AutoTrait(AutoTraitConstraint::Shareable)
    ));
}

#[test]
fn rejects_unknown_auto_trait_constraint() {
    let source = r"
namespace Demo;

public class Holder<T>
    where T : @unknown_trait
{
}
";
    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("unknown auto-trait constraint")),
        "expected diagnostic about unknown auto trait, got {:?}",
        diagnostics
    );
}

#[test]
fn reports_missing_closing_bracket_in_class_generics() {
    let source = r"
public class Wrapper<T
{
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("expected `,` or `>` in generic parameter list")
            || diag
                .message
                .contains("expected `>` to close generic parameter list")),
        "expected diagnostic about missing `>`; got {:?}",
        diagnostics
    );
}

#[test]
fn reports_duplicate_type_parameter_in_class_generics() {
    let source = r"
public class Duplicate<T, T>
{
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("duplicate type parameter `T` in generic parameter list")),
        "expected duplicate type parameter diagnostic; got {:?}",
        diagnostics
    );
}

#[test]
fn reports_where_clause_without_generics_on_class() {
    let source = r"
public class MissingWhere
    where T : struct
{
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`where` clause requires type parameters")),
        "expected diagnostic about where clause without generics; got {:?}",
        diagnostics
    );
}

#[test]
fn reports_conflicting_struct_and_class_constraints() {
    let source = r"
public class Bad<T>
    where T : struct, class
{
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("cannot be constrained as both `struct` and `class`")),
        "expected struct/class conflict diagnostic; got {:?}",
        diagnostics
    );
}

#[test]
fn reports_duplicate_type_constraint_on_parameter() {
    let source = r"
public interface IFoo { }

public class DuplicateConstraint<T>
    where T : IFoo, IFoo
{
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("duplicate constraint `IFoo` on type parameter `T`")),
        "expected duplicate type constraint diagnostic; got {:?}",
        diagnostics
    );
}

#[test]
fn reports_new_constraint_not_last() {
    let source = r"
public interface IFoo { }

public class Misordered<T>
    where T : new(), IFoo
{
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`new()` constraint must be the final constraint")),
        "expected `new()` ordering diagnostic; got {:?}",
        diagnostics
    );
}

#[test]
fn reports_missing_closing_bracket_in_struct_generics() {
    let source = r"
public struct Wrapper<T
{
}
";

    let err = parse_module(source).expect_err("expected parse failure");
    assert!(err.diagnostics().iter().any(|diag| {
        diag.message
            .contains("expected `,` or `>` in generic parameter list")
    }));
}
