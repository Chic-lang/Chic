use crate::decimal::{Decimal128, DecimalError, DecimalErrorKind, DecimalRoundingMode};
use crate::drop_glue::drop_type_identity;
use crate::frontend::diagnostics::Span;
use crate::frontend::metadata::{
    AttributeArgument as MetaAttributeArgument, AttributeDescriptor as MetaAttributeDescriptor,
    ConstructorDescriptor as MetaConstructorDescriptor, FieldDescriptor as MetaFieldDescriptor,
    FieldLayoutDescriptor as MetaFieldLayoutDescriptor, LayoutDescriptor as MetaLayoutDescriptor,
    MemberDescriptor as MetaMemberDescriptor, MemberKind as MetaMemberKind,
    MethodDescriptor as MetaMethodDescriptor, ParameterDescriptor as MetaParameterDescriptor,
    ParameterMode as MetaParameterMode, PropertyDescriptor as MetaPropertyDescriptor,
    TypeDescriptor as MetaTypeDescriptor, TypeHandle as MetaTypeHandle, TypeKind as MetaTypeKind,
    TypeLayoutDescriptor as MetaTypeLayoutDescriptor,
    VisibilityDescriptor as MetaVisibilityDescriptor,
};
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::ConstEvalContext;
use crate::mir::DecimalIntrinsicKind;
use crate::mir::builder::const_eval::diagnostics::{self, ConstEvalError};
use crate::mir::builder::support::resolve_type_layout_name;
use crate::mir::data::{ConstValue, Ty};
use crate::mir::layout::{TypeLayout, UnionFieldMode};

use super::super::ConstEvalResult;
use super::super::environment::EvalEnv;

const DECIMAL_STATUS_TYPE: &str = "Std::Numeric::Decimal::DecimalStatus";
const DECIMAL_VECTORIZE_TYPE: &str = "Std::Numeric::Decimal::DecimalIntrinsicVariant";
const DECIMAL_ROUNDING_TYPE: &str = "Std::Numeric::Decimal::DecimalRoundingMode";
const DECIMAL_VECTORIZE_HINT_TYPE: &str = "Std::Numeric::Decimal::DecimalVectorizeHint";
const DECIMAL_INTRINSIC_RESULT_TYPE: &str = "Std::Numeric::Decimal::DecimalIntrinsicResult";
const META_TYPE_DESCRIPTOR: &str = "Std::Meta::TypeDescriptor";
const META_MEMBER_DESCRIPTOR: &str = "Std::Meta::MemberDescriptor";
const META_PARAMETER_DESCRIPTOR: &str = "Std::Meta::ParameterDescriptor";
const META_LAYOUT_DESCRIPTOR: &str = "Std::Meta::LayoutDescriptor";
const META_TYPE_LAYOUT_DESCRIPTOR: &str = "Std::Meta::TypeLayoutDescriptor";
const META_FIELD_LAYOUT_DESCRIPTOR: &str = "Std::Meta::FieldLayoutDescriptor";
const META_TYPE_KIND: &str = "Std::Meta::TypeKind";
const META_MEMBER_KIND: &str = "Std::Meta::MemberKind";
const META_VISIBILITY: &str = "Std::Meta::VisibilityDescriptor";
const META_PARAMETER_MODE: &str = "Std::Meta::ParameterMode";
const META_TYPE_HANDLE: &str = "Std::Meta::TypeHandle";
const META_ATTRIBUTE_DESCRIPTOR: &str = "Std::Meta::AttributeDescriptor";
const META_ATTRIBUTE_ARGUMENT: &str = "Std::Meta::AttributeArgument";
const META_FIELD_DESCRIPTOR: &str = "Std::Meta::FieldDescriptor";
const META_PROPERTY_DESCRIPTOR: &str = "Std::Meta::PropertyDescriptor";
const META_METHOD_DESCRIPTOR: &str = "Std::Meta::MethodDescriptor";
const META_CONSTRUCTOR_DESCRIPTOR: &str = "Std::Meta::ConstructorDescriptor";

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
        if let DecimalRoundingSource::Argument(_) = self.rounding {
            count += 1;
        }
        if let DecimalVectorizeSource::Argument(_) = self.vectorize {
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

fn decimal_intrinsic_descriptor(method: &str) -> Option<DecimalIntrinsicDescriptor> {
    use DecimalIntrinsicKind as Kind;
    use DecimalRoundingSource as Round;
    use DecimalVectorizeSource as VecSrc;
    let descriptor = match method {
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

impl<'a> ConstEvalContext<'a> {
    pub(super) fn try_evaluate_reflect_intrinsic(
        &mut self,
        segments: &[String],
        generics: Option<&[String]>,
        args: &[(Option<String>, ConstEvalResult)],
        env: &EvalEnv<'_, '_>,
    ) -> Result<Option<ConstValue>, ConstEvalError> {
        let is_reflect = match segments.len() {
            1 => segments[0] == "reflect",
            3 => segments[0] == "Std" && segments[1] == "Meta" && segments[2] == "reflect",
            4 => {
                segments[0] == "Std"
                    && segments[1] == "Meta"
                    && segments[2] == "Reflection"
                    && segments[3] == "reflect"
            }
            _ => false,
        };
        if !is_reflect {
            return Ok(None);
        }

        let Some(type_arguments) = generics else {
            return Err(ConstEvalError {
                message: "`reflect<T>()` requires a single type argument".into(),
                span: env.span,
            });
        };
        if type_arguments.len() != 1 {
            return Err(ConstEvalError {
                message: format!(
                    "`reflect<T>()` expects exactly one type argument, found {}",
                    type_arguments.len()
                ),
                span: env.span,
            });
        }
        if !args.is_empty() {
            return Err(ConstEvalError {
                message: "`reflect<T>()` does not accept value arguments".into(),
                span: env.span,
            });
        }

        let type_text = type_arguments[0].trim();
        let type_expr = parse_type_expression_text(type_text).ok_or_else(|| ConstEvalError {
            message: format!("`{type_text}` is not a valid type argument for `reflect<T>()`"),
            span: env.span,
        })?;
        let ty = Ty::from_type_expr(&type_expr);
        let named = ty.as_named().ok_or_else(|| ConstEvalError {
            message: format!(
                "`reflect<T>()` requires a named type; `{type_text}` is not supported"
            ),
            span: env.span,
        })?;
        let mut canonical = named.canonical_path();
        if let Some(resolved) = resolve_type_layout_name(
            self.type_layouts,
            self.import_resolver(),
            env.namespace,
            env.owner,
            &canonical,
        ) {
            canonical = resolved;
        }

        let mut descriptor = self.symbol_index.reflection_descriptor(&canonical).cloned();
        if descriptor.is_none() {
            let alternate = canonical.replace("::", ".");
            if alternate != canonical {
                descriptor = self.symbol_index.reflection_descriptor(&alternate).cloned();
            }
        }
        if descriptor.is_none() {
            let fallback = named.canonical_path();
            descriptor = self.symbol_index.reflection_descriptor(&fallback).cloned();
        }
        if descriptor.is_none() {
            let base = strip_generics_text(&canonical).to_string();
            if base != canonical {
                descriptor = self.symbol_index.reflection_descriptor(&base).cloned();
            }
        }

        let Some(mut descriptor) = descriptor else {
            let message = if self.type_layouts.types.contains_key(&canonical) {
                format!(
                    "`reflect<T>()` can only be used with public types; `{}` is not exported",
                    canonical
                )
            } else {
                format!(
                    "`reflect<T>()` could not find a descriptor for type `{}`",
                    canonical
                )
            };
            return Err(ConstEvalError {
                message,
                span: env.span,
            });
        };

        if descriptor.namespace.is_none() {
            descriptor.namespace = canonical.rsplit_once("::").map(|(ns, _)| ns.to_string());
        }
        descriptor.full_name = canonical.clone();
        if descriptor.type_id.is_none() {
            descriptor.type_id = Some(drop_type_identity(&canonical));
        }
        if descriptor.layout.is_none() {
            descriptor.layout = self.build_type_layout_descriptor(&canonical);
        }

        let value = self.reflect_type_descriptor_value(&descriptor);
        Ok(Some(value))
    }

    pub(super) fn try_evaluate_decimal_intrinsic(
        &mut self,
        segments: &[String],
        args: &[(Option<String>, ConstEvalResult)],
        span: Option<Span>,
    ) -> Result<Option<ConstValue>, ConstEvalError> {
        if segments.len() < 5 {
            return Ok(None);
        }
        if segments[0] != "Std"
            || segments[1] != "Numeric"
            || segments[2] != "Decimal"
            || segments[3] != "Intrinsics"
        {
            return Ok(None);
        }
        let method = segments[4].as_str();
        let Some(descriptor) = decimal_intrinsic_descriptor(method) else {
            return Err(ConstEvalError {
                message: format!("unsupported decimal intrinsic `{method}`"),
                span,
            });
        };

        let expected = descriptor.expected_arg_count();
        if args.len() != expected {
            return Err(ConstEvalError {
                message: format!(
                    "`Std::Numeric::Decimal::Intrinsics::{method}` expects {expected} arguments but received {}",
                    args.len()
                ),
                span,
            });
        }

        let mut decimal_args = Vec::with_capacity(descriptor.decimal_args);
        for index in 0..descriptor.decimal_args {
            let value = args[index].1.value.clone();
            decimal_args.push(self.decimal_from_const(value, span)?);
        }

        let rounding = match descriptor.rounding {
            DecimalRoundingSource::DefaultTiesToEven => DecimalRoundingMode::TiesToEven,
            DecimalRoundingSource::Argument(index) => {
                let value = args[index].1.value.clone();
                self.decimal_rounding_from_value(&value, span)?
            }
        };

        let _vectorized = match descriptor.vectorize {
            DecimalVectorizeSource::DefaultNone => false,
            DecimalVectorizeSource::ForceDecimal => true,
            DecimalVectorizeSource::Argument(index) => {
                let value = args[index].1.value.clone();
                self.decimal_vectorize_from_value(&value, span)?
            }
        };
        let variant_enum = self.enum_const(DECIMAL_VECTORIZE_TYPE, "Scalar", 0);

        let mut status = ("Success", 0i128);
        let mut result_decimal = Decimal128::zero();

        let lhs = decimal_args[0];
        let rhs = decimal_args[1];
        let operation = match descriptor.kind {
            DecimalIntrinsicKind::Add => lhs.add(rhs, rounding),
            DecimalIntrinsicKind::Sub => lhs.sub(rhs, rounding),
            DecimalIntrinsicKind::Mul => lhs.mul(rhs, rounding),
            DecimalIntrinsicKind::Div => lhs.div(rhs, rounding),
            DecimalIntrinsicKind::Rem => lhs.rem(rhs),
            DecimalIntrinsicKind::Fma => {
                let addend = decimal_args.get(2).copied().ok_or_else(|| ConstEvalError {
                    message: "decimal intrinsic `Fma` expects an addend operand".into(),
                    span,
                })?;
                lhs.fma(rhs, addend, rounding)
            }
        };

        match operation {
            Ok(value) => {
                result_decimal = value;
            }
            Err(err) => {
                status = self.decimal_status_from_error(&err);
            }
        }

        if status.0 != "Success" {
            result_decimal = Decimal128::zero();
        }

        let status_enum = self.enum_const(DECIMAL_STATUS_TYPE, status.0, status.1);

        let result = ConstValue::Struct {
            type_name: DECIMAL_INTRINSIC_RESULT_TYPE.to_string(),
            fields: vec![
                ("Status".to_string(), status_enum),
                ("Value".to_string(), ConstValue::Decimal(result_decimal)),
                ("Variant".to_string(), variant_enum),
            ],
        };

        Ok(Some(result))
    }

    fn build_type_layout_descriptor(&self, type_name: &str) -> Option<MetaTypeLayoutDescriptor> {
        let layout = self.type_layouts.layout_for_name(type_name).or_else(|| {
            let base = strip_generics_text(type_name);
            if base == type_name {
                None
            } else {
                self.type_layouts.layout_for_name(base)
            }
        })?;
        match layout {
            TypeLayout::Struct(data) | TypeLayout::Class(data) => {
                let fields = data
                    .fields
                    .iter()
                    .map(|field| MetaFieldLayoutDescriptor {
                        name: field.name.clone(),
                        offset: field.offset.map(|offset| offset as u64),
                        ty: Some(type_handle_from_name(&field.ty.canonical_name())),
                        readonly: Some(field.is_readonly),
                    })
                    .collect();
                Some(MetaTypeLayoutDescriptor {
                    size: data.size.map(|size| size as u64),
                    align: data.align.map(|align| align as u32),
                    fields,
                })
            }
            TypeLayout::Union(data) => {
                let fields = data
                    .views
                    .iter()
                    .map(|view| MetaFieldLayoutDescriptor {
                        name: view.name.clone(),
                        offset: None,
                        ty: Some(type_handle_from_name(&view.ty.canonical_name())),
                        readonly: Some(matches!(view.mode, UnionFieldMode::Readonly)),
                    })
                    .collect();
                Some(MetaTypeLayoutDescriptor {
                    size: data.size.map(|size| size as u64),
                    align: data.align.map(|align| align as u32),
                    fields,
                })
            }
            TypeLayout::Enum(data) => Some(MetaTypeLayoutDescriptor {
                size: data.size.map(|size| size as u64),
                align: data.align.map(|align| align as u32),
                fields: Vec::new(),
            }),
        }
    }

    fn reflect_type_descriptor_value(&self, descriptor: &MetaTypeDescriptor) -> ConstValue {
        ConstValue::Struct {
            type_name: META_TYPE_DESCRIPTOR.to_string(),
            fields: vec![
                (
                    "Namespace".to_string(),
                    self.reflect_optional_string(descriptor.namespace.as_ref()),
                ),
                (
                    "Name".to_string(),
                    ConstValue::RawStr(descriptor.name.clone()),
                ),
                (
                    "FullName".to_string(),
                    ConstValue::RawStr(descriptor.full_name.clone()),
                ),
                (
                    "TypeId".to_string(),
                    self.reflect_optional_uint(descriptor.type_id),
                ),
                (
                    "Kind".to_string(),
                    self.reflect_type_kind_value(descriptor.kind.clone()),
                ),
                (
                    "Visibility".to_string(),
                    self.reflect_visibility_value(descriptor.visibility.clone()),
                ),
                (
                    "IsGeneric".to_string(),
                    ConstValue::Bool(descriptor.is_generic),
                ),
                (
                    "GenericArguments".to_string(),
                    self.reflect_type_handle_list(&descriptor.generic_arguments),
                ),
                (
                    "Bases".to_string(),
                    self.reflect_type_handle_list(&descriptor.bases),
                ),
                (
                    "Attributes".to_string(),
                    self.reflect_attribute_list(&descriptor.attributes),
                ),
                (
                    "UnderlyingType".to_string(),
                    self.reflect_optional_type_handle(descriptor.underlying_type.as_ref()),
                ),
                (
                    "Members".to_string(),
                    self.reflect_member_list(&descriptor.members),
                ),
                (
                    "Layout".to_string(),
                    self.reflect_type_layout_descriptor(descriptor.layout.as_ref()),
                ),
                (
                    "LayoutHints".to_string(),
                    self.reflect_layout_descriptor(descriptor.layout_hints.as_ref()),
                ),
                (
                    "Readonly".to_string(),
                    ConstValue::Bool(descriptor.readonly),
                ),
            ],
        }
    }

    fn reflect_member_descriptor_value(&self, descriptor: &MetaMemberDescriptor) -> ConstValue {
        ConstValue::Struct {
            type_name: META_MEMBER_DESCRIPTOR.to_string(),
            fields: vec![
                (
                    "Name".to_string(),
                    ConstValue::RawStr(descriptor.name.clone()),
                ),
                (
                    "Kind".to_string(),
                    self.reflect_member_kind_value(descriptor.kind.clone()),
                ),
                (
                    "Visibility".to_string(),
                    self.reflect_visibility_value(descriptor.visibility.clone()),
                ),
                (
                    "DeclaringType".to_string(),
                    self.reflect_type_handle(&descriptor.declaring_type),
                ),
                (
                    "Attributes".to_string(),
                    self.reflect_attribute_list(&descriptor.attributes),
                ),
                (
                    "Field".to_string(),
                    descriptor
                        .field
                        .as_ref()
                        .map(|field| self.reflect_field_descriptor_value(field))
                        .unwrap_or(ConstValue::Null),
                ),
                (
                    "Property".to_string(),
                    descriptor
                        .property
                        .as_ref()
                        .map(|prop| self.reflect_property_descriptor_value(prop))
                        .unwrap_or(ConstValue::Null),
                ),
                (
                    "Method".to_string(),
                    descriptor
                        .method
                        .as_ref()
                        .map(|method| self.reflect_method_descriptor_value(method))
                        .unwrap_or(ConstValue::Null),
                ),
                (
                    "Constructor".to_string(),
                    descriptor
                        .constructor
                        .as_ref()
                        .map(|ctor| self.reflect_constructor_descriptor_value(ctor))
                        .unwrap_or(ConstValue::Null),
                ),
                (
                    "Children".to_string(),
                    self.reflect_member_list(&descriptor.children),
                ),
            ],
        }
    }

    fn reflect_field_descriptor_value(&self, descriptor: &MetaFieldDescriptor) -> ConstValue {
        ConstValue::Struct {
            type_name: META_FIELD_DESCRIPTOR.to_string(),
            fields: vec![
                (
                    "FieldType".to_string(),
                    self.reflect_type_handle(&descriptor.field_type),
                ),
                (
                    "IsStatic".to_string(),
                    ConstValue::Bool(descriptor.is_static),
                ),
                (
                    "IsReadonly".to_string(),
                    ConstValue::Bool(descriptor.is_readonly),
                ),
                (
                    "Offset".to_string(),
                    self.reflect_optional_uint(descriptor.offset),
                ),
            ],
        }
    }

    fn reflect_property_descriptor_value(&self, descriptor: &MetaPropertyDescriptor) -> ConstValue {
        ConstValue::Struct {
            type_name: META_PROPERTY_DESCRIPTOR.to_string(),
            fields: vec![
                (
                    "PropertyType".to_string(),
                    self.reflect_type_handle(&descriptor.property_type),
                ),
                (
                    "HasGetter".to_string(),
                    ConstValue::Bool(descriptor.has_getter),
                ),
                (
                    "HasSetter".to_string(),
                    ConstValue::Bool(descriptor.has_setter),
                ),
                ("HasInit".to_string(), ConstValue::Bool(descriptor.has_init)),
                (
                    "Parameters".to_string(),
                    self.reflect_parameter_list(&descriptor.parameters),
                ),
                (
                    "Getter".to_string(),
                    descriptor
                        .getter
                        .as_ref()
                        .map(|getter| self.reflect_method_descriptor_value(getter))
                        .unwrap_or(ConstValue::Null),
                ),
                (
                    "Setter".to_string(),
                    descriptor
                        .setter
                        .as_ref()
                        .map(|setter| self.reflect_method_descriptor_value(setter))
                        .unwrap_or(ConstValue::Null),
                ),
                (
                    "Init".to_string(),
                    descriptor
                        .init
                        .as_ref()
                        .map(|init| self.reflect_method_descriptor_value(init))
                        .unwrap_or(ConstValue::Null),
                ),
            ],
        }
    }

    fn reflect_method_descriptor_value(&self, descriptor: &MetaMethodDescriptor) -> ConstValue {
        ConstValue::Struct {
            type_name: META_METHOD_DESCRIPTOR.to_string(),
            fields: vec![
                (
                    "ReturnType".to_string(),
                    self.reflect_type_handle(&descriptor.return_type),
                ),
                (
                    "Parameters".to_string(),
                    self.reflect_parameter_list(&descriptor.parameters),
                ),
                (
                    "IsStatic".to_string(),
                    ConstValue::Bool(descriptor.is_static),
                ),
                (
                    "IsVirtual".to_string(),
                    ConstValue::Bool(descriptor.is_virtual),
                ),
                (
                    "IsOverride".to_string(),
                    ConstValue::Bool(descriptor.is_override),
                ),
                (
                    "IsAbstract".to_string(),
                    ConstValue::Bool(descriptor.is_abstract),
                ),
                ("IsAsync".to_string(), ConstValue::Bool(descriptor.is_async)),
                (
                    "Throws".to_string(),
                    self.reflect_string_list(&descriptor.throws),
                ),
                (
                    "ExternAbi".to_string(),
                    self.reflect_optional_string(descriptor.extern_abi.as_ref()),
                ),
            ],
        }
    }

    fn reflect_constructor_descriptor_value(
        &self,
        descriptor: &MetaConstructorDescriptor,
    ) -> ConstValue {
        ConstValue::Struct {
            type_name: META_CONSTRUCTOR_DESCRIPTOR.to_string(),
            fields: vec![
                (
                    "Parameters".to_string(),
                    self.reflect_parameter_list(&descriptor.parameters),
                ),
                (
                    "IsDesignated".to_string(),
                    ConstValue::Bool(descriptor.is_designated),
                ),
                (
                    "IsConvenience".to_string(),
                    ConstValue::Bool(descriptor.is_convenience),
                ),
            ],
        }
    }

    fn reflect_parameter_descriptor_value(
        &self,
        descriptor: &MetaParameterDescriptor,
    ) -> ConstValue {
        ConstValue::Struct {
            type_name: META_PARAMETER_DESCRIPTOR.to_string(),
            fields: vec![
                (
                    "Name".to_string(),
                    ConstValue::RawStr(descriptor.name.clone()),
                ),
                (
                    "ParameterType".to_string(),
                    self.reflect_type_handle(&descriptor.parameter_type),
                ),
                (
                    "Mode".to_string(),
                    self.reflect_parameter_mode_value(&descriptor.mode),
                ),
                (
                    "HasDefault".to_string(),
                    ConstValue::Bool(descriptor.has_default),
                ),
                (
                    "DefaultValue".to_string(),
                    self.reflect_optional_string(descriptor.default_value.as_ref()),
                ),
                (
                    "Attributes".to_string(),
                    self.reflect_attribute_list(&descriptor.attributes),
                ),
            ],
        }
    }

    fn reflect_attribute_descriptor_value(
        &self,
        descriptor: &MetaAttributeDescriptor,
    ) -> ConstValue {
        ConstValue::Struct {
            type_name: META_ATTRIBUTE_DESCRIPTOR.to_string(),
            fields: vec![
                (
                    "Name".to_string(),
                    ConstValue::RawStr(descriptor.name.clone()),
                ),
                (
                    "PositionalArgs".to_string(),
                    self.reflect_attribute_argument_list(&descriptor.positional_args),
                ),
                (
                    "NamedArgs".to_string(),
                    self.reflect_attribute_argument_list(&descriptor.named_args),
                ),
            ],
        }
    }

    fn reflect_attribute_argument_value(&self, arg: &MetaAttributeArgument) -> ConstValue {
        ConstValue::Struct {
            type_name: META_ATTRIBUTE_ARGUMENT.to_string(),
            fields: vec![
                (
                    "Name".to_string(),
                    self.reflect_optional_string(arg.name.as_ref()),
                ),
                ("Value".to_string(), ConstValue::RawStr(arg.value.clone())),
            ],
        }
    }

    fn reflect_type_handle(&self, handle: &MetaTypeHandle) -> ConstValue {
        ConstValue::Struct {
            type_name: META_TYPE_HANDLE.to_string(),
            fields: vec![
                ("Name".to_string(), ConstValue::RawStr(handle.name.clone())),
                (
                    "TypeId".to_string(),
                    self.reflect_optional_uint(handle.type_id),
                ),
            ],
        }
    }

    fn reflect_optional_type_handle(&self, handle: Option<&MetaTypeHandle>) -> ConstValue {
        match handle {
            Some(handle) => self.reflect_type_handle(handle),
            None => ConstValue::Null,
        }
    }

    fn reflect_type_handle_list(&self, handles: &[MetaTypeHandle]) -> ConstValue {
        let items = handles
            .iter()
            .map(|handle| self.reflect_type_handle(handle))
            .collect::<Vec<_>>();
        self.reflect_descriptor_list(META_TYPE_HANDLE, items)
    }

    fn reflect_attribute_list(&self, attrs: &[MetaAttributeDescriptor]) -> ConstValue {
        let items = attrs
            .iter()
            .map(|attr| self.reflect_attribute_descriptor_value(attr))
            .collect::<Vec<_>>();
        self.reflect_descriptor_list(META_ATTRIBUTE_DESCRIPTOR, items)
    }

    fn reflect_attribute_argument_list(&self, args: &[MetaAttributeArgument]) -> ConstValue {
        let items = args
            .iter()
            .map(|arg| self.reflect_attribute_argument_value(arg))
            .collect::<Vec<_>>();
        self.reflect_descriptor_list(META_ATTRIBUTE_ARGUMENT, items)
    }

    pub(super) fn reflect_string_list(&self, values: &[String]) -> ConstValue {
        let items = values
            .iter()
            .map(|value| ConstValue::RawStr(value.clone()))
            .collect::<Vec<_>>();
        self.reflect_descriptor_list("string", items)
    }

    fn reflect_member_list(&self, members: &[MetaMemberDescriptor]) -> ConstValue {
        let items = members
            .iter()
            .map(|member| self.reflect_member_descriptor_value(member))
            .collect::<Vec<_>>();
        self.reflect_descriptor_list(META_MEMBER_DESCRIPTOR, items)
    }

    fn reflect_parameter_list(&self, parameters: &[MetaParameterDescriptor]) -> ConstValue {
        let items = parameters
            .iter()
            .map(|parameter| self.reflect_parameter_descriptor_value(parameter))
            .collect::<Vec<_>>();
        self.reflect_descriptor_list(META_PARAMETER_DESCRIPTOR, items)
    }

    fn reflect_field_layout_list(&self, fields: &[MetaFieldLayoutDescriptor]) -> ConstValue {
        let items = fields
            .iter()
            .map(|field| self.reflect_field_layout_descriptor_value(field))
            .collect::<Vec<_>>();
        self.reflect_descriptor_list(META_FIELD_LAYOUT_DESCRIPTOR, items)
    }

    pub(super) fn reflect_descriptor_list(
        &self,
        element_type: &str,
        values: Vec<ConstValue>,
    ) -> ConstValue {
        let list_type = format!("Std::Meta::DescriptorList<{element_type}>");
        let mut tail = ConstValue::Struct {
            type_name: list_type.clone(),
            fields: vec![
                ("IsEmpty".to_string(), ConstValue::Bool(true)),
                ("Head".to_string(), ConstValue::Unknown),
                ("Tail".to_string(), ConstValue::Null),
            ],
        };
        for value in values.into_iter().rev() {
            tail = ConstValue::Struct {
                type_name: list_type.clone(),
                fields: vec![
                    ("IsEmpty".to_string(), ConstValue::Bool(false)),
                    ("Head".to_string(), value),
                    ("Tail".to_string(), tail),
                ],
            };
        }
        tail
    }

    fn reflect_optional_string(&self, value: Option<&String>) -> ConstValue {
        match value {
            Some(text) => ConstValue::RawStr(text.clone()),
            None => ConstValue::Null,
        }
    }

    fn reflect_optional_bool(&self, value: Option<bool>) -> ConstValue {
        match value {
            Some(flag) => ConstValue::Bool(flag),
            None => ConstValue::Null,
        }
    }

    fn reflect_optional_uint(&self, value: Option<u64>) -> ConstValue {
        match value {
            Some(number) => ConstValue::UInt(u128::from(number)),
            None => ConstValue::Null,
        }
    }

    fn reflect_layout_descriptor(&self, descriptor: Option<&MetaLayoutDescriptor>) -> ConstValue {
        let Some(layout) = descriptor else {
            return ConstValue::Null;
        };
        ConstValue::Struct {
            type_name: META_LAYOUT_DESCRIPTOR.to_string(),
            fields: vec![
                ("ReprC".to_string(), ConstValue::Bool(layout.repr_c)),
                (
                    "Pack".to_string(),
                    self.reflect_optional_uint(layout.pack.map(u64::from)),
                ),
                (
                    "Align".to_string(),
                    self.reflect_optional_uint(layout.align.map(u64::from)),
                ),
            ],
        }
    }

    fn reflect_type_layout_descriptor(
        &self,
        descriptor: Option<&MetaTypeLayoutDescriptor>,
    ) -> ConstValue {
        let Some(layout) = descriptor else {
            return ConstValue::Null;
        };
        ConstValue::Struct {
            type_name: META_TYPE_LAYOUT_DESCRIPTOR.to_string(),
            fields: vec![
                ("Size".to_string(), self.reflect_optional_uint(layout.size)),
                (
                    "Align".to_string(),
                    self.reflect_optional_uint(layout.align.map(u64::from)),
                ),
                (
                    "Fields".to_string(),
                    self.reflect_field_layout_list(&layout.fields),
                ),
            ],
        }
    }

    fn reflect_field_layout_descriptor_value(
        &self,
        descriptor: &MetaFieldLayoutDescriptor,
    ) -> ConstValue {
        ConstValue::Struct {
            type_name: META_FIELD_LAYOUT_DESCRIPTOR.to_string(),
            fields: vec![
                (
                    "Name".to_string(),
                    ConstValue::RawStr(descriptor.name.clone()),
                ),
                (
                    "Offset".to_string(),
                    self.reflect_optional_uint(descriptor.offset),
                ),
                (
                    "Type".to_string(),
                    self.reflect_optional_type_handle(descriptor.ty.as_ref()),
                ),
                (
                    "Readonly".to_string(),
                    self.reflect_optional_bool(descriptor.readonly),
                ),
            ],
        }
    }

    fn reflect_type_kind_value(&self, kind: MetaTypeKind) -> ConstValue {
        match kind {
            MetaTypeKind::Struct => self.enum_const(META_TYPE_KIND, "Struct", 0),
            MetaTypeKind::Record => self.enum_const(META_TYPE_KIND, "Record", 1),
            MetaTypeKind::Class => self.enum_const(META_TYPE_KIND, "Class", 2),
            MetaTypeKind::Enum => self.enum_const(META_TYPE_KIND, "Enum", 3),
            MetaTypeKind::Interface => self.enum_const(META_TYPE_KIND, "Interface", 4),
            MetaTypeKind::Union => self.enum_const(META_TYPE_KIND, "Union", 5),
            MetaTypeKind::Extension => self.enum_const(META_TYPE_KIND, "Extension", 6),
            MetaTypeKind::Trait => self.enum_const(META_TYPE_KIND, "Trait", 7),
            MetaTypeKind::Delegate => self.enum_const(META_TYPE_KIND, "Delegate", 8),
            MetaTypeKind::Impl => self.enum_const(META_TYPE_KIND, "Impl", 9),
            MetaTypeKind::Function => self.enum_const(META_TYPE_KIND, "Function", 10),
            MetaTypeKind::Const => self.enum_const(META_TYPE_KIND, "Const", 11),
            MetaTypeKind::Static => self.enum_const(META_TYPE_KIND, "Static", 12),
        }
    }

    fn reflect_visibility_value(&self, visibility: MetaVisibilityDescriptor) -> ConstValue {
        match visibility {
            MetaVisibilityDescriptor::Public => self.enum_const(META_VISIBILITY, "Public", 0),
            MetaVisibilityDescriptor::Internal => self.enum_const(META_VISIBILITY, "Internal", 1),
            MetaVisibilityDescriptor::Protected => self.enum_const(META_VISIBILITY, "Protected", 2),
            MetaVisibilityDescriptor::Private => self.enum_const(META_VISIBILITY, "Private", 3),
            MetaVisibilityDescriptor::ProtectedInternal => {
                self.enum_const(META_VISIBILITY, "ProtectedInternal", 4)
            }
            MetaVisibilityDescriptor::PrivateProtected => {
                self.enum_const(META_VISIBILITY, "PrivateProtected", 5)
            }
        }
    }

    fn reflect_member_kind_value(&self, kind: MetaMemberKind) -> ConstValue {
        match kind {
            MetaMemberKind::Field => self.enum_const(META_MEMBER_KIND, "Field", 0),
            MetaMemberKind::Property => self.enum_const(META_MEMBER_KIND, "Property", 1),
            MetaMemberKind::Method => self.enum_const(META_MEMBER_KIND, "Method", 2),
            MetaMemberKind::Constructor => self.enum_const(META_MEMBER_KIND, "Constructor", 3),
            MetaMemberKind::Const => self.enum_const(META_MEMBER_KIND, "Const", 4),
            MetaMemberKind::EnumVariant => self.enum_const(META_MEMBER_KIND, "EnumVariant", 5),
            MetaMemberKind::UnionField => self.enum_const(META_MEMBER_KIND, "UnionField", 6),
            MetaMemberKind::UnionView => self.enum_const(META_MEMBER_KIND, "UnionView", 7),
            MetaMemberKind::AssociatedType => {
                self.enum_const(META_MEMBER_KIND, "AssociatedType", 8)
            }
            MetaMemberKind::ExtensionMethod => {
                self.enum_const(META_MEMBER_KIND, "ExtensionMethod", 9)
            }
            MetaMemberKind::TraitMethod => self.enum_const(META_MEMBER_KIND, "TraitMethod", 10),
        }
    }

    fn reflect_parameter_mode_value(&self, mode: &MetaParameterMode) -> ConstValue {
        match mode {
            MetaParameterMode::In => self.enum_const(META_PARAMETER_MODE, "In", 0),
            MetaParameterMode::Ref => self.enum_const(META_PARAMETER_MODE, "Ref", 1),
            MetaParameterMode::Out => self.enum_const(META_PARAMETER_MODE, "Out", 2),
            MetaParameterMode::Value => self.enum_const(META_PARAMETER_MODE, "Value", 3),
        }
    }

    fn decimal_from_const(
        &mut self,
        value: ConstValue,
        span: Option<Span>,
    ) -> Result<Decimal128, ConstEvalError> {
        match value {
            ConstValue::Decimal(decimal) => Ok(decimal),
            other => {
                let converted = self.convert_decimal(other, span)?;
                if let ConstValue::Decimal(decimal) = converted {
                    Ok(decimal)
                } else {
                    unreachable!("convert_decimal must yield decimal value")
                }
            }
        }
    }

    fn decimal_rounding_from_value(
        &self,
        value: &ConstValue,
        span: Option<Span>,
    ) -> Result<DecimalRoundingMode, ConstEvalError> {
        match value {
            ConstValue::Enum {
                type_name,
                discriminant,
                variant,
            } if type_name == DECIMAL_ROUNDING_TYPE
                || diagnostics::simple_name(type_name)
                    == diagnostics::simple_name(DECIMAL_ROUNDING_TYPE) =>
            {
                if let Ok(raw) = u32::try_from(*discriminant) {
                    if let Some(mode) = DecimalRoundingMode::from_discriminant(raw) {
                        return Ok(mode);
                    }
                }
                match variant.as_str() {
                    "TiesToEven" => Ok(DecimalRoundingMode::TiesToEven),
                    "TowardZero" => Ok(DecimalRoundingMode::TowardZero),
                    "AwayFromZero" => Ok(DecimalRoundingMode::AwayFromZero),
                    "TowardPositive" => Ok(DecimalRoundingMode::TowardPositive),
                    "TowardNegative" => Ok(DecimalRoundingMode::TowardNegative),
                    other => Err(ConstEvalError {
                        message: format!("`{other}` is not a valid DecimalRoundingMode value"),
                        span,
                    }),
                }
            }
            ConstValue::Enum { type_name, .. } => Err(ConstEvalError {
                message: format!("expected `{DECIMAL_ROUNDING_TYPE}` value, found `{type_name}`"),
                span,
            }),
            other => Err(ConstEvalError {
                message: format!("expected `{DECIMAL_ROUNDING_TYPE}` value, found {other:?}"),
                span,
            }),
        }
    }

    fn decimal_vectorize_from_value(
        &self,
        value: &ConstValue,
        span: Option<Span>,
    ) -> Result<bool, ConstEvalError> {
        match value {
            ConstValue::Enum {
                type_name,
                discriminant,
                variant,
            } if type_name == DECIMAL_VECTORIZE_HINT_TYPE
                || diagnostics::simple_name(type_name)
                    == diagnostics::simple_name(DECIMAL_VECTORIZE_HINT_TYPE) =>
            {
                if let Ok(raw) = i32::try_from(*discriminant) {
                    return Ok(raw != 0);
                }
                match variant.as_str() {
                    "Decimal" => Ok(true),
                    "None" => Ok(false),
                    other => Err(ConstEvalError {
                        message: format!("`{other}` is not a valid DecimalVectorizeHint value"),
                        span,
                    }),
                }
            }
            ConstValue::Enum { type_name, .. } => Err(ConstEvalError {
                message: format!(
                    "expected `{DECIMAL_VECTORIZE_HINT_TYPE}` value, found `{type_name}`"
                ),
                span,
            }),
            other => Err(ConstEvalError {
                message: format!("expected `{DECIMAL_VECTORIZE_HINT_TYPE}` value, found {other:?}"),
                span,
            }),
        }
    }

    pub(super) fn enum_const(
        &self,
        type_name: &str,
        variant: &str,
        discriminant: i128,
    ) -> ConstValue {
        ConstValue::Enum {
            type_name: type_name.to_string(),
            variant: variant.to_string(),
            discriminant,
        }
    }

    fn decimal_status_from_error(&self, err: &DecimalError) -> (&'static str, i128) {
        match err.kind() {
            DecimalErrorKind::Overflow => ("Overflow", 1),
            DecimalErrorKind::DivideByZero => ("DivideByZero", 2),
            DecimalErrorKind::InvalidLiteral(_) | DecimalErrorKind::InvalidConversion(_) => {
                ("InvalidOperand", 6)
            }
        }
    }
}

fn strip_generics_text(name: &str) -> &str {
    name.split('<').next().unwrap_or(name)
}

fn type_handle_from_name(name: &str) -> MetaTypeHandle {
    MetaTypeHandle {
        name: name.to_string(),
        type_id: Some(drop_type_identity(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::parser::parse_module;
    use crate::mir::TypeLayoutTable;
    use crate::mir::builder::symbol_index::SymbolIndex;

    fn demo_env() -> EvalEnv<'static, 'static> {
        EvalEnv {
            namespace: Some("Demo"),
            owner: None,
            span: None,
            params: None,
            locals: None,
        }
    }

    #[test]
    fn reflect_intrinsic_yields_descriptor_for_public_struct() {
        let parsed = parse_module(
            r#"
namespace Demo;

public struct Point {
    public int X;
    public int Y;
}
"#,
        )
        .expect("module parses");
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}",
            parsed.diagnostics
        );
        let mut symbol_index = SymbolIndex::build(&parsed.module);
        let mut layouts = TypeLayoutTable::default();
        let mut ctx = ConstEvalContext::new(&mut symbol_index, &mut layouts, None);
        let segments = vec!["Std".to_string(), "Meta".to_string(), "reflect".to_string()];
        let generics = vec!["Demo::Point".to_string()];
        let value = ctx
            .try_evaluate_reflect_intrinsic(&segments, Some(&generics), &[], &demo_env())
            .expect("intrinsic evaluation succeeds")
            .expect("reflect returns descriptor");
        match value {
            ConstValue::Struct { type_name, fields } => {
                assert_eq!(type_name, META_TYPE_DESCRIPTOR);
                let name_field = fields
                    .iter()
                    .find(|(name, _)| name == "Name")
                    .expect("name field present");
                match &name_field.1 {
                    ConstValue::RawStr(name) => assert_eq!(name, "Demo::Point"),
                    other => panic!("unexpected name field value: {:?}", other),
                }
            }
            other => panic!("unexpected reflect result: {:?}", other),
        }
    }

    #[test]
    fn reflect_intrinsic_requires_type_argument() {
        let mut symbol_index = SymbolIndex::default();
        let mut layouts = TypeLayoutTable::default();
        let mut ctx = ConstEvalContext::new(&mut symbol_index, &mut layouts, None);
        let segments = vec!["Std".to_string(), "Meta".to_string(), "reflect".to_string()];
        let err = ctx
            .try_evaluate_reflect_intrinsic(&segments, None, &[], &demo_env())
            .expect_err("missing type argument should fail");
        assert!(
            err.message.contains("requires a single type argument"),
            "unexpected error: {}",
            err.message
        );
    }

    #[test]
    fn decimal_intrinsic_add_returns_success_struct() {
        let mut symbol_index = SymbolIndex::default();
        let mut layouts = TypeLayoutTable::default();
        let mut ctx = ConstEvalContext::new(&mut symbol_index, &mut layouts, None);
        let segments = vec![
            "Std".to_string(),
            "Numeric".to_string(),
            "Decimal".to_string(),
            "Intrinsics".to_string(),
            "Add".to_string(),
        ];
        let lhs = Decimal128::parse_literal("1.25").expect("literal parses");
        let rhs = Decimal128::parse_literal("2.75").expect("literal parses");
        let args = vec![
            (None, ConstEvalResult::new(ConstValue::Decimal(lhs))),
            (None, ConstEvalResult::new(ConstValue::Decimal(rhs))),
        ];
        let value = ctx
            .try_evaluate_decimal_intrinsic(&segments, &args, None)
            .expect("decimal evaluation succeeds")
            .expect("intrinsic produces struct");
        match value {
            ConstValue::Struct { type_name, fields } => {
                assert_eq!(type_name, DECIMAL_INTRINSIC_RESULT_TYPE);
                let status_field = fields
                    .iter()
                    .find(|(name, _)| name == "Status")
                    .expect("status field present");
                match &status_field.1 {
                    ConstValue::Enum { variant, .. } => assert_eq!(variant, "Success"),
                    other => panic!("unexpected status value: {:?}", other),
                }
            }
            other => panic!("unexpected decimal result: {:?}", other),
        }
    }

    #[test]
    fn decimal_intrinsic_reports_invalid_rounding_value() {
        let mut symbol_index = SymbolIndex::default();
        let mut layouts = TypeLayoutTable::default();
        let mut ctx = ConstEvalContext::new(&mut symbol_index, &mut layouts, None);
        let segments = vec![
            "Std".to_string(),
            "Numeric".to_string(),
            "Decimal".to_string(),
            "Intrinsics".to_string(),
            "AddWithOptions".to_string(),
        ];
        let lhs = Decimal128::parse_literal("1").expect("literal parses");
        let rhs = Decimal128::parse_literal("2").expect("literal parses");
        let args = vec![
            (None, ConstEvalResult::new(ConstValue::Decimal(lhs))),
            (None, ConstEvalResult::new(ConstValue::Decimal(rhs))),
            (None, ConstEvalResult::new(ConstValue::Bool(true))), // invalid rounding
            (None, ConstEvalResult::new(ConstValue::Bool(false))),
        ];
        let err = ctx
            .try_evaluate_decimal_intrinsic(&segments, &args, None)
            .expect_err("invalid rounding argument should fail");
        assert!(
            err.message.contains(DECIMAL_ROUNDING_TYPE),
            "unexpected error: {}",
            err.message
        );
    }
}
