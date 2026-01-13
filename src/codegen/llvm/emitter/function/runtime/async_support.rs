use std::fmt::Write;

use crate::codegen::llvm::types::map_type_owned;
use crate::error::Error;
use crate::mir::TypeLayoutTable;
use crate::mir::async_types::{future_result_ty, is_future_ty, is_task_ty, task_result_ty};
use crate::mir::{GenericArg, Place, Ty};

use super::super::builder::FunctionEmitter;
use super::super::values::ValueRef;

const TASK_INNER_FUTURE_OFFSET: usize = 20;

#[derive(Debug)]
pub(crate) struct FutureResultLayout {
    pub offset: usize,
    pub llvm_ty: String,
}

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn future_header_ptr(&mut self, future: &Place) -> Result<String, Error> {
        let ty = self.mir_ty_of_place(future)?;
        if is_future_ty(&ty) {
            return self.place_ptr(future);
        }
        if is_task_ty(&ty) {
            let task_ptr = self.place_ptr(future)?;
            if let Some(offset) = self
                .type_layouts
                .layout_for_name(&ty.canonical_name())
                .and_then(|layout| match layout {
                    crate::mir::TypeLayout::Struct(layout)
                    | crate::mir::TypeLayout::Class(layout) => layout
                        .fields
                        .iter()
                        .find(|f| f.name == "Header")
                        .and_then(|f| f.offset),
                    _ => None,
                })
            {
                let field_ptr = self.offset_ptr(&task_ptr, offset)?;
                return Ok(field_ptr);
            } else {
                // Fall back to a runtime helper when layout metadata is absent (e.g. stubbed stdlib builds).
                let header_base = task_ptr;
                let header_ptr = self.new_temp();
                self.externals.insert("chic_rt_async_task_header");
                writeln!(
                    &mut self.builder,
                    "  {header_ptr} = call ptr @chic_rt_async_task_header(ptr {header_base})"
                )
                .ok();
                return Ok(header_ptr);
            }
        }
        Err(Error::Codegen(
            "await operand does not implement Std.Async::Future or Std.Async::Task".into(),
        ))
    }

    pub(crate) fn store_future_result(
        &mut self,
        future: &Place,
        layout: &FutureResultLayout,
        destination: &Place,
    ) -> Result<(), Error> {
        let base_ptr = self.place_ptr(future)?;
        let field_ptr = self.offset_ptr(&base_ptr, layout.offset)?;
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = load {}, ptr {field_ptr}",
            layout.llvm_ty
        )
        .ok();
        self.store_place(destination, &ValueRef::new(tmp, &layout.llvm_ty))
    }

    pub(crate) fn future_result_layout(
        &self,
        future_ty: &Ty,
    ) -> Result<Option<FutureResultLayout>, Error> {
        let Some(result_ty) = future_result_ty(future_ty) else {
            return Ok(None);
        };
        Ok(Some(self.future_result_layout_from_result_ty(&result_ty)?))
    }

    pub(crate) fn future_result_layout_from_result_ty(
        &self,
        result_ty: &Ty,
    ) -> Result<FutureResultLayout, Error> {
        let (offset, _) = self.future_result_offset_and_size(result_ty)?;
        let llvm_ty = map_type_owned(result_ty, Some(self.type_layouts))?.ok_or_else(|| {
            Error::Codegen("future result type lowered to void in LLVM backend".into())
        })?;
        Ok(FutureResultLayout { offset, llvm_ty })
    }

    pub(crate) fn store_task_result(
        &mut self,
        task: &Place,
        result_ty: &Ty,
        destination: &Place,
    ) -> Result<(), Error> {
        let task_ty = self.mir_ty_of_place(task)?;
        let task_ptr = self.place_ptr(task)?;
        let (offset, size) = self.task_result_access(&task_ty, result_ty)?;
        let src_ptr = self.offset_ptr(&task_ptr, offset)?;
        let dest_ptr = self.place_ptr(destination)?;
        self.externals.insert("chic_rt_async_task_result");
        let len = size;
        writeln!(
            &mut self.builder,
            "  call i32 @chic_rt_async_task_result(ptr {src_ptr}, ptr {dest_ptr}, i32 {len})"
        )
        .ok();
        Ok(())
    }

    pub(crate) fn task_result_ty(&self, task_ty: &Ty) -> Option<Ty> {
        task_result_ty(task_ty)
    }

    fn size_and_align(&self, ty: &Ty) -> Result<(usize, usize), Error> {
        self.type_layouts
            .size_and_align_for_ty(ty)
            .or_else(|| synthesize_async_layout(self.type_layouts, ty))
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "layout metadata missing for `{}` in async lowering",
                    ty.canonical_name()
                ))
            })
    }

    fn future_result_offset_and_size(&self, result_ty: &Ty) -> Result<(usize, usize), Error> {
        // Prefer explicit layout metadata for Std.Async.Future<T> to avoid mismatches with padding.
        let future_ty = Ty::named_generic(
            "Std::Async::Future",
            vec![GenericArg::Type(result_ty.clone())],
        );
        if let Some(crate::mir::TypeLayout::Struct(struct_layout)) = self
            .type_layouts
            .layout_for_name(&future_ty.canonical_name())
        {
            if let Some(field) = struct_layout
                .fields
                .iter()
                .find(|f| f.name == "Result")
                .and_then(|f| f.offset)
            {
                let (size, _) = self.size_and_align(result_ty)?;
                if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
                    eprintln!(
                        "[chic-debug] future_result_offset({}) via layout => offset={} size={}",
                        future_ty.canonical_name(),
                        field,
                        size
                    );
                }
                return Ok((field, size));
            }
        }
        let header_ty = Ty::named("Std.Async.FutureHeader");
        let (header_size, header_align) = self.size_and_align(&header_ty)?;
        let bool_ty = Ty::named("bool");
        let (bool_size, bool_align) = self.size_and_align(&bool_ty)?;
        let (result_size, result_align) = self.size_and_align(result_ty)?;

        let mut offset = align_to(0, header_align);
        offset = offset
            .checked_add(header_size)
            .ok_or_else(|| Error::Codegen("future header size exceeds addressable range".into()))?;
        offset = align_to(offset, bool_align);
        offset = offset.checked_add(bool_size).ok_or_else(|| {
            Error::Codegen("future completion flag exceeds addressable range".into())
        })?;
        offset = align_to(offset, result_align);
        if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
            eprintln!(
                "[chic-debug] future_result_offset({}) via fallback => offset={} size={}",
                result_ty.canonical_name(),
                offset,
                result_size
            );
        }
        Ok((offset, result_size))
    }

    fn task_result_access(&self, task_ty: &Ty, result_ty: &Ty) -> Result<(usize, usize), Error> {
        // Task<T> layout embeds InnerFuture after the base header/flags.
        let base_offset =
            if let Some(layout) = self.type_layouts.layout_for_name(&task_ty.canonical_name()) {
                if let crate::mir::TypeLayout::Struct(struct_layout)
                | crate::mir::TypeLayout::Class(struct_layout) = layout
                {
                    if let Some(field) = struct_layout
                        .fields
                        .iter()
                        .find(|f| f.name == "InnerFuture")
                    {
                        field.offset.unwrap_or(TASK_INNER_FUTURE_OFFSET as usize)
                    } else {
                        let base_ty = Ty::named("Std::Async::Task");
                        let (base_size, _) = self.size_and_align(&base_ty)?;
                        let future_ty = Ty::named_generic(
                            "Std::Async::Future",
                            vec![GenericArg::Type(result_ty.clone())],
                        );
                        let (_, future_align) = self.size_and_align(&future_ty)?;
                        align_to(base_size, future_align)
                    }
                } else {
                    TASK_INNER_FUTURE_OFFSET as usize
                }
            } else if let Ok((base_size, _)) = self.size_and_align(&Ty::named("Std::Async::Task")) {
                let future_ty = Ty::named_generic(
                    "Std::Async::Future",
                    vec![GenericArg::Type(result_ty.clone())],
                );
                let (_, future_align) = self.size_and_align(&future_ty)?;
                align_to(base_size, future_align)
            } else {
                TASK_INNER_FUTURE_OFFSET as usize
            };
        let (inner_offset, size) = self.future_result_offset_and_size(result_ty)?;
        if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
            eprintln!(
                "[chic-debug] task_result_access {} -> base_offset={} inner_offset={} size={}",
                task_ty.canonical_name(),
                base_offset,
                inner_offset,
                size
            );
        }
        let total_offset = align_to(base_offset, 1)
            .checked_add(inner_offset)
            .ok_or_else(|| Error::Codegen("task result offset overflow".into()))?;
        if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
            eprintln!(
                "[chic-debug] task_result_access total_offset={} (base={} inner={})",
                total_offset, base_offset, inner_offset
            );
        }
        Ok((total_offset, size))
    }
}

fn align_to(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        (value + align - 1) / align * align
    }
}

fn synthesize_async_layout(layouts: &TypeLayoutTable, ty: &Ty) -> Option<(usize, usize)> {
    if let Some(result_ty) = future_result_ty(ty) {
        // Future<T>
        let header = Ty::named("Std::Async::FutureHeader");
        let (header_size, header_align) = layouts.size_and_align_for_ty(&header)?;
        let (bool_size, bool_align) = layouts.size_and_align_for_ty(&Ty::named("bool"))?;
        let (result_size, result_align) = layouts.size_and_align_for_ty(&result_ty)?;
        let mut offset = align_to(header_size, bool_align);
        offset = align_to(offset + bool_size, result_align);
        let size = align_to(offset + result_size, header_align.max(result_align));
        let align = header_align.max(result_align).max(bool_align);
        return Some((size, align));
    }

    if is_task_ty(ty) {
        // Task<T>
        let header_ty = Ty::named("Std::Async::FutureHeader");
        let (header_size, header_align) = layouts.size_and_align_for_ty(&header_ty)?;
        let (flags_size, flags_align) = layouts.size_and_align_for_ty(&Ty::named("uint"))?;
        let base_size = align_to(header_size + flags_size, header_align.max(flags_align));
        if let Some(inner_ty) = task_result_ty(ty) {
            let future_ty = Ty::named_generic(
                "Std::Async::Future",
                vec![GenericArg::Type(inner_ty.clone())],
            );
            let (future_size, future_align) = synthesize_async_layout(layouts, &future_ty)
                .or_else(|| layouts.size_and_align_for_ty(&future_ty))?;
            let inner_offset = align_to(base_size, future_align);
            let size = align_to(inner_offset + future_size, header_align.max(future_align));
            let align = header_align.max(future_align).max(flags_align);
            return Some((size, align));
        }
        let size = base_size;
        let align = header_align.max(flags_align);
        return Some((size, align));
    }

    None
}
