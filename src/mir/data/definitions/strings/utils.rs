use super::LocalId;
use super::basic_blocks::{MirBody, ParamMode, Place};
use super::types::{
    Abi, ArcTy, ArrayTy, ConstGenericArg, FnTy, GenericArg, PointerQualifiers, PointerTy, RcTy,
    ReadOnlySpanTy, RefTy, SpanTy, TraitObjectTy, TupleTy, Ty, VecTy,
};
#[cfg(test)]
use crate::frontend::ast::GenericArgument;
use crate::frontend::ast::{FnTypeAbi, PointerModifier, RefKind, TypeExpr, TypeSuffix};
use crate::frontend::diagnostics::Span;
use crate::frontend::parser::parse_type_expression_text;
use crate::frontend::type_utils::{SequenceKind, sequence_descriptor, vector_descriptor};

/// Convert a Chic AST type expression into a MIR type.
pub fn ty_from_type_expr(expr: &TypeExpr) -> Ty {
    if expr.suffixes.is_empty()
        && expr.tuple_elements.is_none()
        && expr.fn_signature.is_none()
        && expr.trait_object.is_none()
        && expr.generic_span.is_none()
        && expr.name.contains('<')
    {
        if let Some(parsed) = parse_type_expression_text(expr.name.as_str()) {
            return ty_from_type_expr(&parsed);
        }
    }
    let mut base_expr = expr.clone();
    let nullable = base_expr
        .suffixes
        .iter()
        .any(|suffix| matches!(suffix, TypeSuffix::Nullable));
    if nullable {
        base_expr
            .suffixes
            .retain(|suffix| !matches!(suffix, TypeSuffix::Nullable));
        while base_expr.name.ends_with('?') {
            base_expr.name.pop();
        }
    }
    let pointer_suffixes = base_expr
        .suffixes
        .iter()
        .filter_map(|suffix| match suffix {
            TypeSuffix::Pointer { mutable, modifiers } => Some((*mutable, modifiers.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !pointer_suffixes.is_empty() {
        base_expr
            .suffixes
            .retain(|suffix| !matches!(suffix, TypeSuffix::Pointer { .. }));
    }

    let generic_args = base_expr
        .generic_arguments()
        .map(|args| args.to_vec())
        .unwrap_or_default();
    if !base_expr.base.is_empty() {
        let mut name_expr = base_expr.clone();
        if !generic_args.is_empty() {
            name_expr
                .suffixes
                .retain(|suffix| !matches!(suffix, TypeSuffix::GenericArgs(_)));
        }
        base_expr.name = render_named_type(&name_expr);
    }
    let mut ty = ty_from_type_expr_inner(&base_expr);
    if !generic_args.is_empty() {
        if let Some(named) = ty.as_named_mut() {
            named.args = generic_args
                .iter()
                .map(|arg| {
                    if let Some(ty_arg) = arg.ty() {
                        GenericArg::Type(ty_from_type_expr(ty_arg))
                    } else {
                        let value = arg.evaluated_value().unwrap_or_else(|| {
                            normalise_const_argument(arg.expression().text.as_str())
                        });
                        GenericArg::Const(ConstGenericArg::new(value))
                    }
                })
                .collect();
        }
    }
    for (mutable, modifiers) in pointer_suffixes {
        let qualifiers = pointer_qualifiers_from_modifiers(&modifiers);
        ty = Ty::Pointer(Box::new(PointerTy::with_qualifiers(
            ty, mutable, qualifiers,
        )));
    }
    if nullable {
        ty = Ty::Nullable(Box::new(ty));
    }
    if let Some(kind) = expr.ref_kind {
        let readonly = matches!(kind, RefKind::ReadOnly);
        ty = Ty::Ref(Box::new(RefTy::new(ty, readonly)));
    }
    ty
}

fn pointer_qualifiers_from_modifiers(modifiers: &[PointerModifier]) -> PointerQualifiers {
    let mut qualifiers = PointerQualifiers::default();
    for modifier in modifiers {
        match modifier {
            PointerModifier::Restrict => {
                qualifiers.restrict = true;
                qualifiers.noalias = true;
            }
            PointerModifier::NoAlias => qualifiers.noalias = true,
            PointerModifier::ReadOnly => qualifiers.readonly = true,
            PointerModifier::Aligned(value) => {
                qualifiers.alignment = Some(*value);
            }
            PointerModifier::ExposeAddress => qualifiers.expose_address = true,
        }
    }
    qualifiers
}

fn ty_from_type_expr_inner(expr: &TypeExpr) -> Ty {
    if expr.tuple_elements.is_none()
        && expr.fn_signature.is_none()
        && expr.trait_object.is_none()
        && expr.base.len() == 1
        && expr.base[0].eq_ignore_ascii_case("var")
    {
        return Ty::Unknown;
    }
    if let Some(object) = expr.trait_object() {
        let traits = object
            .bounds
            .iter()
            .map(render_named_type)
            .collect::<Vec<_>>();
        let ty = if object.opaque_impl {
            TraitObjectTy::new_impl(traits)
        } else {
            TraitObjectTy::new(traits)
        };
        return Ty::TraitObject(ty);
    }
    if expr.tuple_elements.is_none()
        && expr.fn_signature.is_none()
        && expr
            .name
            .rsplit('.')
            .next()
            .map(|part| part.eq_ignore_ascii_case("void"))
            .unwrap_or(false)
    {
        return Ty::Unit;
    }
    if expr.is_tuple() && expr.suffixes.is_empty() {
        if let Some(elements) = expr.tuple_elements() {
            let mapped = elements.iter().map(ty_from_type_expr).collect::<Vec<_>>();
            let ty = if let Some(names) = expr.tuple_element_names() {
                Ty::Tuple(TupleTy::with_names(mapped, names.to_vec()))
            } else {
                Ty::Tuple(TupleTy::new(mapped))
            };
            return ty;
        }
    }
    if let Some(fn_sig) = expr.fn_signature() {
        let params = fn_sig
            .params
            .iter()
            .map(ty_from_type_expr)
            .collect::<Vec<_>>();
        let ret = ty_from_type_expr(fn_sig.return_type.as_ref());
        let abi = match &fn_sig.abi {
            FnTypeAbi::Chic => Abi::Chic,
            FnTypeAbi::Extern(name) => Abi::Extern(name.clone()),
        };
        let param_modes = vec![ParamMode::Value; params.len()];
        return Ty::Fn(FnTy::with_modes(
            params,
            param_modes,
            ret,
            abi,
            fn_sig.variadic,
        ));
    }
    if let Some(descriptor) = sequence_descriptor(expr) {
        let element = Box::new(ty_from_type_expr(descriptor.element));
        match descriptor.kind {
            SequenceKind::Array => Ty::Array(ArrayTy::new(element, descriptor.rank)),
            SequenceKind::Vec => Ty::Vec(VecTy::new(element)),
            SequenceKind::Span => Ty::Span(SpanTy::new(element)),
            SequenceKind::ReadOnlySpan => Ty::ReadOnlySpan(ReadOnlySpanTy::new(element)),
        }
    } else {
        if let Some(vector) = vector_descriptor(expr) {
            let element = Box::new(ty_from_type_expr(vector.element));
            let lanes_text = vector
                .lanes
                .evaluated_value()
                .unwrap_or_else(|| vector.lanes.expression().text.clone());
            let lanes = lanes_text.replace('_', "").parse::<u32>().unwrap_or(0);
            return Ty::Vector(crate::mir::data::definitions::strings::types::VectorTy {
                element,
                lanes,
            });
        }
        if let Some(args) = expr
            .generic_arguments()
            .filter(|args| args.len() == 1 && args[0].ty().is_some())
        {
            let base = expr.base.last().map(String::as_str);
            let element = Box::new(ty_from_type_expr(args[0].ty().unwrap()));
            if matches!(base, Some("Rc")) {
                return Ty::Rc(RcTy::new(element));
            }
            if matches!(base, Some("Arc")) {
                return Ty::Arc(ArcTy::new(element));
            }
        }
        if let Some(pos) = expr
            .suffixes
            .iter()
            .rposition(|suffix| matches!(suffix, TypeSuffix::Array(_)))
        {
            if let TypeSuffix::Array(spec) = &expr.suffixes[pos] {
                let mut element_expr = expr.clone();
                element_expr.suffixes.remove(pos);
                if let Some(idx) = element_expr.name.rfind('[') {
                    element_expr.name.truncate(idx);
                }
                let element_ty = ty_from_type_expr(&element_expr);
                return Ty::Array(ArrayTy::new(Box::new(element_ty), spec.dimensions));
            }
        }
        Ty::named(expr.name.clone())
    }
}

/// Produce the canonical textual name for a MIR type.
pub fn canonical_ty_name(ty: &Ty) -> String {
    match ty {
        Ty::Named(named) => {
            if named.args.is_empty() {
                named.name.clone()
            } else {
                let args = named
                    .args
                    .iter()
                    .map(|arg| match arg {
                        GenericArg::Type(ty) => canonical_ty_name(ty),
                        GenericArg::Const(value) => value.value().to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", named.name, args)
            }
        }
        Ty::Array(array) => {
            let mut text = format!("Array<{}>", canonical_ty_name(&array.element));
            if array.rank > 1 {
                text.push('[');
                text.push_str(&",".repeat(array.rank - 1));
                text.push(']');
            }
            text
        }
        Ty::Vec(vec) => format!("Vec<{}>", canonical_ty_name(&vec.element)),
        Ty::Span(span) => format!("Span<{}>", canonical_ty_name(&span.element)),
        Ty::ReadOnlySpan(span) => {
            format!("ReadOnlySpan<{}>", canonical_ty_name(&span.element))
        }
        Ty::Rc(rc) => format!("Rc<{}>", canonical_ty_name(&rc.element)),
        Ty::Arc(arc) => format!("Arc<{}>", canonical_ty_name(&arc.element)),
        Ty::Tuple(tuple) => canonical_tuple_name(tuple),
        Ty::Fn(fn_ty) => canonical_fn_name(fn_ty),
        Ty::Pointer(pointer) => canonical_pointer_name(pointer),
        Ty::Ref(reference) => {
            let prefix = if reference.readonly {
                "ref readonly "
            } else {
                "ref "
            };
            format!("{prefix}{}", canonical_ty_name(&reference.element))
        }
        Ty::Vector(vector) => {
            format!(
                "vector<{}, {}>",
                canonical_ty_name(&vector.element),
                vector.lanes
            )
        }
        Ty::String => "string".into(),
        Ty::Str => "str".into(),
        Ty::Unit => "void".into(),
        Ty::Unknown => "<unknown>".into(),
        Ty::Nullable(inner) => format!("{}?", canonical_ty_name(inner)),
        Ty::TraitObject(object) => {
            let joined = object.traits.join(" + ");
            let prefix = if object.opaque_impl { "impl" } else { "dyn" };
            format!("{prefix} {joined}")
        }
    }
}

/// Canonical name helper for tuple types.
pub fn canonical_tuple_name(tuple: &TupleTy) -> String {
    let parts = tuple
        .elements
        .iter()
        .map(canonical_ty_name)
        .collect::<Vec<_>>()
        .join(", ");
    format!("({parts})")
}

/// Canonical name helper for function types.
pub fn canonical_fn_name(fn_ty: &FnTy) -> String {
    let abi_suffix = match &fn_ty.abi {
        Abi::Chic => String::new(),
        Abi::Extern(name) => format!(" @extern(\"{name}\")"),
    };
    let params = fn_ty
        .params
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let mode = fn_ty
                .param_modes
                .get(index)
                .copied()
                .unwrap_or(ParamMode::Value);
            let modifier = match mode {
                ParamMode::Value => "",
                ParamMode::In => "in ",
                ParamMode::Ref => "ref ",
                ParamMode::Out => "out ",
            };
            format!("{modifier}{}", canonical_ty_name(ty))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let params = if fn_ty.variadic {
        if params.is_empty() {
            "...".to_string()
        } else {
            format!("{params}, ...")
        }
    } else {
        params
    };
    let ret = canonical_ty_name(&fn_ty.ret);
    format!("fn{abi_suffix}({params}) -> {ret}")
}

/// Construct a `MirBody` with default state for locals/blocks.
pub fn new_mir_body(arg_count: usize, span: Option<Span>) -> MirBody {
    MirBody {
        arg_count,
        locals: Vec::new(),
        blocks: Vec::new(),
        span,
        async_machine: None,
        generator: None,
        exception_regions: Vec::new(),
        vectorize_decimal: false,
        effects: Vec::new(),
        stream_metadata: Vec::new(),
        debug_notes: Vec::new(),
    }
}

/// Convenience constructor for `Place`.
pub fn new_place(local: LocalId) -> Place {
    Place {
        local,
        projection: Vec::new(),
    }
}

fn render_named_type(expr: &TypeExpr) -> String {
    if expr.base.is_empty() {
        return expr.name.clone();
    }
    let mut text = expr.base.join("::");
    for suffix in &expr.suffixes {
        match suffix {
            TypeSuffix::GenericArgs(args) => {
                text.push('<');
                for (index, arg) in args.iter().enumerate() {
                    if index > 0 {
                        text.push_str(", ");
                    }
                    if let Some(ty) = arg.ty() {
                        text.push_str(&ty.name);
                    } else if let Some(value) = arg.evaluated_value() {
                        text.push_str(&value);
                    } else {
                        text.push_str(arg.expression().text.as_str());
                    }
                }
                text.push('>');
            }
            TypeSuffix::Array(spec) => {
                text.push('[');
                if spec.dimensions > 1 {
                    text.push_str(&",".repeat(spec.dimensions - 1));
                }
                text.push(']');
            }
            TypeSuffix::Nullable => text.push('?'),
            TypeSuffix::Qualifier(qualifier) => {
                text.push_str("::");
                text.push_str(qualifier);
            }
            TypeSuffix::Pointer { .. } => {
                // Pointers are stripped before rendering named types.
            }
        }
    }
    text
}

fn canonical_pointer_name(pointer: &PointerTy) -> String {
    let mut qualifier = if pointer.mutable {
        "*mut".to_string()
    } else {
        "*const".to_string()
    };
    if let Some(tokens) = pointer.qualifiers.render_tokens() {
        qualifier.push(' ');
        qualifier.push_str(&tokens);
    }
    let inner = canonical_ty_name(&pointer.element);
    format!("{qualifier} {inner}")
}

fn normalise_const_argument(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        raw.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::expressions::Expression;
    use crate::frontend::ast::{ArrayRankSpecifier, TypeExpr, TypeSuffix};
    use crate::frontend::parser::parse_type_expression_text;
    use crate::mir::data::definitions::strings::types::{GenericArg, PointerTy};

    #[test]
    fn ty_from_type_expr_handles_array_rank() {
        let mut array_expr = TypeExpr::simple("Array");
        array_expr.suffixes.push(TypeSuffix::GenericArgs(vec![
            GenericArgument::from_type_expr(TypeExpr::simple("int")),
        ]));
        array_expr
            .suffixes
            .push(TypeSuffix::Array(ArrayRankSpecifier::new(2)));

        let Ty::Array(array) = ty_from_type_expr(&array_expr) else {
            panic!("expected array type");
        };
        assert_eq!(array.rank, 2);
        assert_eq!(
            canonical_ty_name(&Ty::Array(array.clone())),
            "Array<int>[,]"
        );
    }

    #[test]
    fn canonical_ty_name_handles_nullable_fn() {
        let mut fn_expr = TypeExpr::simple("fn");
        fn_expr.suffixes.push(TypeSuffix::GenericArgs(vec![
            GenericArgument::from_type_expr(TypeExpr::simple("string")),
        ]));
        let inner = Ty::Fn(FnTy::new(vec![Ty::String], Ty::Unit, Abi::Chic));
        let ty = Ty::Nullable(Box::new(inner));
        assert_eq!(canonical_ty_name(&ty), "fn(string) -> void?");
    }

    #[test]
    fn ty_from_type_expr_maps_void_to_unit() {
        let expr = TypeExpr::simple("void");
        let ty = ty_from_type_expr(&expr);
        assert!(matches!(ty, Ty::Unit));
    }

    #[test]
    fn ty_from_type_expr_handles_pointer_chain() {
        let expr =
            parse_type_expression_text("*const *mut Env").expect("failed to parse pointer type");
        let Ty::Pointer(outer) = ty_from_type_expr(&expr) else {
            panic!("expected outer pointer type");
        };
        assert!(!outer.mutable, "outer pointer should be const");
        match &outer.element {
            Ty::Pointer(inner) => {
                let inner = inner.as_ref();
                assert!(inner.mutable, "inner pointer should be mutable");
                match &inner.element {
                    Ty::Named(name) => assert_eq!(name.as_str(), "Env"),
                    other => panic!("expected inner named type, got {other:?}"),
                }
            }
            other => panic!("expected nested pointer, got {other:?}"),
        }
    }

    #[test]
    fn ty_from_type_expr_captures_const_arguments() {
        let mut expr = TypeExpr::simple("Demo.Buffer");
        expr.suffixes.push(TypeSuffix::GenericArgs(vec![
            GenericArgument::from_type_expr(TypeExpr::simple("int")),
            GenericArgument::new(None, Expression::new("4", None)),
        ]));
        let Ty::Named(named) = ty_from_type_expr(&expr) else {
            panic!("expected named type");
        };
        assert_eq!(named.args().len(), 2);
        match &named.args()[0] {
            GenericArg::Type(inner) => assert_eq!(inner.canonical_name(), "int"),
            other => panic!("expected type argument, got {other:?}"),
        }
        match &named.args()[1] {
            GenericArg::Const(value) => assert_eq!(value.value(), "4"),
            other => panic!("expected const argument, got {other:?}"),
        }
    }

    #[test]
    fn canonical_ty_name_includes_const_arguments() {
        let mut expr = TypeExpr::simple("Demo.Buffer");
        expr.suffixes.push(TypeSuffix::GenericArgs(vec![
            GenericArgument::from_type_expr(TypeExpr::simple("int")),
            GenericArgument::new(None, Expression::new("4", None)),
        ]));
        let ty = Ty::from_type_expr(&expr);
        assert_eq!(canonical_ty_name(&ty), "Demo::Buffer<int, 4>");
    }

    #[test]
    fn canonical_ty_name_handles_pointer_chain() {
        let inner = Ty::Pointer(Box::new(PointerTy::new(Ty::named("Sample::Env"), false)));
        let outer = Ty::Pointer(Box::new(PointerTy::new(inner, true)));
        assert_eq!(canonical_ty_name(&outer), "*mut *const Sample::Env");
    }

    #[test]
    fn new_mir_body_initialises_defaults() {
        let body = new_mir_body(2, None);
        assert_eq!(body.arg_count, 2);
        assert!(body.locals.is_empty());
        assert!(body.blocks.is_empty());
        assert!(body.async_machine.is_none());
        assert!(body.generator.is_none());
        assert!(body.exception_regions.is_empty());
        assert!(body.debug_notes.is_empty());
    }

    #[test]
    fn new_place_starts_with_empty_projection() {
        let place = new_place(LocalId(3));
        assert_eq!(place.local.0, 3);
        assert!(place.projection.is_empty());
    }
}
