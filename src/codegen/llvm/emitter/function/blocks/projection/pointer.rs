use std::fmt::Write;

use super::*;
use crate::codegen::llvm::types::map_type_owned;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn offset_ptr(&mut self, base_ptr: &str, offset: usize) -> Result<String, Error> {
        if offset == 0 {
            return Ok(base_ptr.to_string());
        }
        let ptrtoint = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {ptrtoint} = ptrtoint ptr {base_ptr} to i64"
        )
        .ok();
        let added = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {added} = add i64 {ptrtoint}, {}",
            offset as i64
        )
        .ok();
        let result = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {result} = inttoptr i64 {added} to ptr"
        )
        .ok();
        Ok(result)
    }

    pub(crate) fn load_struct_field(
        &mut self,
        base_ptr: &str,
        offset: usize,
        ty: &Ty,
    ) -> Result<(String, String), Error> {
        let field_ptr = self.offset_ptr(base_ptr, offset)?;
        let llvm_ty = map_type_owned(ty, Some(self.type_layouts))?
            .ok_or_else(|| Error::Codegen("field type missing LLVM mapping".into()))?;
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = load {llvm_ty}, ptr {field_ptr}"
        )
        .ok();
        Ok((tmp, llvm_ty))
    }

    pub(crate) fn load_struct_usize(
        &mut self,
        base_ptr: &str,
        offset: usize,
        ty: &Ty,
    ) -> Result<String, Error> {
        let (value, llvm_ty) = self.load_struct_field(base_ptr, offset, ty)?;
        if llvm_ty == "i64" {
            Ok(value)
        } else if llvm_ty == "i32" {
            let cast = self.new_temp();
            writeln!(&mut self.builder, "  {cast} = zext i32 {value} to i64").ok();
            Ok(cast)
        } else if llvm_ty == "ptr" {
            let cast = self.new_temp();
            writeln!(&mut self.builder, "  {cast} = ptrtoint ptr {value} to i64").ok();
            Ok(cast)
        } else if llvm_ty.starts_with('{') || llvm_ty.starts_with('[') {
            // Some synthesized fields may point at inline buffer aggregates; treat them as zero-sized.
            Ok("0".to_string())
        } else {
            Err(Error::Codegen(format!(
                "unsupported usize representation `{llvm_ty}` in LLVM backend"
            )))
        }
    }

    pub(crate) fn offset_ptr_dynamic(
        &mut self,
        base_ptr: &str,
        offset: &str,
    ) -> Result<String, Error> {
        let ptrtoint = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {ptrtoint} = ptrtoint ptr {base_ptr} to i64"
        )
        .ok();
        let added = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {added} = add i64 {ptrtoint}, {offset}"
        )
        .ok();
        let result = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {result} = inttoptr i64 {added} to ptr"
        )
        .ok();
        Ok(result)
    }

    pub(crate) fn emit_bounds_check(
        &mut self,
        index: &str,
        len: &str,
        context: &str,
        panic_code: i32,
    ) -> Result<(), Error> {
        let cmp = self.new_temp();
        writeln!(&mut self.builder, "  {cmp} = icmp uge i64 {index}, {len}").ok();
        let panic_label = self.fresh_label(&format!("{context}_bounds_panic"));
        let ok_label = self.fresh_label(&format!("{context}_bounds_ok"));
        writeln!(
            &mut self.builder,
            "  br i1 {cmp}, label %{panic_label}, label %{ok_label}"
        )
        .ok();
        writeln!(&mut self.builder, "{panic_label}:").ok();
        self.externals.insert("chic_rt_panic");
        let call_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {call_tmp} = call i32 @chic_rt_panic(i32 {panic_code})"
        )
        .ok();
        writeln!(&mut self.builder, "  unreachable").ok();
        writeln!(&mut self.builder, "{ok_label}:").ok();
        Ok(())
    }

    pub(crate) fn fixed_elem_size_for(&self, ty: &Ty) -> Option<u64> {
        match ty {
            Ty::String | Ty::Str => self
                .type_layouts
                .size_and_align_for_ty(&Ty::named("char"))
                .map(|(size, _)| size as u64),
            _ => None,
        }
    }

    pub(crate) fn inline_index_projection(
        &mut self,
        base_ptr: &str,
        base_ty: &Ty,
        index_value: &str,
        context: &str,
        panic_code: i32,
    ) -> Result<String, Error> {
        let (ptr_offset, ptr_ty) = self.field_info_by_name(base_ty, "ptr")?;
        let (len_offset, len_ty) = self.field_info_by_name(base_ty, "len")?;
        let elem_size = match self.field_info_by_name(base_ty, "elem_size") {
            Ok((offset, ty)) => {
                InlineElemSize::Dynamic(self.load_struct_usize(base_ptr, offset, &ty)?)
            }
            Err(_) => {
                let fixed = self.fixed_elem_size_for(base_ty).ok_or_else(|| {
                    Error::Codegen(format!(
                        "type `{}` is missing elem_size metadata required for intrinsic indexing",
                        base_ty.canonical_name()
                    ))
                })?;
                InlineElemSize::Const(fixed)
            }
        };
        let (ptr_value, llvm_ty) = self.load_struct_field(base_ptr, ptr_offset, &ptr_ty)?;
        let data_ptr = if llvm_ty == "ptr" {
            ptr_value
        } else {
            let cast = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {cast} = bitcast {llvm_ty} {ptr_value} to ptr"
            )
            .ok();
            cast
        };
        let len_value = self.load_struct_usize(base_ptr, len_offset, &len_ty)?;
        self.emit_bounds_check(index_value, &len_value, context, panic_code)?;
        let scaled = self.new_temp();
        match elem_size {
            InlineElemSize::Dynamic(value) => {
                writeln!(
                    &mut self.builder,
                    "  {scaled} = mul i64 {index_value}, {value}"
                )
                .ok();
            }
            InlineElemSize::Const(value) => {
                writeln!(
                    &mut self.builder,
                    "  {scaled} = mul i64 {index_value}, {value}"
                )
                .ok();
            }
        }
        self.offset_ptr_dynamic(&data_ptr, &scaled)
    }
}
