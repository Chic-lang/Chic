use std::fmt::Write;

use crate::abi::CAbiPass;
use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::signatures::LlvmFunctionSignature;
use crate::codegen::llvm::types::map_type_owned;
use crate::error::Error;
use crate::mir::Operand;

/// Pre-rendered argument list for LLVM call emission.
pub(crate) struct RenderedArgs {
    pub(crate) repr: String,
}

impl RenderedArgs {
    pub(crate) fn new(values: Vec<String>) -> Self {
        let repr = values.join(", ");
        Self { repr }
    }
}

pub(crate) fn render_args_for_signature(
    emitter: &mut FunctionEmitter<'_>,
    signature: &LlvmFunctionSignature,
    args: &[Operand],
    context: &str,
) -> Result<RenderedArgs, Error> {
    let fixed_len = signature.params.len();
    if !signature.variadic && args.len() != fixed_len {
        return Err(Error::Codegen(format!(
            "{context} expects {} arguments but {} were provided (for `{}` in `{}`)",
            fixed_len,
            args.len(),
            signature.symbol,
            emitter.function.name
        )));
    } else if signature.variadic && args.len() < fixed_len {
        return Err(Error::Codegen(format!(
            "{context} expects at least {} arguments but {} were provided (for `{}` in `{}`)",
            fixed_len,
            args.len(),
            signature.symbol,
            emitter.function.name
        )));
    }
    let debug_forgetinit = std::env::var("CHIC_DEBUG_FORGETINIT").is_ok()
        && signature.symbol.contains("MaybeUninit__ForgetInit");
    let mut values = Vec::with_capacity(args.len().max(fixed_len));
    for (index, param_ty) in signature.params.iter().enumerate() {
        let Some(operand) = args.get(index) else {
            values.push(default_arg_value(param_ty));
            continue;
        };
        if signature
            .c_abi
            .as_ref()
            .and_then(|c_abi| c_abi.params.get(index))
            .is_some_and(|param| {
                matches!(
                    param.pass,
                    CAbiPass::IndirectByVal { .. } | CAbiPass::IndirectPtr { .. }
                )
            })
        {
            let (value_ptr, llvm_param_ty) =
                render_byval_arg(emitter, signature, index, index, operand)?;
            values.push(format!("{llvm_param_ty} {value_ptr}"));
            continue;
        }
        if debug_forgetinit {
            let place_ty = match operand {
                Operand::Copy(place) | Operand::Move(place) => emitter.place_type(place).ok(),
                Operand::Borrow(borrow) => emitter.place_type(&borrow.place).ok(),
                _ => None,
            };
            eprintln!(
                "[chic-debug forgetinit] param{index} param_ty={param_ty} operand={operand:?} place_ty={place_ty:?}"
            );
        }
        if std::env::var("CHIC_DEBUG_OPERANDS").is_ok() && param_ty.starts_with("ptr") {
            eprintln!("[chic-debug] render_args param_ty={param_ty} operand={operand:?}");
        }
        let rendered_arg = if let Some(value) = render_pointer_arg(emitter, param_ty, operand)? {
            if debug_forgetinit {
                eprintln!("[chic-debug forgetinit] param{index} using pointer override => {value}");
            }
            value
        } else {
            let value = emitter.emit_operand(operand, Some(param_ty))?;
            let rendered = if param_ty.starts_with('{') && value.ty() == "ptr" {
                let tmp = emitter.new_temp();
                writeln!(
                    &mut emitter.builder,
                    "  {tmp} = load {param_ty}, ptr {}",
                    value.repr()
                )
                .ok();
                tmp
            } else {
                value.repr().to_string()
            };
            format!("{param_ty} {rendered}")
        };
        if debug_forgetinit {
            eprintln!("[chic-debug forgetinit] param{index} rendered={rendered_arg}");
        }
        values.push(rendered_arg);
    }
    if signature.variadic && args.len() > fixed_len {
        let apply_promotions = signature.c_abi.is_some();
        for operand in args.iter().skip(fixed_len) {
            values.push(render_variadic_arg(emitter, operand, apply_promotions)?);
        }
    }
    Ok(RenderedArgs::new(values))
}

fn llvm_int_width(ty: &str) -> Option<u32> {
    ty.strip_prefix('i')
        .and_then(|bits| bits.parse::<u32>().ok())
}

pub(crate) fn render_variadic_arg(
    emitter: &mut FunctionEmitter<'_>,
    operand: &Operand,
    apply_promotions: bool,
) -> Result<String, Error> {
    let llvm_ty = emitter
        .operand_type(operand)?
        .ok_or_else(|| Error::Codegen("variadic argument missing type information".into()))?;
    if llvm_ty.starts_with('{') || llvm_ty.starts_with('[') {
        return Err(Error::Codegen(
            "variadic arguments must be C-ABI-safe scalars or pointers".into(),
        ));
    }

    let mut target_ty = llvm_ty.clone();
    let value = emitter.emit_operand(operand, Some(&target_ty))?;
    let mut value_repr = value.repr().to_string();
    if apply_promotions {
        if llvm_ty == "float" {
            target_ty = "double".to_string();
            let tmp = emitter.new_temp();
            writeln!(
                &mut emitter.builder,
                "  {tmp} = fpext float {} to double",
                value.repr()
            )
            .ok();
            value_repr = tmp;
        } else if let Some(width) = llvm_int_width(&llvm_ty) {
            if (1..32).contains(&width) {
                target_ty = "i32".to_string();
                let tmp = emitter.new_temp();
                writeln!(
                    &mut emitter.builder,
                    "  {tmp} = zext {llvm_ty} {} to i32",
                    value.repr()
                )
                .ok();
                value_repr = tmp;
            }
        }
    }

    Ok(format!("{target_ty} {value_repr}"))
}

pub(crate) fn render_args_for_c_abi_params(
    emitter: &mut FunctionEmitter<'_>,
    signature: &LlvmFunctionSignature,
    args: &[Operand],
    context: &str,
    llvm_arg_offset: usize,
) -> Result<RenderedArgs, Error> {
    let Some(c_abi) = signature.c_abi.as_ref() else {
        return Err(Error::Codegen(format!(
            "{context} requires C ABI metadata for `{}`",
            signature.symbol
        )));
    };
    if !signature.variadic && args.len() != c_abi.params.len() {
        return Err(Error::Codegen(format!(
            "{context} expects {} arguments but {} were provided (for `{}` in `{}`)",
            c_abi.params.len(),
            args.len(),
            signature.symbol,
            emitter.function.name
        )));
    } else if signature.variadic && args.len() < c_abi.params.len() {
        return Err(Error::Codegen(format!(
            "{context} expects at least {} arguments but {} were provided (for `{}` in `{}`)",
            c_abi.params.len(),
            args.len(),
            signature.symbol,
            emitter.function.name
        )));
    }

    let mut rendered = Vec::with_capacity(args.len());
    for (index, operand) in args.iter().enumerate() {
        if index < c_abi.params.len() {
            let Some(param_ty) = signature.params.get(index + llvm_arg_offset) else {
                return Err(Error::Codegen(format!(
                    "{context} tried to access missing parameter {} in `{}`",
                    index + llvm_arg_offset,
                    signature.symbol
                )));
            };
            if matches!(
                c_abi.params[index].pass,
                CAbiPass::IndirectByVal { .. } | CAbiPass::IndirectPtr { .. }
            ) {
                let (value_ptr, llvm_param_ty) =
                    render_byval_arg(emitter, signature, index, index + llvm_arg_offset, operand)?;
                rendered.push(format!("{llvm_param_ty} {value_ptr}"));
                continue;
            }

            if let Some(value) = render_pointer_arg(emitter, param_ty, operand)? {
                rendered.push(value);
                continue;
            }
            let value = emitter.emit_operand(operand, Some(param_ty))?;
            let rendered_val = if param_ty.starts_with('{') && value.ty() == "ptr" {
                let tmp = emitter.new_temp();
                writeln!(
                    &mut emitter.builder,
                    "  {tmp} = load {param_ty}, ptr {}",
                    value.repr()
                )
                .ok();
                tmp
            } else {
                value.repr().to_string()
            };
            rendered.push(format!("{param_ty} {rendered_val}"));
        } else {
            rendered.push(render_variadic_arg(
                emitter, operand, /*apply_promotions*/ true,
            )?);
        }
    }

    Ok(RenderedArgs::new(rendered))
}

pub(crate) fn render_args_for_types(
    emitter: &mut FunctionEmitter<'_>,
    params: &[String],
    args: &[Operand],
    context: &str,
) -> Result<RenderedArgs, Error> {
    if args.len() != params.len() {
        return Err(Error::Codegen(format!(
            "{context} expects {} arguments but {} were provided",
            params.len(),
            args.len()
        )));
    }
    render_args(emitter, params, args, |_, _, _| Ok(None))
}

pub(crate) fn render_args<F>(
    emitter: &mut FunctionEmitter<'_>,
    params: &[String],
    args: &[Operand],
    mut override_value: F,
) -> Result<RenderedArgs, Error>
where
    F: FnMut(usize, &str, &Operand) -> Result<Option<String>, Error>,
{
    let mut rendered = Vec::with_capacity(params.len());
    for (index, param_ty) in params.iter().enumerate() {
        let Some(operand) = args.get(index) else {
            rendered.push(default_arg_value(param_ty));
            continue;
        };
        if let Some(value) = override_value(index, param_ty, operand)? {
            rendered.push(value);
            continue;
        }
        if let Some(value) = render_pointer_arg(emitter, param_ty, operand)? {
            rendered.push(value);
            continue;
        }
        let value = emitter.emit_operand(operand, Some(param_ty))?;
        let rendered_val = if param_ty.starts_with('{') && value.ty() == "ptr" {
            let tmp = emitter.new_temp();
            writeln!(
                &mut emitter.builder,
                "  {tmp} = load {param_ty}, ptr {}",
                value.repr()
            )
            .ok();
            tmp
        } else {
            value.repr().to_string()
        };
        rendered.push(format!("{param_ty} {rendered_val}"));
    }
    Ok(RenderedArgs::new(rendered))
}

fn render_pointer_arg(
    emitter: &mut FunctionEmitter<'_>,
    param_ty: &str,
    operand: &Operand,
) -> Result<Option<String>, Error> {
    if !param_ty.starts_with("ptr") {
        return Ok(None);
    }
    let debug_pointer = emitter.function.name.contains("AsyncEntry::Chain");
    let place_and_ty = match operand {
        Operand::Copy(place) | Operand::Move(place) => {
            emitter.place_type(place).ok().map(|ty| (place, ty))
        }
        Operand::Borrow(borrow) => emitter
            .place_type(&borrow.place)
            .ok()
            .map(|ty| (&borrow.place, ty)),
        _ => None,
    };
    if debug_pointer {
        eprintln!(
            "[chic-debug] pointer arg {} operand={operand:?} place_ty={:?}",
            emitter.function.name,
            place_and_ty.as_ref().and_then(|(_, ty)| ty.clone())
        );
    }
    if let Some((place, place_ty)) = place_and_ty {
        if place_ty.as_deref() == Some("ptr") {
            return Ok(None);
        }
        let ptr = emitter.place_ptr(place)?;
        return Ok(Some(format!("{param_ty} {ptr}")));
    }
    Ok(None)
}

fn render_byval_arg(
    emitter: &mut FunctionEmitter<'_>,
    signature: &LlvmFunctionSignature,
    user_index: usize,
    llvm_param_index: usize,
    operand: &Operand,
) -> Result<(String, &'static str), Error> {
    let Some(c_abi) = signature.c_abi.as_ref() else {
        return Err(Error::Codegen(
            "render_byval_arg called without C ABI metadata".into(),
        ));
    };
    let Some(param) = c_abi.params.get(user_index) else {
        return Err(Error::Codegen(format!(
            "render_byval_arg called for out-of-range parameter index {user_index}"
        )));
    };
    let align = match param.pass {
        CAbiPass::IndirectByVal { align } | CAbiPass::IndirectPtr { align } => align,
        CAbiPass::Direct => {
            return Err(Error::Codegen(
                "render_byval_arg called for direct parameter".into(),
            ));
        }
    };

    let Some(llvm_param_ty) = signature.params.get(llvm_param_index) else {
        return Err(Error::Codegen(format!(
            "render_byval_arg called for missing LLVM parameter at index {llvm_param_index}"
        )));
    };
    if llvm_param_ty != "ptr" {
        return Err(Error::Codegen(format!(
            "render_byval_arg expected `ptr` parameter type but found `{llvm_param_ty}`"
        )));
    }

    let Some(value_ty) = map_type_owned(&param.ty, Some(emitter.type_layouts))? else {
        return Err(Error::Codegen(format!(
            "byval argument `{}` lowered to void LLVM type",
            param.ty.canonical_name()
        )));
    };

    if let Some(pointer) = render_pointer_arg(emitter, llvm_param_ty, operand)? {
        let ptr = pointer
            .strip_prefix("ptr ")
            .unwrap_or(pointer.as_str())
            .to_string();
        return Ok((ptr, "ptr"));
    }

    let value = emitter.emit_operand(operand, Some(&value_ty))?;
    let value_repr =
        if (value_ty.starts_with('{') || value_ty.starts_with('[')) && value.ty() == "ptr" {
            let tmp = emitter.new_temp();
            writeln!(
                &mut emitter.builder,
                "  {tmp} = load {value_ty}, ptr {}",
                value.repr()
            )
            .ok();
            tmp
        } else {
            value.repr().to_string()
        };

    let tmp_ptr = emitter.new_temp();
    writeln!(
        &mut emitter.builder,
        "  {tmp_ptr} = alloca {value_ty}, align {align}"
    )
    .ok();
    writeln!(
        &mut emitter.builder,
        "  store {value_ty} {value_repr}, ptr {tmp_ptr}, align {align}"
    )
    .ok();
    Ok((tmp_ptr, "ptr"))
}

fn default_arg_value(param_ty: &str) -> String {
    if param_ty.starts_with("ptr") {
        return format!("{param_ty} null");
    }
    if param_ty.starts_with('i') || param_ty.starts_with('u') {
        return format!("{param_ty} 0");
    }
    if param_ty == "float" || param_ty == "double" {
        return format!("{param_ty} 0.0");
    }
    format!("{param_ty} zeroinitializer")
}
