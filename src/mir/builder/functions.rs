use super::body_builder::opaque_return_info_from_ty;
use super::module_lowering::driver::TypeDeclInfo;
use super::module_lowering::traits::TraitLoweringInfo;
use super::static_registry::StaticRegistry;
use super::symbol_index::SymbolIndex;
use super::*;
use crate::frontend::ast::{
    Attribute, BindingModifier, ConstructorDecl, FunctionDecl, GenericParamKind, GenericParams,
    MemberDispatch, Parameter, PropertyAccessorKind, TestCaseDecl, TypeExpr,
};
use crate::frontend::attributes::{
    AttributeError, OptimizationHints, collect_optimization_hints, extract_conditional_attribute,
};
use crate::frontend::diagnostics::Span;
use crate::frontend::import_resolver::ImportResolver;
use crate::frontend::type_utils::type_expr_surface;
use crate::mir::async_types::task_result_ty;
use crate::mir::builder::FunctionSpecialization;
use crate::mir::data::{Rvalue, StatementKind};
use crate::mir::operators::OperatorRegistry;
use crate::mir::{
    ASYNC_DIAG_ATTRIBUTE, AsyncFramePolicy, AttrSource, FnSig, FrameLimitAttr, MirExternSpec,
    NoCaptureAttr, NoCaptureMode, TestCaseParameterMetadata,
};
use crate::primitives::PrimitiveRegistry;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) struct LoweredFunction {
    pub(super) function: MirFunction,
    pub(super) diagnostics: Vec<LoweringDiagnostic>,
    pub(super) constraints: Vec<TypeConstraint>,
    pub(super) nested_functions: Vec<MirFunction>,
    pub(super) method_metadata: Option<LoweredMethodMetadata>,
    pub(super) test_metadata: Option<TestCaseLoweringMetadata>,
}

#[derive(Clone)]
pub(super) struct LoweredMethodMetadata {
    pub(super) owner: String,
    pub(super) member: String,
    pub(super) dispatch: MemberDispatch,
    pub(super) accessor: Option<PropertyAccessorKind>,
}

#[derive(Clone, Debug)]
pub(super) struct TestCaseLoweringMetadata {
    pub(super) namespace: Option<String>,
    pub(super) categories: Vec<String>,
    pub(super) explicit_id: Option<String>,
    pub(super) span: Option<Span>,
    pub(super) parameters: Vec<TestCaseParameterMetadata>,
}

fn collect_type_param_names(params: Option<&GenericParams>) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(params) = params {
        for param in &params.params {
            if matches!(param.kind, GenericParamKind::Type(_)) {
                names.push(param.name.clone());
            }
        }
    }
    names
}

fn normalized_attr_name(name: &str) -> String {
    let mut name = name.to_ascii_lowercase();
    name.retain(|ch| ch != '_' && ch != '-');
    name
}

fn find_attr<'a>(attrs: &'a [Attribute], target: &str) -> Option<&'a Attribute> {
    attrs
        .iter()
        .find(|attr| normalized_attr_name(&attr.name) == target)
}

pub(super) fn lower_constructor(
    class_name: &str,
    ctor: &ConstructorDecl,
    qualified_name: &str,
    namespace: Option<&str>,
    current_package: Option<&str>,
    type_generics: Option<&GenericParams>,
    type_layouts: &mut TypeLayoutTable,
    type_visibilities: &HashMap<String, TypeDeclInfo>,
    primitive_registry: &PrimitiveRegistry,
    default_arguments: DefaultArgumentStore,
    function_packages: &HashMap<String, String>,
    operator_registry: &OperatorRegistry,
    string_interner: &mut StringInterner,
    symbol_index: &SymbolIndex,
    import_resolver: &ImportResolver,
    static_registry: &StaticRegistry,
    class_bases: &HashMap<String, Vec<String>>,
    class_virtual_slots: &HashMap<String, HashMap<String, u32>>,
    trait_registry: &HashMap<String, TraitLoweringInfo>,
    generic_specializations: Rc<RefCell<Vec<FunctionSpecialization>>>,
    layout: &StructLayout,
) -> LoweredFunction {
    let mut parameters = Vec::with_capacity(ctor.parameters.len() + 1);
    parameters.push(make_constructor_self_parameter(
        class_name,
        type_layouts,
        symbol_index,
    ));
    parameters.extend(ctor.parameters.iter().cloned());

    let sig = FnSig {
        params: parameters
            .iter()
            .map(|param| Ty::from_type_expr(&param.ty))
            .collect(),
        ret: Ty::Unit,
        abi: Abi::Chic,
        effects: Vec::new(),

        lends_to_return: None,

        variadic: false,
    };
    let generic_param_names = collect_type_param_names(type_generics);
    let span = ctor.span;
    let (optimization_hints, mut hint_diagnostics) = collect_function_hints(&ctor.attributes);
    let (async_policy, mut async_attr_diagnostics) =
        collect_async_frame_policy(&ctor.attributes, false);
    hint_diagnostics.append(&mut async_attr_diagnostics);

    let mut builder = BodyBuilder::new(
        &sig,
        span,
        qualified_name,
        false,
        false,
        generic_param_names,
        type_layouts,
        type_visibilities,
        primitive_registry,
        default_arguments.clone(),
        namespace,
        current_package.map(str::to_string),
        function_packages,
        operator_registry,
        string_interner,
        symbol_index,
        import_resolver,
        static_registry,
        class_bases,
        class_virtual_slots,
        trait_registry,
        FunctionKind::Constructor,
        false,
        crate::threading::thread_runtime_mode(),
        sig.lends_to_return.clone(),
        None,
        generic_specializations.clone(),
    );
    builder.set_async_policy(async_policy);
    builder.lower_parameters(&parameters);

    let self_local = builder
        .lookup_name("self")
        .expect("constructor self parameter missing");

    if let Some(initializer) = &ctor.initializer {
        super::constructors::emit_constructor_initializer(
            &mut builder,
            initializer,
            self_local,
            class_name,
        );
    }

    if let Some(body) = &ctor.body {
        builder.lower_block(body);
    }

    let mut lowered = finish_lowering(
        false,
        FunctionKind::Constructor,
        qualified_name,
        span,
        sig,
        builder,
        None,
        optimization_hints,
        hint_diagnostics,
        None,
        false,
        false,
    );

    let mut ctor_diags = super::constructors::check_constructor_field_initialization(
        &lowered.function.body,
        self_local,
        layout,
        ctor.kind,
        ctor.initializer.as_ref(),
        span,
        symbol_index,
    );
    lowered.diagnostics.append(&mut ctor_diags);
    lowered
}

pub(super) fn lower_function(
    func: &FunctionDecl,
    qualified_name: &str,
    kind: FunctionKind,
    namespace: Option<&str>,
    current_package: Option<&str>,
    type_generics: Option<&GenericParams>,
    type_layouts: &mut TypeLayoutTable,
    type_visibilities: &HashMap<String, TypeDeclInfo>,
    primitive_registry: &PrimitiveRegistry,
    default_arguments: DefaultArgumentStore,
    function_packages: &HashMap<String, String>,
    operator_registry: &OperatorRegistry,
    string_interner: &mut StringInterner,
    symbol_index: &SymbolIndex,
    import_resolver: &ImportResolver,
    static_registry: &StaticRegistry,
    class_bases: &HashMap<String, Vec<String>>,
    class_virtual_slots: &HashMap<String, HashMap<String, u32>>,
    trait_registry: &HashMap<String, TraitLoweringInfo>,
    generic_specializations: Rc<RefCell<Vec<FunctionSpecialization>>>,
) -> LoweredFunction {
    let mut signature = func.signature.clone();
    let mut parameters = signature.parameters.clone();
    if matches!(kind, FunctionKind::Method) {
        let is_static = func
            .modifiers
            .iter()
            .any(|modifier| modifier.eq_ignore_ascii_case("static"));
        if !is_static {
            let has_explicit_receiver = parameters.first().is_some_and(|param| {
                param.is_extension_this
                    || param.name.eq_ignore_ascii_case("this")
                    || param.name == "self"
            });
            if !has_explicit_receiver {
                let receiver = Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "self".to_string(),
                    name_span: None,
                    ty: TypeExpr::self_type(),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                };
                parameters.insert(0, receiver);
            } else if let Some(receiver) = parameters.first_mut() {
                if receiver.ty.name.eq_ignore_ascii_case("var")
                    && (receiver.is_extension_this
                        || receiver.name.eq_ignore_ascii_case("this")
                        || receiver.name == "self")
                {
                    receiver.ty = TypeExpr::self_type();
                }
            }
        }
    }
    signature.parameters = parameters.clone();
    let mut sig = lower_signature(&signature);
    let mut generic_param_names = collect_type_param_names(type_generics);
    generic_param_names.extend(collect_type_param_names(func.generics.as_ref()));
    let mut mir_extern_spec = None;
    let extern_abi = func.extern_abi.clone();
    if let Some(abi) = extern_abi {
        if func.is_extern {
            mir_extern_spec = Some(MirExternSpec::from_ast(func.extern_options.as_ref(), &abi));
        }
        sig.abi = Abi::Extern(abi);
    }
    let weak_attr = find_attr(&func.attributes, "weak");
    let mut weak_import_attr = find_attr(&func.attributes, "weakimport");
    let span = func.body.as_ref().and_then(|body| body.span);
    let (optimization_hints, mut hint_diagnostics) = collect_function_hints(&func.attributes);
    let (async_policy, mut async_attr_diagnostics) =
        collect_async_frame_policy(&func.attributes, func.is_async);
    hint_diagnostics.append(&mut async_attr_diagnostics);
    let (conditional_attr, conditional_errors) = extract_conditional_attribute(&func.attributes);
    hint_diagnostics.append(&mut convert_attribute_errors(conditional_errors));
    if let Some(attr) = conditional_attr {
        if !matches!(sig.ret, Ty::Unit) {
            hint_diagnostics.push(LoweringDiagnostic {
                message: "[MIRL0330] `@conditional` requires a void return type".to_string(),
                span: attr.span,
            });
        }
    }
    if weak_attr.is_some() && weak_import_attr.is_some() {
        hint_diagnostics.push(LoweringDiagnostic {
            message:
                "[MIRL0450] `@weak` and `@weak_import` cannot be combined on the same declaration"
                    .to_string(),
            span: weak_import_attr
                .and_then(|attr| attr.span)
                .or_else(|| weak_attr.and_then(|attr| attr.span)),
        });
        weak_import_attr = None;
    }
    let mut is_weak_import = weak_import_attr.is_some();
    if is_weak_import && !func.is_extern {
        hint_diagnostics.push(LoweringDiagnostic {
            message:
                "[MIRL0451] `@weak_import` is only valid on `@extern` declarations without a body"
                    .to_string(),
            span: weak_import_attr.and_then(|attr| attr.span),
        });
        is_weak_import = false;
    }
    if is_weak_import && func.body.is_some() {
        hint_diagnostics.push(LoweringDiagnostic {
            message: "[MIRL0452] `@weak_import` declarations must not provide a body".to_string(),
            span: weak_import_attr.and_then(|attr| attr.span),
        });
        is_weak_import = false;
    }
    if is_weak_import {
        if let Some(spec) = mir_extern_spec.as_mut() {
            spec.weak = true;
        } else {
            hint_diagnostics.push(LoweringDiagnostic {
                message: "[MIRL0453] `@weak_import` requires an explicit `@extern` ABI".to_string(),
                span: weak_import_attr.and_then(|attr| attr.span),
            });
            is_weak_import = false;
        }
    }
    let is_weak = weak_attr.is_some();
    let opaque_return = opaque_return_info_from_ty(&sig.ret, span);
    let mut builder = BodyBuilder::new(
        &sig,
        span,
        qualified_name,
        func.is_async,
        func.is_unsafe,
        generic_param_names,
        type_layouts,
        type_visibilities,
        primitive_registry,
        default_arguments.clone(),
        namespace,
        current_package.map(str::to_string),
        function_packages,
        operator_registry,
        string_interner,
        symbol_index,
        import_resolver,
        static_registry,
        class_bases,
        class_virtual_slots,
        trait_registry,
        kind,
        func.vectorize_hint.map_or(false, |hint| hint.is_decimal()),
        crate::threading::thread_runtime_mode(),
        sig.lends_to_return.clone(),
        opaque_return,
        generic_specializations.clone(),
    );
    builder.set_async_policy(async_policy);
    builder.lower_parameters(&signature.parameters);

    if let Some(body) = &func.body {
        builder.lower_block(body);
    }

    finish_lowering(
        func.is_async,
        kind,
        qualified_name,
        span,
        sig,
        builder,
        mir_extern_spec,
        optimization_hints,
        hint_diagnostics,
        None,
        is_weak,
        is_weak_import,
    )
}

pub(super) fn lower_testcase(
    test: &TestCaseDecl,
    qualified_name: &str,
    namespace: Option<&str>,
    current_package: Option<&str>,
    type_layouts: &mut TypeLayoutTable,
    type_visibilities: &HashMap<String, TypeDeclInfo>,
    primitive_registry: &PrimitiveRegistry,
    default_arguments: DefaultArgumentStore,
    function_packages: &HashMap<String, String>,
    operator_registry: &OperatorRegistry,
    string_interner: &mut StringInterner,
    symbol_index: &SymbolIndex,
    import_resolver: &ImportResolver,
    static_registry: &StaticRegistry,
    class_bases: &HashMap<String, Vec<String>>,
    class_virtual_slots: &HashMap<String, HashMap<String, u32>>,
    trait_registry: &HashMap<String, TraitLoweringInfo>,
    generic_specializations: Rc<RefCell<Vec<FunctionSpecialization>>>,
) -> LoweredFunction {
    let sig = if let Some(sig) = &test.signature {
        lower_signature(sig)
    } else {
        FnSig {
            params: Vec::new(),
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        }
    };

    let span = test.body.span;
    let (optimization_hints, mut hint_diagnostics) = collect_function_hints(&test.attributes);
    let (async_policy, mut async_attr_diagnostics) =
        collect_async_frame_policy(&test.attributes, test.is_async);
    hint_diagnostics.append(&mut async_attr_diagnostics);
    let (categories, explicit_id, mut testcase_attr_diagnostics) =
        collect_testcase_attributes(&test.attributes);
    hint_diagnostics.append(&mut testcase_attr_diagnostics);
    let parameters = testcase_parameters(test.signature.as_ref());
    let opaque_return = opaque_return_info_from_ty(&sig.ret, span);
    let mut builder = BodyBuilder::new(
        &sig,
        span,
        qualified_name,
        test.is_async,
        false,
        Vec::new(),
        type_layouts,
        type_visibilities,
        primitive_registry,
        default_arguments,
        namespace,
        current_package.map(str::to_string),
        function_packages,
        operator_registry,
        string_interner,
        symbol_index,
        import_resolver,
        static_registry,
        class_bases,
        class_virtual_slots,
        trait_registry,
        FunctionKind::Testcase,
        false,
        crate::threading::thread_runtime_mode(),
        sig.lends_to_return.clone(),
        opaque_return,
        generic_specializations.clone(),
    );
    builder.set_async_policy(async_policy);
    if let Some(signature) = &test.signature {
        builder.lower_parameters(&signature.parameters);
    }
    builder.lower_block(&test.body);

    finish_lowering(
        test.is_async,
        FunctionKind::Testcase,
        qualified_name,
        span,
        sig,
        builder,
        None,
        optimization_hints,
        hint_diagnostics,
        Some(TestCaseLoweringMetadata {
            namespace: namespace.map(str::to_string),
            categories,
            explicit_id,
            span,
            parameters,
        }),
        false,
        false,
    )
}

pub(super) fn lower_signature(sig: &crate::frontend::ast::Signature) -> FnSig {
    let lends = sig
        .lends_to_return
        .as_ref()
        .map(|clause| clause.targets.clone());
    let params = sig
        .parameters
        .iter()
        .map(|param| Ty::from_type_expr(&param.ty))
        .collect::<Vec<_>>();
    let effects = sig
        .throws
        .as_ref()
        .map(|clause| {
            clause
                .types
                .iter()
                .map(Ty::from_type_expr)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    FnSig {
        params,
        ret: Ty::from_type_expr(&sig.return_type),
        abi: Abi::Chic,
        effects,
        lends_to_return: lends,
        variadic: sig.variadic,
    }
}

fn collect_function_hints(attrs: &[Attribute]) -> (OptimizationHints, Vec<LoweringDiagnostic>) {
    let (hints, errors) = collect_optimization_hints(attrs);
    (hints, convert_attribute_errors(errors))
}

fn collect_async_frame_policy(
    attrs: &[Attribute],
    is_async: bool,
) -> (AsyncFramePolicy, Vec<LoweringDiagnostic>) {
    let mut policy = AsyncFramePolicy::default();
    let mut diagnostics = Vec::new();

    for attr in attrs {
        let lowered = attr.name.to_ascii_lowercase();
        match lowered.as_str() {
            "stack_only" | "stackonly" => {
                if !is_async {
                    diagnostics.push(async_attr_error(
                        ASYNC_DIAG_ATTRIBUTE,
                        "`@stack_only` is only valid on async functions or testcases",
                        attr.span,
                    ));
                    continue;
                }
                if policy.stack_only.is_some() {
                    diagnostics.push(async_attr_error(
                        ASYNC_DIAG_ATTRIBUTE,
                        "duplicate `@stack_only` attribute",
                        attr.span,
                    ));
                    continue;
                }
                policy.stack_only = Some(AttrSource { span: attr.span });
            }
            "frame_limit" | "framelimit" => {
                if !is_async {
                    diagnostics.push(async_attr_error(
                        ASYNC_DIAG_ATTRIBUTE,
                        "`@frame_limit` is only valid on async functions or testcases",
                        attr.span,
                    ));
                    continue;
                }
                if policy.frame_limit.is_some() {
                    diagnostics.push(async_attr_error(
                        ASYNC_DIAG_ATTRIBUTE,
                        "duplicate `@frame_limit` attribute",
                        attr.span,
                    ));
                    continue;
                }
                match parse_frame_limit(attr) {
                    Ok((bytes, span)) => {
                        policy.frame_limit = Some(FrameLimitAttr {
                            bytes,
                            span: span.or(attr.span),
                        });
                    }
                    Err((message, span)) => diagnostics.push(async_attr_error(
                        ASYNC_DIAG_ATTRIBUTE,
                        message,
                        span.or(attr.span),
                    )),
                }
            }
            "no_capture" | "nocapture" => {
                if !is_async {
                    diagnostics.push(async_attr_error(
                        ASYNC_DIAG_ATTRIBUTE,
                        "`@no_capture` is only valid on async functions or testcases",
                        attr.span,
                    ));
                    continue;
                }
                if policy.no_capture.is_some() {
                    diagnostics.push(async_attr_error(
                        ASYNC_DIAG_ATTRIBUTE,
                        "duplicate `@no_capture` attribute",
                        attr.span,
                    ));
                    continue;
                }
                match parse_no_capture(attr) {
                    Ok((mode, span)) => {
                        policy.no_capture = Some(NoCaptureAttr {
                            mode,
                            span: span.or(attr.span),
                        });
                    }
                    Err((message, span)) => diagnostics.push(async_attr_error(
                        ASYNC_DIAG_ATTRIBUTE,
                        message,
                        span.or(attr.span),
                    )),
                }
            }
            _ => {}
        }
    }

    (policy, diagnostics)
}

fn parse_frame_limit(attr: &Attribute) -> Result<(u64, Option<Span>), (String, Option<Span>)> {
    let Some(argument) = attr.arguments.iter().find(|arg| {
        arg.name
            .as_ref()
            .map_or(true, |name| name.eq_ignore_ascii_case("bytes"))
    }) else {
        return Err((
            "expected byte limit for `@frame_limit`, e.g., `@frame_limit(4096)`".into(),
            attr.span,
        ));
    };
    let span = argument.span;
    let value = argument.value.trim();
    let Some(bytes) = parse_numeric_literal(value) else {
        return Err((
            format!(
                "could not parse `{value}` as a byte count for `@frame_limit` (expected integer literal)"
            ),
            span,
        ));
    };
    if bytes == 0 {
        return Err((
            "`@frame_limit` must be greater than zero bytes".into(),
            span.or(attr.span),
        ));
    }
    Ok((bytes, span))
}

fn parse_no_capture(
    attr: &Attribute,
) -> Result<(NoCaptureMode, Option<Span>), (String, Option<Span>)> {
    if attr.arguments.is_empty() {
        return Ok((NoCaptureMode::Any, attr.span));
    }
    let argument = &attr.arguments[0];
    let value = argument
        .value
        .trim_matches(|ch| ch == '"' || ch == '\'')
        .to_ascii_lowercase();
    let mode = match value.as_str() {
        "any" | "" => NoCaptureMode::Any,
        "move" | "moveonly" | "move_only" | "move-only" => NoCaptureMode::MoveOnly,
        other => {
            return Err((
                format!(
                    "unsupported mode `{other}` for `@no_capture` (expected `move` or omitted)"
                ),
                argument.span,
            ));
        }
    };
    Ok((mode, argument.span.or(attr.span)))
}

fn parse_numeric_literal(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        })
        .unwrap_or(trimmed);
    let cleaned = unquoted.replace('_', "");
    if cleaned.is_empty() {
        return None;
    }
    let (radix, digits) = if let Some(rest) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        (16, rest)
    } else if let Some(rest) = cleaned
        .strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        (2, rest)
    } else if let Some(rest) = cleaned
        .strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        (8, rest)
    } else {
        (10, cleaned.as_str())
    };
    if digits.is_empty() {
        return None;
    }
    u64::from_str_radix(digits, radix).ok()
}

fn async_attr_error(
    code: &str,
    message: impl Into<String>,
    span: Option<Span>,
) -> LoweringDiagnostic {
    LoweringDiagnostic {
        message: format!("[{code}] {}", message.into()),
        span,
    }
}

fn convert_attribute_errors(errors: Vec<AttributeError>) -> Vec<LoweringDiagnostic> {
    errors
        .into_iter()
        .map(|error| LoweringDiagnostic {
            message: error.message,
            span: error.span,
        })
        .collect()
}

fn collect_testcase_attributes(
    attrs: &[Attribute],
) -> (Vec<String>, Option<String>, Vec<LoweringDiagnostic>) {
    let mut categories = Vec::new();
    let mut explicit_id = None;
    let mut diagnostics = Vec::new();

    for attr in attrs {
        let lowered = attr.name.to_ascii_lowercase();
        match lowered.as_str() {
            "category" | "categories" | "tag" | "test_group" | "testgroup" | "group" => {
                if attr.arguments.is_empty() {
                    diagnostics.push(LoweringDiagnostic {
                        message: "testcase category/tag attributes require at least one argument"
                            .to_string(),
                        span: attr.span,
                    });
                }
                for arg in &attr.arguments {
                    let normalized = normalize_testcase_attribute_value(&arg.value);
                    let value = normalized.trim();
                    if value.is_empty() {
                        diagnostics.push(LoweringDiagnostic {
                            message: "empty testcase category/tag argument".to_string(),
                            span: arg.span.or(attr.span),
                        });
                        continue;
                    }
                    let normalized = value.to_ascii_lowercase();
                    if !categories.contains(&normalized) {
                        categories.push(normalized);
                    }
                }
            }
            "id" | "test_id" | "testid" => {
                for arg in &attr.arguments {
                    let normalized = normalize_testcase_attribute_value(&arg.value);
                    let value = normalized.trim();
                    if value.is_empty() {
                        diagnostics.push(LoweringDiagnostic {
                            message: "testcase id cannot be empty".to_string(),
                            span: arg.span.or(attr.span),
                        });
                        continue;
                    }
                    if !value
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
                    {
                        diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "testcase id `{value}` contains unsupported characters; use letters, digits, '-', '_', or '.'"
                            ),
                            span: arg.span.or(attr.span),
                        });
                        continue;
                    }
                    if explicit_id.as_ref().is_some_and(|current| current != value) {
                        diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "duplicate testcase id attributes; keeping `{}` and ignoring `{value}`",
                                explicit_id.as_ref().unwrap()
                            ),
                            span: arg.span.or(attr.span),
                        });
                        continue;
                    }
                    explicit_id = Some(value.to_string());
                }
                if attr.arguments.is_empty() {
                    diagnostics.push(LoweringDiagnostic {
                        message: "testcase id attribute requires a value".to_string(),
                        span: attr.span,
                    });
                }
            }
            _ => diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "attribute `@{}` is not supported on testcase declarations",
                    attr.name
                ),
                span: attr.span,
            }),
        }
    }

    categories.sort();
    categories.dedup();
    (categories, explicit_id, diagnostics)
}

fn normalize_testcase_attribute_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let first = trimmed.as_bytes()[0] as char;
        let last = trimmed.as_bytes()[trimmed.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

fn testcase_parameters(signature: Option<&Signature>) -> Vec<TestCaseParameterMetadata> {
    let Some(signature) = signature else {
        return Vec::new();
    };
    signature
        .parameters
        .iter()
        .map(|param| TestCaseParameterMetadata {
            name: param.name.clone(),
            ty: Some(type_expr_surface(&param.ty)),
            has_default: param.default.is_some(),
        })
        .collect()
}

fn finish_lowering(
    is_async: bool,
    kind: FunctionKind,
    qualified_name: &str,
    span: Option<Span>,
    mut sig: FnSig,
    builder: BodyBuilder<'_>,
    extern_spec: Option<MirExternSpec>,
    optimization_hints: OptimizationHints,
    mut extra_diagnostics: Vec<LoweringDiagnostic>,
    test_metadata: Option<TestCaseLoweringMetadata>,
    is_weak: bool,
    is_weak_import: bool,
) -> LoweredFunction {
    let (body, mut diagnostics, constraints, nested_functions) = builder.finish();
    diagnostics.append(&mut extra_diagnostics);
    if let Some(ret_local) = body.locals.first() {
        sig.ret = ret_local.ty.clone();
    }
    if body.arg_count > 0 {
        let end = 1 + body.arg_count;
        if body.locals.len() >= end {
            sig.params = body.locals[1..end]
                .iter()
                .map(|decl| decl.ty.clone())
                .collect();
        }
    } else {
        sig.params.clear();
    }
    let diag_span = span.or(body.span);
    let has_decimal_intrinsics = body.blocks.iter().any(|block| {
        block.statements.iter().any(|statement| {
            matches!(
                &statement.kind,
                StatementKind::Assign {
                    value: Rvalue::DecimalIntrinsic(_),
                    ..
                }
            )
        })
    });
    if body.vectorize_decimal && !has_decimal_intrinsics {
        diagnostics.push(LoweringDiagnostic {
            message: format!(
                "DM0001: `@vectorize(decimal)` applied to `{qualified_name}` has no effect because the body emits no decimal intrinsics; remove the attribute or adopt `Std.Numeric.Decimal.Fast` helpers."
            ),
            span: diag_span,
        });
    }
    if !body.vectorize_decimal && has_decimal_intrinsics {
        diagnostics.push(LoweringDiagnostic {
            message: format!(
                "DM0002: `{qualified_name}` calls decimal intrinsics without enabling `@vectorize(decimal)`; annotate the function or migrate to `Std.Numeric.Decimal.Fast` APIs for SIMD dispatch."
            ),
            span: diag_span,
        });
    }
    let is_generator = body.generator.is_some();
    let inferred_async_result = if is_async {
        let result_ty = task_result_ty(&sig.ret);
        match kind {
            FunctionKind::Testcase => result_ty.or_else(|| Some(Ty::named("bool"))),
            _ => result_ty,
        }
    } else {
        None
    };
    let function = MirFunction {
        name: qualified_name.to_string(),
        kind,
        signature: sig,
        body,
        is_async,
        async_result: inferred_async_result,
        is_generator,
        span,
        optimization_hints,
        extern_spec,
        is_weak,
        is_weak_import,
    };

    LoweredFunction {
        function,
        diagnostics,
        constraints,
        nested_functions,
        method_metadata: None,
        test_metadata,
    }
}

fn make_constructor_self_parameter(
    owner_name: &str,
    type_layouts: &crate::mir::layout::TypeLayoutTable,
    symbol_index: &SymbolIndex,
) -> Parameter {
    let layout = type_layouts
        .types
        .get(owner_name)
        .or_else(|| {
            type_layouts
                .resolve_type_key(owner_name)
                .and_then(|key| type_layouts.types.get(key))
        })
        .or_else(|| type_layouts.layout_for_name(owner_name));

    let binding = match layout {
        Some(crate::mir::layout::TypeLayout::Class(_)) => BindingModifier::Value,
        Some(_) => BindingModifier::Out,
        None => symbol_index
            .reflection_descriptor(owner_name)
            .map(|descriptor| match descriptor.kind {
                crate::frontend::metadata::reflection::TypeKind::Class => BindingModifier::Value,
                _ => BindingModifier::Out,
            })
            .unwrap_or(BindingModifier::Out),
    };

    Parameter {
        binding,
        binding_nullable: false,
        name: "self".to_string(),
        name_span: None,
        ty: type_expr_from_class(owner_name),
        attributes: Vec::new(),
        di_inject: None,
        default: None,
        default_span: None,
        lends: None,
        is_extension_this: false,
    }
}

fn type_expr_from_class(name: &str) -> TypeExpr {
    TypeExpr {
        name: name.to_string(),
        base: name.split("::").map(str::to_string).collect(),
        suffixes: Vec::new(),
        span: None,
        generic_span: None,
        tuple_elements: None,
        tuple_element_names: None,
        fn_signature: None,
        trait_object: None,
        ref_kind: None,
        is_view: false,
    }
}
