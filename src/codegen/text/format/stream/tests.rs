use crate::chic_kind::ChicKind;
use crate::codegen::text::generate_text;
use crate::frontend::ast::{DocComment, Item, MemberDispatch, Module, TypeExpr};
use crate::frontend::parser::parse_module;
use crate::target::Target;

use super::*;

fn doc(lines: &[&str]) -> Option<DocComment> {
    Some(DocComment::new(
        lines.iter().map(|line| (*line).to_string()).collect(),
    ))
}

#[test]
fn write_module_indents_nested_items() {
    let source = r#"
namespace Outer {
namespace Inner {
public int Example(int value) { return value; }
}
}
"#;
    let parsed = parse_module(source).expect("parse module");
    let mut output = String::new();
    write_module(&mut output, &parsed.module).expect("format succeeds");
    assert!(
        output.contains("namespace Outer {"),
        "expected namespace header, got {output}"
    );
    assert!(
        output.contains("    namespace Outer.Inner {"),
        "expected nested indent, got {output:?}"
    );
    assert!(
        output.contains("        public fn Example(int value) -> int {"),
        "expected function line to be indented twice, got {output:?}"
    );
    assert!(
        output.contains("            stmt Return"),
        "expected statement indent"
    );
}

#[test]
fn writes_properties_and_constructors() {
    use crate::frontend::ast::{
        BindingModifier, Block, ClassDecl, ClassKind, ClassMember, ConstructorDecl,
        ConstructorInitTarget, ConstructorInitializer, ConstructorKind, Expression, Parameter,
        PropertyAccessor, PropertyAccessorBody, PropertyAccessorKind, PropertyDecl, Statement,
        StatementKind, UsingDirective, UsingKind, Visibility,
    };

    let property = PropertyDecl {
        visibility: Visibility::Public,
        modifiers: vec!["static".into()],
        name: "Value".into(),
        ty: TypeExpr::simple("int"),
        parameters: Vec::new(),
        accessors: vec![
            PropertyAccessor {
                kind: PropertyAccessorKind::Get,
                visibility: None,
                body: PropertyAccessorBody::Auto,
                doc: doc(&["Getter"]),
                span: None,
                attributes: None,
                dispatch: MemberDispatch::default(),
            },
            PropertyAccessor {
                kind: PropertyAccessorKind::Set,
                visibility: Some(Visibility::Protected),
                body: PropertyAccessorBody::Expression(Expression::new("value * 2", None)),
                doc: None,
                span: None,
                attributes: None,
                dispatch: MemberDispatch::default(),
            },
            PropertyAccessor {
                kind: PropertyAccessorKind::Init,
                visibility: None,
                body: PropertyAccessorBody::Block(Block {
                    statements: vec![Statement::new(
                        None,
                        StatementKind::Return { expression: None },
                    )],
                    span: None,
                }),
                doc: None,
                span: None,
                attributes: None,
                dispatch: MemberDispatch::default(),
            },
        ],
        doc: doc(&["Static property"]),
        is_required: true,
        is_static: false,
        initializer: None,
        span: None,
        attributes: Vec::new(),
        di_inject: None,
        dispatch: MemberDispatch::default(),
        explicit_interface: None,
    };

    let ctor = ConstructorDecl {
        visibility: Visibility::Public,
        kind: ConstructorKind::Designated,
        parameters: vec![Parameter {
            binding: BindingModifier::Value,
            binding_nullable: false,
            name: "seed".into(),
            name_span: None,
            ty: TypeExpr::simple("int"),
            attributes: Vec::new(),
            di_inject: None,
            default: None,
            default_span: None,
            lends: None,
            is_extension_this: false,
        }],
        body: Some(Block {
            statements: vec![Statement::new(
                None,
                StatementKind::Return { expression: None },
            )],
            span: None,
        }),
        initializer: Some(ConstructorInitializer {
            target: ConstructorInitTarget::Super,
            arguments: vec![Expression::new("seed", None)],
            span: None,
        }),
        doc: doc(&["Ctor"]),
        span: None,
        attributes: Vec::new(),
        di_inject: None,
    };

    let class = ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Config".into(),
        bases: Vec::new(),
        members: vec![
            ClassMember::Constructor(ctor),
            ClassMember::Property(property),
        ],
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: doc(&["Config class"]),
        generics: None,
        attributes: Vec::new(),
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    };

    let mut module = Module::new(None);
    module.items = vec![
        Item::Import(UsingDirective {
            doc: None,
            is_global: false,
            span: None,
            kind: UsingKind::Static {
                target: "Std.Math".into(),
            },
        }),
        Item::Class(class),
    ];

    let mut output = String::new();
    write_module(&mut output, &module).expect("format succeeds");
    assert!(
        output.contains("public init(int seed) : super(seed) {"),
        "constructor should include initializer: {output}"
    );
    assert!(
        output.contains("public required Value: int {"),
        "property header missing: {output}"
    );
    assert!(
        output.contains("protected set => value * 2;"),
        "set accessor expression missing: {output}"
    );
    assert!(
        output.contains("init { /* ... */ }"),
        "init accessor block missing"
    );
}

#[test]
fn writes_docs_with_blank_lines() {
    use crate::frontend::ast::{
        FieldDecl, Item, Module, StructDecl, UsingDirective, UsingKind, Visibility,
    };

    let mut module = Module::new(None);
    module.items = vec![
        Item::Import(UsingDirective {
            doc: doc(&["Import summary", "", "Trailing detail"]),
            is_global: false,
            span: None,
            kind: UsingKind::Namespace {
                path: "Std.Platform.IO".to_string(),
            },
        }),
        Item::Struct(StructDecl {
            visibility: Visibility::Public,
            name: "Documented".into(),
            fields: vec![FieldDecl {
                visibility: Visibility::Public,
                name: "Value".into(),
                ty: TypeExpr::simple("int"),
                initializer: None,
                doc: doc(&["Field docs"]),
                attributes: Vec::new(),
                mmio: None,
                is_required: false,
                display_name: None,
                is_readonly: false,
                is_static: false,
                view_of: None,
            }],
            properties: Vec::new(),
            constructors: Vec::new(),
            consts: Vec::new(),
            methods: Vec::new(),
            nested_types: Vec::new(),
            bases: Vec::new(),
            thread_safe_override: None,
            shareable_override: None,
            copy_override: None,
            doc: doc(&["Summary line", "", "Detailed line"]),
            mmio: None,
            generics: None,
            attributes: Vec::new(),
            is_readonly: false,
            layout: None,
            is_intrinsic: false,
            inline_attr: None,
            is_record: false,
            record_positional_fields: Vec::new(),
        }),
    ];

    let mut output = String::new();
    write_module(&mut output, &module).expect("format succeeds");
    assert!(
        output.contains("/// Summary line\n///\n/// Detailed line"),
        "struct docs should preserve blank lines:\n{output}"
    );
    assert!(
        output.contains("/// Import summary\n///\n/// Trailing detail"),
        "import docs should also preserve blank lines:\n{output}"
    );
}

#[test]
fn generates_textual_output_for_functions() {
    let source = r"
namespace Math;

public double Add(double x, double y)
{
return x + y;
}
";
    let parsed = match parse_module(source) {
        Ok(parsed) => parsed,
        Err(err) => panic!("parse failed: {err:?}"),
    };
    let target = match Target::parse("x86_64-unknown-linux-gnu") {
        Ok(target) => target,
        Err(err) => panic!("target parse failed: {err:?}"),
    };
    let module = parsed.module;
    let output = generate_text(&module, &target, ChicKind::Executable);
    assert!(output.contains("fn Add(double x, double y) -> double"));
    assert!(output.contains("stmt Return"));
}

#[test]
fn generates_struct_and_enum() {
    let source = r"
namespace Geometry;

public struct Point { public int X; public int Y; }

public enum Shape
{
Circle { public double Radius; },
Square,
}
";
    let parsed = match parse_module(source) {
        Ok(parsed) => parsed,
        Err(err) => panic!("parse failed: {err:?}"),
    };
    let module = parsed.module;
    let output = generate_text(&module, &Target::host(), ChicKind::StaticLibrary);
    assert!(output.contains("struct Point"));
    assert!(output.contains("enum Shape"));
    assert!(output.contains("Circle {"));
}

#[test]
fn generates_testcase() {
    let source = r"
testcase EnsuresEquality()
{
Assert.That(2 + 2).IsEqualTo(4);
}
";
    let parsed = match parse_module(source) {
        Ok(parsed) => parsed,
        Err(err) => panic!("parse failed: {err:?}"),
    };
    let module = parsed.module;
    let output = generate_text(&module, &Target::host(), ChicKind::Executable);
    assert!(output.contains("testcase EnsuresEquality"));
}

#[test]
fn renders_full_module_with_docs_and_modifiers() {
    use crate::frontend::ast::{
        BindingModifier, Block, ClassDecl, ClassMember, DocComment, EnumDecl, EnumVariant,
        ExtensionDecl, ExtensionMember, ExtensionMethodDecl, FieldDecl, FunctionDecl,
        InterfaceDecl, InterfaceMember, Item, Module, NamespaceDecl, Parameter, Statement,
        StatementKind, StructDecl, TestCaseDecl, TypeExpr, UnionDecl, UnionField, UnionMember,
        UnionViewDecl, UsingDirective, UsingKind, Visibility,
    };

    fn doc(lines: &[&str]) -> Option<DocComment> {
        Some(DocComment::new(
            lines.iter().map(|line| (*line).to_string()).collect(),
        ))
    }

    let method_body = Block {
        statements: vec![Statement::new(
            None,
            StatementKind::Return { expression: None },
        )],
        span: None,
    };

    let mut module = Module::new(Some("Root.Core".into()));
    module.items = vec![
        Item::Import(UsingDirective {
            doc: doc(&["Using namespace"]),
            is_global: true,
            span: None,
            kind: UsingKind::Namespace {
                path: "Utilities.Logging".into(),
            },
        }),
        Item::Import(UsingDirective {
            doc: None,
            is_global: false,
            span: None,
            kind: UsingKind::Alias {
                alias: "Alias".into(),
                target: "Std.Tasks".into(),
            },
        }),
        Item::Import(UsingDirective {
            doc: None,
            is_global: false,
            span: None,
            kind: UsingKind::Static {
                target: "Std.Math".into(),
            },
        }),
        Item::Namespace(NamespaceDecl {
            name: "Inner".into(),
            items: vec![
                Item::Function(FunctionDecl {
                    visibility: Visibility::Public,
                    name: "Process".into(),
                    name_span: None,
                    signature: crate::frontend::ast::Signature {
                        parameters: vec![
                            Parameter {
                                binding: BindingModifier::In,
                                binding_nullable: false,
                                name: "input".into(),
                                name_span: None,
                                ty: TypeExpr::simple("int"),
                                attributes: Vec::new(),
                                di_inject: None,
                                default: None,
                                default_span: None,
                                lends: None,
                                is_extension_this: false,
                            },
                            Parameter {
                                binding: BindingModifier::Ref,
                                binding_nullable: false,
                                name: "value".into(),
                                name_span: None,
                                ty: TypeExpr::simple("string"),
                                attributes: Vec::new(),
                                di_inject: None,
                                default: None,
                                default_span: None,
                                lends: None,
                                is_extension_this: false,
                            },
                            Parameter {
                                binding: BindingModifier::Out,
                                binding_nullable: false,
                                name: "output".into(),
                                name_span: None,
                                ty: TypeExpr::simple("Result"),
                                attributes: Vec::new(),
                                di_inject: None,
                                default: None,
                                default_span: None,
                                lends: None,
                                is_extension_this: false,
                            },
                        ],
                        return_type: TypeExpr::simple("bool"),
                        lends_to_return: None,
                        variadic: false,
                        throws: None,
                    },
                    body: Some(method_body.clone()),
                    is_async: true,
                    is_constexpr: false,
                    doc: doc(&["Performs processing", "", "Returns true on success"]),
                    modifiers: Vec::new(),
                    is_unsafe: false,
                    attributes: Vec::new(),
                    is_extern: false,
                    extern_abi: None,
                    extern_options: None,
                    link_name: None,
                    link_library: None,
                    operator: None,
                    generics: None,
                    vectorize_hint: None,
                    dispatch: MemberDispatch::default(),
                }),
                Item::Struct(StructDecl {
                    visibility: Visibility::Public,
                    name: "Point".into(),
                    fields: vec![
                        FieldDecl {
                            visibility: Visibility::Public,
                            name: "X".into(),
                            ty: TypeExpr::simple("int"),
                            initializer: None,
                            doc: doc(&["X coordinate"]),
                            attributes: Vec::new(),
                            mmio: None,
                            is_required: false,
                            display_name: None,
                            is_readonly: false,
                            is_static: false,
                            view_of: None,
                        },
                        FieldDecl {
                            visibility: Visibility::Private,
                            name: "Y".into(),
                            ty: TypeExpr::simple("int"),
                            initializer: None,
                            doc: None,
                            attributes: Vec::new(),
                            mmio: None,
                            is_required: false,
                            display_name: None,
                            is_readonly: false,
                            is_static: false,
                            view_of: None,
                        },
                    ],
                    properties: Vec::new(),
                    constructors: Vec::new(),
                    consts: Vec::new(),
                    methods: Vec::new(),
                    nested_types: Vec::new(),
                    bases: Vec::new(),
                    thread_safe_override: None,
                    shareable_override: None,
                    copy_override: None,
                    doc: doc(&["Point data structure"]),
                    mmio: None,
                    generics: None,
                    attributes: Vec::new(),
                    is_readonly: false,
                    layout: None,
                    is_intrinsic: false,
                    inline_attr: None,
                    is_record: false,
                    record_positional_fields: Vec::new(),
                }),
                Item::Union(UnionDecl {
                    visibility: Visibility::Public,
                    name: "Value".into(),
                    members: vec![
                        UnionMember::Field(UnionField {
                            visibility: Visibility::Public,
                            name: "Number".into(),
                            ty: TypeExpr::simple("int"),
                            is_readonly: true,
                            doc: doc(&["Read-only number"]),
                            attributes: Vec::new(),
                        }),
                        UnionMember::Field(UnionField {
                            visibility: Visibility::Internal,
                            name: "Handle".into(),
                            ty: TypeExpr::simple("Handle"),
                            is_readonly: false,
                            doc: None,
                            attributes: Vec::new(),
                        }),
                        UnionMember::View(UnionViewDecl {
                            visibility: Visibility::Public,
                            name: "RefView".into(),
                            fields: vec![FieldDecl {
                                visibility: Visibility::Public,
                                name: "Value".into(),
                                ty: TypeExpr::simple("int"),
                                initializer: None,
                                doc: None,
                                attributes: Vec::new(),
                                mmio: None,
                                is_required: false,
                                display_name: None,
                                is_readonly: false,
                                is_static: false,
                                view_of: None,
                            }],
                            is_readonly: true,
                            doc: doc(&["Reference view"]),
                            attributes: Vec::new(),
                        }),
                    ],
                    thread_safe_override: None,
                    shareable_override: None,
                    copy_override: None,
                    doc: doc(&["Union documentation"]),
                    generics: None,
                    attributes: Vec::new(),
                }),
                Item::Enum(EnumDecl {
                    visibility: Visibility::Public,
                    name: "Mode".into(),
                    underlying_type: None,
                    variants: vec![
                        EnumVariant {
                            name: "Alpha".into(),
                            fields: Vec::new(),
                            discriminant: None,
                            doc: doc(&["Alpha variant"]),
                        },
                        EnumVariant {
                            name: "Beta".into(),
                            fields: vec![FieldDecl {
                                visibility: Visibility::Public,
                                name: "Level".into(),
                                ty: TypeExpr::simple("int"),
                                initializer: None,
                                doc: None,
                                attributes: Vec::new(),
                                mmio: None,
                                is_required: false,
                                display_name: None,
                                is_readonly: false,
                                is_static: false,
                                view_of: None,
                            }],
                            discriminant: None,
                            doc: None,
                        },
                    ],
                    thread_safe_override: None,
                    shareable_override: None,
                    copy_override: None,
                    is_flags: false,
                    doc: doc(&["Operating modes"]),
                    generics: None,
                    attributes: Vec::new(),
                }),
                Item::Class(ClassDecl {
                    visibility: Visibility::Public,
                    kind: crate::frontend::ast::ClassKind::Class,
                    name: "Derived".into(),
                    bases: vec![TypeExpr::simple("BaseType")],
                    members: vec![
                        ClassMember::Field(FieldDecl {
                            visibility: Visibility::Internal,
                            name: "State".into(),
                            ty: TypeExpr::simple("int"),
                            initializer: None,
                            doc: None,
                            attributes: Vec::new(),
                            mmio: None,
                            is_required: false,
                            display_name: None,
                            is_readonly: false,
                            is_static: false,
                            view_of: None,
                        }),
                        ClassMember::Method(FunctionDecl {
                            visibility: Visibility::Private,
                            name: "Compute".into(),
                            name_span: None,
                            signature: crate::frontend::ast::Signature {
                                parameters: vec![],
                                return_type: TypeExpr::simple("void"),
                                lends_to_return: None,
                                variadic: false,
                                throws: None,
                            },
                            body: Some(method_body.clone()),
                            is_async: false,
                            is_constexpr: false,
                            doc: None,
                            modifiers: Vec::new(),
                            is_unsafe: false,
                            attributes: Vec::new(),
                            is_extern: false,
                            extern_abi: None,
                            extern_options: None,
                            link_name: None,
                            link_library: None,
                            operator: None,
                            generics: None,
                            vectorize_hint: None,
                            dispatch: MemberDispatch::default(),
                        }),
                    ],
                    nested_types: Vec::new(),
                    thread_safe_override: None,
                    shareable_override: None,
                    copy_override: None,
                    doc: doc(&["Class documentation"]),
                    generics: None,
                    attributes: Vec::new(),
                    di_service: None,
                    di_module: false,
                    is_static: false,
                    is_abstract: false,
                    is_sealed: false,
                }),
                Item::Interface(InterfaceDecl {
                    visibility: Visibility::Public,
                    name: "IService".into(),
                    bases: vec![TypeExpr::simple("IDisposable")],
                    members: vec![InterfaceMember::Method(FunctionDecl {
                        visibility: Visibility::Public,
                        name: "Call".into(),
                        name_span: None,
                        signature: crate::frontend::ast::Signature {
                            parameters: vec![],
                            return_type: TypeExpr::simple("void"),
                            lends_to_return: None,
                            variadic: false,
                            throws: None,
                        },
                        body: None,
                        is_async: false,
                        is_constexpr: false,
                        doc: None,
                        modifiers: Vec::new(),
                        is_unsafe: false,
                        attributes: Vec::new(),
                        is_extern: false,
                        extern_abi: None,
                        extern_options: None,
                        link_name: None,
                        link_library: None,
                        operator: None,
                        generics: None,
                        vectorize_hint: None,
                        dispatch: MemberDispatch::default(),
                    })],
                    thread_safe_override: None,
                    shareable_override: None,
                    copy_override: None,
                    doc: doc(&["Interface documentation"]),
                    generics: None,
                    attributes: Vec::new(),
                }),
                Item::Extension(ExtensionDecl {
                    visibility: Visibility::Public,
                    target: TypeExpr::simple("String"),
                    generics: None,
                    members: vec![ExtensionMember::Method(ExtensionMethodDecl {
                        function: FunctionDecl {
                            visibility: Visibility::Public,
                            name: "TrimEx".into(),
                            name_span: None,
                            signature: crate::frontend::ast::Signature {
                                parameters: vec![],
                                return_type: TypeExpr::simple("string"),
                                lends_to_return: None,
                                variadic: false,
                                throws: None,
                            },
                            body: Some(method_body.clone()),
                            is_async: false,
                            is_constexpr: false,
                            doc: None,
                            modifiers: Vec::new(),
                            is_unsafe: false,
                            attributes: Vec::new(),
                            is_extern: false,
                            extern_abi: None,
                            extern_options: None,
                            link_name: None,
                            link_library: None,
                            operator: None,
                            generics: None,
                            vectorize_hint: None,
                            dispatch: MemberDispatch::default(),
                        },
                        is_default: false,
                    })],
                    doc: doc(&["Extension documentation"]),
                    attributes: Vec::new(),
                    conditions: Vec::new(),
                }),
                Item::TestCase(TestCaseDecl {
                    name: "Runs".into(),
                    signature: None,
                    body: method_body.clone(),
                    is_async: true,
                    doc: doc(&["Testcase documentation"]),
                    attributes: Vec::new(),
                }),
            ],
            doc: doc(&["Inner namespace", ""]),
            attributes: Vec::new(),
            span: None,
        }),
    ];

    let text = generate_text(&module, &Target::host(), ChicKind::DynamicLibrary);
    assert!(text.contains("package-namespace Root.Core"));
    assert!(text.contains("global import Utilities.Logging;"));
    assert!(text.contains("import Alias = Std.Tasks;"));
    assert!(text.contains("import static Std.Math;"));
    assert!(text.contains("/// Inner namespace"));
    assert!(text.contains("/// "));
    assert!(text.contains("namespace Inner {"));
    assert!(text.contains(
        "public async fn Process(in int input, ref string value, out Result output) -> bool"
    ));
    assert!(text.contains("struct Point"));
    assert!(text.contains("public readonly int Number;"));
    assert!(text.contains("readonly struct RefView"));
    assert!(!text.contains("ref readonly struct RefView"));
    assert!(text.contains("internal Handle Handle;"));
    assert!(text.contains("enum Mode"));
    assert!(text.contains("class Derived : BaseType"));
    assert!(text.contains("interface IService : IDisposable"));
    assert!(text.contains("extension String"));
    assert!(text.contains("async testcase Runs"));
    assert!(text.contains("stmt Return"));
    assert!(text.contains("internal State: int;"));
    assert!(text.contains("private fn Compute() -> void"));
}

#[test]
fn writes_all_item_variants_and_branches() {
    use crate::frontend::ast::expressions::Expression;
    use crate::frontend::ast::{
        Attribute, AttributeKind, BindingModifier, Block, ClassDecl, ClassKind, ClassMember,
        ConstDeclaration, ConstDeclarator, ConstItemDecl, ConstMemberDecl, ConstructorDecl,
        ConstructorInitTarget, ConstructorInitializer, ConstructorKind, EnumDecl, EnumVariant,
        ExtensionDecl, ExtensionMember, ExtensionMethodDecl, FieldDecl, FunctionDecl,
        GenericParams, ImplDecl, ImplMember, InterfaceDecl, InterfaceMember, Item, NamespaceDecl,
        Parameter, Signature, Statement, StatementKind, StructDecl, TestCaseDecl, TraitDecl,
        TraitMember, TypeExpr, UnionDecl, UnionField, UnionMember, UnionViewDecl, UsingDirective,
        UsingKind, Visibility,
    };

    let mut module = Module::new(Some("Pkg.Core".into()));
    module.items.push(Item::Import(UsingDirective {
        doc: doc(&["C import directive"]),
        is_global: true,
        span: None,
        kind: UsingKind::CImport {
            header: "stdio.h".into(),
        },
    }));
    module.items.push(Item::Const(ConstItemDecl {
        visibility: Visibility::Public,
        declaration: ConstDeclaration {
            ty: TypeExpr::simple("int"),
            declarators: vec![
                ConstDeclarator {
                    name: "A".into(),
                    initializer: Expression::new("1", None),
                    span: None,
                },
                ConstDeclarator {
                    name: "B".into(),
                    initializer: Expression::new("2", None),
                    span: None,
                },
            ],
            doc: doc(&["Const docs"]),
            span: None,
        },
    }));
    module.items.push(Item::Namespace(NamespaceDecl {
        name: "Nested".into(),
        items: vec![
            Item::Trait(TraitDecl {
                visibility: Visibility::Public,
                name: "Marker".into(),
                super_traits: Vec::new(),
                members: vec![TraitMember::Method(FunctionDecl {
                    visibility: Visibility::Public,
                    name: "Tag".into(),
                    name_span: None,
                    signature: Signature {
                        parameters: Vec::new(),
                        return_type: TypeExpr::simple("void"),
                        lends_to_return: None,
                        variadic: false,
                        throws: None,
                    },
                    body: None,
                    is_async: false,
                    is_constexpr: false,
                    doc: None,
                    modifiers: Vec::new(),
                    is_unsafe: false,
                    attributes: Vec::new(),
                    is_extern: false,
                    extern_abi: None,
                    extern_options: None,
                    link_name: None,
                    link_library: None,
                    operator: None,
                    generics: None,
                    vectorize_hint: None,
                    dispatch: MemberDispatch::default(),
                })],
                thread_safe_override: None,
                shareable_override: None,
                copy_override: None,
                doc: doc(&["Trait doc"]),
                generics: None,
                attributes: Vec::new(),
                span: None,
            }),
            Item::Impl(ImplDecl {
                visibility: Visibility::Public,
                trait_ref: None,
                target: TypeExpr::simple("Marker"),
                generics: None,
                members: vec![ImplMember::Method(FunctionDecl {
                    visibility: Visibility::Public,
                    name: "ImplFn".into(),
                    name_span: None,
                    signature: Signature {
                        parameters: Vec::new(),
                        return_type: TypeExpr::simple("void"),
                        lends_to_return: None,
                        variadic: false,
                        throws: None,
                    },
                    body: None,
                    is_async: false,
                    is_constexpr: false,
                    doc: None,
                    modifiers: Vec::new(),
                    is_unsafe: false,
                    attributes: Vec::new(),
                    is_extern: false,
                    extern_abi: None,
                    extern_options: None,
                    link_name: None,
                    link_library: None,
                    operator: None,
                    generics: None,
                    vectorize_hint: None,
                    dispatch: MemberDispatch::default(),
                })],
                doc: None,
                attributes: Vec::new(),
                span: None,
            }),
        ],
        doc: doc(&["Nested ns"]),
        attributes: Vec::new(),
        span: None,
    }));
    module.items.push(Item::Function(FunctionDecl {
        visibility: Visibility::Public,
        name: "NoBody".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("void"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: None,
        is_async: false,
        is_constexpr: false,
        doc: None,
        modifiers: Vec::new(),
        is_unsafe: false,
        attributes: Vec::new(),
        is_extern: false,
        extern_abi: None,
        extern_options: None,
        link_name: None,
        link_library: None,
        operator: None,
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    }));
    module.items.push(Item::Struct(StructDecl {
        visibility: Visibility::Public,
        name: "Bag".into(),
        fields: vec![FieldDecl {
            visibility: Visibility::Public,
            name: "Value".into(),
            ty: TypeExpr::simple("int"),
            initializer: None,
            doc: None,
            attributes: Vec::new(),
            mmio: None,
            is_required: false,
            display_name: None,
            is_readonly: false,
            is_static: false,
            view_of: None,
        }],
        properties: Vec::new(),
        constructors: Vec::new(),
        consts: vec![ConstMemberDecl {
            visibility: Visibility::Public,
            modifiers: Vec::new(),
            declaration: ConstDeclaration {
                ty: TypeExpr::simple("int"),
                declarators: vec![ConstDeclarator {
                    name: "Inner".into(),
                    initializer: Expression::new("5", None),
                    span: None,
                }],
                doc: None,
                span: None,
            },
        }],
        methods: Vec::new(),
        nested_types: Vec::new(),
        bases: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        mmio: None,
        generics: None,
        attributes: Vec::new(),
        is_readonly: true,
        layout: None,
        is_intrinsic: false,
        inline_attr: None,
        is_record: false,
        record_positional_fields: Vec::new(),
    }));
    module.items.push(Item::Union(UnionDecl {
        visibility: Visibility::Public,
        name: "Choices".into(),
        members: vec![
            UnionMember::Field(UnionField {
                visibility: Visibility::Public,
                name: "Number".into(),
                ty: TypeExpr::simple("int"),
                is_readonly: true,
                doc: None,
                attributes: Vec::new(),
            }),
            UnionMember::View(UnionViewDecl {
                visibility: Visibility::Public,
                name: "View".into(),
                fields: vec![FieldDecl {
                    visibility: Visibility::Public,
                    name: "Raw".into(),
                    ty: TypeExpr::simple("uint"),
                    initializer: None,
                    doc: None,
                    attributes: Vec::new(),
                    mmio: None,
                    is_required: false,
                    display_name: None,
                    is_readonly: false,
                    is_static: false,
                    view_of: None,
                }],
                is_readonly: true,
                doc: None,
                attributes: Vec::new(),
            }),
        ],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: None,
        attributes: Vec::new(),
    }));
    module.items.push(Item::Enum(EnumDecl {
        visibility: Visibility::Public,
        name: "Colors".into(),
        underlying_type: None,
        variants: vec![
            EnumVariant {
                name: "Red".into(),
                fields: Vec::new(),
                discriminant: None,
                doc: None,
            },
            EnumVariant {
                name: "Green".into(),
                fields: vec![FieldDecl {
                    visibility: Visibility::Public,
                    name: "Code".into(),
                    ty: TypeExpr::simple("int"),
                    initializer: None,
                    doc: None,
                    attributes: Vec::new(),
                    mmio: None,
                    is_required: false,
                    display_name: None,
                    is_readonly: false,
                    is_static: false,
                    view_of: None,
                }],
                discriminant: None,
                doc: None,
            },
        ],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        is_flags: false,
        doc: None,
        generics: None,
        attributes: Vec::new(),
    }));
    module.items.push(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Example".into(),
        bases: vec![TypeExpr::simple("Base")],
        members: vec![
            ClassMember::Constructor(ConstructorDecl {
                visibility: Visibility::Public,
                kind: ConstructorKind::Convenience,
                parameters: Vec::new(),
                body: None,
                initializer: Some(ConstructorInitializer {
                    target: ConstructorInitTarget::Super,
                    arguments: vec![],
                    span: None,
                }),
                doc: doc(&["Ctor doc"]),
                span: None,
                attributes: Vec::new(),
                di_inject: None,
            }),
            ClassMember::Field(FieldDecl {
                visibility: Visibility::Protected,
                name: "Inner".into(),
                ty: TypeExpr::simple("int"),
                initializer: None,
                doc: None,
                attributes: Vec::new(),
                mmio: None,
                is_required: false,
                display_name: None,
                is_readonly: false,
                is_static: false,
                view_of: None,
            }),
        ],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: Some(GenericParams::new(None, Vec::new())),
        attributes: Vec::new(),
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
        nested_types: Vec::new(),
    }));
    module.items.push(Item::Interface(InterfaceDecl {
        visibility: Visibility::Public,
        name: "Iface".into(),
        bases: vec![TypeExpr::simple("Disposable")],
        members: vec![InterfaceMember::Method(FunctionDecl {
            visibility: Visibility::Public,
            name: "Call".into(),
            name_span: None,
            signature: Signature {
                parameters: Vec::new(),
                return_type: TypeExpr::simple("void"),
                lends_to_return: None,
                variadic: false,
                throws: None,
            },
            body: None,
            is_async: false,
            is_constexpr: false,
            doc: None,
            modifiers: Vec::new(),
            is_unsafe: false,
            attributes: Vec::new(),
            is_extern: false,
            extern_abi: None,
            extern_options: None,
            link_name: None,
            link_library: None,
            operator: None,
            generics: None,
            vectorize_hint: None,
            dispatch: MemberDispatch::default(),
        })],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: None,
        attributes: Vec::new(),
    }));
    module.items.push(Item::Extension(ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("Example"),
        generics: None,
        members: vec![ExtensionMember::Method(ExtensionMethodDecl {
            function: FunctionDecl {
                visibility: Visibility::Public,
                name: "Decorate".into(),
                name_span: None,
                signature: Signature {
                    parameters: vec![Parameter {
                        binding: BindingModifier::Value,
                        binding_nullable: false,
                        name: "this".into(),
                        name_span: None,
                        ty: TypeExpr::simple("Self"),
                        attributes: Vec::new(),
                        di_inject: None,
                        default: None,
                        default_span: None,
                        lends: None,
                        is_extension_this: true,
                    }],
                    return_type: TypeExpr::simple("Example"),
                    lends_to_return: None,
                    variadic: false,
                    throws: None,
                },
                body: None,
                is_async: false,
                is_constexpr: false,
                doc: None,
                modifiers: Vec::new(),
                is_unsafe: false,
                attributes: Vec::new(),
                is_extern: false,
                extern_abi: None,
                extern_options: None,
                link_name: None,
                link_library: None,
                operator: None,
                generics: None,
                vectorize_hint: None,
                dispatch: MemberDispatch::default(),
            },
            is_default: false,
        })],
        doc: None,
        attributes: vec![Attribute::new(
            "Helper",
            Vec::new(),
            None,
            None,
            AttributeKind::Builtin,
        )],
        conditions: Vec::new(),
    }));
    module.items.push(Item::TestCase(TestCaseDecl {
        doc: doc(&["test doc"]),
        name: "Smoke".into(),
        signature: None,
        body: Block {
            statements: vec![Statement::new(
                None,
                StatementKind::Return { expression: None },
            )],
            span: None,
        },
        is_async: false,
        attributes: Vec::new(),
    }));

    let mut out = String::new();
    write_module(&mut out, &module).expect("write succeeds");
    assert!(out.contains("package-namespace Pkg.Core"));
    assert!(out.contains("@cimport \"stdio.h\";"));
    assert!(out.contains("const int A = 1, B = 2;"));
    assert!(out.contains("fn NoBody() -> void;"));
    assert!(out.contains("readonly struct Bag"));
    assert!(out.contains("union Choices"));
    assert!(out.contains("enum Colors"));
    assert!(out.contains("class Example"));
    assert!(out.contains("interface Iface"));
    assert!(out.contains("extension Example"));
    assert!(out.contains("testcase Smoke"));
}
