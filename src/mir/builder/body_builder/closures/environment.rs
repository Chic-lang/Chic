use super::super::*;
use super::analysis::CapturedLocal;
use crate::frontend::ast::Expression;
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::AggregateKind;
use crate::mir::layout::{MIN_ALIGN, align_to, pointer_align, pointer_size};
use crate::syntax::expr::{LambdaParam, LambdaParamModifier};

#[derive(Clone, Debug)]
pub(crate) struct LambdaParameterInfo {
    pub(crate) name: String,
    pub(crate) ty: Ty,
    pub(crate) mode: ParamMode,
    pub(crate) mutable: bool,
    pub(crate) is_nullable: bool,
    pub(crate) default: Option<Expression>,
}

pub(crate) fn convert_lambda_parameters(
    builder: &mut BodyBuilder<'_>,
    params: &[LambdaParam],
    span: Option<Span>,
) -> Vec<LambdaParameterInfo> {
    let mut infos = Vec::with_capacity(params.len());
    for param in params {
        let ty = if let Some(text) = &param.ty {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Ty::Unknown
            } else if let Some(expr) = parse_type_expression_text(trimmed) {
                let ty = Ty::from_type_expr(&expr);
                builder.ensure_ty_layout_for_ty(&ty);
                ty
            } else {
                builder.diagnostics.push(LoweringDiagnostic {
                    message: format!("failed to parse type `{trimmed}` in lambda parameter"),
                    span: param.span.or(span),
                });
                Ty::Unknown
            }
        } else {
            Ty::Unknown
        };

        let mode = match param.modifier {
            Some(LambdaParamModifier::In) => ParamMode::In,
            Some(LambdaParamModifier::Ref) => ParamMode::Ref,
            Some(LambdaParamModifier::Out) => ParamMode::Out,
            None => ParamMode::Value,
        };

        let mutable = matches!(mode, ParamMode::Ref | ParamMode::Out);

        infos.push(LambdaParameterInfo {
            name: param.name.clone(),
            ty,
            mode,
            mutable,
            is_nullable: false,
            default: param.default.clone(),
        });
    }
    infos
}

pub(crate) fn register_closure_layout(
    builder: &mut BodyBuilder<'_>,
    ty_name: &str,
    captures: &[CapturedLocal],
) {
    if builder.type_layouts.types.contains_key(ty_name) {
        return;
    }

    let mut fields = Vec::with_capacity(captures.len());
    let mut positional = Vec::with_capacity(captures.len());
    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;
    for (index, capture) in captures.iter().enumerate() {
        let field_name = capture.name.clone();
        let idx = u32::try_from(index).unwrap_or(u32::MAX);
        let (field_size, field_align) = builder
            .type_layouts
            .size_and_align_for_ty(&capture.ty)
            .unwrap_or_else(|| {
                builder.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "unable to compute layout for captured field `{field_name}` in `{ty_name}`"
                    ),
                    span: None,
                });
                (pointer_size(), pointer_align())
            });
        let aligned = align_to(offset, field_align.max(MIN_ALIGN));
        offset = aligned.saturating_add(field_size);
        struct_align = struct_align.max(field_align.max(MIN_ALIGN));
        fields.push(FieldLayout {
            name: field_name.clone(),
            ty: capture.ty.clone(),
            index: idx,
            offset: Some(aligned),
            span: None,
            mmio: None,
            display_name: Some(field_name.clone()),
            is_required: false,
            is_nullable: capture.is_nullable,
            is_readonly: false,
            view_of: None,
        });
        positional.push(PositionalElement {
            field_index: idx,
            name: Some(field_name),
            span: None,
        });
    }

    let layout = StructLayout {
        name: ty_name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields,
        positional,
        list: None,
        size: Some(align_to(offset, struct_align)),
        align: Some(struct_align),
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };

    builder
        .type_layouts
        .types
        .insert(ty_name.to_string(), TypeLayout::Struct(layout));
}

pub(crate) fn closure_temp_operand(
    builder: &mut BodyBuilder<'_>,
    span: Option<Span>,
    ty_name: &str,
    captures: &[CapturedLocal],
) -> Operand {
    let temp = builder.create_temp(span);
    if let Some(local) = builder.locals.get_mut(temp.0) {
        local.ty = Ty::named(ty_name.to_string());
        local.is_nullable = false;
    }

    builder.record_borrow_capture_constraints(captures, span, ty_name);

    let fields = captures
        .iter()
        .map(|capture| Operand::Copy(Place::new(capture.local)))
        .collect::<Vec<_>>();

    builder.push_statement(MirStatement {
        span,
        kind: MirStatementKind::Assign {
            place: Place::new(temp),
            value: Rvalue::Aggregate {
                kind: AggregateKind::Adt {
                    name: ty_name.to_string(),
                    variant: None,
                },
                fields,
            },
        },
    });

    Operand::Copy(Place::new(temp))
}
