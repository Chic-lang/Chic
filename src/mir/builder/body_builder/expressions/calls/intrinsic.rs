use super::call_support::{CallBindingInfo, EvaluatedArg};
use super::context::CallContext;
use super::*;
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::{
    AtomicFenceScope, AtomicOrdering, AtomicRmwOp, DecimalIntrinsic, DecimalIntrinsicKind,
    NumericIntrinsic, NumericIntrinsicKind, NumericWidth, Place, Rvalue, SpanTy,
    Statement as MirStatement, StatementKind as MirStatementKind, Ty,
};
use crate::runtime::numeric::numeric_intrinsics_with_pointer;
use crate::syntax::expr::CallArgumentModifier;
use std::collections::HashMap;
use std::sync::OnceLock;

const DECIMAL_INTRINSIC_PREFIX: &str = "Std::Numeric::Decimal::Intrinsics::";
const DECIMAL_INTRINSIC_RESULT_TY: &str = "Std::Numeric::Decimal::DecimalIntrinsicResult";
const DECIMAL_ROUNDING_TY: &str = "Std::Numeric::Decimal::DecimalRoundingMode";
const DECIMAL_VECTORIZE_TY: &str = "Std::Numeric::Decimal::DecimalVectorizeHint";
const ZERO_INIT_INTRINSIC: &str = "Std::Memory::Intrinsics::ZeroInit";
const ZERO_INIT_RAW_INTRINSIC: &str = "Std::Memory::Intrinsics::ZeroInitRaw";
const BOOL_TY: &str = "bool";
const INT_TY: &str = "int";

#[derive(Clone, Copy)]
struct NumericIntrinsicDescriptor {
    kind: NumericIntrinsicKind,
    width: NumericWidth,
    signed: bool,
    operands: usize,
    requires_out: bool,
}

fn map_width(runtime: crate::runtime::numeric::NumericWidth) -> NumericWidth {
    match runtime {
        crate::runtime::numeric::NumericWidth::W8 => NumericWidth::W8,
        crate::runtime::numeric::NumericWidth::W16 => NumericWidth::W16,
        crate::runtime::numeric::NumericWidth::W32 => NumericWidth::W32,
        crate::runtime::numeric::NumericWidth::W64 => NumericWidth::W64,
        crate::runtime::numeric::NumericWidth::W128 => NumericWidth::W128,
        crate::runtime::numeric::NumericWidth::Pointer => NumericWidth::Pointer,
    }
}

fn numeric_intrinsic_descriptor(symbol: &str) -> Option<NumericIntrinsicDescriptor> {
    static CACHE: OnceLock<HashMap<&'static str, NumericIntrinsicDescriptor>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        numeric_intrinsics_with_pointer()
            .into_iter()
            .map(|entry| (entry.symbol, entry))
            .filter_map(|(symbol, entry)| {
                let kind = match entry.kind {
                    crate::runtime::numeric::NumericIntrinsicKind::TryAdd => {
                        NumericIntrinsicKind::TryAdd
                    }
                    crate::runtime::numeric::NumericIntrinsicKind::TrySub => {
                        NumericIntrinsicKind::TrySub
                    }
                    crate::runtime::numeric::NumericIntrinsicKind::TryMul => {
                        NumericIntrinsicKind::TryMul
                    }
                    crate::runtime::numeric::NumericIntrinsicKind::TryNeg => {
                        NumericIntrinsicKind::TryNeg
                    }
                    crate::runtime::numeric::NumericIntrinsicKind::LeadingZeroCount => {
                        NumericIntrinsicKind::LeadingZeroCount
                    }
                    crate::runtime::numeric::NumericIntrinsicKind::TrailingZeroCount => {
                        NumericIntrinsicKind::TrailingZeroCount
                    }
                    crate::runtime::numeric::NumericIntrinsicKind::PopCount => {
                        NumericIntrinsicKind::PopCount
                    }
                    crate::runtime::numeric::NumericIntrinsicKind::RotateLeft => {
                        NumericIntrinsicKind::RotateLeft
                    }
                    crate::runtime::numeric::NumericIntrinsicKind::RotateRight => {
                        NumericIntrinsicKind::RotateRight
                    }
                    crate::runtime::numeric::NumericIntrinsicKind::ReverseEndianness => {
                        NumericIntrinsicKind::ReverseEndianness
                    }
                    crate::runtime::numeric::NumericIntrinsicKind::IsPowerOfTwo => {
                        NumericIntrinsicKind::IsPowerOfTwo
                    }
                };
                Some((
                    symbol,
                    NumericIntrinsicDescriptor {
                        kind,
                        width: map_width(entry.width),
                        signed: entry.signed,
                        operands: entry.operands as usize,
                        requires_out: matches!(
                            kind,
                            NumericIntrinsicKind::TryAdd
                                | NumericIntrinsicKind::TrySub
                                | NumericIntrinsicKind::TryMul
                                | NumericIntrinsicKind::TryNeg
                        ),
                    },
                ))
            })
            .collect()
    });
    cache.get(symbol).copied()
}

fn numeric_result_ty(desc: &NumericIntrinsicDescriptor) -> Option<Ty> {
    match desc.kind {
        NumericIntrinsicKind::TryAdd
        | NumericIntrinsicKind::TrySub
        | NumericIntrinsicKind::TryMul
        | NumericIntrinsicKind::TryNeg
        | NumericIntrinsicKind::IsPowerOfTwo => Some(Ty::named(BOOL_TY)),
        NumericIntrinsicKind::LeadingZeroCount
        | NumericIntrinsicKind::TrailingZeroCount
        | NumericIntrinsicKind::PopCount => Some(Ty::named(INT_TY)),
        NumericIntrinsicKind::RotateLeft
        | NumericIntrinsicKind::RotateRight
        | NumericIntrinsicKind::ReverseEndianness => {
            Some(Ty::named(type_name_for_width(desc.width, desc.signed)))
        }
    }
}

fn type_name_for_width(width: NumericWidth, signed: bool) -> &'static str {
    match width {
        NumericWidth::W8 => {
            if signed {
                "sbyte"
            } else {
                "byte"
            }
        }
        NumericWidth::W16 => {
            if signed {
                "short"
            } else {
                "ushort"
            }
        }
        NumericWidth::W32 => {
            if signed {
                "int"
            } else {
                "uint"
            }
        }
        NumericWidth::W64 => {
            if signed {
                "long"
            } else {
                "ulong"
            }
        }
        NumericWidth::W128 => {
            if signed {
                "int128"
            } else {
                "uint128"
            }
        }
        NumericWidth::Pointer => {
            if signed {
                "nint"
            } else {
                "nuint"
            }
        }
    }
}

#[derive(Clone, Copy)]
struct DecimalIntrinsicDescriptor {
    kind: DecimalIntrinsicKind,
    decimal_args: usize,
    rounding: DecimalRoundingSource,
    vectorize: DecimalVectorizeSource,
}

impl DecimalIntrinsicDescriptor {
    fn expected_arg_count(self) -> usize {
        let mut count = self.decimal_args;
        if matches!(self.rounding, DecimalRoundingSource::Argument(_)) {
            count += 1;
        }
        if matches!(self.vectorize, DecimalVectorizeSource::Argument(_)) {
            count += 1;
        }
        count
    }
}

#[derive(Clone, Copy)]
enum DecimalRoundingSource {
    DefaultTiesToEven,
    Argument(usize),
}

#[derive(Clone, Copy)]
enum DecimalVectorizeSource {
    DefaultNone,
    ForceDecimal,
    Argument(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ZeroInitIntrinsicKind {
    Managed,
    Raw,
}

fn resolve_decimal_intrinsic(name: &str) -> Option<DecimalIntrinsicDescriptor> {
    if !name.starts_with(DECIMAL_INTRINSIC_PREFIX) {
        return None;
    }
    let suffix = &name[DECIMAL_INTRINSIC_PREFIX.len()..];
    use DecimalIntrinsicKind as Kind;
    use DecimalRoundingSource as Round;
    use DecimalVectorizeSource as VecSrc;
    let descriptor = match suffix {
        "Add" => DecimalIntrinsicDescriptor {
            kind: Kind::Add,
            decimal_args: 2,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::DefaultNone,
        },
        "AddWithOptions" => DecimalIntrinsicDescriptor {
            kind: Kind::Add,
            decimal_args: 2,
            rounding: Round::Argument(2),
            vectorize: VecSrc::Argument(3),
        },
        "AddVectorized" => DecimalIntrinsicDescriptor {
            kind: Kind::Add,
            decimal_args: 2,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::ForceDecimal,
        },
        "AddVectorizedWithRounding" => DecimalIntrinsicDescriptor {
            kind: Kind::Add,
            decimal_args: 2,
            rounding: Round::Argument(2),
            vectorize: VecSrc::ForceDecimal,
        },
        "Sub" => DecimalIntrinsicDescriptor {
            kind: Kind::Sub,
            decimal_args: 2,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::DefaultNone,
        },
        "SubWithOptions" => DecimalIntrinsicDescriptor {
            kind: Kind::Sub,
            decimal_args: 2,
            rounding: Round::Argument(2),
            vectorize: VecSrc::Argument(3),
        },
        "SubVectorized" => DecimalIntrinsicDescriptor {
            kind: Kind::Sub,
            decimal_args: 2,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::ForceDecimal,
        },
        "SubVectorizedWithRounding" => DecimalIntrinsicDescriptor {
            kind: Kind::Sub,
            decimal_args: 2,
            rounding: Round::Argument(2),
            vectorize: VecSrc::ForceDecimal,
        },
        "Mul" => DecimalIntrinsicDescriptor {
            kind: Kind::Mul,
            decimal_args: 2,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::DefaultNone,
        },
        "MulWithOptions" => DecimalIntrinsicDescriptor {
            kind: Kind::Mul,
            decimal_args: 2,
            rounding: Round::Argument(2),
            vectorize: VecSrc::Argument(3),
        },
        "MulVectorized" => DecimalIntrinsicDescriptor {
            kind: Kind::Mul,
            decimal_args: 2,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::ForceDecimal,
        },
        "MulVectorizedWithRounding" => DecimalIntrinsicDescriptor {
            kind: Kind::Mul,
            decimal_args: 2,
            rounding: Round::Argument(2),
            vectorize: VecSrc::ForceDecimal,
        },
        "Div" => DecimalIntrinsicDescriptor {
            kind: Kind::Div,
            decimal_args: 2,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::DefaultNone,
        },
        "DivWithOptions" => DecimalIntrinsicDescriptor {
            kind: Kind::Div,
            decimal_args: 2,
            rounding: Round::Argument(2),
            vectorize: VecSrc::Argument(3),
        },
        "DivVectorized" => DecimalIntrinsicDescriptor {
            kind: Kind::Div,
            decimal_args: 2,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::ForceDecimal,
        },
        "DivVectorizedWithRounding" => DecimalIntrinsicDescriptor {
            kind: Kind::Div,
            decimal_args: 2,
            rounding: Round::Argument(2),
            vectorize: VecSrc::ForceDecimal,
        },
        "Rem" => DecimalIntrinsicDescriptor {
            kind: Kind::Rem,
            decimal_args: 2,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::DefaultNone,
        },
        "RemWithOptions" => DecimalIntrinsicDescriptor {
            kind: Kind::Rem,
            decimal_args: 2,
            rounding: Round::Argument(2),
            vectorize: VecSrc::Argument(3),
        },
        "RemVectorized" => DecimalIntrinsicDescriptor {
            kind: Kind::Rem,
            decimal_args: 2,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::ForceDecimal,
        },
        "RemVectorizedWithRounding" => DecimalIntrinsicDescriptor {
            kind: Kind::Rem,
            decimal_args: 2,
            rounding: Round::Argument(2),
            vectorize: VecSrc::ForceDecimal,
        },
        "Fma" => DecimalIntrinsicDescriptor {
            kind: Kind::Fma,
            decimal_args: 3,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::DefaultNone,
        },
        "FmaWithOptions" => DecimalIntrinsicDescriptor {
            kind: Kind::Fma,
            decimal_args: 3,
            rounding: Round::Argument(3),
            vectorize: VecSrc::Argument(4),
        },
        "FmaVectorized" => DecimalIntrinsicDescriptor {
            kind: Kind::Fma,
            decimal_args: 3,
            rounding: Round::DefaultTiesToEven,
            vectorize: VecSrc::ForceDecimal,
        },
        "FmaVectorizedWithRounding" => DecimalIntrinsicDescriptor {
            kind: Kind::Fma,
            decimal_args: 3,
            rounding: Round::Argument(3),
            vectorize: VecSrc::ForceDecimal,
        },
        _ => return None,
    };
    Some(descriptor)
}

fn resolve_zero_init_intrinsic(name: &str) -> Option<ZeroInitIntrinsicKind> {
    let normalized = name.replace('.', "::");
    let trimmed = normalized.strip_prefix("global::").unwrap_or(&normalized);
    let lowered = trimmed.to_ascii_lowercase();
    let mangled = lowered.replace("__", "::");
    let candidates = [lowered.as_str(), mangled.as_str()];

    let base_raw = ZERO_INIT_RAW_INTRINSIC.to_ascii_lowercase();
    let base = ZERO_INIT_INTRINSIC.to_ascii_lowercase();

    for candidate in candidates {
        if candidate == base_raw
            || candidate.starts_with(&format!("{base_raw}<"))
            || candidate.starts_with(&format!("{base_raw}_"))
        {
            return Some(ZeroInitIntrinsicKind::Raw);
        }
        if candidate == base
            || candidate.starts_with(&format!("{base}<"))
            || candidate.starts_with(&format!("{base}_"))
        {
            return Some(ZeroInitIntrinsicKind::Managed);
        }
    }

    None
}

body_builder_impl! {
    pub(super) fn try_lower_decimal_intrinsic(
        &mut self,
        func_operand: &Operand,
        ctx: &CallContext,
        args: &[EvaluatedArg],
        destination_place: Option<Place>,
    ) -> Option<Operand> {
        if ctx.has_receiver() {
            return None;
        }
        let span = ctx.span();
        let canonical = self
            .canonical_name_for_call(func_operand, ctx.info())
            .or_else(|| {
                ctx.info().static_owner.as_ref().and_then(|owner| {
                    ctx.info()
                        .member_name
                        .as_ref()
                        .map(|member| format!("{owner}::{member}"))
                })
            })?;
        let descriptor = resolve_decimal_intrinsic(&canonical)?;
        if args.len() != descriptor.expected_arg_count() {
            return None;
        }

        let lhs = args.get(0)?.operand.clone();
        let rhs = args.get(1)?.operand.clone();
        let addend = if descriptor.decimal_args == 3 {
            Some(args.get(2)?.operand.clone())
        } else {
            None
        };

        let rounding_operand = match descriptor.rounding {
            DecimalRoundingSource::DefaultTiesToEven => {
                self.make_decimal_enum_operand(DECIMAL_ROUNDING_TY, "TiesToEven", 0, span)
            }
            DecimalRoundingSource::Argument(index) => args.get(index).map(|arg| arg.operand.clone())?,
        };

        let vectorize_operand = match descriptor.vectorize {
            DecimalVectorizeSource::DefaultNone => {
                self.make_decimal_enum_operand(DECIMAL_VECTORIZE_TY, "None", 0, span)
            }
            DecimalVectorizeSource::ForceDecimal => {
                self.make_decimal_enum_operand(DECIMAL_VECTORIZE_TY, "Decimal", 1, span)
            }
            DecimalVectorizeSource::Argument(index) => {
                args.get(index).map(|arg| arg.operand.clone())?
            }
        };

        let (place, temp) = match destination_place {
            Some(place) => (place, None),
            None => {
                let temp = self.create_temp(span);
                (Place::new(temp), Some(temp))
            }
        };

        self.hint_local_ty(place.local, Ty::named(DECIMAL_INTRINSIC_RESULT_TY));

        let rvalue = Rvalue::DecimalIntrinsic(DecimalIntrinsic {
            kind: descriptor.kind,
            lhs,
            rhs,
            addend,
            rounding: rounding_operand,
            vectorize: vectorize_operand,
        });

        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: place.clone(),
                value: rvalue,
            },
        });

        if let Some(temp) = temp {
            self.emit_storage_dead(temp, span);
            return Some(Operand::Const(ConstOperand::new(ConstValue::Unit)));
        }

        Some(Operand::Copy(place))
    }

    pub(super) fn try_lower_numeric_intrinsic(
        &mut self,
        func_operand: &Operand,
        ctx: &CallContext,
        args: &[EvaluatedArg],
        destination_place: Option<Place>,
    ) -> Option<Operand> {
        if ctx.has_receiver() {
            return None;
        }
        let span = ctx.span();
        let canonical = self
            .canonical_name_for_call(func_operand, ctx.info())
            .or_else(|| {
                ctx.info().static_owner.as_ref().and_then(|owner| {
                    ctx.info()
                        .member_name
                        .as_ref()
                        .map(|member| format!("{owner}::{member}"))
                })
            })?;
        let descriptor = numeric_intrinsic_descriptor(&canonical)?;
        let expected = descriptor.operands + if descriptor.requires_out { 1 } else { 0 };
        if args.len() != expected {
            return None;
        }

        let mut operands = Vec::with_capacity(descriptor.operands);
        for idx in 0..descriptor.operands {
            operands.push(args.get(idx)?.operand.clone());
        }

        let out_place = if descriptor.requires_out {
            let out_arg = args.get(descriptor.operands)?;
            if !matches!(out_arg.modifier, Some(CallArgumentModifier::Out)) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "`{}` requires its result parameter to be passed with `out`",
                        canonical
                    ),
                    span: out_arg
                        .modifier_span
                        .or(out_arg.span)
                        .or(out_arg.value_span)
                        .or(span),
                });
                return None;
            }
            let place_span = out_arg.value_span.or(out_arg.span).or(span);
            Some(self.operand_to_place(out_arg.operand.clone(), place_span))
        } else {
            None
        };

        let (place, temp) = self.prepare_call_destination(destination_place, span);

        if let Some(result_ty) = numeric_result_ty(&descriptor) {
            self.hint_local_ty(place.local, result_ty);
        }

        let rvalue = Rvalue::NumericIntrinsic(NumericIntrinsic {
            kind: descriptor.kind,
            width: descriptor.width,
            signed: descriptor.signed,
            symbol: canonical.clone(),
            operands,
            out: out_place,
        });

        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: place.clone(),
                value: rvalue,
            },
        });

        if let Some(temp) = temp {
            self.emit_storage_dead(temp, span);
            return Some(Operand::Const(ConstOperand::new(ConstValue::Unit)));
        }

        Some(Operand::Copy(place))
    }

    pub(super) fn try_lower_span_intrinsic(
        &mut self,
        func_operand: &Operand,
        ctx: &CallContext,
        args: &[EvaluatedArg],
        mut destination_place: Option<Place>,
    ) -> Option<Operand> {
        let span = ctx.span();
        let canonical = self.canonical_name_for_call(func_operand, ctx.info());
        let Some(member_name) = ctx
            .info()
            .member_name
            .as_deref()
            .or_else(|| canonical.as_deref().and_then(|name| name.rsplit("::").next()))
        else {
            return None;
        };
        if member_name != "StackAlloc" {
            return None;
        }

        let canonical_owner = canonical
            .as_deref()
            .and_then(|name| name.rsplit_once("::").map(|(owner, _)| owner));
        let owner_matches = canonical_owner
            .map(Self::is_std_span_owner)
            .unwrap_or(false)
            || ctx
                .info()
                .static_owner
                .as_deref()
                .map(Self::is_std_span_owner)
                .unwrap_or(false)
            || ctx
                .info()
                .receiver_owner
                .as_deref()
                .map(Self::is_std_span_owner)
                .unwrap_or(false)
            || ctx
                .info()
                .static_base
                .as_deref()
                .map(Self::is_std_span_owner)
                .unwrap_or(false);
        if !owner_matches {
            return None;
        }

        let call_name = canonical
            .as_deref()
            .map(str::to_string)
            .or_else(|| {
                ctx.info()
                    .static_base
                    .as_ref()
                    .map(|base| format!("{base}.{member_name}"))
            })
            .or_else(|| {
                ctx.info()
                    .static_owner
                    .as_ref()
                    .map(|owner| format!("{owner}::{member_name}"))
            })
            .unwrap_or_else(|| "Span<T>.StackAlloc".to_string());

        if args.len() != 1 {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "`{call_name}` expects a single argument (length or source span)"
                ),
                span,
            });
            return None;
        }

        let arg = args
            .first()
            .expect("argument count verified earlier")
            .clone();

        let source_span_ty = match &arg.operand {
            Operand::Copy(place) | Operand::Move(place) => self.place_ty(place),
            Operand::Borrow(borrow) => self.place_ty(&borrow.place),
            Operand::Mmio(_) | Operand::Const(_) | Operand::Pending(_) => None,
        };

        let mut element_ty = self.resolve_span_element_ty(
            destination_place.as_ref(),
            ctx.info(),
            canonical_owner,
            func_operand,
            &call_name,
            span,
        );
        if let Some(source_span_ty) = source_span_ty.as_ref() {
            let element_from_source = match source_span_ty {
                Ty::Span(span_ty) => Some((*span_ty.element).clone()),
                Ty::ReadOnlySpan(span_ty) => Some((*span_ty.element).clone()),
                _ => None,
            };
            if let Some(source_element) = element_from_source {
                if let Some(existing) = element_ty.as_ref() {
                    if existing != &source_element {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "`{call_name}` source element type `{}` does not match destination `{}`",
                                source_element.canonical_name(),
                                existing.canonical_name()
                            ),
                            span,
                        });
                        return None;
                    }
                } else {
                    element_ty = Some(source_element);
                }
            }
        }
        let element_ty = element_ty?;
        let span_ty = Ty::Span(SpanTy::new(Box::new(element_ty.clone())));

        let mut source_operand = None;
        let mut length_operand = arg.operand.clone();

        if let Some(Ty::Span(_) | Ty::ReadOnlySpan(_)) = source_span_ty {
            let place_span = arg.value_span.or(arg.span).or(span);
            let source_place = self.operand_to_place(length_operand.clone(), place_span);
            let len_local = self.create_temp(place_span);
            self.hint_local_ty(len_local, Ty::named("usize"));
            self.push_statement(MirStatement {
                span: place_span,
                kind: MirStatementKind::Assign {
                    place: Place::new(len_local),
                    value: Rvalue::Len(source_place.clone()),
                },
            });
            length_operand = Operand::Copy(Place::new(len_local));
            source_operand = Some(arg.operand.clone());
        } else {
            length_operand =
                self.coerce_operand_to_ty(length_operand, &Ty::named("usize"), false, span);
        }

        let rvalue = Rvalue::SpanStackAlloc {
            element: element_ty,
            length: length_operand,
            source: source_operand,
        };

        let (place, temp) = match destination_place.take() {
            Some(place) => (place, None),
            None => {
                let temp = self.create_temp(span);
                (Place::new(temp), Some(temp))
            }
        };
        self.ensure_ty_layout_for_ty(&span_ty);
        self.hint_local_ty(place.local, span_ty.clone());

        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: place.clone(),
                value: rvalue,
            },
        });

        if let Some(temp) = temp {
            self.emit_storage_dead(temp, span);
            Some(Operand::Const(ConstOperand::new(ConstValue::Unit)))
        } else {
            Some(Operand::Copy(place))
        }
    }

    pub(super) fn try_lower_zero_init_intrinsic(
        &mut self,
        func_operand: &Operand,
        ctx: &CallContext,
        args: &[EvaluatedArg],
    ) -> Option<Operand> {
        if ctx.has_receiver() {
            return None;
        }
        let kind = self.zero_init_intrinsic_kind(func_operand, ctx.info())?;
        match kind {
            ZeroInitIntrinsicKind::Managed => self.lower_zero_init_managed(args, ctx.span()),
            ZeroInitIntrinsicKind::Raw => self.lower_zero_init_raw(args, ctx.span()),
        }
    }

    pub(super) fn try_lower_atomic_call(
        &mut self,
        func_operand: &Operand,
        ctx: &CallContext,
        args: &[EvaluatedArg],
        destination: Option<Place>,
    ) -> Option<Operand> {
        let span = ctx.span();
        let info = ctx.info();
        if ctx.has_receiver() {
            if let Some(method_name) = info.member_name.as_deref() {
                if let Some(owner) = info
                    .receiver_owner
                    .as_deref()
                    .or(info.static_owner.as_deref())
                {
                    if BodyBuilder::is_atomic_type_name(owner) {
                        return self.lower_atomic_method_call(
                            owner,
                            method_name,
                            args,
                            destination.clone(),
                            span,
                        );
                    }
                }
            }
        }

        let canonical = self.canonical_name_for_call(func_operand, info)?;
        if let Some((owner, method)) = canonical.rsplit_once("::") {
            if BodyBuilder::is_atomic_type_name(owner) {
                return self.lower_atomic_method_call(owner, method, args, destination, span);
            }
        }
        self.lower_atomic_function_call(&canonical, args, span)
    }

    pub(super) fn infer_decimal_ty_from_name(name: &str) -> Option<Ty> {
        let lower = name.to_ascii_lowercase();
        if lower.contains("std::decimal::intrinsics::") {
            Some(Ty::named(
                "Std::Numeric::Decimal::DecimalIntrinsicResult".to_string(),
            ))
        } else if lower.contains("runtimeintrinsics::chic_rt_decimal_matmul") {
            Some(Ty::named("Std::Numeric::Decimal::DecimalStatus".to_string()))
        } else if lower.contains("runtimeintrinsics::chic_rt_decimal_") {
            Some(Ty::named(
                "Std::Numeric::Decimal::DecimalRuntimeCall".to_string(),
            ))
        } else {
            None
        }
    }

    fn zero_init_intrinsic_kind(
        &self,
        func_operand: &Operand,
        info: &CallBindingInfo,
    ) -> Option<ZeroInitIntrinsicKind> {
        if let Some(name) = self.canonical_name_for_call(func_operand, info) {
            if let Some(kind) = resolve_zero_init_intrinsic(&name) {
                return Some(kind);
            }
        }

        if let (Some(owner), Some(member)) = (
            info.static_owner.as_deref(),
            info.member_name.as_deref(),
        ) {
            let candidate = format!("{owner}::{member}");
            if let Some(kind) = resolve_zero_init_intrinsic(&candidate) {
                return Some(kind);
            }
        }

        for candidate in &info.pending_candidates {
            let canonical = candidate.replace('.', "::");
            if let Some(kind) = resolve_zero_init_intrinsic(&canonical) {
                return Some(kind);
            }
        }

        None
    }

    fn lower_zero_init_managed(
        &mut self,
        args: &[EvaluatedArg],
        span: Option<Span>,
    ) -> Option<Operand> {
        if args.len() != 1 {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`Std.Memory.Intrinsics.ZeroInit` expects a single `out` argument".into(),
                span,
            });
            return None;
        }
        let target = &args[0];
        if !matches!(target.modifier, Some(CallArgumentModifier::Out)) {
            self.diagnostics.push(LoweringDiagnostic {
                message:
                    "`Std.Memory.Intrinsics.ZeroInit` requires its argument to be passed with `out`"
                        .into(),
                span: target
                    .modifier_span
                    .or(target.span)
                    .or(target.value_span)
                    .or(span),
            });
            return None;
        }
        let place_span = target.value_span.or(target.span).or(span);
        let place = match &target.operand {
            Operand::Borrow(borrow) => {
                if let Some(last) = self.blocks[self.current_block.0].statements.last() {
                    if matches!(
                        &last.kind,
                        MirStatementKind::Borrow {
                            kind,
                            place,
                            ..
                        } if *kind == borrow.kind
                            && place.local == borrow.place.local
                            && place.projection == borrow.place.projection
                    ) {
                        self.blocks[self.current_block.0].statements.pop();
                    }
                }
                borrow.place.clone()
            }
            _ => self.operand_to_place(target.operand.clone(), place_span),
        };
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::ZeroInit { place },
        });
        Some(Operand::Const(ConstOperand::new(ConstValue::Unit)))
    }

    fn lower_zero_init_raw(
        &mut self,
        args: &[EvaluatedArg],
        span: Option<Span>,
    ) -> Option<Operand> {
        if args.len() != 2 {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`Std.Memory.Intrinsics.ZeroInitRaw` expects pointer and length arguments"
                    .into(),
                span,
            });
            return None;
        }
        let pointer = args[0].operand.clone();
        let length = args[1].operand.clone();
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::ZeroInitRaw { pointer, length },
        });
        Some(Operand::Const(ConstOperand::new(ConstValue::Unit)))
    }

    fn is_std_span_owner(raw_owner: &str) -> bool {
        let with_namespace = raw_owner.replace('.', "::");
        let trimmed = with_namespace
            .strip_prefix("global::")
            .unwrap_or(&with_namespace);
        let base = trimmed.split('<').next().unwrap_or(trimmed);
        matches!(base, "Std::Span::Span" | "Span")
    }

    fn resolve_span_element_ty(
        &mut self,
        destination_place: Option<&Place>,
        info: &CallBindingInfo,
        canonical_owner: Option<&str>,
        func_operand: &Operand,
        call_name: &str,
        span: Option<Span>,
    ) -> Option<Ty> {
        if let Some(place) = destination_place {
            if let Some(local_decl) = self.locals.get(place.local.0) {
                match &local_decl.ty {
                    Ty::Span(span_ty) => {
                        return Some((*span_ty.element).clone());
                    }
                    Ty::Unknown => {}
                    other => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "`{call_name}` must initialize `Span<T>` but attempted to assign `{}`",
                                other.canonical_name()
                            ),
                            span,
                        });
                        return None;
                    }
                }
            }
        }

        let mut candidates: Vec<String> = Vec::new();
        if let Some(base) = info.static_base.as_deref() {
            candidates.push(base.to_string());
        }
        if let Some(owner) = canonical_owner {
            candidates.push(owner.to_string());
        }
        if let Some(owner) = info.static_owner.as_deref() {
            candidates.push(owner.to_string());
        }
        if let Some(owner) = Self::func_operand_owner_repr(func_operand) {
            candidates.push(owner);
        }

        for candidate in candidates {
            if let Some(element_ty) = Self::parse_span_element_ty(&candidate) {
                return Some(element_ty);
            }
        }

        self.diagnostics.push(LoweringDiagnostic {
            message: format!(
                "`{call_name}` requires a concrete `Span<T>` context; supply an explicit type argument"
            ),
            span,
        });
        None
    }

    fn parse_span_element_ty(text: &str) -> Option<Ty> {
        let expr = parse_type_expression_text(text)?;
        let ty = Ty::from_type_expr(&expr);
        if let Ty::Span(span_ty) = ty {
            return Some((*span_ty.element).clone());
        }
        None
    }

    fn make_decimal_enum_operand(
        &mut self,
        type_name: &str,
        variant: &str,
        discriminant: i128,
        span: Option<Span>,
    ) -> Operand {
        let value = ConstValue::Enum {
            type_name: type_name.to_string(),
            variant: variant.to_string(),
            discriminant,
        };
        let value = self.normalise_const(value, span);
        Operand::Const(ConstOperand::new(value))
    }

    fn lower_atomic_method_call(
        &mut self,
        type_name: &str,
        method_name: &str,
        args: &[EvaluatedArg],
        destination: Option<Place>,
        span: Option<Span>,
    ) -> Option<Operand> {
        if args.is_empty() {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "`{type_name}::{method_name}` is missing the atomic receiver operand"
                ),
                span,
            });
            return None;
        }

        let receiver_meta = &args[0];
        let receiver_span = receiver_meta.value_span.or(receiver_meta.span).or(span);
        let target_place = self.operand_to_place(receiver_meta.operand.clone(), receiver_span);
        let qualified = format!("{type_name}::{method_name}");
        let ordering_label = format!("{qualified} ordering");

        match method_name {
            "Load" => {
                let order = if args.len() >= 2 {
                    let arg = &args[1];
                    let order_span = arg.value_span.or(arg.span).or(span);
                    self.atomic_order_from_operand(&arg.operand, order_span, &ordering_label)?
                } else {
                    AtomicOrdering::SeqCst
                };
                let (dest_place, temp_local) = self.prepare_call_destination(destination, span);
                let rvalue = Rvalue::AtomicLoad {
                    target: target_place.clone(),
                    order,
                };
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: dest_place.clone(),
                        value: rvalue,
                    },
                });
                if let Some(temp) = temp_local {
                    self.emit_storage_dead(temp, span);
                    return Some(Operand::Const(ConstOperand::new(ConstValue::Unit)));
                }
                Some(Operand::Copy(dest_place))
            }
            "Store" => {
                if args.len() < 2 {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("`{qualified}` requires a value argument"),
                        span,
                    });
                    return None;
                }
                let value = args[1].operand.clone();
                let order = if args.len() >= 3 {
                    let arg = &args[2];
                    let order_span = arg.value_span.or(arg.span).or(span);
                    self.atomic_order_from_operand(&arg.operand, order_span, &ordering_label)?
                } else {
                    AtomicOrdering::SeqCst
                };
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::AtomicStore {
                        target: target_place,
                        value,
                        order,
                    },
                });
                Some(Operand::Const(ConstOperand::new(ConstValue::Unit)))
            }
            "CompareExchange" | "CompareExchangeWeak" => {
                if args.len() < 5 {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "`{qualified}` expects expected, desired, success order, and failure order arguments"
                        ),
                        span,
                    });
                    return None;
                }
                let expected = args[1].operand.clone();
                let desired = args[2].operand.clone();
                let success_arg = &args[3];
                let failure_arg = &args[4];
                let success_span = success_arg.value_span.or(success_arg.span).or(span);
                let failure_span = failure_arg.value_span.or(failure_arg.span).or(span);
                let success_label = format!("{type_name}::{method_name} success ordering");
                let failure_label = format!("{type_name}::{method_name} failure ordering");
                let success = self.atomic_order_from_operand(
                    &success_arg.operand,
                    success_span,
                    &success_label,
                )?;
                let failure = self.atomic_order_from_operand(
                    &failure_arg.operand,
                    failure_span,
                    &failure_label,
                )?;
                let (dest_place, temp_local) = self.prepare_call_destination(destination, span);
                let rvalue = Rvalue::AtomicCompareExchange {
                    target: target_place.clone(),
                    expected,
                    desired,
                    success,
                    failure,
                    weak: method_name == "CompareExchangeWeak",
                };
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: dest_place.clone(),
                        value: rvalue,
                    },
                });
                self.hint_local_ty(dest_place.local, Ty::named("bool"));
                if let Some(temp) = temp_local {
                    self.emit_storage_dead(temp, span);
                    return Some(Operand::Const(ConstOperand::new(ConstValue::Unit)));
                }
                Some(Operand::Copy(dest_place))
            }
            other => {
                if let Some(op) = Self::atomic_rmw_op(other) {
                    if args.len() < 3 {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "`{qualified}` expects a value and ordering argument"
                            ),
                            span,
                        });
                        return None;
                    }
                    let value = args[1].operand.clone();
                    let order_arg = &args[2];
                    let order_span = order_arg.value_span.or(order_arg.span).or(span);
                    let order = self.atomic_order_from_operand(
                        &order_arg.operand,
                        order_span,
                        &ordering_label,
                    )?;
                    let (dest_place, temp_local) = self.prepare_call_destination(destination, span);
                    let rvalue = Rvalue::AtomicRmw {
                        op,
                        target: target_place.clone(),
                        value,
                        order,
                    };
                    self.push_statement(MirStatement {
                        span,
                        kind: MirStatementKind::Assign {
                            place: dest_place.clone(),
                            value: rvalue,
                        },
                    });
                    if let Some(temp) = temp_local {
                        self.emit_storage_dead(temp, span);
                        return Some(Operand::Const(ConstOperand::new(ConstValue::Unit)));
                    }
                    Some(Operand::Copy(dest_place))
                } else {
                    None
                }
            }
        }
    }

    fn lower_atomic_function_call(
        &mut self,
        canonical: &str,
        args: &[EvaluatedArg],
        span: Option<Span>,
    ) -> Option<Operand> {
        let is_fence = matches!(
            canonical,
            "Std::Sync::Fence"
                | "std::sync::Fence"
                | "Std::Sync::Fences::Fence"
                | "std::sync::Fences::Fence"
        );
        if !is_fence {
            return None;
        }
        if args.len() != 1 {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`{canonical}` expects a single ordering argument"),
                span,
            });
            return None;
        }
        let order_arg = &args[0];
        let order_span = order_arg.value_span.or(order_arg.span).or(span);
        let order = self.atomic_order_from_operand(
            &order_arg.operand,
            order_span,
            "Std.Sync.Fence ordering",
        )?;
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::AtomicFence {
                order,
                scope: AtomicFenceScope::Full,
            },
        });
        Some(Operand::Const(ConstOperand::new(ConstValue::Unit)))
    }

    fn atomic_rmw_op(method: &str) -> Option<AtomicRmwOp> {
        match method {
            "Exchange" => Some(AtomicRmwOp::Exchange),
            "FetchAdd" => Some(AtomicRmwOp::Add),
            "FetchSub" => Some(AtomicRmwOp::Sub),
            "FetchAnd" => Some(AtomicRmwOp::And),
            "FetchOr" => Some(AtomicRmwOp::Or),
            "FetchXor" => Some(AtomicRmwOp::Xor),
            "FetchMin" => Some(AtomicRmwOp::Min),
            "FetchMax" => Some(AtomicRmwOp::Max),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn test_lower_decimal_intrinsic(
        &mut self,
        canonical: &str,
        args: Vec<Operand>,
        destination: Option<Place>,
    ) -> Option<Operand> {
        let call_info = CallBindingInfo {
            canonical_hint: Some(canonical.to_string()),
            pending_candidates: Vec::new(),
            member_name: None,
            receiver_owner: None,
            static_owner: None,
            static_base: None,
            is_constructor: false,
            resolved_symbol: None,
            force_base_receiver: false,
            method_type_args: None,
        };
        let args_meta = args
            .into_iter()
            .map(|operand| EvaluatedArg {
                operand,
                modifier: None,
                modifier_span: None,
                name: None,
                name_span: None,
                span: None,
                value_span: None,
                inline_binding: None,
                param_slot: None,
            })
            .collect::<Vec<_>>();
        let ctx = CallContext::new(None, &call_info, false);
        self.try_lower_decimal_intrinsic(
            &Operand::Const(ConstOperand::new(ConstValue::Symbol(canonical.to_string()))),
            &ctx,
            &args_meta,
            destination,
        )
    }
}
