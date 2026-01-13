use crate::frontend::ast::{FnTypeAbi, PointerModifier, TypeSuffix};
use crate::frontend::parser::parse_type_expression_text;
use crate::frontend::parser::tests::fixtures::{parse_fail, parse_ok};
use crate::frontend::type_utils::{SequenceKind, sequence_descriptor, vector_descriptor};

#[test]
fn parses_array_type_with_generic_argument() {
    let source = r"
namespace Sample;

public void Use(Array<int> values) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let item = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &item.signature.parameters[0];
    assert_eq!(param.ty.base, vec!["Array".to_string()]);
    let args = param.ty.generic_arguments().expect("missing generic args");
    assert_eq!(args.len(), 1, "expected single generic argument");
    assert_eq!(args[0].ty().expect("type arg").name, "int");
    assert!(
        param.ty.array_ranks().next().is_none(),
        "unexpected explicit array rank"
    );
    let descriptor = sequence_descriptor(&param.ty).expect("expected array descriptor");
    assert_eq!(descriptor.kind, SequenceKind::Array);
    assert_eq!(descriptor.rank, 1);
    assert_eq!(descriptor.element.name, "int");
}

#[test]
fn parses_array_type_with_rank_specifier() {
    let source = r"
namespace Sample;

public void Use(Array<int>[,] values) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    let ranks: Vec<_> = param.ty.array_ranks().collect();
    assert_eq!(ranks.len(), 1, "expected one rank specifier");
    assert_eq!(
        ranks[0].dimensions, 2,
        "expected two-dimensional array rank"
    );
    let descriptor = sequence_descriptor(&param.ty).expect("expected array descriptor");
    assert_eq!(descriptor.kind, SequenceKind::Array);
    assert_eq!(
        descriptor.rank, 2,
        "descriptor rank should match dimensions"
    );
}

#[test]
fn parses_vec_type_with_element_argument() {
    let source = r"
namespace Sample;

public void Use(Vec<string> values) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    assert_eq!(param.ty.base, vec!["Vec".to_string()]);
    let args = param
        .ty
        .generic_arguments()
        .expect("expected generic arguments");
    assert_eq!(args.len(), 1);
    assert_eq!(args[0].ty().expect("type arg").name, "string");
    let descriptor = sequence_descriptor(&param.ty).expect("expected vec descriptor");
    assert_eq!(descriptor.kind, SequenceKind::Vec);
    assert_eq!(descriptor.rank, 1);
    assert_eq!(descriptor.element.name, "string");
}

#[test]
fn parses_span_type_with_element_argument() {
    let source = r"
namespace Sample;

public void Use(Span<int> values) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    assert_eq!(param.ty.base, vec!["Span".to_string()]);
    let args = param
        .ty
        .generic_arguments()
        .expect("expected generic arguments");
    assert_eq!(args.len(), 1);
    assert_eq!(args[0].ty().expect("type arg").name, "int");
    let descriptor = sequence_descriptor(&param.ty).expect("expected span descriptor");
    assert_eq!(descriptor.kind, SequenceKind::Span);
    assert_eq!(descriptor.rank, 1);
    assert_eq!(descriptor.element.name, "int");
}

#[test]
fn parses_nested_generic_arguments() {
    let source = r"
namespace Sample;

public void Use(Map<string, Vec<int>> lookup) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );

    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    assert_eq!(param.ty.base, vec!["Map".to_string()]);
    let args = param
        .ty
        .generic_arguments()
        .expect("expected outer generic arguments");
    assert_eq!(args.len(), 2, "expected two outer generic arguments");
    assert!(
        args[0].expression().span.is_some(),
        "outer argument span should be captured"
    );
    assert_eq!(
        args[0].ty().expect("type argument").name,
        "string",
        "first argument should be `string`"
    );
    let nested = args[1].ty().expect("second argument should be type");
    assert_eq!(nested.base, vec!["Vec".to_string()]);
    assert!(
        args[1].expression().span.is_some(),
        "outer nested argument span should be captured"
    );
    let nested_args = nested
        .generic_arguments()
        .expect("expected nested generic arguments");
    assert_eq!(nested_args.len(), 1, "expected single nested argument");
    assert_eq!(
        nested_args[0].ty().expect("nested type argument").name,
        "int"
    );
    assert!(
        nested_args[0].expression().span.is_some(),
        "nested argument span should be captured"
    );
}

#[test]
fn parses_readonly_span_type_with_element_argument() {
    let source = r"
namespace Sample;

public void Use(ReadOnlySpan<byte> values) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    assert_eq!(param.ty.base, vec!["ReadOnlySpan".to_string()]);
    let args = param
        .ty
        .generic_arguments()
        .expect("expected generic arguments");
    assert_eq!(args.len(), 1);
    assert_eq!(args[0].ty().expect("type arg").name, "byte");
    let descriptor = sequence_descriptor(&param.ty).expect("expected span descriptor");
    assert_eq!(descriptor.kind, SequenceKind::ReadOnlySpan);
    assert_eq!(descriptor.rank, 1);
    assert_eq!(descriptor.element.name, "byte");
}

#[test]
fn parses_dyn_trait_parameter_type() {
    let source = r"
namespace Sample;

public void Use(dyn Formatter formatter) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    let object = param.ty.trait_object().expect("expected trait object");
    assert_eq!(object.bounds.len(), 1);
    assert_eq!(object.bounds[0].name, "Formatter");
}

#[test]
fn parses_dyn_trait_with_multiple_bounds() {
    let source = r"
namespace Sample;

public void Use(dyn Formatter + Display target) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    let object = param.ty.trait_object().expect("expected trait object");
    assert_eq!(object.bounds.len(), 2);
    assert_eq!(object.bounds[0].name, "Formatter");
    assert_eq!(object.bounds[1].name, "Display");
}

#[test]
fn parses_impl_trait_parameter_type() {
    let source = r"
namespace Sample;

public void Use(impl Formatter target) { }
";
    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("`impl` is no longer supported")),
        "expected impl trait parameter to be rejected, found {:?}",
        diagnostics
    );
}

#[test]
fn parses_impl_trait_return_type() {
    let source = r"
namespace Sample;

public impl Formatter Build() { return new FormatterImpl(); }
";
    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("`impl` is no longer supported")),
        "expected impl trait return to be rejected, found {:?}",
        diagnostics
    );
}

#[test]
fn parses_str_parameter_type() {
    let source = r"
namespace Sample;

public void Use(str value) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    assert_eq!(param.ty.name, "str");
    assert!(param.ty.generic_arguments().is_none());
    assert_eq!(param.ty.base, vec!["str".to_string()]);
}

#[test]
fn parses_char_parameter_type() {
    let source = r"
namespace Sample;

public void Use(char value) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    assert_eq!(param.ty.name, "char");
    assert!(param.ty.generic_arguments().is_none());
}

#[test]
fn parses_tuple_parameter_type() {
    let source = r"
namespace Sample;

public (int, string) Pair((int, string) values) { return values; }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");

    assert!(
        func.signature.return_type.is_tuple(),
        "return type should be tuple"
    );
    let ret_elements = func
        .signature
        .return_type
        .tuple_elements()
        .expect("tuple elements missing");
    assert_eq!(ret_elements.len(), 2, "expected two return elements");
    assert_eq!(ret_elements[0].name, "int");
    assert_eq!(ret_elements[1].name, "string");

    let param = &func.signature.parameters[0];
    assert!(param.ty.is_tuple(), "parameter type should be tuple");
    let elements = param.ty.tuple_elements().expect("tuple elements missing");
    assert_eq!(elements.len(), 2, "expected two tuple elements");
    assert_eq!(elements[0].name, "int");
    assert_eq!(elements[1].name, "string");
}

#[test]
fn parses_named_tuple_parameter_type() {
    let source = r"
namespace Sample;

public int Consume((int X, int Y) pair) { return pair.X + pair.Y; }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    let names = param
        .ty
        .tuple_element_names()
        .expect("tuple element names missing");
    assert_eq!(names.len(), 2, "expected two tuple element names");
    assert_eq!(names[0].as_deref(), Some("X"));
    assert_eq!(names[1].as_deref(), Some("Y"));
}

#[test]
fn parses_function_pointer_parameter_type() {
    let source = r"
namespace Callbacks;

public void Register(fn(int, int) -> int comparator) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    assert!(param.ty.is_fn(), "expected fn type, found {:?}", param.ty);
    let sig = param.ty.fn_signature().expect("missing fn signature");
    assert!(matches!(sig.abi, FnTypeAbi::Chic));
    assert_eq!(sig.params.len(), 2, "expected two parameter types");
    assert_eq!(sig.params[0].name, "int");
    assert_eq!(sig.params[1].name, "int");
    assert_eq!(sig.return_type.name, "int");
}

#[test]
fn parses_function_pointer_with_generic_argument() {
    let source = r"
namespace Callbacks;

public void Register(fn(Vec<int>) -> int comparator) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    let sig = param
        .ty
        .fn_signature()
        .expect("expected function pointer signature");
    assert_eq!(sig.params.len(), 1, "expected single parameter");
    let param_ty = &sig.params[0];
    assert_eq!(param_ty.base, vec!["Vec".to_string()]);
    let args = param_ty
        .generic_arguments()
        .expect("missing generic arguments on Vec");
    assert_eq!(args.len(), 1, "expected single generic argument");
    assert_eq!(args[0].ty().expect("type argument").name, "int");
    assert_eq!(sig.return_type.name, "int");
}

#[test]
fn parses_nested_generic_return_and_nullable_parameters() {
    let source = r#"
namespace DataMapping;

public static class DbMappingExtensions
{
    public static Task<AsyncEnumerable<T>> QueryAsync<T>(
        this DbConnection connection,
        string sql,
        object? args = null,
        DbTransaction? tx = null,
        int? timeoutSeconds = null,
        CommandType? type = null,
        MappingOptions options = default(MappingOptions),
        CancellationToken ct = default
    )
    {
    }
}
"#;

    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
}

#[test]
fn parses_extern_function_pointer_type() {
    let source = r#"
namespace Callbacks;

public void Register(fn @extern("C")(void*, void*) -> int comparator) { }
"#;
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    let sig = param
        .ty
        .fn_signature()
        .expect("expected function pointer signature");
    match &sig.abi {
        FnTypeAbi::Extern(abi) => assert_eq!(abi, "C"),
        other => panic!("expected extern ABI, found {other:?}"),
    }
    assert_eq!(sig.params.len(), 2, "expected two parameter types");
    assert_eq!(sig.params[0].base, vec!["void".to_string()]);
    assert_eq!(sig.params[1].base, vec!["void".to_string()]);
    assert_eq!(sig.params[0].pointer_depth(), 1);
    assert_eq!(sig.params[1].pointer_depth(), 1);
    assert!(matches!(
        sig.params[0].suffixes.last(),
        Some(TypeSuffix::Pointer { mutable, .. }) if !*mutable
    ));
    assert!(matches!(
        sig.params[1].suffixes.last(),
        Some(TypeSuffix::Pointer { mutable, .. }) if !*mutable
    ));
    assert_eq!(sig.return_type.name, "int");
}

#[test]
fn rejects_multiple_nullable_suffixes() {
    let source = r"
namespace Sample;

public void Use(string?? value) { }
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("type annotation accepts at most one `?`")),
        "expected diagnostic about invalid nullable suffix, got {:?}",
        diagnostics
    );
}

#[test]
fn parses_tuple_array_type() {
    let source = r"
namespace Sample;

public void Use((int, string)[] values) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );

    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    assert!(param.ty.is_tuple(), "tuple array element should be tuple");
    let ranks: Vec<_> = param.ty.array_ranks().collect();
    assert_eq!(ranks.len(), 1, "expected single array rank");
    let elements = param.ty.tuple_elements().expect("tuple elements missing");
    assert_eq!(elements.len(), 2);
    assert_eq!(elements[0].name, "int");
    assert_eq!(elements[1].name, "string");
}

#[test]
fn parses_pointer_parameter_type() {
    let source = r"
namespace Sample;

public void Register(*mut ClosureEnv env) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );

    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    assert_eq!(param.name, "env");
    assert_eq!(param.ty.base, vec!["ClosureEnv".to_string()]);
    assert_eq!(param.ty.pointer_depth(), 1, "expected single pointer layer");
    let last_suffix = param.ty.suffixes.last().expect("pointer suffix missing");
    assert!(
        matches!(last_suffix, TypeSuffix::Pointer { mutable: true, .. }),
        "expected mutable pointer suffix, found {last_suffix:?}"
    );
}

#[test]
fn parses_nested_pointer_parameter_type() {
    let source = r"
namespace Sample;

public void Register(*const *mut ClosureEnv env) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );

    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    assert_eq!(param.ty.pointer_depth(), 2, "expected double pointer");
    let mut pointer_flags = param.ty.suffixes.iter().filter_map(|suffix| match suffix {
        TypeSuffix::Pointer { mutable, .. } => Some(*mutable),
        _ => None,
    });
    assert_eq!(
        pointer_flags.next(),
        Some(true),
        "inner pointer should be mutable"
    );
    assert_eq!(
        pointer_flags.next(),
        Some(false),
        "outer pointer should be const"
    );
}

#[test]
fn parses_pointer_modifiers() {
    let source = r"
namespace Sample;

public unsafe void Use(*mut @restrict @readonly @aligned(32) @expose_address byte data) { }
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );

    let func = parsed
        .module
        .items
        .iter()
        .find_map(|item| match item {
            crate::frontend::ast::Item::Function(func) => Some(func),
            _ => None,
        })
        .expect("missing function");
    let param = &func.signature.parameters[0];
    let suffix = param.ty.suffixes.last().expect("missing pointer suffix");
    let TypeSuffix::Pointer { modifiers, .. } = suffix else {
        panic!("expected pointer suffix with modifiers");
    };
    assert!(
        modifiers.contains(&PointerModifier::Restrict),
        "restrict modifier missing: {modifiers:?}"
    );
    assert!(
        modifiers.contains(&PointerModifier::ReadOnly),
        "readonly modifier missing: {modifiers:?}"
    );
    assert!(
        modifiers.contains(&PointerModifier::ExposeAddress),
        "expose_address modifier missing: {modifiers:?}"
    );
    assert!(
        modifiers
            .iter()
            .any(|modifier| matches!(modifier, PointerModifier::Aligned(32))),
        "aligned modifier missing or incorrect: {modifiers:?}"
    );

    assert!(
        modifiers.contains(&PointerModifier::ExposeAddress),
        "expose_address modifier missing: {modifiers:?}"
    );
}

#[test]
fn parses_pointer_modifiers_in_extern_function() {
    let source = r#"
namespace Sample;

public static class Native
{
    @extern("C")
    public static extern void Copy(*mut @restrict byte dest, *const @restrict byte src);
}
"#;

    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
}

#[test]
fn parses_tuple_field_type_in_struct() {
    let source = r"
namespace Records;

public struct Holder
{
    internal (int, string) Data;
}
";
    let parsed = parse_ok(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let structure = match &parsed.module.items[0] {
        crate::frontend::ast::Item::Struct(def) => def,
        other => panic!("expected struct item, found {other:?}"),
    };
    assert_eq!(structure.fields.len(), 1, "expected single field");
    let field = &structure.fields[0];
    assert!(
        field.ty.is_tuple(),
        "struct field type should be parsed as tuple"
    );
    let elements = field.ty.tuple_elements().expect("tuple elements missing");
    assert_eq!(elements.len(), 2, "expected two tuple elements");
    assert_eq!(elements[0].name, "int");
    assert_eq!(elements[1].name, "string");
}

#[test]
fn parses_vector_type_expression() {
    let expr =
        parse_type_expression_text("vector<float, 4>").expect("expected vector type expression");
    let descriptor = vector_descriptor(&expr).expect("expected vector descriptor");
    assert_eq!(descriptor.element.name, "float");
    assert_eq!(descriptor.lanes.expression().text, "4");
}

#[test]
fn vector_descriptor_requires_const_lane_argument() {
    let expr =
        parse_type_expression_text("vector<int, lanes>").expect("expected vector type expression");
    // The descriptor still surfaces the shape but callers validate const-evaluability.
    let descriptor = vector_descriptor(&expr).expect("expected vector descriptor");
    assert_eq!(descriptor.element.name, "int");
    assert_eq!(descriptor.lanes.expression().text, "lanes");
}
