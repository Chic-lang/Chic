use std::fmt::Write;

use super::args::{
    render_args, render_args_for_signature, render_args_for_types, render_variadic_arg,
};
use crate::abi::{CAbiPass, CAbiReturn, classify_c_abi_signature};
use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::values::ValueRef;
use crate::codegen::llvm::signatures::resolve_function_name;
use crate::codegen::llvm::types::map_type_owned;
use crate::error::Error;
use crate::mir::pointer_size;
use crate::mir::{
    Abi, BlockId, FnSig, FnTy, Operand, ParamMode, Place, PointerTy, TraitObjectDispatch, Ty,
    VirtualDispatch,
};

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_trait_object_call(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
        unwind: Option<BlockId>,
        dispatch: &TraitObjectDispatch,
    ) -> Result<(), Error> {
        let debug = std::env::var("CHIC_DEBUG_TRAIT_CALL").is_ok();
        if debug {
            eprintln!(
                "[chic-debug trait-call] {} dispatch={:?} args_len={} dest={}",
                self.function.name,
                dispatch,
                args.len(),
                destination.is_some()
            );
        }
        let receiver = args
            .get(dispatch.receiver_index)
            .ok_or_else(|| Error::Codegen("trait object call missing receiver argument".into()))?;
        let place = match receiver {
            Operand::Copy(place) | Operand::Move(place) => place,
            Operand::Borrow(borrow) => &borrow.place,
            _ => {
                return Err(Error::Codegen(
                    "trait object receiver must be addressable in LLVM backend".into(),
                ));
            }
        };
        let (data_ptr, fn_ptr) = if let Some(impl_type) = dispatch.impl_type.as_deref()
            && impl_type != dispatch.trait_name
        {
            let table = self
                .trait_vtables
                .iter()
                .find(|table| {
                    table.trait_name == dispatch.trait_name && table.impl_type == impl_type
                })
                .or_else(|| {
                    self.trait_vtables
                        .iter()
                        .find(|table| table.trait_name == dispatch.trait_name)
                })
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "trait `{}` does not have vtable metadata for impl `{impl_type}`",
                        dispatch.trait_name
                    ))
                })?;
            let slot_count = table.slots.len();
            if dispatch.slot_index as usize >= slot_count {
                return Err(Error::Codegen(format!(
                    "trait `{}` vtable slot {} is out of range for impl `{impl_type}`",
                    dispatch.trait_name, dispatch.slot_index
                )));
            }
            let ptr = self.place_ptr(place)?;
            let data_ptr_tmp = self.new_temp();
            writeln!(&mut self.builder, "  {data_ptr_tmp} = load ptr, ptr {ptr}").ok();
            let slot_gep = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {slot_gep} = getelementptr inbounds [{} x ptr], ptr @{}, i32 0, i32 {}",
                slot_count, table.symbol, dispatch.slot_index
            )
            .ok();
            let fn_ptr = self.new_temp();
            writeln!(&mut self.builder, "  {fn_ptr} = load ptr, ptr {slot_gep}").ok();
            (data_ptr_tmp, fn_ptr)
        } else {
            let trait_ty = self
                .place_type(place)?
                .ok_or_else(|| Error::Codegen("trait object place has unknown LLVM type".into()))?;
            let ptr = self.place_ptr(place)?;
            let data_gep = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {data_gep} = getelementptr inbounds {trait_ty}, ptr {ptr}, i32 0, i32 0"
            )
            .ok();
            let data_ptr = self.new_temp();
            writeln!(&mut self.builder, "  {data_ptr} = load ptr, ptr {data_gep}").ok();
            let vtable_gep = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {vtable_gep} = getelementptr inbounds {trait_ty}, ptr {ptr}, i32 0, i32 1"
            )
            .ok();
            let vtable_ptr = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {vtable_ptr} = load ptr, ptr {vtable_gep}"
            )
            .ok();
            let slot_type = format!("[{} x ptr]", dispatch.slot_count);
            let slot_gep = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {slot_gep} = getelementptr inbounds {slot_type}, ptr {vtable_ptr}, i32 0, i32 {}",
                dispatch.slot_index
            )
            .ok();
            let fn_ptr = self.new_temp();
            writeln!(&mut self.builder, "  {fn_ptr} = load ptr, ptr {slot_gep}").ok();
            (data_ptr, fn_ptr)
        };

        let symbol = self.trait_vtable_slot_symbol(dispatch)?;
        let trait_label = dispatch
            .trait_name
            .rsplit("::")
            .next()
            .unwrap_or(dispatch.trait_name.as_str());
        let canonical = if self.signatures.contains_key(&symbol) {
            resolve_function_name(self.signatures, &symbol).unwrap_or_else(|| symbol.clone())
        } else {
            resolve_function_name(self.signatures, &symbol)
                .filter(|name| name.contains(&dispatch.trait_name) || name.contains(trait_label))
                .unwrap_or_else(|| symbol.clone())
        };
        let receiver_data = data_ptr.clone();
        let signature = self
            .signatures
            .get(&canonical)
            .filter(|sig| sig.params.len() == args.len());
        if debug {
            if let Some(sig) = signature {
                eprintln!(
                    "[chic-debug trait-call] using signature {} params={:?} ret={:?}",
                    canonical, sig.params, sig.ret
                );
            } else {
                eprintln!(
                    "[chic-debug trait-call] no signature for {} (args_len={})",
                    canonical,
                    args.len()
                );
            }
        }
        let (ret_ty, rendered_args, param_types) = if let Some(signature) = signature {
            let rendered_args = render_args(self, &signature.params, args, move |index, ty, _| {
                if index == dispatch.receiver_index {
                    Ok(Some(format!("{ty} {receiver_data}")))
                } else {
                    Ok(None)
                }
            })?;
            (
                signature.ret.clone().unwrap_or_else(|| "void".to_string()),
                rendered_args,
                signature.params.clone(),
            )
        } else {
            let canonical_base = canonical.split('#').next().unwrap_or(&canonical);
            let canonical_leaf = canonical_base.rsplit("::").next().unwrap_or(canonical_base);
            let has_candidate = self.signatures.contains_key(&canonical)
                || self.signatures.keys().any(|name| {
                    let base = name.split('#').next().unwrap_or(name);
                    base == canonical_base || name.rsplit("::").next() == Some(canonical_leaf)
                });
            if !has_candidate {
                return Err(Error::Codegen(format!(
                    "missing LLVM signature for trait method `{canonical}`"
                )));
            }
            let mut params = Vec::new();
            for (index, operand) in args.iter().enumerate() {
                if index == dispatch.receiver_index {
                    params.push("ptr".to_string());
                } else {
                    let ty = self.operand_type(operand)?.ok_or_else(|| {
                        Error::Codegen("trait call argument missing type information".into())
                    })?;
                    params.push(ty);
                }
            }
            let ret_ty = if let Some(dest) = destination {
                self.place_type(dest)?.ok_or_else(|| {
                    Error::Codegen("trait call destination missing type information".into())
                })?
            } else {
                "void".to_string()
            };
            let rendered_args = render_args(self, &params, args, |index, ty, _| {
                if index == dispatch.receiver_index {
                    Ok(Some(format!("{ty} {receiver_data}")))
                } else {
                    Ok(None)
                }
            })?;
            (ret_ty, rendered_args, params)
        };
        let dest_label = self.block_label(target)?;

        let fn_ptr_ty = format!("{} ({})*", ret_ty, param_types.join(", "));
        let cast_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {cast_tmp} = bitcast ptr {fn_ptr} to {fn_ptr_ty}"
        )
        .ok();

        if let Some(dest) = destination {
            if ret_ty == "void" {
                return Err(Error::Codegen(
                    "void trait-object call cannot assign to destination".into(),
                ));
            }
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = call {ret_ty} {cast_tmp}({})",
                rendered_args.repr
            )
            .ok();
            self.store_place(dest, &ValueRef::new(tmp, &ret_ty))?;
        } else {
            writeln!(
                &mut self.builder,
                "  call {ret_ty} {cast_tmp}({})",
                rendered_args.repr
            )
            .ok();
        }

        self.emit_branch_to_labels_or_unwind(dest_label.as_str(), unwind)?;
        Ok(())
    }

    pub(super) fn emit_virtual_call(
        &mut self,
        func: &Operand,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
        unwind: Option<BlockId>,
        dispatch: &VirtualDispatch,
    ) -> Result<(), Error> {
        let receiver = args
            .get(dispatch.receiver_index)
            .ok_or_else(|| Error::Codegen("virtual dispatch missing receiver argument".into()))?;
        let place = match receiver {
            Operand::Copy(place) | Operand::Move(place) => place,
            Operand::Borrow(borrow) => &borrow.place,
            _ => {
                return Err(Error::Codegen(
                    "virtual dispatch receiver must be addressable".into(),
                ));
            }
        };

        let fn_ptr = if let Some(owner) = dispatch.base_owner.as_deref() {
            let table = self
                .class_vtables
                .iter()
                .find(|table| table.type_name == owner)
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "class `{owner}` does not have vtable metadata in this module"
                    ))
                })?;
            let slot_count = table.slots.len();
            if dispatch.slot_index as usize >= slot_count {
                return Err(Error::Codegen(format!(
                    "class `{owner}` vtable slot {} is out of range",
                    dispatch.slot_index
                )));
            }
            let slot_gep = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {slot_gep} = getelementptr inbounds [{} x ptr], ptr @{}, i32 0, i32 {}",
                slot_count, table.symbol, dispatch.slot_index
            )
            .ok();
            let fn_ptr = self.new_temp();
            writeln!(&mut self.builder, "  {fn_ptr} = load ptr, ptr {slot_gep}").ok();
            fn_ptr
        } else {
            let ptr = self.place_ptr(place)?;
            let table_ptr = self.new_temp();
            writeln!(&mut self.builder, "  {table_ptr} = load ptr, ptr {ptr}").ok();
            let slot_gep = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {slot_gep} = getelementptr inbounds ptr, ptr {table_ptr}, i32 {}",
                dispatch.slot_index
            )
            .ok();
            let fn_ptr = self.new_temp();
            writeln!(&mut self.builder, "  {fn_ptr} = load ptr, ptr {slot_gep}").ok();
            fn_ptr
        };

        let repr = self.call_operand_repr(func)?;
        let callee = resolve_function_name(self.signatures, &repr)
            .ok_or_else(|| Error::Codegen(format!("unknown call target `{repr}`")))?;
        let signature = self.signatures.get(&callee).ok_or_else(|| {
            Error::Codegen(format!(
                "missing signature for `{callee}` in virtual dispatch"
            ))
        })?;
        let rendered_args =
            render_args_for_signature(self, signature, args, "virtual call arguments")?;

        let dest_label = self.block_label(target)?;
        let ret_ty = signature.ret.clone().unwrap_or_else(|| "void".to_string());
        let fn_ptr_ty = format!("{} ({})*", ret_ty, signature.params.join(", "));
        let cast_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {cast_tmp} = bitcast ptr {fn_ptr} to {fn_ptr_ty}"
        )
        .ok();

        if let Some(dest) = destination {
            let dest_ty = self
                .place_type(dest)?
                .ok_or_else(|| Error::Codegen("call destination is missing LLVM type".into()))?;
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = call {ret_ty} {cast_tmp}({})",
                rendered_args.repr
            )
            .ok();
            self.store_place(dest, &ValueRef::new(tmp, &dest_ty))?;
        } else {
            writeln!(
                &mut self.builder,
                "  call {ret_ty} {cast_tmp}({})",
                rendered_args.repr
            )
            .ok();
        }
        self.emit_branch_to_labels_or_unwind(dest_label.as_str(), unwind)?;
        Ok(())
    }

    pub(crate) fn emit_indirect_call(
        &mut self,
        func: &Operand,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
        unwind: Option<BlockId>,
        fn_ty: FnTy,
    ) -> Result<(), Error> {
        if matches!(fn_ty.abi, Abi::Extern(_)) {
            return self.emit_indirect_c_abi_call(func, args, destination, target, unwind, &fn_ty);
        }

        let place = match func {
            Operand::Copy(place) | Operand::Move(place) => place.clone(),
            Operand::Borrow(borrow) => borrow.place.clone(),
            _ => unreachable!("call_operand_fn_ty guarantees a place-backed operand"),
        };
        debug_assert!(
            place.projection.is_empty(),
            "call_operand_fn_ty filters projected places"
        );

        let Some(layout) = self
            .type_layouts
            .types
            .get(&fn_ty.canonical_name())
            .and_then(|layout| match layout {
                crate::mir::TypeLayout::Struct(data) | crate::mir::TypeLayout::Class(data) => {
                    Some(data)
                }
                _ => None,
            })
        else {
            // Synthesize a minimal layout with invoke/context pointers when metadata is absent.
            let base_ptr = self.place_ptr(&place)?;

            let load_synth = |builder: &mut FunctionEmitter<'a>,
                              base: &str,
                              offset: usize|
             -> Result<ValueRef, Error> {
                let gep_tmp = builder.new_temp();
                writeln!(
                    &mut builder.builder,
                    "  {gep_tmp} = getelementptr i8, ptr {base}, i64 {offset}",
                    offset = offset as i64
                )
                .ok();
                let llvm_ty = map_type_owned(&Ty::named("usize"), Some(builder.type_layouts))?
                    .ok_or_else(|| {
                        Error::Codegen("function pointer field lowered to void".into())
                    })?;
                let load_tmp = builder.new_temp();
                writeln!(
                    &mut builder.builder,
                    "  {load_tmp} = load {llvm_ty}, ptr {gep_tmp}"
                )
                .ok();
                Ok(ValueRef::new(load_tmp, &llvm_ty))
            };

            let invoke_ptr = load_synth(self, &base_ptr, 0)?;
            let context_ptr = load_synth(self, &base_ptr, pointer_size())?;
            return self.finish_function_pointer_call(
                invoke_ptr,
                context_ptr,
                &fn_ty,
                args,
                target,
                unwind,
                destination,
            );
        };

        let base_ptr = self.place_ptr(&place)?;

        let load_ptr_field = |builder: &mut FunctionEmitter<'a>,
                              base: &str,
                              field_name: &str,
                              field_ty: &Ty|
         -> Result<ValueRef, Error> {
            let field = layout
                .fields
                .iter()
                .find(|f| f.name == field_name)
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "function pointer layout missing `{field_name}` field"
                    ))
                })?;
            let offset = field.offset.ok_or_else(|| {
                Error::Codegen(format!(
                    "function pointer field `{field_name}` missing offset metadata"
                ))
            })?;
            let gep_tmp = builder.new_temp();
            writeln!(
                &mut builder.builder,
                "  {gep_tmp} = getelementptr i8, ptr {base}, i64 {offset}"
            )
            .ok();
            let llvm_ty = map_type_owned(field_ty, Some(builder.type_layouts))?
                .ok_or_else(|| Error::Codegen("function pointer field lowered to void".into()))?;
            let load_tmp = builder.new_temp();
            writeln!(
                &mut builder.builder,
                "  {load_tmp} = load {llvm_ty}, ptr {gep_tmp}"
            )
            .ok();
            Ok(ValueRef::new(load_tmp, &llvm_ty))
        };

        let invoke_ptr = load_ptr_field(
            self,
            &base_ptr,
            "invoke",
            &Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true))),
        )?;
        let context_ptr = load_ptr_field(
            self,
            &base_ptr,
            "context",
            &Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true))),
        )?;
        self.finish_function_pointer_call(
            invoke_ptr,
            context_ptr,
            &fn_ty,
            args,
            target,
            unwind,
            destination,
        )
    }

    fn emit_indirect_c_abi_call(
        &mut self,
        func: &Operand,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
        unwind: Option<BlockId>,
        fn_ty: &FnTy,
    ) -> Result<(), Error> {
        let dest_label = self.block_label(target)?;
        let fn_ptr_value = self.emit_operand(func, Some("ptr"))?;

        let sig = FnSig {
            params: fn_ty.params.clone(),
            ret: (*fn_ty.ret).clone(),
            abi: fn_ty.abi.clone(),
            effects: Vec::new(),
            lends_to_return: None,
            variadic: fn_ty.variadic,
        };

        let param_modes = if fn_ty.param_modes.is_empty() {
            vec![ParamMode::Value; sig.params.len()]
        } else {
            fn_ty.param_modes.clone()
        };

        let c_abi = classify_c_abi_signature(&sig, &param_modes, self.type_layouts, self.target)
            .map_err(|err| {
                Error::Codegen(format!(
                    "C ABI classification failed for indirect call `{}`: {err}",
                    fn_ty.canonical_name()
                ))
            })?;
        let is_variadic = fn_ty.variadic;

        let mut llvm_params = Vec::with_capacity(sig.params.len() + 1);
        let mut llvm_param_attrs = Vec::with_capacity(sig.params.len() + 1);
        for (index, ty) in sig.params.iter().enumerate() {
            let mode = param_modes.get(index).copied().unwrap_or(ParamMode::Value);
            let param_ty = match mode {
                ParamMode::Value => {
                    map_type_owned(ty, Some(self.type_layouts))?.ok_or_else(|| {
                        Error::Codegen("parameter cannot have unit type in LLVM backend".into())
                    })?
                }
                ParamMode::In | ParamMode::Ref | ParamMode::Out => "ptr".to_string(),
            };
            llvm_params.push(param_ty);
            llvm_param_attrs.push(Vec::<String>::new());
        }

        let mut llvm_ret =
            map_type_owned(&sig.ret, Some(self.type_layouts))?.filter(|ty| ty != "void");

        for param in &c_abi.params {
            if let Some(coerce) = &param.coerce {
                if let Some(slot_ty) = llvm_params.get_mut(param.index) {
                    *slot_ty = coerce.clone();
                }
            }
        }

        for param in &c_abi.params {
            let (is_byval, align) = match param.pass {
                CAbiPass::IndirectByVal { align } => (true, align),
                CAbiPass::IndirectPtr { align } => (false, align),
                CAbiPass::Direct => continue,
            };
            let Some(slot_ty) = llvm_params.get_mut(param.index) else {
                return Err(Error::Codegen(format!(
                    "C ABI classification requested parameter {} out of range for indirect call",
                    param.index
                )));
            };
            *slot_ty = "ptr".to_string();
            let Some(attr_slot) = llvm_param_attrs.get_mut(param.index) else {
                return Err(Error::Codegen(format!(
                    "C ABI classification missing parameter attributes at index {} for indirect call",
                    param.index
                )));
            };
            if is_byval {
                let llvm_value_ty = map_type_owned(&param.ty, Some(self.type_layouts))?
                    .ok_or_else(|| {
                        Error::Codegen(format!(
                            "C ABI byval parameter type `{}` lowered to void LLVM type",
                            param.ty.canonical_name()
                        ))
                    })?;
                attr_slot.push(format!("byval({llvm_value_ty})"));
            }
            attr_slot.push(format!("align {align}"));
        }

        let sret_ptr = match &c_abi.ret {
            CAbiReturn::Direct { coerce, .. } => {
                if let Some(coerce) = coerce {
                    llvm_ret = Some(coerce.clone());
                }
                None
            }
            CAbiReturn::IndirectSret { ty, align } => {
                let ret_llvm_ty =
                    map_type_owned(ty, Some(self.type_layouts))?.ok_or_else(|| {
                        Error::Codegen(format!(
                            "C ABI sret return type `{}` lowered to void LLVM type",
                            ty.canonical_name()
                        ))
                    })?;
                llvm_params.insert(0, "ptr".to_string());
                llvm_param_attrs.insert(
                    0,
                    vec![format!("sret({ret_llvm_ty})"), format!("align {align}")],
                );
                llvm_ret = None;

                let ptr = if let Some(dest) = destination {
                    self.place_ptr(dest)?
                } else {
                    let tmp_ptr = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp_ptr} = alloca {ret_llvm_ty}, align {align}"
                    )
                    .ok();
                    tmp_ptr
                };
                Some(ptr)
            }
        };

        let ret_ty = llvm_ret.clone().unwrap_or_else(|| "void".to_string());
        let params_repr = if llvm_params.is_empty() {
            String::new()
        } else {
            llvm_params.join(", ")
        };
        let fn_ptr_ty = if is_variadic {
            if params_repr.is_empty() {
                format!("{ret_ty} (...)*")
            } else {
                format!("{ret_ty} ({params_repr}, ...)*")
            }
        } else if params_repr.is_empty() {
            format!("{ret_ty} ()*")
        } else {
            format!("{ret_ty} ({params_repr})*")
        };
        let cast_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {cast_tmp} = bitcast ptr {} to {fn_ptr_ty}",
            fn_ptr_value.repr()
        )
        .ok();

        let abi_attr_suffix = |attrs: &[String]| -> String {
            let mut filtered = Vec::new();
            for attr in attrs {
                if attr.starts_with("byval(")
                    || attr.starts_with("sret(")
                    || attr.starts_with("align ")
                {
                    filtered.push(attr.as_str());
                }
            }
            if filtered.is_empty() {
                String::new()
            } else {
                format!(" {}", filtered.join(" "))
            }
        };

        let mut call_args = Vec::with_capacity(args.len() + usize::from(sret_ptr.is_some()));
        if let Some(sret_ptr) = sret_ptr.as_ref() {
            let attrs = llvm_param_attrs
                .get(0)
                .map(|attrs| abi_attr_suffix(attrs))
                .unwrap_or_default();
            call_args.push(format!("ptr{attrs} {sret_ptr}"));
        }

        let user_offset = usize::from(sret_ptr.is_some());
        if !is_variadic && args.len() != c_abi.params.len() {
            return Err(Error::Codegen(format!(
                "indirect C ABI call expects {} arguments but {} were provided",
                c_abi.params.len(),
                args.len()
            )));
        } else if is_variadic && args.len() < c_abi.params.len() {
            return Err(Error::Codegen(format!(
                "indirect C ABI call expects at least {} arguments but {} were provided",
                c_abi.params.len(),
                args.len()
            )));
        }
        for (user_index, operand) in args.iter().enumerate() {
            let llvm_index = user_index + user_offset;
            if user_index >= c_abi.params.len() {
                call_args.push(render_variadic_arg(self, operand, true)?);
                continue;
            }
            let Some(param_ty) = llvm_params.get(llvm_index) else {
                return Err(Error::Codegen(format!(
                    "indirect C ABI call tried to access missing parameter {llvm_index}"
                )));
            };
            let attrs = llvm_param_attrs
                .get(llvm_index)
                .map(|attrs| abi_attr_suffix(attrs))
                .unwrap_or_default();

            let rendered = if matches!(
                c_abi.params[user_index].pass,
                CAbiPass::IndirectByVal { .. } | CAbiPass::IndirectPtr { .. }
            ) {
                let align = match c_abi.params[user_index].pass {
                    CAbiPass::IndirectByVal { align } | CAbiPass::IndirectPtr { align } => align,
                    CAbiPass::Direct => unreachable!(),
                };
                let Some(value_ty) =
                    map_type_owned(&c_abi.params[user_index].ty, Some(self.type_layouts))?
                else {
                    return Err(Error::Codegen(format!(
                        "byval argument `{}` lowered to void LLVM type",
                        c_abi.params[user_index].ty.canonical_name()
                    )));
                };
                let ptr = if let Some(pointer) = (match operand {
                    Operand::Copy(place) | Operand::Move(place) => {
                        self.place_type(place).ok().map(|ty| (place, ty))
                    }
                    Operand::Borrow(borrow) => self
                        .place_type(&borrow.place)
                        .ok()
                        .map(|ty| (&borrow.place, ty)),
                    _ => None,
                })
                .and_then(|(place, place_ty)| {
                    (place_ty.as_deref() != Some("ptr")).then(|| self.place_ptr(place))
                })
                .transpose()?
                {
                    pointer
                } else {
                    let value = self.emit_operand(operand, Some(&value_ty))?;
                    let value_repr = if (value_ty.starts_with('{') || value_ty.starts_with('['))
                        && value.ty() == "ptr"
                    {
                        let tmp = self.new_temp();
                        writeln!(
                            &mut self.builder,
                            "  {tmp} = load {value_ty}, ptr {}",
                            value.repr()
                        )
                        .ok();
                        tmp
                    } else {
                        value.repr().to_string()
                    };
                    let tmp_ptr = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp_ptr} = alloca {value_ty}, align {align}"
                    )
                    .ok();
                    writeln!(
                        &mut self.builder,
                        "  store {value_ty} {value_repr}, ptr {tmp_ptr}, align {align}"
                    )
                    .ok();
                    tmp_ptr
                };
                format!("ptr{attrs} {ptr}")
            } else if param_ty.starts_with("ptr") {
                let place_and_ty = match operand {
                    Operand::Copy(place) | Operand::Move(place) => {
                        self.place_type(place).ok().map(|ty| (place, ty))
                    }
                    Operand::Borrow(borrow) => self
                        .place_type(&borrow.place)
                        .ok()
                        .map(|ty| (&borrow.place, ty)),
                    _ => None,
                };
                if let Some((place, place_ty)) = place_and_ty {
                    if place_ty.as_deref() != Some("ptr") {
                        let ptr = self.place_ptr(place)?;
                        format!("{param_ty}{attrs} {ptr}")
                    } else {
                        let value = self.emit_operand(operand, Some(param_ty))?;
                        format!("{param_ty}{attrs} {}", value.repr())
                    }
                } else {
                    let value = self.emit_operand(operand, Some(param_ty))?;
                    format!("{param_ty}{attrs} {}", value.repr())
                }
            } else {
                let value = self.emit_operand(operand, Some(param_ty))?;
                let repr = if param_ty.starts_with('{') && value.ty() == "ptr" {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = load {param_ty}, ptr {}",
                        value.repr()
                    )
                    .ok();
                    tmp
                } else {
                    value.repr().to_string()
                };
                format!("{param_ty}{attrs} {repr}")
            };
            call_args.push(rendered);
        }

        let args_repr = call_args.join(", ");
        match (&c_abi.ret, destination, llvm_ret.as_deref()) {
            (CAbiReturn::IndirectSret { .. }, _, _) => {
                writeln!(&mut self.builder, "  call void {cast_tmp}({args_repr})").ok();
            }
            (CAbiReturn::Direct { .. }, Some(dest), Some(ret_llvm_ty)) => {
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = call {ret_llvm_ty} {cast_tmp}({args_repr})"
                )
                .ok();
                self.store_direct_c_abi_return(dest, &ValueRef::new(tmp, ret_llvm_ty), &c_abi.ret)?;
            }
            (CAbiReturn::Direct { .. }, Some(_dest), None) => {
                return Err(Error::Codegen(
                    "void function pointer cannot assign to destination".into(),
                ));
            }
            (CAbiReturn::Direct { .. }, None, Some(ret_llvm_ty)) => {
                writeln!(
                    &mut self.builder,
                    "  call {ret_llvm_ty} {cast_tmp}({args_repr})"
                )
                .ok();
            }
            (CAbiReturn::Direct { .. }, None, None) => {
                writeln!(&mut self.builder, "  call void {cast_tmp}({args_repr})").ok();
            }
        }

        self.emit_branch_to_labels_or_unwind(dest_label.as_str(), unwind)?;
        Ok(())
    }

    fn finish_function_pointer_call(
        &mut self,
        invoke_ptr: ValueRef,
        context_ptr: ValueRef,
        fn_ty: &FnTy,
        args: &[Operand],
        target: crate::mir::BlockId,
        unwind: Option<BlockId>,
        destination: Option<&Place>,
    ) -> Result<(), Error> {
        let mut param_types = Vec::with_capacity(fn_ty.params.len() + 1);
        param_types.push("ptr".to_string());
        for ty in &fn_ty.params {
            let mapped = map_type_owned(ty, Some(self.type_layouts))?.ok_or_else(|| {
                Error::Codegen("function pointer parameter cannot have unit type".into())
            })?;
            param_types.push(mapped);
        }

        let rendered_args =
            render_args_for_types(self, &param_types[1..], args, "function pointer call")?;
        let dest_label = self.block_label(target)?;

        let ret_ty = map_type_owned(&fn_ty.ret, Some(self.type_layouts))?;
        let fn_ptr_ty = format!(
            "{} ({})*",
            ret_ty.clone().unwrap_or_else(|| "void".into()),
            param_types.join(", ")
        );
        let cast_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {cast_tmp} = bitcast ptr {} to {fn_ptr_ty}",
            invoke_ptr.repr()
        )
        .ok();

        let args_repr = if rendered_args.repr.is_empty() {
            format!("ptr {}", context_ptr.repr())
        } else {
            format!("ptr {}, {}", context_ptr.repr(), rendered_args.repr)
        };

        if let Some(dest) = destination {
            let ret = ret_ty.clone().ok_or_else(|| {
                Error::Codegen("void function pointer cannot assign to destination".into())
            })?;
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = call {ret} {cast_tmp}({args_repr})"
            )
            .ok();
            self.store_place(dest, &ValueRef::new(tmp, &ret))?;
        } else {
            let ret = ret_ty.clone().unwrap_or_else(|| "void".into());
            writeln!(&mut self.builder, "  call {ret} {cast_tmp}({args_repr})").ok();
        }

        self.emit_branch_to_labels_or_unwind(dest_label.as_str(), unwind)?;
        Ok(())
    }

    pub(super) fn store_direct_c_abi_return(
        &mut self,
        dest: &Place,
        value: &ValueRef,
        ret: &CAbiReturn,
    ) -> Result<(), Error> {
        if let CAbiReturn::Direct {
            coerce: Some(coerce),
            ty,
        } = ret
        {
            let ptr = self.place_ptr(dest)?;
            let align = self.align_for_ty(ty);
            let alias_suffix = self.alias_suffix_for_place(dest).unwrap_or_default();
            writeln!(
                &mut self.builder,
                "  store {coerce} {}, ptr {ptr}, align {align}{alias_suffix}",
                value.repr()
            )
            .ok();
            return Ok(());
        }
        self.store_place(dest, value)
    }

    fn trait_vtable_slot_symbol(&self, dispatch: &TraitObjectDispatch) -> Result<String, Error> {
        if let Some(table) = dispatch
            .impl_type
            .as_deref()
            .and_then(|impl_type| {
                self.trait_vtables.iter().find(|table| {
                    table.trait_name == dispatch.trait_name && table.impl_type == impl_type
                })
            })
            .or_else(|| {
                self.trait_vtables
                    .iter()
                    .find(|table| table.trait_name == dispatch.trait_name)
            })
        {
            let slot = table
                .slots
                .get(dispatch.slot_index as usize)
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "trait `{}` vtable is missing slot {}",
                        dispatch.trait_name, dispatch.slot_index
                    ))
                })?;
            return Ok(slot.symbol.clone());
        }

        Err(Error::Codegen(format!(
            "trait `{}` does not have vtable metadata in this module",
            dispatch.trait_name
        )))
    }
}
