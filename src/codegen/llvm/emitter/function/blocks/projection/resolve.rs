use std::fmt::Write;

use super::*;
use crate::codegen::llvm::emitter::literals::{LLVM_STR_TYPE, LLVM_STRING_TYPE};
use crate::codegen::llvm::types::map_type_owned;
use crate::mir::{Ty, pointer_align};

fn is_str_llvm_ty(ty: &str) -> bool {
    let normalized = ty.replace(' ', "").replace("i8*", "ptr");
    normalized == "{ptr,i64}"
}

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn infer_unknown_local_ty(&self, local: usize, current_ty: Ty) -> Ty {
        if !matches!(current_ty, Ty::Unknown) {
            return current_ty;
        }

        if std::env::var("CHIC_DEBUG_LAYOUT").is_ok() {
            let llvm_ty = self.local_tys.get(local).and_then(|ty| ty.as_deref());
            eprintln!(
                "[projection-debug] attempting to infer unknown local {} in {} (llvm_ty={:?})",
                local, self.function.name, llvm_ty
            );
        }

        if let Some(override_ty) = self.decimal_struct_override(local) {
            return override_ty;
        }

        if let Some(local_llvm_ty) = self.local_tys.get(local).and_then(|ty| ty.as_deref()) {
            if std::env::var("CHIC_DEBUG_LAYOUT").is_ok() {
                eprintln!(
                    "[projection-debug] inferring type for {} local {} with llvm_ty `{}`",
                    self.function.name, local, local_llvm_ty
                );
            }
            let byte_ty = Ty::named("byte");
            let candidates = [
                Ty::named("Std::Runtime::Collections::SpanPtr"),
                Ty::named("Std::Runtime::Collections::ReadOnlySpanPtr"),
                Ty::Span(crate::mir::SpanTy::new(Box::new(byte_ty.clone()))),
                Ty::ReadOnlySpan(crate::mir::ReadOnlySpanTy::new(Box::new(byte_ty.clone()))),
                Ty::String,
                Ty::Str,
            ];
            for candidate in candidates {
                if let Some(mapped) = map_type_owned(&candidate, Some(self.type_layouts))
                    .ok()
                    .flatten()
                {
                    if std::env::var("CHIC_DEBUG_LAYOUT").is_ok() {
                        eprintln!(
                            "[projection-debug] candidate {} maps to `{}`",
                            candidate.canonical_name(),
                            mapped
                        );
                    }
                    if mapped == local_llvm_ty {
                        if std::env::var("CHIC_DEBUG_LAYOUT").is_ok() {
                            eprintln!(
                                "[projection-debug] selected candidate {} for local {} in {}",
                                candidate.canonical_name(),
                                local,
                                self.function.name
                            );
                        }
                        return candidate;
                    }
                }
            }
            if std::env::var("CHIC_DEBUG_LAYOUT").is_ok() {
                eprintln!(
                    "[projection-debug] unable to match llvm_ty `{}` for local {} in {}",
                    local_llvm_ty, local, self.function.name
                );
            }
        }

        current_ty
    }

    pub(crate) fn place_type(&self, place: &Place) -> Result<Option<String>, Error> {
        if place.projection.is_empty() {
            if self.is_reference_param(place.local.0) {
                if let Ok(value_ty) = self.param_value_type(place.local.0) {
                    return Ok(Some(value_ty));
                }
            }
            return self
                .local_tys
                .get(place.local.0)
                .cloned()
                .ok_or_else(|| Error::Codegen("place referenced unknown local".into()));
        }
        let ty = self.mir_ty_of_place(place)?;
        map_type_owned(&ty, Some(self.type_layouts))
    }

    pub(crate) fn local_param_mode(&self, index: usize) -> Option<ParamMode> {
        self.function
            .body
            .locals
            .get(index)
            .and_then(|decl| decl.param_mode)
    }

    pub(crate) fn is_reference_param(&self, index: usize) -> bool {
        matches!(
            self.local_param_mode(index),
            Some(ParamMode::In | ParamMode::Ref | ParamMode::Out)
        )
    }

    pub(crate) fn param_value_type(&self, index: usize) -> Result<String, Error> {
        let decl = self
            .function
            .body
            .locals
            .get(index)
            .ok_or_else(|| Error::Codegen("parameter referenced unknown local".into()))?;
        map_type_owned(&decl.ty, Some(self.type_layouts))?
            .ok_or_else(|| Error::Codegen("reference parameter cannot have unit type".into()))
    }

    pub(crate) fn store_place(&mut self, place: &Place, value: &ValueRef) -> Result<(), Error> {
        if self.is_reference_param(place.local.0) && place.projection.is_empty() {
            let ptr_ptr = self.place_ptr(place)?;
            let ptr_tmp = self.new_temp();
            let pointer_align = pointer_align();
            writeln!(
                &mut self.builder,
                "  {ptr_tmp} = load ptr, ptr {ptr_ptr}, align {pointer_align}"
            )
            .ok();
            let value_ty = self.param_value_type(place.local.0)?;
            let value_align = self.align_for_ty(&self.mir_ty_of_place(place)?);
            let alias_suffix = self.alias_suffix_for_place(place).unwrap_or_default();
            writeln!(
                &mut self.builder,
                "  store {value_ty} {}, ptr {ptr_tmp}, align {value_align}{alias_suffix}",
                value.repr(),
            )
            .ok();
            return Ok(());
        }

        if place.projection.is_empty() {
            let int_width = |ty: &str| ty.strip_prefix('i').and_then(|w| w.parse::<u32>().ok());
            let local_ty = self.local_tys.get(place.local.0).and_then(|ty| ty.clone());
            if local_ty.is_none() || local_ty.as_deref() == Some("void") {
                return Ok(());
            }
            let mut value = ValueRef::new(value.repr().to_string(), value.ty());
            let is_str_local = local_ty.as_deref().map(is_str_llvm_ty).unwrap_or(false);
            if is_str_local {
                if std::env::var("CHIC_DEBUG_STR_STORE").is_ok()
                    && self.function.name.contains("ReportFail")
                {
                    eprintln!(
                        "[str-store-debug] local store pre-coerce func={} local={} value={} value_ty={} local_ty={:?}",
                        self.function.name,
                        place.local.0,
                        value.repr(),
                        value.ty(),
                        local_ty
                    );
                }
                value = self.coerce_value_to_str(value);
            }
            if let Some(override_ty) = self.decimal_struct_override(place.local.0) {
                let desired =
                    map_type_owned(&override_ty, Some(self.type_layouts))?.ok_or_else(|| {
                        Error::Codegen("decimal override missing LLVM mapping".into())
                    })?;
                let current_ty = self
                    .local_tys
                    .get(place.local.0)
                    .and_then(|ty| ty.as_deref());
                let current_ptr = self
                    .local_ptrs
                    .get(place.local.0)
                    .and_then(|ptr| ptr.as_deref());
                let needs_realloc = current_ptr
                    .map(|name| !name.ends_with("_decimal"))
                    .unwrap_or(true);
                if current_ty != Some(desired.as_str()) || needs_realloc {
                    let ptr_name = format!("%l{}_decimal", place.local.0);
                    writeln!(&mut self.builder, "  {ptr_name} = alloca {desired}").ok();
                    if let Some(entry) = self.local_tys.get_mut(place.local.0) {
                        *entry = Some(desired.clone());
                    }
                    if let Some(slot) = self.local_ptrs.get_mut(place.local.0) {
                        *slot = Some(ptr_name);
                    }
                }
            }
            if let (Some(dest_width), Some(src_width)) = (
                local_ty.as_deref().and_then(int_width),
                int_width(value.ty()),
            ) {
                if src_width > dest_width {
                    let widened_ty = value.ty().to_string();
                    let ptr_name = format!("%l{}_widen", place.local.0);
                    writeln!(&mut self.builder, "  {ptr_name} = alloca {widened_ty}").ok();
                    if let Some(entry) = self.local_tys.get_mut(place.local.0) {
                        *entry = Some(widened_ty.clone());
                    }
                    if let Some(slot) = self.local_ptrs.get_mut(place.local.0) {
                        *slot = Some(ptr_name);
                    }
                }
            }
            let ptr = self.place_ptr(place)?;
            let ty = self
                .local_tys
                .get(place.local.0)
                .and_then(|ty| ty.as_ref())
                .cloned()
                .ok_or_else(|| Error::Codegen("assignment type missing".into()))?;
            if ty == "ptr" && value.ty() != "ptr" {
                let cast_tmp = self.new_temp();
                if value.ty().starts_with('i') {
                    writeln!(
                        &mut self.builder,
                        "  {cast_tmp} = inttoptr {} {} to ptr",
                        value.ty(),
                        value.repr()
                    )
                    .ok();
                } else {
                    writeln!(
                        &mut self.builder,
                        "  {cast_tmp} = bitcast {} {} to ptr",
                        value.ty(),
                        value.repr()
                    )
                    .ok();
                }
                value = ValueRef::new(cast_tmp, "ptr");
            }
            if is_str_llvm_ty(&ty) && !is_str_llvm_ty(value.ty()) {
                value = self.coerce_value_to_str(value);
            }
            let mir_ty = self
                .function
                .body
                .locals
                .get(place.local.0)
                .map(|decl| decl.ty.clone())
                .unwrap_or_else(|| Ty::Unknown);
            let align = self.align_for_ty(&mir_ty);
            let alias_suffix = self.alias_suffix_for_place(place).unwrap_or_default();
            writeln!(
                &mut self.builder,
                "  store {ty} {}, ptr {ptr}, align {align}{alias_suffix}",
                value.repr(),
            )
            .ok();
            return Ok(());
        }

        let Some(store_ty) = self.place_type(place)? else {
            return Ok(());
        };
        let mut value = ValueRef::new(value.repr().to_string(), value.ty());
        if std::env::var("CHIC_DEBUG_STR_STORE").is_ok()
            && self.function.name.contains("ReportFail")
            && is_str_llvm_ty(&store_ty)
        {
            eprintln!(
                "[str-store-debug] projected store func={} value={} value_ty={} store_ty={}",
                self.function.name,
                value.repr(),
                value.ty(),
                store_ty
            );
        }
        if store_ty == "ptr" && value.ty() != "ptr" {
            let cast_tmp = self.new_temp();
            if value.ty().starts_with('i') {
                writeln!(
                    &mut self.builder,
                    "  {cast_tmp} = inttoptr {} {} to ptr",
                    value.ty(),
                    value.repr()
                )
                .ok();
            } else {
                writeln!(
                    &mut self.builder,
                    "  {cast_tmp} = bitcast {} {} to ptr",
                    value.ty(),
                    value.repr()
                )
                .ok();
            }
            value = ValueRef::new(cast_tmp, "ptr");
        }
        if is_str_llvm_ty(&store_ty) && !is_str_llvm_ty(value.ty()) {
            value = self.coerce_value_to_str(value);
        }
        let ptr = self.place_ptr(place)?;
        let align = self.place_alignment(place)?;
        let alias_suffix = self.alias_suffix_for_place(place).unwrap_or_default();
        writeln!(
            &mut self.builder,
            "  store {store_ty} {}, ptr {ptr}, align {align}{alias_suffix}",
            value.repr(),
        )
        .ok();
        Ok(())
    }

    fn coerce_value_to_str(&mut self, value: ValueRef) -> ValueRef {
        let declared_str = is_str_llvm_ty(value.ty());

        let mut source_ty = self
            .local_ptrs
            .iter()
            .position(|ptr| ptr.as_deref() == Some(value.repr()))
            .and_then(|idx| self.local_tys.get(idx).and_then(|ty| ty.as_deref()));

        if source_ty.is_none() {
            let index = value.repr().strip_prefix("%l").and_then(|rest| {
                let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
                digits.parse::<usize>().ok()
            });
            source_ty = index
                .and_then(|idx| self.local_tys.get(idx))
                .and_then(|ty| ty.as_deref());
        }

        if std::env::var("CHIC_DEBUG_STR_STORE").is_ok()
            && self.function.name.contains("ReportFail")
        {
            eprintln!(
                "[str-store-debug] coerce func={} value={} value_ty={} source_ty={:?}",
                self.function.name,
                value.repr(),
                value.ty(),
                source_ty
            );
        }

        if let Some(src_ty) = source_ty {
            if src_ty == LLVM_STRING_TYPE {
                self.externals.insert("chic_rt_string_as_slice");
                let slice_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {slice_tmp} = call {LLVM_STR_TYPE} @chic_rt_string_as_slice(ptr {})",
                    value.repr()
                )
                .ok();
                return ValueRef::new(slice_tmp, LLVM_STR_TYPE);
            } else if is_str_llvm_ty(src_ty) {
                let load_tmp = self.new_temp();
                let align = self.align_for_ty(&Ty::Str);
                writeln!(
                    &mut self.builder,
                    "  {load_tmp} = load {LLVM_STR_TYPE}, ptr {}, align {align}",
                    value.repr()
                )
                .ok();
                return ValueRef::new(load_tmp, LLVM_STR_TYPE);
            }
        }

        if declared_str {
            return value;
        }

        let ptr = if value.ty().starts_with("ptr") {
            value.repr().to_string()
        } else {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = bitcast {} {} to ptr",
                value.ty(),
                value.repr()
            )
            .ok();
            tmp
        };
        self.externals.insert("chic_rt_string_as_slice");
        let slice_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {slice_tmp} = call {LLVM_STR_TYPE} @chic_rt_string_as_slice(ptr {ptr})"
        )
        .ok();
        ValueRef::new(slice_tmp, LLVM_STR_TYPE)
    }

    pub(crate) fn load_local_as_i64(&mut self, local: LocalId) -> Result<String, Error> {
        let ptr = self
            .local_ptrs
            .get(local.0)
            .and_then(|opt| opt.as_ref())
            .cloned()
            .ok_or_else(|| Error::Codegen("index local missing storage".into()))?;
        let ty = self
            .local_tys
            .get(local.0)
            .and_then(|ty| ty.as_deref())
            .ok_or_else(|| Error::Codegen("index local missing type".into()))?
            .to_string();
        let mir_ty = self
            .function
            .body
            .locals
            .get(local.0)
            .map(|decl| decl.ty.clone())
            .unwrap_or_else(|| Ty::Unknown);
        let align = self.align_for_ty(&mir_ty);
        let loaded = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {loaded} = load {ty}, ptr {ptr}, align {align}"
        )
        .ok();
        if ty == "i64" {
            return Ok(loaded);
        }
        if ty.starts_with('i') {
            let ext = self.new_temp();
            writeln!(&mut self.builder, "  {ext} = zext {ty} {loaded} to i64").ok();
            return Ok(ext);
        }
        if ty == "ptr" || ty.ends_with('*') {
            let int = self.new_temp();
            writeln!(&mut self.builder, "  {int} = ptrtoint {ty} {loaded} to i64").ok();
            return Ok(int);
        }
        let int = self.new_temp();
        writeln!(&mut self.builder, "  {int} = ptrtoint ptr {ptr} to i64").ok();
        Ok(int)
    }

    pub(crate) fn place_ptr(&mut self, place: &Place) -> Result<String, Error> {
        let base_ptr = self
            .local_ptrs
            .get(place.local.0)
            .and_then(|opt| opt.as_ref())
            .cloned()
            .ok_or_else(|| {
                if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
                    eprintln!(
                        "[chic-debug] missing storage for local {} in {} (locals={:?})",
                        place.local.0, self.function.name, self.local_ptrs
                    );
                }
                Error::Codegen(format!(
                    "place referenced unknown local storage (local {})",
                    place.local.0
                ))
            })?;

        if place.projection.is_empty() {
            return Ok(base_ptr);
        }

        let mut current_ptr = base_ptr;
        let mut needs_load = self
            .local_tys
            .get(place.local.0)
            .and_then(|ty| ty.as_deref())
            .map(|ty| ty == "ptr")
            .unwrap_or(false);
        let mut current_ty = self
            .function
            .body
            .locals
            .get(place.local.0)
            .ok_or_else(|| Error::Codegen("place referenced unknown local".into()))?
            .ty
            .clone();
        current_ty = self.infer_unknown_local_ty(place.local.0, current_ty);

        for elem in &place.projection {
            match elem {
                ProjectionElem::Field(index) => {
                    if needs_load {
                        let loaded = self.new_temp();
                        writeln!(
                            &mut self.builder,
                            "  {loaded} = load ptr, ptr {current_ptr}"
                        )
                        .ok();
                        current_ptr = loaded;
                        needs_load = false;
                    }
                    let (offset, ty) = self.field_info_by_index(&current_ty, *index)?;
                    current_ptr = self.offset_ptr(&current_ptr, offset)?;
                    current_ty = ty;
                    if let Some(mapped) = map_type_owned(&current_ty, Some(self.type_layouts))? {
                        needs_load = mapped == "ptr" || mapped.ends_with('*');
                    }
                }
                ProjectionElem::FieldNamed(name) => {
                    if needs_load {
                        let loaded = self.new_temp();
                        writeln!(
                            &mut self.builder,
                            "  {loaded} = load ptr, ptr {current_ptr}"
                        )
                        .ok();
                        current_ptr = loaded;
                        needs_load = false;
                    }
                    let (offset, ty) = self.field_info_by_name(&current_ty, name)?;
                    current_ptr = self.offset_ptr(&current_ptr, offset)?;
                    current_ty = ty;
                    if let Some(mapped) = map_type_owned(&current_ty, Some(self.type_layouts))? {
                        needs_load = mapped == "ptr" || mapped.ends_with('*');
                    }
                }
                ProjectionElem::Deref => {
                    let loaded = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {loaded} = load ptr, ptr {current_ptr}"
                    )
                    .ok();
                    current_ptr = loaded;
                    current_ty = self.deref_ty(&current_ty)?;
                    needs_load = false;
                }
                ProjectionElem::Index(local_id) => {
                    if needs_load {
                        let loaded = self.new_temp();
                        writeln!(
                            &mut self.builder,
                            "  {loaded} = load ptr, ptr {current_ptr}"
                        )
                        .ok();
                        current_ptr = loaded;
                        needs_load = false;
                    }
                    let index_i64 = self.load_local_as_i64(*local_id)?;
                    let mut projection_ty = current_ty.clone();
                    while let Ty::Nullable(inner) = projection_ty {
                        projection_ty = (*inner).clone();
                    }
                    current_ty = match &projection_ty {
                        Ty::Vec(vec_ty) => {
                            let element_ty = (*vec_ty.element).clone();
                            let next_ptr = self.inline_index_projection(
                                &current_ptr,
                                &Ty::Vec(vec_ty.clone()),
                                &index_i64,
                                "vec",
                                VEC_BOUNDS_PANIC_CODE,
                            )?;
                            current_ptr = next_ptr;
                            element_ty
                        }
                        Ty::Array(array_ty) => {
                            let element_ty = if array_ty.rank > 1 {
                                Ty::Array(ArrayTy::new(array_ty.element.clone(), array_ty.rank - 1))
                            } else {
                                (*array_ty.element).clone()
                            };
                            let next_ptr = self.inline_index_projection(
                                &current_ptr,
                                &Ty::Array(array_ty.clone()),
                                &index_i64,
                                "array",
                                ARRAY_BOUNDS_PANIC_CODE,
                            )?;
                            current_ptr = next_ptr;
                            element_ty
                        }
                        Ty::Span(span_ty) => {
                            let element_ty = (*span_ty.element).clone();
                            let next_ptr = self.inline_index_projection(
                                &current_ptr,
                                &Ty::Span(span_ty.clone()),
                                &index_i64,
                                "span",
                                SPAN_BOUNDS_PANIC_CODE,
                            )?;
                            current_ptr = next_ptr;
                            element_ty
                        }
                        Ty::ReadOnlySpan(span_ty) => {
                            let element_ty = (*span_ty.element).clone();
                            let next_ptr = self.inline_index_projection(
                                &current_ptr,
                                &Ty::ReadOnlySpan(span_ty.clone()),
                                &index_i64,
                                "rospan",
                                READONLY_SPAN_BOUNDS_PANIC_CODE,
                            )?;
                            current_ptr = next_ptr;
                            element_ty
                        }
                        Ty::String => {
                            let element_ty = Ty::named("char");
                            let next_ptr = self.inline_index_projection(
                                &current_ptr,
                                &Ty::String,
                                &index_i64,
                                "string",
                                STRING_BOUNDS_PANIC_CODE,
                            )?;
                            current_ptr = next_ptr;
                            element_ty
                        }
                        Ty::Str => {
                            let element_ty = Ty::named("char");
                            let next_ptr = self.inline_index_projection(
                                &current_ptr,
                                &Ty::Str,
                                &index_i64,
                                "str",
                                STR_BOUNDS_PANIC_CODE,
                            )?;
                            current_ptr = next_ptr;
                            element_ty
                        }
                        Ty::Named(named)
                            if matches!(
                                named.canonical_path().as_str(),
                                "Std::Runtime::Collections::SpanPtr"
                                    | "Std::Runtime::Collections::ReadOnlySpanPtr"
                            ) =>
                        {
                            let data_ptr_tmp = self.new_temp();
                            writeln!(
                                &mut self.builder,
                                "  {data_ptr_tmp} = load ptr, ptr {current_ptr}"
                            )
                            .ok();
                            let len_ptr = self.offset_ptr(&current_ptr, 24)?;
                            let len_tmp = self.new_temp();
                            writeln!(&mut self.builder, "  {len_tmp} = load i64, ptr {len_ptr}")
                                .ok();
                            self.emit_bounds_check(
                                &index_i64,
                                &len_tmp,
                                "spanptr_index",
                                SPAN_BOUNDS_PANIC_CODE,
                            )?;
                            let elem_size_ptr = self.offset_ptr(&current_ptr, 32)?;
                            let elem_size_tmp = self.new_temp();
                            writeln!(
                                &mut self.builder,
                                "  {elem_size_tmp} = load i64, ptr {elem_size_ptr}"
                            )
                            .ok();
                            let offset_tmp = self.new_temp();
                            writeln!(
                                &mut self.builder,
                                "  {offset_tmp} = mul i64 {index_i64}, {elem_size_tmp}"
                            )
                            .ok();
                            let next_ptr = self.offset_ptr_dynamic(&data_ptr_tmp, &offset_tmp)?;
                            current_ptr = next_ptr;
                            Ty::named("byte")
                        }
                        Ty::Unknown => {
                            let data_ptr_tmp = self.new_temp();
                            writeln!(
                                &mut self.builder,
                                "  {data_ptr_tmp} = load ptr, ptr {current_ptr}"
                            )
                            .ok();
                            let len_ptr = self.offset_ptr(&current_ptr, 24)?;
                            let len_tmp = self.new_temp();
                            writeln!(&mut self.builder, "  {len_tmp} = load i64, ptr {len_ptr}")
                                .ok();
                            self.emit_bounds_check(
                                &index_i64,
                                &len_tmp,
                                "unknown_index",
                                SPAN_BOUNDS_PANIC_CODE,
                            )?;
                            let elem_size_ptr = self.offset_ptr(&current_ptr, 32)?;
                            let elem_size_tmp = self.new_temp();
                            writeln!(
                                &mut self.builder,
                                "  {elem_size_tmp} = load i64, ptr {elem_size_ptr}"
                            )
                            .ok();
                            let offset_tmp = self.new_temp();
                            writeln!(
                                &mut self.builder,
                                "  {offset_tmp} = mul i64 {index_i64}, {elem_size_tmp}"
                            )
                            .ok();
                            let next_ptr = self.offset_ptr_dynamic(&data_ptr_tmp, &offset_tmp)?;
                            current_ptr = next_ptr;
                            Ty::Unknown
                        }
                        _ => {
                            let ty_name = current_ty.canonical_name();
                            return Err(Error::Codegen(format!(
                                "index projection on unsupported type `{ty_name}` in LLVM backend (place={place:?}, func={})",
                                self.function.name
                            )));
                        }
                    };
                }
                other => {
                    return Err(Error::Codegen(format!(
                        "projection {other:?} is not yet supported in LLVM backend"
                    )));
                }
            }
        }

        Ok(current_ptr)
    }

    pub(crate) fn deref_ty(&self, ty: &Ty) -> Result<Ty, Error> {
        match ty {
            Ty::Pointer(pointer) => Ok(pointer.element.clone()),
            Ty::Ref(reference) => Ok(reference.element.clone()),
            Ty::Named(name) => {
                let canonical = name.canonical_path();
                if canonical == "Self" {
                    if matches!(
                        self.function.kind,
                        crate::mir::FunctionKind::Method | crate::mir::FunctionKind::Constructor
                    ) {
                        if let Some((owner, _)) = self.function.name.rsplit_once("::") {
                            return Ok(Ty::named(owner.to_string()));
                        }
                    }
                }
                if self.type_layouts.layout_for_name(&canonical).is_some() {
                    return Ok(Ty::named(canonical));
                }
                let trimmed = name.trim_end();
                if let Some(without) = trimmed.strip_suffix('*') {
                    let base = without.trim_end();
                    if base.is_empty() {
                        return Err(Error::Codegen(
                            "unable to determine pointee type for deref projection in LLVM backend"
                                .into(),
                        ));
                    }
                    return Ok(Ty::named(base.to_string()));
                }
                Err(Error::Codegen(format!(
                    "deref projection applied to non-pointer `{canonical}` in LLVM backend"
                )))
            }
            Ty::Nullable(inner) => self.deref_ty(inner),
            _ => Err(Error::Codegen(
                "deref projection on unsupported type in LLVM backend".into(),
            )),
        }
    }

    fn index_element_ty(&self, ty: &Ty) -> Result<Ty, Error> {
        match ty {
            Ty::Vec(vec_ty) => Ok((*vec_ty.element).clone()),
            Ty::Array(array_ty) => Ok((*array_ty.element).clone()),
            Ty::Span(span_ty) => Ok((*span_ty.element).clone()),
            Ty::ReadOnlySpan(span_ty) => Ok((*span_ty.element).clone()),
            Ty::Named(named)
                if matches!(
                    named.canonical_path().as_str(),
                    "Std::Runtime::Collections::SpanPtr"
                        | "Std::Runtime::Collections::ReadOnlySpanPtr"
                ) =>
            {
                Ok(Ty::named("byte"))
            }
            Ty::Unknown => Ok(Ty::named("byte")),
            Ty::String | Ty::Str => Ok(Ty::named("byte")),
            Ty::Nullable(inner) => self.index_element_ty(inner),
            _ => Err(Error::Codegen(format!(
                "index projection on unsupported type `{}` in LLVM backend",
                ty.canonical_name()
            ))),
        }
    }

    pub(crate) fn projection_offset(
        &self,
        base_ty: &Ty,
        projection: &[ProjectionElem],
    ) -> Result<(usize, Ty), Error> {
        if projection.is_empty() {
            return Ok((0, base_ty.clone()));
        }

        let mut offset = 0usize;
        let mut current_ty = base_ty.clone();
        for elem in projection {
            match elem {
                ProjectionElem::Field(index) => {
                    let (field_offset, field_ty) = self.field_info_by_index(&current_ty, *index)?;
                    offset = offset.checked_add(field_offset).ok_or_else(|| {
                        Error::Codegen("field offset exceeds LLVM addressable range".into())
                    })?;
                    current_ty = field_ty;
                }
                ProjectionElem::FieldNamed(name) => {
                    if name == "IsEmpty" && matches!(current_ty, Ty::Span(_) | Ty::ReadOnlySpan(_))
                    {
                        current_ty = Ty::named("bool");
                        continue;
                    }
                    let (field_offset, field_ty) = self.field_info_by_name(&current_ty, name)?;
                    offset = offset.checked_add(field_offset).ok_or_else(|| {
                        Error::Codegen("field offset exceeds LLVM addressable range".into())
                    })?;
                    current_ty = field_ty;
                }
                ProjectionElem::Deref => {
                    current_ty = self.deref_ty(&current_ty)?;
                }
                ProjectionElem::Index(_) => {
                    current_ty = self.index_element_ty(&current_ty)?;
                }
                _ => {
                    return Err(Error::Codegen(
                        "unsupported projection in LLVM backend".into(),
                    ));
                }
            }
        }
        Ok((offset, current_ty))
    }

    pub(crate) fn dispose_symbol_for_ty(&self, ty: &Ty) -> Option<&String> {
        match ty {
            Ty::Named(name) => match self.type_layouts.types.get(name.as_str())? {
                TypeLayout::Struct(layout) | TypeLayout::Class(layout) => layout.dispose.as_ref(),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn mir_ty_of_place(&self, place: &Place) -> Result<Ty, Error> {
        let base_ty = self
            .function
            .body
            .locals
            .get(place.local.0)
            .ok_or_else(|| Error::Codegen("place referenced unknown local".into()))?
            .ty
            .clone();
        let base = self.infer_unknown_local_ty(place.local.0, base_ty);
        let (_, ty) = self.projection_offset(&base, &place.projection)?;
        if std::env::var("CHIC_DEBUG_PLACE_TYPES").is_ok() && matches!(ty, Ty::Unknown) {
            eprintln!(
                "[chic-debug] place type unknown in {}: place={:?} base_ty={} projection={:?}",
                self.function.name,
                place,
                base.canonical_name(),
                place.projection
            );
        }
        Ok(ty)
    }
}
