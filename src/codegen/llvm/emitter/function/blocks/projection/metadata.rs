use super::*;
use crate::mir::{GenericArg, casts::short_type_name, pointer_size};
use std::collections::HashSet;

const FUTURE_COMPLETED_OFFSET: usize = 32;
const FUTURE_RESULT_OFFSET: usize = 36;
const FUTURE_HEADER_FLAGS_OFFSET: usize = 24;
const TASK_FLAGS_OFFSET: usize = 32;
const TASK_INNER_FUTURE_OFFSET: usize = 40;

impl<'a> FunctionEmitter<'a> {
    fn layout_for_field_access(&self, key: &str) -> Option<&TypeLayout> {
        self.type_layouts
            .layout_for_name(key)
            .or_else(|| {
                self.type_layouts
                    .primitive_registry
                    .descriptor_for_name(key)
                    .and_then(|desc| desc.std_wrapper_type.as_deref())
                    .and_then(|wrapper| self.type_layouts.layout_for_name(wrapper))
            })
            .or_else(|| self.layout_for_field_access_in_owner_namespace(key))
    }

    fn layout_for_field_access_in_owner_namespace(&self, key: &str) -> Option<&TypeLayout> {
        if key.contains("::") || key.contains('.') || key.contains('<') {
            return None;
        }
        let owner = self
            .function
            .name
            .rsplit_once("::")
            .map(|(owner, _)| owner)?;
        let candidates = [
            Some(format!("{owner}::{key}")),
            owner
                .rsplit_once("::")
                .map(|(namespace, _)| format!("{namespace}::{key}")),
        ];
        candidates
            .into_iter()
            .flatten()
            .find_map(|candidate| self.type_layouts.layout_for_name(&candidate))
    }

    pub(crate) fn field_info_by_index(&self, ty: &Ty, index: u32) -> Result<(usize, Ty), Error> {
        match ty {
            Ty::Named(_) => {
                let canonical = ty.canonical_name();
                self.field_info_by_index_named(&canonical, index)
            }
            Ty::Vec(vec_ty) => {
                let canonical = Ty::Vec(vec_ty.clone()).canonical_name();
                self.field_info_by_index_named(&canonical, index)
            }
            Ty::Span(span_ty) => {
                let canonical = Ty::Span(span_ty.clone()).canonical_name();
                self.field_info_by_index_named(&canonical, index)
            }
            Ty::ReadOnlySpan(span_ty) => {
                let canonical = Ty::ReadOnlySpan(span_ty.clone()).canonical_name();
                self.field_info_by_index_named(&canonical, index)
            }
            Ty::String => self.field_info_by_index_named("string", index),
            Ty::Str => self.field_info_by_index_named("str", index),
            Ty::Rc(rc_ty) => {
                let canonical = Ty::Rc(rc_ty.clone()).canonical_name();
                self.field_info_by_index_named(&canonical, index)
            }
            Ty::Arc(arc_ty) => {
                let canonical = Ty::Arc(arc_ty.clone()).canonical_name();
                self.field_info_by_index_named(&canonical, index)
            }
            Ty::Fn(fn_ty) => {
                let canonical = fn_ty.canonical_name();
                self.field_info_by_index_named(&canonical, index)
            }
            Ty::Nullable(inner) => {
                let canonical = Ty::Nullable(inner.clone()).canonical_name();
                self.field_info_by_index_named(&canonical, index)
            }
            Ty::Tuple(tuple) => {
                let name = tuple.canonical_name();
                let layout =
                    self.type_layouts.types.get(&name).ok_or_else(|| {
                        Error::Codegen(format!("tuple layout `{name}` not recorded"))
                    })?;
                let struct_layout = match layout {
                    TypeLayout::Struct(layout) => layout,
                    _ => {
                        return Err(Error::Codegen(
                            "tuple layout missing struct representation".into(),
                        ));
                    }
                };
                let field = struct_layout
                    .fields
                    .iter()
                    .find(|field| field.index == index)
                    .ok_or_else(|| Error::Codegen("tuple field layout missing".into()))?;
                let offset = field
                    .offset
                    .ok_or_else(|| Error::Codegen("tuple field offset metadata missing".into()))?;
                Ok((offset, field.ty.clone()))
            }
            other => Err(Error::Codegen(format!(
                "field projection on unsupported type in LLVM backend (ty={other:?} index={index})"
            ))),
        }
    }

    pub(crate) fn field_info_by_index_named(
        &self,
        key: &str,
        index: u32,
    ) -> Result<(usize, Ty), Error> {
        if short_type_name(key) == "Self" {
            let resolved = self.resolve_self_type_name().ok_or_else(|| {
                Error::Codegen("unable to resolve `Self` for field projection".into())
            })?;
            return self.field_info_by_index_named(&resolved, index);
        }
        let layout = self
            .layout_for_field_access(key)
            .ok_or_else(|| Error::Codegen(format!("type layout for `{key}` not recorded")))?;
        let struct_layout = match layout {
            TypeLayout::Struct(layout) | TypeLayout::Class(layout) => layout,
            _ => {
                return Err(Error::Codegen(
                    "field projection on non-struct/class type is not supported in LLVM backend"
                        .into(),
                ));
            }
        };
        let field = struct_layout
            .fields
            .iter()
            .find(|field| field.index == index);
        let field = field.ok_or_else(|| {
            eprintln!(
                "projection: missing field index {index} on type `{}` in function {} ({} fields)",
                struct_layout.name,
                self.function.name,
                struct_layout.fields.len()
            );
            Error::Codegen(format!(
                "field index {index} missing on type `{}`",
                struct_layout.name
            ))
        })?;
        let offset = field.offset.ok_or_else(|| {
            Error::Codegen(format!(
                "field `{}` missing offset metadata for LLVM backend (type `{}`)",
                field.name, struct_layout.name
            ))
        })?;
        Ok((offset, field.ty.clone()))
    }

    pub(crate) fn field_info_by_name(&self, ty: &Ty, name: &str) -> Result<(usize, Ty), Error> {
        let mut visited = HashSet::new();
        self.field_info_by_name_inner(ty, name, &mut visited)
    }

    fn field_info_by_name_inner(
        &self,
        ty: &Ty,
        name: &str,
        visited: &mut HashSet<String>,
    ) -> Result<(usize, Ty), Error> {
        if let Some((offset, field_ty)) = self.async_field_fallback(ty, name) {
            return Ok((offset, field_ty));
        }
        match ty {
            Ty::Unknown => match name {
                "Status" => Ok((0, Ty::named("Std::Numeric::Decimal::DecimalStatus"))),
                "Value" => Ok((16, Ty::named("decimal"))),
                "Variant" => Ok((
                    32,
                    Ty::named("Std::Numeric::Decimal::DecimalIntrinsicVariant"),
                )),
                _ => {
                    eprintln!(
                        "[field_info_by_name] func={} ty={} name={}",
                        self.function.name,
                        ty.canonical_name(),
                        name
                    );
                    Err(Error::Codegen(format!(
                        "named field projection on unsupported type `{}` in LLVM backend",
                        ty.canonical_name()
                    )))
                }
            },
            Ty::Named(_) => {
                let canonical = ty.canonical_name();
                self.field_info_by_name_named_inner(&canonical, name, visited)
            }
            Ty::Vec(vec_ty) => {
                let canonical = Ty::Vec(vec_ty.clone()).canonical_name();
                self.field_info_by_name_named_inner(&canonical, name, visited)
            }
            Ty::Array(array_ty) => {
                let canonical = Ty::Array(array_ty.clone()).canonical_name();
                self.field_info_by_name_named_inner(&canonical, name, visited)
            }
            Ty::Span(span_ty) => {
                let canonical = Ty::Span(span_ty.clone()).canonical_name();
                self.field_info_by_name_named_inner(&canonical, name, visited)
            }
            Ty::ReadOnlySpan(span_ty) => {
                let canonical = Ty::ReadOnlySpan(span_ty.clone()).canonical_name();
                self.field_info_by_name_named_inner(&canonical, name, visited)
            }
            Ty::Rc(rc_ty) => {
                let canonical = Ty::Rc(rc_ty.clone()).canonical_name();
                self.field_info_by_name_named_inner(&canonical, name, visited)
            }
            Ty::Arc(arc_ty) => {
                let canonical = Ty::Arc(arc_ty.clone()).canonical_name();
                self.field_info_by_name_named_inner(&canonical, name, visited)
            }
            Ty::Fn(fn_ty) => {
                let canonical = fn_ty.canonical_name();
                self.field_info_by_name_named_inner(&canonical, name, visited)
            }
            Ty::String => self.field_info_by_name_named_inner("string", name, visited),
            Ty::Str => self.field_info_by_name_named_inner("str", name, visited),
            Ty::Pointer(pointer) => self.field_info_by_name_inner(&pointer.element, name, visited),
            Ty::Ref(reference) => self.field_info_by_name_inner(&reference.element, name, visited),
            Ty::Nullable(inner) => self.field_info_by_name_inner(inner, name, visited),
            other => Err(Error::Codegen(format!(
                "named field projection on unsupported type `{}` in LLVM backend",
                other.canonical_name()
            ))),
        }
    }

    fn async_field_fallback(&self, ty: &Ty, name: &str) -> Option<(usize, Ty)> {
        let canonical = ty.canonical_name();
        let canonical_dot = canonical.replace("::", ".");
        if let Some(layout) = self.layout_for_field_access(&canonical) {
            if let TypeLayout::Struct(struct_layout) | TypeLayout::Class(struct_layout) = layout {
                if let Some(field) = struct_layout.fields.iter().find(|f| f.name == name) {
                    if let Some(offset) = field.offset {
                        return Some((offset, field.ty.clone()));
                    }
                }
            }
        }
        let is_task = canonical == "Std::Async::Task"
            || canonical_dot == "Std.Async.Task"
            || canonical == "Task"
            || canonical.starts_with("Std::Async::Task<")
            || canonical_dot.starts_with("Std.Async.Task<")
            || canonical.starts_with("Task<");
        if is_task {
            return match name {
                "Header" => Some((0, Ty::named("Std.Async.FutureHeader"))),
                "Flags" => Some((TASK_FLAGS_OFFSET, Ty::named("uint"))),
                "InnerFuture" => {
                    let inner_ty = ty.as_named().and_then(|named| named.args.get(0));
                    let inner_future = inner_ty.and_then(|arg| match arg {
                        GenericArg::Type(inner) => Some(inner.clone()),
                        _ => None,
                    });
                    let future_ty = inner_future.map_or_else(
                        || Ty::named("Std.Async.Future"),
                        |inner| {
                            Ty::named_generic(
                                "Std.Async.Future",
                                vec![GenericArg::Type(inner.clone())],
                            )
                        },
                    );
                    Some((TASK_INNER_FUTURE_OFFSET, future_ty))
                }
                _ => None,
            };
        }

        let is_future = canonical == "Std::Async::Future"
            || canonical_dot == "Std.Async.Future"
            || canonical == "Future"
            || canonical.starts_with("Std::Async::Future<")
            || canonical_dot.starts_with("Std.Async.Future<")
            || canonical.starts_with("Future<");
        if is_future {
            return match name {
                "Header" => Some((0, Ty::named("Std.Async.FutureHeader"))),
                "Completed" => Some((FUTURE_COMPLETED_OFFSET, Ty::named("bool"))),
                "Result" => {
                    let inner_ty = ty.as_named().and_then(|named| named.args.get(0));
                    let result_ty = inner_ty.and_then(|arg| match arg {
                        GenericArg::Type(inner) => Some(inner.clone()),
                        _ => None,
                    });
                    Some((FUTURE_RESULT_OFFSET, result_ty.unwrap_or(Ty::Unknown)))
                }
                _ => None,
            };
        }

        if canonical == "Std::Async::FutureHeader"
            || canonical_dot == "Std.Async.FutureHeader"
            || canonical == "FutureHeader"
        {
            return match name {
                "Flags" => Some((FUTURE_HEADER_FLAGS_OFFSET, Ty::named("uint"))),
                _ => None,
            };
        }

        None
    }

    fn field_info_by_name_named_inner(
        &self,
        key: &str,
        name: &str,
        visited: &mut HashSet<String>,
    ) -> Result<(usize, Ty), Error> {
        if short_type_name(key) == "Self" {
            let resolved = self.resolve_self_type_name().ok_or_else(|| {
                Error::Codegen("unable to resolve `Self` for field projection".into())
            })?;
            return self.field_info_by_name_named_inner(&resolved, name, visited);
        }
        let canonical_key = if key == "DecimalIntrinsicResult"
            || key.ends_with("::DecimalIntrinsicResult")
        {
            "Std::Numeric::Decimal::DecimalIntrinsicResult"
        } else if key == "DecimalRuntimeCall" || key.ends_with("::DecimalRuntimeCall") {
            "Std::Numeric::Decimal::DecimalRuntimeCall"
        } else if key == "DecimalIntrinsicVariant" || key.ends_with("::DecimalIntrinsicVariant") {
            "Std::Numeric::Decimal::DecimalIntrinsicVariant"
        } else if key == "DecimalConstPtr"
            || key.ends_with("::DecimalConstPtr")
            || key.ends_with("::Decimal::DecimalConstPtr")
        {
            "Std::Runtime::Native::DecimalConstPtr"
        } else if key == "DecimalMutPtr"
            || key.ends_with("::DecimalMutPtr")
            || key.ends_with("::Decimal::DecimalMutPtr")
        {
            "Std::Runtime::Native::DecimalMutPtr"
        } else {
            key
        };
        let visit_key = canonical_key.replace('.', "::");
        if !visited.insert(visit_key.clone()) {
            return Err(Error::Codegen(format!(
                "cyclic field lookup for `{visit_key}` while resolving `{name}`"
            )));
        }
        let result = (|| {
            let base_name = canonical_key.split('<').next().unwrap_or(canonical_key);
            let short_base = crate::mir::casts::short_type_name(base_name);
            if canonical_key == "str" || short_base == "str" || base_name.ends_with("::str") {
                let mut qualifiers = crate::mir::PointerQualifiers::default();
                qualifiers.expose_address = true;
                let ptr_ty = Ty::Pointer(Box::new(crate::mir::PointerTy::with_qualifiers(
                    Ty::named("byte"),
                    true,
                    qualifiers,
                )));
                return match name {
                    "ptr" | "Pointer" => Ok((0, ptr_ty)),
                    "len" | "Length" => Ok((pointer_size(), Ty::named("usize"))),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{key}` (function: {})",
                        self.function.name
                    ))),
                };
            }
            if short_base == "StringInlineBytes32" || base_name.ends_with("::StringInlineBytes32") {
                if let Some(index) = name
                    .strip_prefix('b')
                    .and_then(|digits| digits.parse::<usize>().ok())
                {
                    if index < 32 {
                        return Ok((index, Ty::named("byte")));
                    }
                }
            }
            if short_base == "StringInlineBytes64" || base_name.ends_with("::StringInlineBytes64") {
                if let Some(index) = name
                    .strip_prefix('b')
                    .and_then(|digits| digits.parse::<usize>().ok())
                {
                    if index < 64 {
                        return Ok((index, Ty::named("byte")));
                    }
                }
            }
            let layout = if let Some(layout) = self.layout_for_field_access(canonical_key) {
                layout
            } else if short_base == "ReadOnlySpan" || base_name.ends_with("::ReadOnlySpan") {
                if name == "Handle" || name == "Raw" {
                    return Ok((0, Ty::named("Std::Span::ReadOnlySpanPtr".to_string())));
                }
                if matches!(name, "Data" | "ptr" | "len" | "elem_size") {
                    let raw_ty = Ty::named("Std::Span::ReadOnlySpanPtr".to_string());
                    let raw_offset = 0usize;
                    match name {
                        "Data" => {
                            let (data_offset, data_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                            return Ok((raw_offset + data_offset, data_ty));
                        }
                        "ptr" => {
                            let (data_offset, data_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                            let (ptr_offset, ptr_ty) =
                                self.field_info_by_name_inner(&data_ty, "Pointer", visited)?;
                            return Ok((raw_offset + data_offset + ptr_offset, ptr_ty));
                        }
                        "len" => {
                            let (len_offset, len_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Length", visited)?;
                            return Ok((raw_offset + len_offset, len_ty));
                        }
                        "elem_size" => {
                            let (elem_offset, elem_ty) =
                                self.field_info_by_name_inner(&raw_ty, "ElementSize", visited)?;
                            return Ok((raw_offset + elem_offset, elem_ty));
                        }
                        _ => {}
                    }
                }
                return Err(Error::Codegen(format!(
                    "field `{name}` missing on type `{key}`"
                )));
            } else if short_base == "Span" || base_name.ends_with("::Span") {
                if name == "Handle" || name == "Raw" {
                    return Ok((0, Ty::named("Std::Span::SpanPtr".to_string())));
                }
                if matches!(name, "Data" | "ptr" | "len" | "elem_size") {
                    let raw_ty = Ty::named("Std::Span::SpanPtr".to_string());
                    let raw_offset = 0usize;
                    match name {
                        "Data" => {
                            let (data_offset, data_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                            return Ok((raw_offset + data_offset, data_ty));
                        }
                        "ptr" => {
                            let (data_offset, data_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                            let (ptr_offset, ptr_ty) =
                                self.field_info_by_name_inner(&data_ty, "Pointer", visited)?;
                            return Ok((raw_offset + data_offset + ptr_offset, ptr_ty));
                        }
                        "len" => {
                            let (len_offset, len_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Length", visited)?;
                            return Ok((raw_offset + len_offset, len_ty));
                        }
                        "elem_size" => {
                            let (elem_offset, elem_ty) =
                                self.field_info_by_name_inner(&raw_ty, "ElementSize", visited)?;
                            return Ok((raw_offset + elem_offset, elem_ty));
                        }
                        _ => {}
                    }
                }
                if let Some((offset, ty)) =
                    self.async_field_fallback(&Ty::named(canonical_key.to_string()), name)
                {
                    return Ok((offset, ty));
                }
                return Err(Error::Codegen(format!(
                    "field `{name}` missing on type `{key}` (function: {})",
                    self.function.name
                )));
            } else if short_base == "SpanPtr" || base_name.ends_with("::SpanPtr") {
                let data_ty = Ty::named("Std::Runtime::Collections::ValueMutPtr");
                let base_offset = 3usize.saturating_mul(pointer_size());
                return match name {
                    "Data" => Ok((0, data_ty)),
                    "Pointer" | "ptr" => {
                        let (offset, ty) =
                            self.field_info_by_name_inner(&data_ty, "Pointer", visited)?;
                        Ok((offset, ty))
                    }
                    "Length" | "len" => Ok((base_offset, Ty::named("usize"))),
                    "ElementSize" | "elem_size" => Ok((
                        base_offset.saturating_add(pointer_size()),
                        Ty::named("usize"),
                    )),
                    "ElementAlignment" => Ok((
                        base_offset.saturating_add(pointer_size() * 2),
                        Ty::named("usize"),
                    )),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{key}` (function: {})",
                        self.function.name
                    ))),
                };
            } else if short_base == "ReadOnlySpanPtr" || base_name.ends_with("::ReadOnlySpanPtr") {
                if std::env::var("CHIC_DEBUG_PROJECTIONS").is_ok() {
                    eprintln!(
                        "[projection-debug] readonly span ptr field lookup key={} name={}",
                        key, name
                    );
                }
                let data_ty = Ty::named("Std::Runtime::Collections::ValueConstPtr");
                let base_offset = 3usize.saturating_mul(pointer_size());
                return match name {
                    "Data" => Ok((0, data_ty)),
                    "Pointer" | "ptr" => {
                        let (offset, ty) =
                            self.field_info_by_name_inner(&data_ty, "Pointer", visited)?;
                        Ok((offset, ty))
                    }
                    "Length" | "len" => Ok((base_offset, Ty::named("usize"))),
                    "ElementSize" | "elem_size" => Ok((
                        base_offset.saturating_add(pointer_size()),
                        Ty::named("usize"),
                    )),
                    "ElementAlignment" => Ok((
                        base_offset.saturating_add(pointer_size() * 2),
                        Ty::named("usize"),
                    )),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{key}` (function: {})",
                        self.function.name
                    ))),
                };
            } else if short_base == "ValueMutPtr" || base_name.ends_with("::ValueMutPtr") {
                let mut qualifiers = crate::mir::PointerQualifiers::default();
                qualifiers.expose_address = true;
                let ptr_ty = Ty::Pointer(Box::new(crate::mir::PointerTy {
                    element: Ty::named("byte"),
                    mutable: true,
                    qualifiers,
                }));
                return match name {
                    "Pointer" | "ptr" => Ok((0, ptr_ty)),
                    "Size" | "size" => Ok((pointer_size(), Ty::named("usize"))),
                    "Alignment" | "alignment" => {
                        Ok((pointer_size().saturating_mul(2), Ty::named("usize")))
                    }
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{key}` (function: {})",
                        self.function.name
                    ))),
                };
            } else if short_base == "ValueConstPtr" || base_name.ends_with("::ValueConstPtr") {
                let mut qualifiers = crate::mir::PointerQualifiers::default();
                qualifiers.readonly = true;
                qualifiers.expose_address = true;
                let ptr_ty = Ty::Pointer(Box::new(crate::mir::PointerTy {
                    element: Ty::named("byte"),
                    mutable: false,
                    qualifiers,
                }));
                return match name {
                    "Pointer" | "ptr" => Ok((0, ptr_ty)),
                    "Size" | "size" => Ok((pointer_size(), Ty::named("usize"))),
                    "Alignment" | "alignment" => {
                        Ok((pointer_size().saturating_mul(2), Ty::named("usize")))
                    }
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{key}` (function: {})",
                        self.function.name
                    ))),
                };
            } else if short_base == "StrPtr" || base_name.ends_with("::StrPtr") {
                let mut qualifiers = crate::mir::PointerQualifiers::default();
                qualifiers.readonly = true;
                qualifiers.expose_address = true;
                let ptr_ty = Ty::Pointer(Box::new(crate::mir::PointerTy {
                    element: Ty::named("byte"),
                    mutable: false,
                    qualifiers,
                }));
                return match name {
                    "Pointer" | "ptr" => Ok((0, ptr_ty)),
                    "Length" | "len" => Ok((pointer_size(), Ty::named("usize"))),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{key}` (function: {})",
                        self.function.name
                    ))),
                };
            } else if short_base == "CharSpanPtr" || base_name.ends_with("::CharSpanPtr") {
                let mut qualifiers = crate::mir::PointerQualifiers::default();
                qualifiers.readonly = true;
                qualifiers.expose_address = true;
                let ptr_ty = Ty::Pointer(Box::new(crate::mir::PointerTy {
                    element: Ty::named("ushort"),
                    mutable: false,
                    qualifiers,
                }));
                return match name {
                    "Pointer" | "ptr" => Ok((0, ptr_ty)),
                    "Length" | "len" => Ok((pointer_size(), Ty::named("usize"))),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{key}` (function: {})",
                        self.function.name
                    ))),
                };
            } else if short_base == "DecimalConstPtr" || base_name.ends_with("::DecimalConstPtr") {
                let mut qualifiers = crate::mir::PointerQualifiers::default();
                qualifiers.readonly = true;
                qualifiers.expose_address = true;
                let ptr_ty = Ty::Pointer(Box::new(crate::mir::PointerTy {
                    element: Ty::named("Std::Runtime::Native::Decimal128Parts"),
                    mutable: false,
                    qualifiers,
                }));
                return match name {
                    "ptr" | "Pointer" => Ok((0, ptr_ty)),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{key}` (function: {})",
                        self.function.name
                    ))),
                };
            } else if short_base == "DecimalMutPtr" || base_name.ends_with("::DecimalMutPtr") {
                let mut qualifiers = crate::mir::PointerQualifiers::default();
                qualifiers.expose_address = true;
                let ptr_ty = Ty::Pointer(Box::new(crate::mir::PointerTy {
                    element: Ty::named("Std::Runtime::Native::Decimal128Parts"),
                    mutable: true,
                    qualifiers,
                }));
                return match name {
                    "ptr" | "Pointer" => Ok((0, ptr_ty)),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{key}` (function: {})",
                        self.function.name
                    ))),
                };
            } else if short_base == "RegionHandle" || base_name.ends_with("::RegionHandle") {
                let mut qualifiers = crate::mir::PointerQualifiers::default();
                qualifiers.expose_address = true;
                let ptr_ty = Ty::Pointer(Box::new(crate::mir::PointerTy {
                    element: Ty::named("byte"),
                    mutable: true,
                    qualifiers,
                }));
                if name == "Pointer" || name == "ptr" {
                    return Ok((0, ptr_ty));
                }
                let profile_offset = pointer_size();
                let generation_offset = pointer_size().saturating_mul(2);
                return match name {
                    "Profile" => Ok((profile_offset, Ty::named("ulong"))),
                    "Generation" => Ok((generation_offset, Ty::named("ulong"))),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{key}` (function: {})",
                        self.function.name
                    ))),
                };
            } else if short_base == "StringInlineBytes32"
                || base_name.ends_with("::StringInlineBytes32")
            {
                if let Some(index) = name
                    .strip_prefix('b')
                    .and_then(|digits| digits.parse::<usize>().ok())
                {
                    if index < 32 {
                        return Ok((index, Ty::named("byte")));
                    }
                }
                return Err(Error::Codegen(format!(
                    "field `{name}` missing on type `{key}` (function: {})",
                    self.function.name
                )));
            } else if short_base == "StringInlineBytes64"
                || base_name.ends_with("::StringInlineBytes64")
            {
                if let Some(index) = name
                    .strip_prefix('b')
                    .and_then(|digits| digits.parse::<usize>().ok())
                {
                    if index < 64 {
                        return Ok((index, Ty::named("byte")));
                    }
                }
                return Err(Error::Codegen(format!(
                    "field `{name}` missing on type `{key}` (function: {})",
                    self.function.name
                )));
            } else {
                if short_base == "StringInlineBytes32"
                    || base_name.ends_with("::StringInlineBytes32")
                {
                    if let Some(index) = name
                        .strip_prefix('b')
                        .and_then(|digits| digits.parse::<usize>().ok())
                    {
                        if index < 32 {
                            return Ok((index, Ty::named("byte")));
                        }
                    }
                }
                if short_base == "StringInlineBytes64"
                    || base_name.ends_with("::StringInlineBytes64")
                {
                    if let Some(index) = name
                        .strip_prefix('b')
                        .and_then(|digits| digits.parse::<usize>().ok())
                    {
                        if index < 64 {
                            return Ok((index, Ty::named("byte")));
                        }
                    }
                }
                if let Some((offset, ty)) =
                    self.async_field_fallback(&Ty::named(canonical_key.to_string()), name)
                {
                    return Ok((offset, ty));
                }
                if std::env::var("CHIC_DEBUG_PROJECTIONS").is_ok() {
                    eprintln!(
                        "[projection] missing layout for `{key}` (canonical `{canonical_key}`, short `{short_base}`), field `{name}`; using unknown fallback"
                    );
                }
                return Err(Error::Codegen(format!(
                    "type layout for `{key}` not recorded; missing field `{name}` metadata (function: {})",
                    self.function.name
                )));
            };
            let struct_layout = match layout {
                TypeLayout::Struct(layout) | TypeLayout::Class(layout) => layout,
                _ => {
                    if std::env::var("CHIC_DEBUG_PROJECTIONS").is_ok() {
                        eprintln!(
                            "[projection-debug] non-struct/class type `{key}` for field `{name}` in `{}`; synthesizing int field at offset 0",
                            self.function.name
                        );
                    }
                    return Ok((0, Ty::named("int")));
                }
            };
            let field_name = if (struct_layout.name == "string"
                || base_name == "Array"
                || base_name.ends_with("::Array"))
                && matches!(name, "Length" | "Count")
            {
                "len"
            } else {
                name
            };
            let field = if let Some(field) = struct_layout
                .fields
                .iter()
                .find(|field| field.name == field_name)
            {
                field
            } else if short_base == "ReadOnlySpan" || base_name.ends_with("::ReadOnlySpan") {
                let raw_ty = Ty::named("Std::Span::ReadOnlySpanPtr".to_string());
                let raw_offset = 0usize;
                return match name {
                    "Handle" | "Raw" => Ok((raw_offset, raw_ty.clone())),
                    "Data" => {
                        let (data_offset, data_ty) =
                            self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                        Ok((raw_offset + data_offset, data_ty))
                    }
                    "ptr" | "Pointer" => {
                        let (data_offset, data_ty) =
                            self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                        let (ptr_offset, ptr_ty) =
                            self.field_info_by_name_inner(&data_ty, "Pointer", visited)?;
                        Ok((raw_offset + data_offset + ptr_offset, ptr_ty))
                    }
                    "len" | "Length" => {
                        let (len_offset, len_ty) =
                            self.field_info_by_name_inner(&raw_ty, "Length", visited)?;
                        Ok((raw_offset + len_offset, len_ty))
                    }
                    "elem_size" | "ElementSize" => {
                        let (elem_offset, elem_ty) =
                            self.field_info_by_name_inner(&raw_ty, "ElementSize", visited)?;
                        Ok((raw_offset + elem_offset, elem_ty))
                    }
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{}`",
                        struct_layout.name
                    ))),
                };
            } else if short_base == "Span" || base_name.ends_with("::Span") {
                let raw_ty = Ty::named("Std::Span::SpanPtr".to_string());
                let raw_offset = 0usize;
                return match name {
                    "Handle" | "Raw" => Ok((raw_offset, raw_ty.clone())),
                    "Data" => {
                        let (data_offset, data_ty) =
                            self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                        Ok((raw_offset + data_offset, data_ty))
                    }
                    "ptr" | "Pointer" => {
                        let (data_offset, data_ty) =
                            self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                        let (ptr_offset, ptr_ty) =
                            self.field_info_by_name_inner(&data_ty, "Pointer", visited)?;
                        Ok((raw_offset + data_offset + ptr_offset, ptr_ty))
                    }
                    "len" | "Length" => {
                        let (len_offset, len_ty) =
                            self.field_info_by_name_inner(&raw_ty, "Length", visited)?;
                        Ok((raw_offset + len_offset, len_ty))
                    }
                    "elem_size" | "ElementSize" => {
                        let (elem_offset, elem_ty) =
                            self.field_info_by_name_inner(&raw_ty, "ElementSize", visited)?;
                        Ok((raw_offset + elem_offset, elem_ty))
                    }
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{}`",
                        struct_layout.name
                    ))),
                };
            } else if short_base == "ReadOnlySpanPtr" || base_name.ends_with("::ReadOnlySpanPtr") {
                let data_ty = Ty::named("Std::Runtime::Collections::ValueConstPtr");
                let base_offset = 3usize.saturating_mul(pointer_size());
                return match name {
                    "Data" => Ok((0, data_ty)),
                    "ptr" | "Pointer" => {
                        let (ptr_offset, ptr_ty) =
                            self.field_info_by_name_inner(&data_ty, "Pointer", visited)?;
                        Ok((ptr_offset, ptr_ty))
                    }
                    "len" | "Length" => Ok((base_offset, Ty::named("usize"))),
                    "elem_size" | "ElementSize" => Ok((
                        base_offset.saturating_add(pointer_size()),
                        Ty::named("usize"),
                    )),
                    "ElementAlignment" => Ok((
                        base_offset.saturating_add(pointer_size() * 2),
                        Ty::named("usize"),
                    )),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{}`",
                        struct_layout.name
                    ))),
                };
            } else if short_base == "SpanPtr" || base_name.ends_with("::SpanPtr") {
                let data_ty = Ty::named("Std::Runtime::Collections::ValueMutPtr");
                let base_offset = 3usize.saturating_mul(pointer_size());
                return match name {
                    "Data" => Ok((0, data_ty)),
                    "ptr" | "Pointer" => {
                        let (ptr_offset, ptr_ty) =
                            self.field_info_by_name_inner(&data_ty, "Pointer", visited)?;
                        Ok((ptr_offset, ptr_ty))
                    }
                    "len" | "Length" => Ok((base_offset, Ty::named("usize"))),
                    "elem_size" | "ElementSize" => Ok((
                        base_offset.saturating_add(pointer_size()),
                        Ty::named("usize"),
                    )),
                    "ElementAlignment" => Ok((
                        base_offset.saturating_add(pointer_size() * 2),
                        Ty::named("usize"),
                    )),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{}`",
                        struct_layout.name
                    ))),
                };
            } else if short_base == "StrPtr" || base_name.ends_with("::StrPtr") {
                let mut qualifiers = crate::mir::PointerQualifiers::default();
                qualifiers.readonly = true;
                qualifiers.expose_address = true;
                let ptr_ty = Ty::Pointer(Box::new(crate::mir::PointerTy {
                    element: Ty::named("byte"),
                    mutable: false,
                    qualifiers,
                }));
                return match name {
                    "Pointer" | "ptr" => Ok((0, ptr_ty)),
                    "Length" | "len" => Ok((pointer_size(), Ty::named("usize"))),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{}`",
                        struct_layout.name
                    ))),
                };
            } else if short_base == "CharSpanPtr" || base_name.ends_with("::CharSpanPtr") {
                let mut qualifiers = crate::mir::PointerQualifiers::default();
                qualifiers.readonly = true;
                qualifiers.expose_address = true;
                let ptr_ty = Ty::Pointer(Box::new(crate::mir::PointerTy {
                    element: Ty::named("ushort"),
                    mutable: false,
                    qualifiers,
                }));
                return match name {
                    "Pointer" | "ptr" => Ok((0, ptr_ty)),
                    "Length" | "len" => Ok((pointer_size(), Ty::named("usize"))),
                    _ => Err(Error::Codegen(format!(
                        "field `{name}` missing on type `{}`",
                        struct_layout.name
                    ))),
                };
            } else if short_base == "ReadOnlySpan" || base_name.ends_with("::ReadOnlySpan") {
                if name == "Handle" || name == "Raw" {
                    return Ok((0, Ty::named("Std::Span::ReadOnlySpanPtr".to_string())));
                }
                if matches!(
                    name,
                    "Data"
                        | "ptr"
                        | "Pointer"
                        | "len"
                        | "Length"
                        | "elem_size"
                        | "ElementSize"
                        | "ElementAlignment"
                ) {
                    let raw_ty = Ty::named("Std::Span::ReadOnlySpanPtr".to_string());
                    let raw_offset = 0usize;
                    return match name {
                        "Data" => {
                            let (data_offset, data_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                            Ok((raw_offset + data_offset, data_ty))
                        }
                        "ptr" | "Pointer" => {
                            let (data_offset, data_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                            let (ptr_offset, ptr_ty) =
                                self.field_info_by_name_inner(&data_ty, "Pointer", visited)?;
                            Ok((raw_offset + data_offset + ptr_offset, ptr_ty))
                        }
                        "len" | "Length" => {
                            let (len_offset, len_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Length", visited)?;
                            Ok((raw_offset + len_offset, len_ty))
                        }
                        "elem_size" | "ElementSize" => {
                            let (elem_offset, elem_ty) =
                                self.field_info_by_name_inner(&raw_ty, "ElementSize", visited)?;
                            Ok((raw_offset + elem_offset, elem_ty))
                        }
                        "ElementAlignment" => {
                            let (elem_offset, elem_ty) = self.field_info_by_name_inner(
                                &raw_ty,
                                "ElementAlignment",
                                visited,
                            )?;
                            Ok((raw_offset + elem_offset, elem_ty))
                        }
                        _ => unreachable!(),
                    };
                }
                return Err(Error::Codegen(format!(
                    "field `{name}` missing on type `{}`",
                    struct_layout.name
                )));
            } else if short_base == "Span" || base_name.ends_with("::Span") {
                if name == "Handle" || name == "Raw" {
                    return Ok((0, Ty::named("Std::Span::SpanPtr".to_string())));
                }
                if matches!(
                    name,
                    "Data"
                        | "ptr"
                        | "Pointer"
                        | "len"
                        | "Length"
                        | "elem_size"
                        | "ElementSize"
                        | "ElementAlignment"
                ) {
                    let raw_ty = Ty::named("Std::Span::SpanPtr".to_string());
                    let raw_offset = 0usize;
                    return match name {
                        "Data" => {
                            let (data_offset, data_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                            Ok((raw_offset + data_offset, data_ty))
                        }
                        "ptr" | "Pointer" => {
                            let (data_offset, data_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Data", visited)?;
                            let (ptr_offset, ptr_ty) =
                                self.field_info_by_name_inner(&data_ty, "Pointer", visited)?;
                            Ok((raw_offset + data_offset + ptr_offset, ptr_ty))
                        }
                        "len" | "Length" => {
                            let (len_offset, len_ty) =
                                self.field_info_by_name_inner(&raw_ty, "Length", visited)?;
                            Ok((raw_offset + len_offset, len_ty))
                        }
                        "elem_size" | "ElementSize" => {
                            let (elem_offset, elem_ty) =
                                self.field_info_by_name_inner(&raw_ty, "ElementSize", visited)?;
                            Ok((raw_offset + elem_offset, elem_ty))
                        }
                        "ElementAlignment" => {
                            let (elem_offset, elem_ty) = self.field_info_by_name_inner(
                                &raw_ty,
                                "ElementAlignment",
                                visited,
                            )?;
                            Ok((raw_offset + elem_offset, elem_ty))
                        }
                        _ => unreachable!(),
                    };
                }
                if std::env::var("CHIC_DEBUG_LAYOUT").is_ok() {
                    let fields: Vec<String> = struct_layout
                        .fields
                        .iter()
                        .map(|f| format!("{}:{}", f.name, f.ty.canonical_name()))
                        .collect();
                    eprintln!(
                        "[chic-debug] missing field `{name}` on `{}` in `{}`; known fields: {:?}",
                        struct_layout.name, self.function.name, fields
                    );
                }
                if let Some(class_info) = self.type_layouts.class_layout_info(canonical_key) {
                    for base in class_info.bases {
                        let canonical = base.replace('.', "::");
                        if canonical == canonical_key || canonical == struct_layout.name {
                            continue;
                        }
                        if let Ok(resolved) =
                            self.field_info_by_name_named_inner(&canonical, name, visited)
                        {
                            return Ok(resolved);
                        }
                    }
                }
                return Err(Error::Codegen(format!(
                    "field `{name}` missing on type `{}`",
                    struct_layout.name
                )));
            } else {
                if let Some((offset, ty)) =
                    self.async_field_fallback(&Ty::named(canonical_key.to_string()), name)
                {
                    return Ok((offset, ty));
                }
                // Heuristic: if the field is missing but one of the child fields contains it, recurse.
                for field in &struct_layout.fields {
                    if let Some(parent_offset) = field.offset {
                        if let Ok((inner_offset, inner_ty)) =
                            self.field_info_by_name_inner(&field.ty, canonical_key, visited)
                        {
                            if std::env::var("CHIC_DEBUG_LAYOUT").is_ok() {
                                eprintln!(
                                    "[chic-debug] synthesizing field `{name}` via child `{}` on `{}` (parent_offset={}, inner_offset={})",
                                    field.name, struct_layout.name, parent_offset, inner_offset
                                );
                            }
                            return Ok((parent_offset + inner_offset, inner_ty));
                        }
                    }
                }
                if std::env::var("CHIC_DEBUG_LAYOUT").is_ok() {
                    let fields: Vec<String> = struct_layout
                        .fields
                        .iter()
                        .map(|f| format!("{}:{}", f.name, f.ty.canonical_name()))
                        .collect();
                    eprintln!(
                        "[chic-debug] missing field `{name}` on `{}` in `{}`; known fields: {:?}",
                        struct_layout.name, self.function.name, fields
                    );
                }
                if let Some(class_info) = self.type_layouts.class_layout_info(canonical_key) {
                    for base in class_info.bases {
                        let canonical = base.replace('.', "::");
                        if canonical == canonical_key || canonical == struct_layout.name {
                            continue;
                        }
                        if let Ok(resolved) =
                            self.field_info_by_name_named_inner(&canonical, name, visited)
                        {
                            return Ok(resolved);
                        }
                    }
                }
                return Err(Error::Codegen(format!(
                    "field `{name}` missing on type `{}`",
                    struct_layout.name
                )));
            };
            if let Some(offset) = field.offset {
                return Ok((offset, field.ty.clone()));
            }
            if let Some((offset, ty)) =
                self.async_field_fallback(&Ty::named(canonical_key.to_string()), field_name)
            {
                return Ok((offset, ty));
            }
            if let Some(class_info) = self.type_layouts.class_layout_info(canonical_key) {
                for base in class_info.bases {
                    let canonical = base.replace('.', "::");
                    if canonical == canonical_key || canonical == struct_layout.name {
                        continue;
                    }
                    if let Ok(resolved) =
                        self.field_info_by_name_named_inner(&canonical, name, visited)
                    {
                        return Ok(resolved);
                    }
                }
            }
            Err(Error::Codegen(format!(
                "field `{name}` missing on type `{}`",
                struct_layout.name
            )))
        })();
        visited.remove(&visit_key);
        result
    }

    pub(crate) fn resolve_self_type_name(&self) -> Option<String> {
        match self.function.kind {
            FunctionKind::Method | FunctionKind::Constructor => {
                let mut parts: Vec<&str> = self.function.name.split("::").collect();
                if parts.len() < 2 {
                    return None;
                }
                parts.pop();
                Some(parts.join("::"))
            }
            _ => None,
        }
    }
}
