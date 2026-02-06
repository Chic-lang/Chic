use std::fmt;

use crate::mir::{Abi, FnSig, ParamMode, Ty, TypeLayout, TypeLayoutTable};
use crate::target::{Target, TargetArch, TargetOs};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct CAbiSignature {
    pub ret: CAbiReturn,
    pub params: Vec<CAbiParam>,
    pub variadic: bool,
}

#[derive(Debug, Clone)]
pub enum CAbiReturn {
    Direct { ty: Ty, coerce: Option<String> },
    IndirectSret { ty: Ty, align: usize },
}

#[derive(Debug, Clone)]
pub struct CAbiParam {
    pub index: usize,
    pub ty: Ty,
    pub mode: ParamMode,
    pub pass: CAbiPass,
    pub coerce: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CAbiPass {
    Direct,
    IndirectByVal { align: usize },
    IndirectPtr { align: usize },
}

#[derive(Debug, Clone)]
pub struct CAbiError {
    message: String,
}

impl CAbiError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CAbiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CAbiError {}

pub fn classify_c_abi_signature(
    sig: &FnSig,
    param_modes: &[ParamMode],
    layouts: &TypeLayoutTable,
    target: &Target,
) -> Result<CAbiSignature, CAbiError> {
    match &sig.abi {
        Abi::Extern(name) if name.eq_ignore_ascii_case("c") => {}
        Abi::Extern(name) => {
            return Err(CAbiError::new(format!(
                "unsupported extern ABI `{name}` for C ABI classifier"
            )));
        }
        Abi::Chic => {
            return Err(CAbiError::new(
                "C ABI classifier invoked for Chic ABI signature",
            ));
        }
    }

    let mut params = Vec::with_capacity(sig.params.len());
    for (index, ty) in sig.params.iter().cloned().enumerate() {
        let mode = param_modes.get(index).copied().unwrap_or(ParamMode::Value);
        let (pass, coerce) = classify_param_pass(&ty, mode, layouts, target)?;
        params.push(CAbiParam {
            index,
            ty,
            mode,
            pass,
            coerce,
        });
    }

    let ret = classify_return_pass(&sig.ret, layouts, target)?;
    Ok(CAbiSignature {
        ret,
        params,
        variadic: sig.variadic,
    })
}

fn classify_param_pass(
    ty: &Ty,
    mode: ParamMode,
    layouts: &TypeLayoutTable,
    target: &Target,
) -> Result<(CAbiPass, Option<String>), CAbiError> {
    if mode != ParamMode::Value {
        return Ok((CAbiPass::Direct, None));
    }
    if is_c_abi_scalar(ty, layouts)? {
        return Ok((CAbiPass::Direct, None));
    }
    let Some((size, align)) = layouts.size_and_align_for_ty(ty) else {
        return Err(CAbiError::new(format!(
            "missing layout metadata for `{}` in C ABI classification",
            ty.canonical_name()
        )));
    };
    if is_aggregate_passed_indirect(target, ty, size, align, layouts)? {
        match target.arch() {
            TargetArch::Aarch64 => Ok((CAbiPass::IndirectPtr { align }, None)),
            TargetArch::X86_64 => Ok((
                CAbiPass::IndirectByVal {
                    align: sysv_x86_64_indirect_byval_align(target, align),
                },
                None,
            )),
        }
    } else {
        let coerce = aggregate_coerce_type(ty, size, align, layouts, target, false)?;
        Ok((CAbiPass::Direct, coerce))
    }
}

fn classify_return_pass(
    ty: &Ty,
    layouts: &TypeLayoutTable,
    target: &Target,
) -> Result<CAbiReturn, CAbiError> {
    if matches!(ty, Ty::Unit) {
        return Ok(CAbiReturn::Direct {
            ty: Ty::Unit,
            coerce: None,
        });
    }
    if is_c_abi_scalar(ty, layouts)? {
        return Ok(CAbiReturn::Direct {
            ty: ty.clone(),
            coerce: None,
        });
    }
    let Some((size, align)) = layouts.size_and_align_for_ty(ty) else {
        return Err(CAbiError::new(format!(
            "missing layout metadata for `{}` in C ABI classification",
            ty.canonical_name()
        )));
    };
    if is_aggregate_returned_indirect(target, ty, size, align, layouts)? {
        Ok(CAbiReturn::IndirectSret {
            ty: ty.clone(),
            align,
        })
    } else {
        let coerce = aggregate_coerce_type(ty, size, align, layouts, target, true)?;
        Ok(CAbiReturn::Direct {
            ty: ty.clone(),
            coerce,
        })
    }
}

fn aggregate_coerce_type(
    ty: &Ty,
    size: usize,
    _align: usize,
    layouts: &TypeLayoutTable,
    target: &Target,
    is_return: bool,
) -> Result<Option<String>, CAbiError> {
    if is_c_abi_scalar(ty, layouts)? {
        return Ok(None);
    }
    match target.arch() {
        TargetArch::Aarch64 => {
            if is_aarch64_hfa(ty, layouts).is_some() {
                return Ok(None);
            }
            if matches!(target.os(), TargetOs::Windows) && !matches!(size, 1 | 2 | 4 | 8) {
                return Ok(None);
            }
            if size <= 8 {
                return Ok(Some(format!("i{}", size * 8)));
            }
            if size <= 16 {
                let parts = (size + 7) / 8;
                return Ok(Some(format!("[{parts} x i64]")));
            }
        }
        TargetArch::X86_64 => {
            if matches!(target.os(), TargetOs::Windows) && !matches!(size, 1 | 2 | 4 | 8) {
                return Ok(None);
            }
            if size <= 8 {
                return Ok(Some(format!("i{}", size * 8)));
            }
            if size <= 16 {
                let second_bytes = size.saturating_sub(8);
                let second_bits = second_bytes.saturating_mul(8);
                let second = if second_bits == 64 {
                    "i64".to_string()
                } else {
                    format!("i{second_bits}")
                };
                return Ok(Some(format!("{{ i64, {second} }}")));
            }
        }
    }
    if is_return {
        Ok(None)
    } else {
        // Conservative fallback: if we decided the param is direct, use its natural mapping.
        let Some((_size, _align)) = layouts.size_and_align_for_ty(ty) else {
            return Err(CAbiError::new(format!(
                "missing layout metadata for `{}` in C ABI coercion",
                ty.canonical_name()
            )));
        };
        Ok(None)
    }
}

fn is_c_abi_scalar(ty: &Ty, layouts: &TypeLayoutTable) -> Result<bool, CAbiError> {
    match ty {
        Ty::Pointer(_) | Ty::Ref(_) | Ty::Rc(_) | Ty::Arc(_) => Ok(true),
        Ty::Fn(fn_ty) => Ok(matches!(fn_ty.abi, Abi::Extern(_))),
        Ty::Named(name) => {
            let short = name.name.rsplit("::").next().unwrap_or(name.name.as_str());
            let lower = short.to_ascii_lowercase();
            if matches!(
                lower.as_str(),
                "bool"
                    | "byte"
                    | "sbyte"
                    | "i8"
                    | "u8"
                    | "char"
                    | "short"
                    | "ushort"
                    | "i16"
                    | "u16"
                    | "int"
                    | "uint"
                    | "i32"
                    | "u32"
                    | "long"
                    | "ulong"
                    | "i64"
                    | "u64"
                    | "isize"
                    | "usize"
                    | "nint"
                    | "nuint"
                    | "float"
                    | "double"
                    | "f32"
                    | "f64"
            ) {
                return Ok(true);
            }
            let canonical = Ty::Named(name.clone()).canonical_name();
            match layouts.layout_for_name(&canonical) {
                Some(TypeLayout::Enum(_)) => Ok(true),
                Some(TypeLayout::Class(_)) => Ok(true),
                Some(TypeLayout::Struct(_)) | Some(TypeLayout::Union(_)) => Ok(false),
                None => Ok(false),
            }
        }
        Ty::Vector(_)
        | Ty::Array(_)
        | Ty::Vec(_)
        | Ty::Span(_)
        | Ty::ReadOnlySpan(_)
        | Ty::Tuple(_)
        | Ty::String
        | Ty::Str
        | Ty::TraitObject(_)
        | Ty::Nullable(_)
        | Ty::Unknown => Ok(false),
        Ty::Unit => Ok(true),
    }
}

fn is_aggregate_passed_indirect(
    target: &Target,
    ty: &Ty,
    size: usize,
    _align: usize,
    layouts: &TypeLayoutTable,
) -> Result<bool, CAbiError> {
    match (target.arch(), target.os()) {
        (
            TargetArch::X86_64,
            &TargetOs::Macos | &TargetOs::Linux | &TargetOs::None | &TargetOs::Other(_),
        ) => Ok(size > 16 || x86_64_sysv_aggregate_contains_unaligned_fields(ty, layouts)?),
        (
            TargetArch::Aarch64,
            &TargetOs::Macos | &TargetOs::Linux | &TargetOs::None | &TargetOs::Other(_),
        ) => {
            if is_aarch64_hfa(ty, layouts).is_some() {
                return Ok(false);
            }
            Ok(size > 16)
        }
        (TargetArch::X86_64, &TargetOs::Windows) | (TargetArch::Aarch64, &TargetOs::Windows) => {
            Ok(!matches!(size, 1 | 2 | 4 | 8))
        }
    }
}

fn is_aggregate_returned_indirect(
    target: &Target,
    ty: &Ty,
    size: usize,
    _align: usize,
    layouts: &TypeLayoutTable,
) -> Result<bool, CAbiError> {
    match (target.arch(), target.os()) {
        (
            TargetArch::X86_64,
            &TargetOs::Macos | &TargetOs::Linux | &TargetOs::None | &TargetOs::Other(_),
        ) => Ok(size > 16 || x86_64_sysv_aggregate_contains_unaligned_fields(ty, layouts)?),
        (
            TargetArch::Aarch64,
            &TargetOs::Macos | &TargetOs::Linux | &TargetOs::None | &TargetOs::Other(_),
        ) => {
            if is_aarch64_hfa(ty, layouts).is_some() {
                return Ok(false);
            }
            Ok(size > 16)
        }
        (TargetArch::X86_64, &TargetOs::Windows) | (TargetArch::Aarch64, &TargetOs::Windows) => {
            Ok(!matches!(size, 1 | 2 | 4 | 8))
        }
    }
}

fn sysv_x86_64_indirect_byval_align(target: &Target, align: usize) -> usize {
    match (target.arch(), target.os()) {
        (
            TargetArch::X86_64,
            &TargetOs::Macos | &TargetOs::Linux | &TargetOs::None | &TargetOs::Other(_),
        ) => align.max(8),
        _ => align,
    }
}

fn x86_64_sysv_aggregate_contains_unaligned_fields(
    ty: &Ty,
    layouts: &TypeLayoutTable,
) -> Result<bool, CAbiError> {
    let mut visited = HashSet::new();
    x86_64_sysv_aggregate_contains_unaligned_fields_inner(ty, layouts, &mut visited)
}

fn x86_64_sysv_aggregate_contains_unaligned_fields_inner(
    ty: &Ty,
    layouts: &TypeLayoutTable,
    visited: &mut HashSet<String>,
) -> Result<bool, CAbiError> {
    if is_c_abi_scalar(ty, layouts)? {
        return Ok(false);
    }

    match ty {
        Ty::Named(name) => {
            let canonical = Ty::Named(name.clone()).canonical_name();
            if !visited.insert(canonical.clone()) {
                return Ok(false);
            }

            let Some(layout) = layouts.layout_for_name(&canonical) else {
                return Ok(false);
            };

            match layout {
                TypeLayout::Struct(layout) | TypeLayout::Class(layout) => {
                    for field in &layout.fields {
                        let Some(offset) = field.offset else {
                            continue;
                        };
                        let Some((_size, align)) = layouts.size_and_align_for_ty(&field.ty) else {
                            continue;
                        };
                        if align != 0 && offset % align != 0 {
                            return Ok(true);
                        }
                        if x86_64_sysv_aggregate_contains_unaligned_fields_inner(
                            &field.ty, layouts, visited,
                        )? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                TypeLayout::Enum(_) => Ok(false),
                TypeLayout::Union(layout) => {
                    for view in &layout.views {
                        if x86_64_sysv_aggregate_contains_unaligned_fields_inner(
                            &view.ty, layouts, visited,
                        )? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
            }
        }
        Ty::Array(array) => x86_64_sysv_aggregate_contains_unaligned_fields_inner(
            array.element.as_ref(),
            layouts,
            visited,
        ),
        Ty::Vec(vec) => x86_64_sysv_aggregate_contains_unaligned_fields_inner(
            vec.element.as_ref(),
            layouts,
            visited,
        ),
        Ty::Span(span) => x86_64_sysv_aggregate_contains_unaligned_fields_inner(
            span.element.as_ref(),
            layouts,
            visited,
        ),
        Ty::ReadOnlySpan(span) => x86_64_sysv_aggregate_contains_unaligned_fields_inner(
            span.element.as_ref(),
            layouts,
            visited,
        ),
        Ty::Tuple(tuple) => {
            for elem in &tuple.elements {
                if x86_64_sysv_aggregate_contains_unaligned_fields_inner(elem, layouts, visited)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Ty::Vector(_)
        | Ty::String
        | Ty::Str
        | Ty::TraitObject(_)
        | Ty::Nullable(_)
        | Ty::Unknown => Ok(false),
        Ty::Unit => Ok(false),
        Ty::Pointer(_) | Ty::Ref(_) | Ty::Rc(_) | Ty::Arc(_) | Ty::Fn(_) => Ok(false),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HfaElem {
    F32,
    F64,
}

fn is_aarch64_hfa(ty: &Ty, layouts: &TypeLayoutTable) -> Option<(HfaElem, usize)> {
    let mut acc = None;
    let mut count = 0usize;
    if !accumulate_hfa_fields(ty, layouts, &mut acc, &mut count) {
        return None;
    }
    if matches!(count, 1..=4) {
        acc.map(|elem| (elem, count))
    } else {
        None
    }
}

fn accumulate_hfa_fields(
    ty: &Ty,
    layouts: &TypeLayoutTable,
    elem: &mut Option<HfaElem>,
    count: &mut usize,
) -> bool {
    if let Some(field_elem) = hfa_scalar_elem(ty) {
        if elem.is_none() {
            *elem = Some(field_elem);
        } else if elem != &Some(field_elem) {
            return false;
        }
        *count += 1;
        return true;
    }

    let name = ty.canonical_name();
    let Some(layout) = layouts.layout_for_name(&name) else {
        return false;
    };
    match layout {
        TypeLayout::Struct(layout) => layout
            .fields
            .iter()
            .all(|field| accumulate_hfa_fields(&field.ty, layouts, elem, count)),
        _ => false,
    }
}

fn hfa_scalar_elem(ty: &Ty) -> Option<HfaElem> {
    match ty {
        Ty::Named(named) => {
            let short = named
                .name
                .rsplit("::")
                .next()
                .unwrap_or(named.name.as_str());
            match short.to_ascii_lowercase().as_str() {
                "float" | "f32" => Some(HfaElem::F32),
                "double" | "f64" => Some(HfaElem::F64),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        AutoTraitOverride, AutoTraitSet, AutoTraitStatus, FieldLayout, PositionalElement,
        StructLayout, TypeRepr,
    };

    fn insert_struct(
        layouts: &mut TypeLayoutTable,
        name: &str,
        fields: Vec<(&str, Ty, usize)>,
        size: usize,
        align: usize,
    ) {
        let mut field_layouts = Vec::new();
        let mut positional = Vec::new();
        for (index, (field_name, ty, offset)) in fields.into_iter().enumerate() {
            let idx = u32::try_from(index).unwrap_or(u32::MAX);
            field_layouts.push(FieldLayout {
                name: field_name.to_string(),
                ty: ty.clone(),
                index: idx,
                offset: Some(offset),
                span: None,
                mmio: None,
                display_name: Some(field_name.to_string()),
                is_required: false,
                is_nullable: false,
                is_readonly: false,
                view_of: None,
            });
            positional.push(PositionalElement {
                field_index: idx,
                name: Some(field_name.to_string()),
                span: None,
            });
        }
        layouts.types.insert(
            name.to_string(),
            TypeLayout::Struct(StructLayout {
                name: name.to_string(),
                repr: TypeRepr::Default,
                packing: None,
                fields: field_layouts,
                positional,
                list: None,
                size: Some(size),
                align: Some(align),
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::new(
                    AutoTraitStatus::Unknown,
                    AutoTraitStatus::Unknown,
                    AutoTraitStatus::Unknown,
                ),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: None,
                class: None,
            }),
        );
    }

    #[test]
    fn aarch64_hfa_allows_4_doubles_by_value() {
        let mut layouts = TypeLayoutTable::default();
        insert_struct(
            &mut layouts,
            "Test::Hfa4d",
            vec![
                ("a", Ty::named("double"), 0),
                ("b", Ty::named("double"), 8),
                ("c", Ty::named("double"), 16),
                ("d", Ty::named("double"), 24),
            ],
            32,
            8,
        );
        let sig = FnSig {
            params: vec![],
            ret: Ty::named("Test::Hfa4d"),
            abi: Abi::Extern("C".into()),
            effects: Vec::new(),
            lends_to_return: None,
            variadic: false,
        };
        let target = Target::parse("aarch64-unknown-linux-gnu").expect("aarch64 target");
        let lowered = classify_c_abi_signature(&sig, &[], &layouts, &target).expect("ok");
        assert!(matches!(lowered.ret, CAbiReturn::Direct { .. }));
    }

    #[test]
    fn sysv_x86_64_indirects_large_aggregate_return() {
        let mut layouts = TypeLayoutTable::default();
        insert_struct(
            &mut layouts,
            "Test::Big",
            vec![
                ("a", Ty::named("i64"), 0),
                ("b", Ty::named("i64"), 8),
                ("c", Ty::named("i64"), 16),
            ],
            24,
            8,
        );
        let sig = FnSig {
            params: vec![],
            ret: Ty::named("Test::Big"),
            abi: Abi::Extern("C".into()),
            effects: Vec::new(),
            lends_to_return: None,
            variadic: false,
        };
        let target = Target::parse("x86_64-unknown-linux-gnu").expect("x86_64 target");
        let lowered = classify_c_abi_signature(&sig, &[], &layouts, &target).expect("ok");
        assert!(matches!(lowered.ret, CAbiReturn::IndirectSret { .. }));
    }

    #[test]
    fn sysv_x86_64_indirects_unaligned_packed_aggregate_return_and_param() {
        let mut layouts = TypeLayoutTable::default();
        // Mimic a packed(1) struct with an unaligned u16 field at offset 1.
        insert_struct(
            &mut layouts,
            "Test::PackedS3",
            vec![("a", Ty::named("byte"), 0), ("b", Ty::named("ushort"), 1)],
            3,
            1,
        );

        let ret_sig = FnSig {
            params: vec![],
            ret: Ty::named("Test::PackedS3"),
            abi: Abi::Extern("C".into()),
            effects: Vec::new(),
            lends_to_return: None,
            variadic: false,
        };
        let param_sig = FnSig {
            params: vec![Ty::named("Test::PackedS3")],
            ret: Ty::named("int"),
            abi: Abi::Extern("C".into()),
            effects: Vec::new(),
            lends_to_return: None,
            variadic: false,
        };

        let target = Target::parse("x86_64-unknown-linux-gnu").expect("x86_64 target");
        let lowered_ret = classify_c_abi_signature(&ret_sig, &[], &layouts, &target).expect("ok");
        assert!(
            matches!(lowered_ret.ret, CAbiReturn::IndirectSret { .. }),
            "unaligned packed aggregates must use sret on SysV x86_64"
        );

        let lowered_param =
            classify_c_abi_signature(&param_sig, &[ParamMode::Value], &layouts, &target)
                .expect("ok");
        assert!(
            matches!(lowered_param.params[0].pass, CAbiPass::IndirectByVal { .. }),
            "unaligned packed aggregates must be passed indirectly on SysV x86_64"
        );
        if let CAbiPass::IndirectByVal { align } = lowered_param.params[0].pass {
            assert!(
                align >= 8,
                "SysV x86_64 byval alignment should be at least 8 (got {align})"
            );
        }
    }

    #[test]
    fn windows_indirects_non_scalar_aggregate_param() {
        let mut layouts = TypeLayoutTable::default();
        insert_struct(
            &mut layouts,
            "Test::Pair",
            vec![("a", Ty::named("i32"), 0), ("b", Ty::named("i32"), 4)],
            8,
            4,
        );
        insert_struct(
            &mut layouts,
            "Test::Trio",
            vec![
                ("a", Ty::named("i32"), 0),
                ("b", Ty::named("i32"), 4),
                ("c", Ty::named("i32"), 8),
            ],
            12,
            4,
        );
        let sig = FnSig {
            params: vec![Ty::named("Test::Pair"), Ty::named("Test::Trio")],
            ret: Ty::named("int"),
            abi: Abi::Extern("C".into()),
            effects: Vec::new(),
            lends_to_return: None,
            variadic: false,
        };
        let target = Target::parse("x86_64-pc-windows-msvc").expect("windows target");
        let lowered = classify_c_abi_signature(
            &sig,
            &[ParamMode::Value, ParamMode::Value],
            &layouts,
            &target,
        )
        .expect("ok");
        assert!(matches!(lowered.params[0].pass, CAbiPass::Direct));
        assert!(matches!(
            lowered.params[1].pass,
            CAbiPass::IndirectByVal { .. }
        ));
    }

    #[test]
    fn classify_module_functions_collects_extern_c_entries() {
        use crate::frontend::attributes::OptimizationHints;
        use crate::mir::{FunctionKind, MirFunction, MirModule, new_mir_body};

        let mut layouts = TypeLayoutTable::default();
        insert_struct(
            &mut layouts,
            "Demo::Big",
            vec![
                ("a", Ty::named("i64"), 0),
                ("b", Ty::named("i64"), 8),
                ("c", Ty::named("i64"), 16),
            ],
            24,
            8,
        );

        let sig = FnSig {
            params: vec![Ty::named("int")],
            ret: Ty::named("Demo::Big"),
            abi: Abi::Extern("C".into()),
            effects: Vec::new(),
            lends_to_return: None,
            variadic: false,
        };
        let body = new_mir_body(sig.params.len(), None);
        let extern_fn = MirFunction {
            name: "Demo::make_big".into(),
            kind: FunctionKind::Function,
            signature: sig,
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        };

        let mut module = MirModule::default();
        module.type_layouts = layouts;
        module.functions.push(extern_fn);
        // Ensure a Chic-only function is ignored.
        module.functions.push(MirFunction {
            name: "Demo::helper".into(),
            kind: FunctionKind::Function,
            signature: FnSig {
                params: vec![],
                ret: Ty::Unit,
                abi: Abi::Chic,
                effects: Vec::new(),
                lends_to_return: None,
                variadic: false,
            },
            body: new_mir_body(0, None),
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        });

        let target = Target::parse("x86_64-unknown-linux-gnu").expect("target");
        let classified = crate::abi::classify_module_functions(&module, &target)
            .expect("classification should succeed");
        assert_eq!(
            classified.len(),
            1,
            "only extern C functions should be classified"
        );
        let sig = classified
            .get("Demo::make_big")
            .expect("extern function present");
        assert!(
            matches!(sig.ret, CAbiReturn::IndirectSret { .. }),
            "Big aggregate should use sret on SysV x86_64"
        );
        assert_eq!(sig.params.len(), 1);
        assert!(sig.variadic == false);
    }
}
