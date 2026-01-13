use super::*;
use crate::chic_kind::ChicKind;
use crate::error::Error;
use crate::frontend::ast::{
    Block, ClassDecl, ClassMember, DocComment, EnumDecl, EnumVariant, ExtensionDecl,
    ExtensionMember, ExtensionMethodDecl, ExternBinding, FieldDecl, FunctionDecl, InlineAttr,
    InterfaceDecl, InterfaceMember, Item, MemberDispatch, Module, NamespaceDecl, Signature,
    StructDecl, TestCaseDecl, TypeExpr, UnionDecl, UnionField, UnionMember, UnionViewDecl,
    UsingDirective, UsingKind, Visibility,
};
use crate::mir::module_metadata::{Export, GlobalAllocator, StdProfile};
use crate::mir::{
    Abi, ConstValue, DefaultArgumentKind, DefaultArgumentRecord, FnSig, FunctionKind, MirBody,
    MirExternSpec, MirFunction, MirModule, Ty,
};
use crate::mir::{
    AutoTraitOverride, AutoTraitSet, FieldLayout, StructLayout, TypeLayout, TypeRepr,
};
use crate::perf::PerfMetadata;
use crate::target::Target;
use object::macho;
use object::{
    Architecture, BinaryFormat, Endianness, Object as _, ObjectSection, ObjectSymbol,
    SymbolKind as ObjSymbolKind,
};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use target_lexicon::{OperatingSystem, Triple};
use tempfile::tempdir;

fn doc(lines: &[&str]) -> DocComment {
    DocComment::new(lines.iter().map(|line| line.to_string()).collect())
}

#[test]
fn encodes_version_components() {
    assert_eq!(debug::encode_macos_version(11, 0, 0), 0x000B_0000);
    assert_eq!(debug::encode_macos_version(13, 3, 1), 0x000D_0301);
}

#[test]
fn macos_build_version_defaults_for_darwin() {
    let version = debug::macos_build_version(&OperatingSystem::Darwin(None))
        .expect("expected build version for darwin targets");
    assert_eq!(version.0, 0x000B_0000);
    assert_eq!(version.1, 0x000B_0000);
}

fn sample_module() -> Module {
    let mut module = Module::new(Some("Root.Core".into()));

    let mut namespace = NamespaceDecl {
        name: "Inner".into(),
        items: Vec::new(),
        doc: Some(doc(&["Namespace docs"])),
        attributes: Vec::new(),
        span: None,
    };

    namespace.items.push(Item::Function(FunctionDecl {
        visibility: Visibility::Public,
        name: "FooFunction".into(),
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
        doc: Some(doc(&["Function", "Doc"])),
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

    namespace.items.push(Item::Struct(StructDecl {
        visibility: Visibility::Public,
        name: "MyStruct".into(),
        fields: vec![FieldDecl {
            visibility: Visibility::Public,
            name: "field".into(),
            ty: TypeExpr::simple("int"),
            initializer: None,
            doc: Some(doc(&["FieldDoc"])),
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
        doc: Some(doc(&["StructDoc"])),
        mmio: None,
        generics: None,
        attributes: Vec::new(),
        is_readonly: false,
        layout: None,
        is_intrinsic: false,
        inline_attr: Some(InlineAttr::Cross),
        is_record: false,
        record_positional_fields: Vec::new(),
    }));

    namespace.items.push(Item::Union(UnionDecl {
        visibility: Visibility::Public,
        name: "MyUnion".into(),
        members: vec![
            UnionMember::Field(UnionField {
                visibility: Visibility::Public,
                name: "value".into(),
                ty: TypeExpr::simple("int"),
                is_readonly: false,
                doc: Some(doc(&["UnionFieldDoc"])),
                attributes: Vec::new(),
            }),
            UnionMember::View(UnionViewDecl {
                visibility: Visibility::Public,
                name: "View".into(),
                fields: vec![FieldDecl {
                    visibility: Visibility::Public,
                    name: "inner".into(),
                    ty: TypeExpr::simple("int"),
                    initializer: None,
                    doc: Some(doc(&["ViewFieldDoc"])),
                    attributes: Vec::new(),
                    mmio: None,
                    is_required: false,
                    display_name: None,
                    is_readonly: false,
                    is_static: false,
                    view_of: None,
                }],
                is_readonly: false,
                doc: Some(doc(&["ViewDoc"])),
                attributes: Vec::new(),
            }),
        ],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: Some(doc(&["UnionDoc"])),
        generics: None,
        attributes: Vec::new(),
    }));

    namespace.items.push(Item::Enum(EnumDecl {
        visibility: Visibility::Public,
        name: "MyEnum".into(),
        underlying_type: None,
        variants: vec![EnumVariant {
            name: "Variant".into(),
            fields: vec![FieldDecl {
                visibility: Visibility::Public,
                name: "payload".into(),
                ty: TypeExpr::simple("int"),
                initializer: None,
                doc: Some(doc(&["EnumFieldDoc"])),
                attributes: Vec::new(),
                mmio: None,
                is_required: false,
                display_name: None,
                is_readonly: false,
                is_static: false,
                view_of: None,
            }],
            discriminant: None,
            doc: Some(doc(&["EnumVariantDoc"])),
        }],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        is_flags: false,
        doc: Some(doc(&["EnumDoc"])),
        generics: None,
        attributes: Vec::new(),
    }));

    namespace.items.push(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: crate::frontend::ast::ClassKind::Class,
        name: "MyClass".into(),
        bases: Vec::new(),
        members: vec![
            ClassMember::Field(FieldDecl {
                visibility: Visibility::Public,
                name: "property".into(),
                ty: TypeExpr::simple("int"),
                initializer: None,
                doc: Some(doc(&["ClassFieldDoc"])),
                attributes: Vec::new(),
                mmio: None,
                is_required: false,
                display_name: None,
                is_readonly: false,
                is_static: false,
                view_of: None,
            }),
            ClassMember::Method(FunctionDecl {
                visibility: Visibility::Public,
                name: "Method".into(),
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
                doc: Some(doc(&["ClassMethodDoc"])),
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
        doc: Some(doc(&["ClassDoc"])),
        generics: None,
        attributes: Vec::new(),
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }));

    namespace.items.push(Item::Interface(InterfaceDecl {
        visibility: Visibility::Public,
        name: "MyInterface".into(),
        bases: Vec::new(),
        members: vec![InterfaceMember::Method(FunctionDecl {
            visibility: Visibility::Public,
            name: "Contract".into(),
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
            doc: Some(doc(&["InterfaceMethodDoc"])),
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
        doc: Some(doc(&["InterfaceDoc"])),
        generics: None,
        attributes: Vec::new(),
    }));

    namespace.items.push(Item::Extension(ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("SomeType"),
        generics: None,
        members: vec![ExtensionMember::Method(ExtensionMethodDecl {
            function: FunctionDecl {
                visibility: Visibility::Public,
                name: "Helper".into(),
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
                doc: Some(doc(&["ExtensionMethodDoc"])),
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
        doc: Some(doc(&["ExtensionDoc"])),
        attributes: Vec::new(),
        conditions: Vec::new(),
    }));

    namespace.items.push(Item::TestCase(TestCaseDecl {
        name: "SampleTest".into(),
        signature: None,
        body: Block {
            statements: Vec::new(),
            span: None,
        },
        is_async: false,
        doc: Some(doc(&["TestDoc"])),
        attributes: Vec::new(),
    }));

    namespace.items.push(Item::Import(UsingDirective {
        doc: Some(doc(&["ImportDoc"])),
        is_global: false,
        span: None,
        kind: UsingKind::Alias {
            alias: "Alias".into(),
            target: "Root.Core.Inner.Target".into(),
        },
    }));

    module.push_item(Item::Namespace(namespace));

    module
}

fn linux_target() -> Target {
    Target::parse("x86_64-unknown-linux-gnu").expect("linux target")
}

#[test]
fn metadata_object_path_appends_meta_suffix() {
    let path = metadata_object_path(Path::new("/tmp/output.o"));
    assert_eq!(path, PathBuf::from("/tmp/output.o.meta.o"));

    let standalone = metadata_object_path(Path::new("artifact"));
    assert_eq!(standalone, PathBuf::from("artifact.meta.o"));
}

#[test]
fn metadata_payload_records_docs() {
    let module = sample_module();
    let target = linux_target();
    let mir = MirModule::default();

    let mut caches = MetadataCaches::default();
    let payload = metadata_payload(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::StaticLibrary,
        None,
        &mut caches,
    );
    assert_eq!(payload.last(), Some(&0), "payload must be null-terminated");

    let text = String::from_utf8(payload[..payload.len() - 1].to_vec()).expect("utf8 payload");
    assert!(text.starts_with("Chic Metadata"), "missing header");
    assert!(text.contains("target-requested=x86_64-unknown-linux-gnu"));
    assert!(text.contains("target-canonical=x86_64-unknown-linux-gnu"));
    assert!(text.contains("kind=static-library"));
    assert!(
        text.contains("doc:Root.Core.Inner=Namespace docs"),
        "namespace doc missing: {text}"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.FooFunction=Function\\nDoc"),
        "function doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyStruct=StructDoc"),
        "struct doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyStruct.field=FieldDoc"),
        "struct field doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyUnion=UnionDoc"),
        "union doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyUnion.value=UnionFieldDoc"),
        "union field doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyUnion.View=ViewDoc"),
        "union view doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyUnion.View.inner=ViewFieldDoc"),
        "union view field doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyEnum=EnumDoc"),
        "enum doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyEnum.Variant=EnumVariantDoc"),
        "enum variant doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyEnum.Variant.payload=EnumFieldDoc"),
        "enum payload doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyClass=ClassDoc"),
        "class doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyClass.property=ClassFieldDoc"),
        "class field doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyClass.Method=ClassMethodDoc"),
        "class method doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyInterface=InterfaceDoc"),
        "interface doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.MyInterface.Contract=InterfaceMethodDoc"),
        "interface method doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.extension SomeType=ExtensionDoc"),
        "extension doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.extension SomeType.Helper=ExtensionMethodDoc"),
        "extension method doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.SampleTest=TestDoc"),
        "testcase doc missing"
    );
    assert!(
        text.contains("doc:Root.Core.Inner.import Alias = Root.Core.Inner.Target=ImportDoc"),
        "import doc missing"
    );
}

#[test]
fn metadata_payload_records_inline_opt_in() {
    let module = sample_module();
    let target = linux_target();
    let mir = MirModule::default();
    let mut caches = MetadataCaches::default();
    let payload = metadata_payload(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::StaticLibrary,
        None,
        &mut caches,
    );

    let text = String::from_utf8(payload[..payload.len() - 1].to_vec()).expect("utf8 payload");
    assert!(
        text.contains("inline:Root.Core.Inner.MyStruct=cross"),
        "inline metadata missing: {text}"
    );
}

#[test]
fn build_metadata_bytes_embeds_payload_and_symbol() {
    let module = sample_module();
    let target = linux_target();
    let mir = MirModule::default();
    let perf = PerfMetadata::default();
    let mut caches = MetadataCaches::default();
    let payload = metadata_payload(
        &module,
        &mir,
        &perf,
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::Executable,
        None,
        &mut caches,
    );

    let object_bytes = build_metadata_bytes(
        &module,
        &mir,
        &perf,
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::Executable,
        None,
    )
    .expect("metadata object");

    let file = object::File::parse(&*object_bytes).expect("parse object");
    let section = file
        .section_by_name(".chic.meta")
        .expect("metadata section present");
    assert_eq!(
        section.data().expect("section data"),
        &payload[..],
        "section data must match payload"
    );

    let symbol = file
        .symbol_by_name("__chic_metadata")
        .expect("metadata symbol present");
    assert_eq!(symbol.kind(), ObjSymbolKind::Data);
    assert_eq!(symbol.size(), payload.len() as u64);
}

#[test]
fn write_metadata_object_creates_parent_directories() {
    let dir = tempdir().expect("tempdir");
    let output = dir.path().join("nested/output.clbin");
    let module = sample_module();
    let target = linux_target();
    let mir = MirModule::default();

    let (path, telemetry) = write_metadata_object(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::StaticLibrary,
        None,
        &output,
    )
    .expect("written path");

    assert!(path.exists(), "metadata file should exist");
    let disk_bytes = std::fs::read(&path).expect("read metadata file");
    let expected = build_metadata_bytes(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::StaticLibrary,
        None,
    )
    .expect("expected bytes");
    assert_eq!(
        disk_bytes, expected,
        "written bytes must match object payload"
    );
    assert_eq!(telemetry.functions.cached_entries, 0);
    assert_eq!(telemetry.types.cached_entries, 6);
}

#[test]
fn metadata_payload_reflects_mir_attributes() {
    let module = Module::new(Some("Kernel".into()));
    let target = linux_target();
    let mut mir = MirModule::default();
    mir.attributes.std_profile = StdProfile::NoStd;
    mir.attributes.global_allocator = Some(GlobalAllocator {
        type_name: "Kernel::Alloc".into(),
        target: Some("AllocShim".into()),
        span: None,
    });
    mir.exports.push(Export {
        function: "Kernel::Entry::Start".into(),
        symbol: "_start".into(),
        span: None,
    });

    let mut caches = MetadataCaches::default();
    let payload = metadata_payload(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::Executable,
        None,
        &mut caches,
    );
    let text = String::from_utf8(payload[..payload.len() - 1].to_vec()).expect("utf8");
    assert!(text.contains("profile=no_std"));
    assert!(text.contains("global_allocator=Kernel::Alloc"));
    assert!(text.contains("global_allocator_target=AllocShim"));
    assert!(text.contains("export:Kernel::Entry::Start=_start"));
}

#[test]
fn metadata_payload_records_extern_definitions() {
    let module = Module::new(Some("Interop".into()));
    let target = linux_target();
    let mut mir = MirModule::default();
    let body = MirBody::new(0, None);
    mir.functions.push(MirFunction {
        name: "Interop::MessageBox".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("string")],
            ret: Ty::named("int"),
            abi: Abi::Extern("system".into()),
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: Some(MirExternSpec {
            convention: "system".into(),
            library: Some("user32".into()),
            alias: Some("MessageBoxW".into()),
            binding: ExternBinding::Eager,
            optional: true,
            charset: Some("utf16".into()),
            weak: false,
        }),
        is_weak: false,
        is_weak_import: false,
    });

    let mut caches = MetadataCaches::default();
    let payload = metadata_payload(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::Executable,
        None,
        &mut caches,
    );
    let text = String::from_utf8(payload[..payload.len() - 1].to_vec()).expect("utf8");
    assert!(text.contains(
        "extern:Interop::MessageBox=convention=system;binding=eager;library=user32;alias=MessageBoxW;optional=true;charset=utf16"
    ));
}

#[test]
fn metadata_payload_records_lending_and_views() {
    let module = Module::new(Some("Demo".into()));
    let target = linux_target();
    let mut mir = MirModule::default();
    let body = MirBody::new(0, None);
    mir.functions.push(MirFunction {
        name: "Demo::Slice".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Str],
            ret: Ty::Str,
            abi: Abi::Chic,
            effects: Vec::new(),
            lends_to_return: Some(vec!["src".into()]),
            variadic: false,
        },
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    });
    mir.type_layouts.types.insert(
        "Demo::LineView".into(),
        TypeLayout::Struct(StructLayout {
            name: "Demo::LineView".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![
                FieldLayout {
                    name: "Text".into(),
                    ty: Ty::String,
                    index: 0,
                    offset: None,
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
                FieldLayout {
                    name: "View".into(),
                    ty: Ty::Str,
                    index: 1,
                    offset: None,
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: Some("Text".into()),
                },
            ],
            positional: Vec::new(),
            list: None,
            size: None,
            align: None,
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );

    let mut caches = MetadataCaches::default();
    let payload = metadata_payload(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::Executable,
        None,
        &mut caches,
    );
    let text = String::from_utf8(payload[..payload.len() - 1].to_vec()).expect("utf8 payload");
    assert!(
        text.contains("lends_return:Demo::Slice=src"),
        "lending metadata missing: {text}"
    );
    assert!(
        text.contains("view:Demo::LineView::View=of:Text"),
        "view metadata missing: {text}"
    );
}

#[test]
fn metadata_payload_records_default_arguments() {
    let module = Module::new(Some("Demo".into()));
    let target = linux_target();
    let mut mir = MirModule::default();
    mir.default_arguments = vec![
        DefaultArgumentRecord {
            function: "Demo::Math::Scale".into(),
            internal: "Demo::Math::Scale".into(),
            param_name: "factor".into(),
            param_index: 1,
            span: None,
            value: DefaultArgumentKind::Const(ConstValue::Int(2)),
        },
        DefaultArgumentRecord {
            function: "Demo::Math::Scale".into(),
            internal: "Demo::Math::Scale".into(),
            param_name: "offset".into(),
            param_index: 2,
            span: None,
            value: DefaultArgumentKind::Thunk {
                symbol: "Demo::Math::Scale::default_arg#2".into(),
                metadata_count: 0,
            },
        },
    ];

    let mut caches = MetadataCaches::default();
    let payload = metadata_payload(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::Executable,
        None,
        &mut caches,
    );
    let text = String::from_utf8(payload[..payload.len() - 1].to_vec()).expect("utf8");
    assert!(
        text.contains(
            "default_arg:Demo::Math::Scale#1=factor|const:Int(2)|internal=Demo::Math::Scale"
        ),
        "missing const metadata: {text}"
    );
    assert!(
        text.contains("default_arg:Demo::Math::Scale#2=offset|thunk:Demo::Math::Scale::default_arg#2;meta=0|internal=Demo::Math::Scale"),
        "missing thunk metadata: {text}"
    );
}

#[test]
fn build_metadata_bytes_errors_on_invalid_triple_string() {
    let module = sample_module();
    let target = linux_target();
    let mir = MirModule::default();
    let err = build_metadata_bytes(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "not a triple",
        ChicKind::Executable,
        None,
    )
    .expect_err("expected parse failure");
    match err {
        Error::Codegen { message, .. } => {
            assert!(
                message.contains("failed to parse target triple 'not a triple'"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected codegen error, found {other:?}"),
    }
}

#[test]
fn build_metadata_bytes_errors_on_unsupported_triple() {
    let module = sample_module();
    let target = linux_target();
    let mir = MirModule::default();
    let err = build_metadata_bytes(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "wasm32-unknown-unknown",
        ChicKind::Executable,
        None,
    )
    .expect_err("expected unsupported triple");
    match err {
        Error::Codegen { message, .. } => assert!(
            message.contains("unsupported target triple 'x86_64-unknown-linux-gnu'"),
            "unexpected message: {message}"
        ),
        other => panic!("expected codegen error, found {other:?}"),
    }
}

#[test]
fn build_metadata_bytes_surfaces_invalid_section_errors() {
    let module = sample_module();
    let target = linux_target();
    let err = build_metadata_bytes_with_writer(
        &module,
        &MirModule::default(),
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::Executable,
        None,
        |_object| -> std::result::Result<Vec<u8>, &str> { Err("invalid section: metadata") },
    )
    .expect_err("expected invalid section failure");
    match err {
        Error::Codegen { message, .. } => assert!(
            message.contains("invalid section"),
            "unexpected message: {message}"
        ),
        other => panic!("expected codegen error, found {other:?}"),
    }
}

#[test]
fn build_metadata_bytes_surfaces_serialise_errors() {
    let module = sample_module();
    let target = linux_target();
    let err = build_metadata_bytes_with_writer(
        &module,
        &MirModule::default(),
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::Executable,
        None,
        |_object| -> std::result::Result<Vec<u8>, &str> { Err("failed to serialise metadata") },
    )
    .expect_err("expected serialisation failure");
    match err {
        Error::Codegen { message, .. } => assert!(
            message.contains("failed to serialise metadata"),
            "unexpected message: {message}"
        ),
        other => panic!("expected codegen error, found {other:?}"),
    }
}

#[test]
fn mach_metadata_includes_build_version_command() {
    let module = Module::new(None);
    let target = Target::parse("x86_64-apple-darwin").expect("parse darwin triple");
    let mir = MirModule::default();
    let bytes = build_metadata_bytes(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-apple-darwin",
        ChicKind::StaticLibrary,
        None,
    )
    .expect("metadata bytes");
    use std::convert::TryInto;
    assert!(bytes.len() > 32, "mach header truncated");
    let ncmds = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    let mut offset = 32usize;
    let mut found = false;
    for _ in 0..ncmds {
        let cmd = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let cmdsize = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
        if cmd == macho::LC_BUILD_VERSION {
            assert!(cmdsize >= 20, "build version command too small");
            let platform = u32::from_le_bytes(bytes[offset + 8..offset + 12].try_into().unwrap());
            let minos = u32::from_le_bytes(bytes[offset + 12..offset + 16].try_into().unwrap());
            let sdk = u32::from_le_bytes(bytes[offset + 16..offset + 20].try_into().unwrap());
            assert_eq!(platform, macho::PLATFORM_MACOS);
            assert_eq!(minos, 0x000B_0000);
            assert_eq!(sdk, 0x000B_0000);
            found = true;
            break;
        }
        offset += cmdsize as usize;
    }
    assert!(found, "LC_BUILD_VERSION load command missing");
}

#[test]
fn map_triple_supports_known_targets() {
    let darwin = Triple::from_str("x86_64-apple-darwin").expect("darwin triple");
    let (format, arch, endianness) =
        debug::map_triple(&darwin).expect("darwin triple should map successfully");
    assert_eq!(format, BinaryFormat::MachO);
    assert_eq!(arch, Architecture::X86_64);
    assert_eq!(endianness, Endianness::Little);

    let linux = Triple::from_str("aarch64-unknown-linux-gnu").expect("linux triple");
    let (format, arch, endianness) =
        debug::map_triple(&linux).expect("linux triple should map successfully");
    assert_eq!(format, BinaryFormat::Elf);
    assert_eq!(arch, Architecture::Aarch64);
    assert_eq!(endianness, Endianness::Little);

    let wasm = Triple::from_str("wasm32-unknown-unknown").expect("wasm triple");
    assert!(
        debug::map_triple(&wasm).is_none(),
        "wasm32 should be unsupported for metadata"
    );

    let windows = Triple::from_str("x86_64-pc-windows-msvc").expect("windows triple");
    assert!(
        debug::map_triple(&windows).is_none(),
        "windows should be unsupported for metadata"
    );
}

#[test]
fn section_name_for_format_matches_binary_format() {
    assert_eq!(
        debug::section_name_for_format(BinaryFormat::MachO),
        b"__chxmeta".to_vec()
    );
    assert_eq!(
        debug::section_name_for_format(BinaryFormat::Coff),
        b".chicxmeta".to_vec()
    );
    assert_eq!(
        debug::section_name_for_format(BinaryFormat::Elf),
        b".chic.meta".to_vec()
    );
    assert_eq!(
        debug::reflection_section_name_for_format(BinaryFormat::MachO),
        b"__chxreflect".to_vec()
    );
    assert_eq!(
        debug::reflection_section_name_for_format(BinaryFormat::Coff),
        b".chicxreflect".to_vec()
    );
    assert_eq!(
        debug::reflection_section_name_for_format(BinaryFormat::Elf),
        b".chic.reflect".to_vec()
    );
}

#[test]
fn metadata_caches_report_telemetry() {
    let module = sample_module();
    let target = linux_target();
    let mut mir = MirModule::default();
    mir.exports.push(Export {
        function: "Root.Core.Inner.MyClass.Method".into(),
        symbol: "_ZN5Root4Core5Inner7MyClass6Method".into(),
        span: None,
    });

    let mut caches = MetadataCaches::default();
    let _ = build_metadata_bytes_with_writer_internal(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::Executable,
        None,
        &mut caches,
        |object| object.write(),
    )
    .expect("metadata bytes");

    let telemetry = caches.telemetry();
    assert_eq!(telemetry.functions.function_hits, 0);
    assert_eq!(telemetry.functions.function_misses, 1);
    assert_eq!(telemetry.functions.cached_entries, 1);
    assert_eq!(telemetry.types.cached_entries, 6);

    let _ = build_metadata_bytes_with_writer_internal(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::Executable,
        None,
        &mut caches,
        |object| object.write(),
    )
    .expect("metadata bytes");

    let telemetry_again = caches.telemetry();
    assert_eq!(telemetry_again.functions.function_hits, 1);
    assert_eq!(telemetry_again.functions.function_misses, 1);
    assert_eq!(telemetry_again.types.type_hits, 6);
    assert_eq!(telemetry_again.types.cached_entries, 6);
}

#[test]
fn metadata_object_embeds_reflection_section() {
    let module = sample_module();
    let target = linux_target();
    let mir = MirModule::default();
    let bytes = build_metadata_bytes(
        &module,
        &mir,
        &PerfMetadata::default(),
        &target,
        "x86_64-unknown-linux-gnu",
        ChicKind::StaticLibrary,
        None,
    )
    .expect("metadata object");

    let obj = object::File::parse(&*bytes).expect("parse metadata object");
    let section_name = debug::reflection_section_name_for_format(obj.format());
    let mut found = false;
    for section in obj.sections() {
        if section.name_bytes() == Ok(section_name.as_slice()) {
            let data = section.data().expect("section data");
            let json: serde_json::Value =
                serde_json::from_slice(data).expect("reflection json should parse");
            assert_eq!(json["version"].as_u64(), Some(2));
            found = true;
            break;
        }
    }
    assert!(found, "reflection section missing from metadata object");
}
