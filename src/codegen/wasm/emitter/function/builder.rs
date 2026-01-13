use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::Instant;

use crate::error::Error;
use crate::mir::{
    ArrayTy, BlockId, ClassVTable, EnumLayout, EnumVariantLayout, FieldLayout, FnTy, GenericArg,
    LocalKind, MirFunction, Operand, ParamMode, PointerTy, RefTy, StaticId, StaticVar, StrId,
    StructLayout, Terminator, TraitVTable, TupleTy, Ty, TypeLayout, TypeLayoutTable,
    UnionFieldLayout, UnionLayout,
};
use crate::perf::Tracepoint;
use crate::runtime::wasm_executor::hooks::PANIC_EXIT_CODE;

use crate::codegen::wasm::module_builder::{FunctionSignature, WasmStrLiteral};
use crate::codegen::wasm::{
    AggregateAllocation, RuntimeHook, STACK_POINTER_GLOBAL_INDEX, ValueType, ensure_u32,
    local_requires_memory, lookup_layout, map_type, write_u32,
};

fn collect_address_taken_by_call_modes(function: &MirFunction) -> HashSet<usize> {
    let mut address_taken = HashSet::new();
    for block in &function.body.blocks {
        let Some(terminator) = block.terminator.as_ref() else {
            continue;
        };
        let Terminator::Call {
            args, arg_modes, ..
        } = terminator
        else {
            continue;
        };
        for (arg, mode) in args.iter().zip(arg_modes.iter()) {
            if matches!(mode, ParamMode::Value) {
                continue;
            }
            match arg {
                Operand::Copy(place) | Operand::Move(place) => {
                    address_taken.insert(place.local.0);
                }
                Operand::Borrow(borrow) => {
                    address_taken.insert(borrow.place.local.0);
                }
                _ => {}
            }
        }
    }
    address_taken
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn emit_function<'a>(
    function: &MirFunction,
    functions: &HashMap<String, u32>,
    layouts: &TypeLayoutTable,
    string_literals: Option<&HashMap<StrId, WasmStrLiteral>>,
    trait_vtable_offsets: Option<&HashMap<String, u32>>,
    class_vtable_offsets: Option<&HashMap<String, u32>>,
    signature_indices: Option<&HashMap<FunctionSignature, u32>>,
    function_signatures: Option<&HashMap<String, FunctionSignature>>,
    trait_vtables: &'a [TraitVTable],
    class_vtables: &'a [ClassVTable],
    statics: Option<&'a [StaticVar]>,
    static_offsets: Option<&'a HashMap<StaticId, u32>>,
    tracepoint: Option<&'a Tracepoint>,
) -> Result<Vec<u8>, Error> {
    emit_function_with_async(
        function,
        functions,
        empty_fn_return_ty_map(),
        None,
        layouts,
        string_literals,
        trait_vtable_offsets,
        class_vtable_offsets,
        None,
        signature_indices,
        function_signatures,
        trait_vtables,
        class_vtables,
        statics,
        static_offsets,
        tracepoint,
        false,
        0,
    )
}

pub(crate) fn emit_function_with_async<'a>(
    function: &MirFunction,
    functions: &HashMap<String, u32>,
    function_return_tys: &HashMap<String, Ty>,
    function_param_tys: Option<&HashMap<String, Vec<Ty>>>,
    layouts: &TypeLayoutTable,
    string_literals: Option<&HashMap<StrId, WasmStrLiteral>>,
    trait_vtable_offsets: Option<&HashMap<String, u32>>,
    class_vtable_offsets: Option<&HashMap<String, u32>>,
    async_vtable_offsets: Option<&HashMap<String, u32>>,
    signature_indices: Option<&HashMap<FunctionSignature, u32>>,
    function_signatures: Option<&HashMap<String, FunctionSignature>>,
    trait_vtables: &'a [TraitVTable],
    class_vtables: &'a [ClassVTable],
    statics: Option<&'a [StaticVar]>,
    static_offsets: Option<&'a HashMap<StaticId, u32>>,
    tracepoint: Option<&'a Tracepoint>,
    coverage_enabled: bool,
    coverage_function_index: u32,
) -> Result<Vec<u8>, Error> {
    let mut emitter = FunctionEmitter::new_with_async(
        function,
        functions,
        function_return_tys,
        function_param_tys,
        layouts,
        string_literals,
        trait_vtable_offsets,
        class_vtable_offsets,
        async_vtable_offsets,
        signature_indices,
        function_signatures,
        trait_vtables,
        class_vtables,
        statics,
        static_offsets,
        tracepoint,
        coverage_enabled,
        coverage_function_index,
    )?;
    emitter.emit_body()
}

fn empty_string_literal_map() -> &'static HashMap<StrId, WasmStrLiteral> {
    static EMPTY: OnceLock<HashMap<StrId, WasmStrLiteral>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

fn empty_signature_map() -> &'static HashMap<FunctionSignature, u32> {
    static EMPTY: OnceLock<HashMap<FunctionSignature, u32>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

fn empty_fn_signature_cache() -> &'static HashMap<String, FunctionSignature> {
    static EMPTY: OnceLock<HashMap<String, FunctionSignature>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

fn empty_fn_param_ty_map() -> &'static HashMap<String, Vec<Ty>> {
    static EMPTY: OnceLock<HashMap<String, Vec<Ty>>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

#[cfg(test)]
fn empty_fn_return_ty_map() -> &'static HashMap<String, Ty> {
    static EMPTY: OnceLock<HashMap<String, Ty>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

fn empty_trait_vtable_map() -> &'static HashMap<String, u32> {
    static EMPTY: OnceLock<HashMap<String, u32>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

fn empty_class_vtable_map() -> &'static HashMap<String, u32> {
    static EMPTY: OnceLock<HashMap<String, u32>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

fn empty_static_slice<'a>() -> &'a [StaticVar] {
    &[]
}

fn empty_static_offset_map() -> &'static HashMap<StaticId, u32> {
    static EMPTY: OnceLock<HashMap<StaticId, u32>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

fn empty_async_vtable_map() -> &'static HashMap<String, u32> {
    static EMPTY: OnceLock<HashMap<String, u32>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

use super::borrows::{self, BorrowMeta};
use super::locals::{LocalPlan, LocalRepresentation, plan_locals};
use super::ops::{Op, emit_instruction};

pub(crate) struct FunctionEmitter<'a> {
    pub(super) function: &'a MirFunction,
    pub(super) functions: &'a HashMap<String, u32>,
    pub(super) function_return_tys: &'a HashMap<String, Ty>,
    pub(super) function_param_tys: &'a HashMap<String, Vec<Ty>>,
    pub(super) signature_indices: &'a HashMap<FunctionSignature, u32>,
    pub(super) layouts: &'a TypeLayoutTable,
    pub(super) string_literals: &'a HashMap<StrId, WasmStrLiteral>,
    pub(super) trait_vtable_offsets: &'a HashMap<String, u32>,
    pub(super) class_vtable_offsets: &'a HashMap<String, u32>,
    #[allow(dead_code)]
    pub(super) async_vtable_offsets: &'a HashMap<String, u32>,
    pub(super) function_signatures: &'a HashMap<String, FunctionSignature>,
    pub(super) trait_vtables: &'a [TraitVTable],
    pub(super) class_vtables: &'a [ClassVTable],
    pub(super) statics: &'a [StaticVar],
    pub(super) static_offsets: &'a HashMap<StaticId, u32>,
    pub(crate) locals: Vec<Option<u32>>,
    pub(crate) local_types: Vec<ValueType>,
    pub(crate) local_tys: Vec<Ty>,
    pub(crate) local_value_types: Vec<Option<ValueType>>,
    pub(crate) representations: Vec<LocalRepresentation>,
    pub(crate) aggregate_allocations: Vec<Option<AggregateAllocation>>,
    pub(super) block_map: HashMap<BlockId, usize>,
    pub(super) return_local: Option<u32>,
    pub(super) block_local: u32,
    pub(super) temp_local: u32,
    pub(super) float_temp_local: u32,
    pub(super) double_temp_local: u32,
    pub(super) wide_temp_local: u32,
    pub(super) wide_temp_local_hi: u32,
    pub(crate) stack_temp_local: u32,
    pub(crate) stack_adjust_local: u32,
    pub(crate) scratch_local: u32,
    pub(crate) frame_local: Option<u32>,
    pub(crate) frame_size: u32,
    pub(super) borrow_destinations: HashMap<usize, BorrowMeta>,
    pub(super) borrow_regions: HashMap<usize, BorrowMeta>,
    pub(super) initialised_borrow_locals: HashSet<usize>,
    pub(super) tracepoint: Option<&'a Tracepoint>,
    pub(crate) param_shift: u32,
    pub(super) coverage_enabled: bool,
    pub(super) coverage_function_index: u32,
    pub(super) coverage_statement_index: u32,
    pub(super) coverage_statement_indices: HashMap<usize, u32>,
}
impl<'a> FunctionEmitter<'a> {
    #[cfg(test)]
    pub(crate) fn new(
        function: &'a MirFunction,
        functions: &'a HashMap<String, u32>,
        function_return_tys: &'a HashMap<String, Ty>,
        function_param_tys: Option<&'a HashMap<String, Vec<Ty>>>,
        layouts: &'a TypeLayoutTable,
        string_literals: Option<&'a HashMap<StrId, WasmStrLiteral>>,
        trait_vtable_offsets: Option<&'a HashMap<String, u32>>,
        class_vtable_offsets: Option<&'a HashMap<String, u32>>,
        signature_indices: Option<&'a HashMap<FunctionSignature, u32>>,
        function_signatures: Option<&'a HashMap<String, FunctionSignature>>,
        trait_vtables: &'a [TraitVTable],
        class_vtables: &'a [ClassVTable],
        statics: Option<&'a [StaticVar]>,
        static_offsets: Option<&'a HashMap<StaticId, u32>>,
        tracepoint: Option<&'a Tracepoint>,
    ) -> Result<Self, Error> {
        Self::new_with_async(
            function,
            functions,
            function_return_tys,
            function_param_tys,
            layouts,
            string_literals,
            trait_vtable_offsets,
            class_vtable_offsets,
            None,
            signature_indices,
            function_signatures,
            trait_vtables,
            class_vtables,
            statics,
            static_offsets,
            tracepoint,
            false,
            0,
        )
    }

    pub(crate) fn new_with_async(
        function: &'a MirFunction,
        functions: &'a HashMap<String, u32>,
        function_return_tys: &'a HashMap<String, Ty>,
        function_param_tys: Option<&'a HashMap<String, Vec<Ty>>>,
        layouts: &'a TypeLayoutTable,
        string_literals: Option<&'a HashMap<StrId, WasmStrLiteral>>,
        trait_vtable_offsets: Option<&'a HashMap<String, u32>>,
        class_vtable_offsets: Option<&'a HashMap<String, u32>>,
        async_vtable_offsets: Option<&'a HashMap<String, u32>>,
        signature_indices: Option<&'a HashMap<FunctionSignature, u32>>,
        function_signatures: Option<&'a HashMap<String, FunctionSignature>>,
        trait_vtables: &'a [TraitVTable],
        class_vtables: &'a [ClassVTable],
        statics: Option<&'a [StaticVar]>,
        static_offsets: Option<&'a HashMap<StaticId, u32>>,
        tracepoint: Option<&'a Tracepoint>,
        coverage_enabled: bool,
        coverage_function_index: u32,
    ) -> Result<Self, Error> {
        wasm_debug!(
            "    FunctionEmitter::new `{}` ({} locals, {} args)",
            function.name,
            function.body.locals.len(),
            function.body.arg_count
        );
        if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
            let locals: Vec<String> = function
                .body
                .locals
                .iter()
                .enumerate()
                .map(|(idx, decl)| {
                    format!(
                        "{}:{}:{}",
                        idx,
                        decl.name.clone().unwrap_or_else(|| "_".into()),
                        decl.ty.canonical_name()
                    )
                })
                .collect();
            eprintln!(
                "[wasm-async] building {} locals={:?}",
                function.name, locals
            );
        }
        let (mut address_taken, borrow_destinations, borrow_regions) =
            borrows::collect_borrow_metadata(function, layouts);
        address_taken.extend(collect_address_taken_by_call_modes(function));
        let has_sret = !matches!(function.signature.ret, Ty::Unit)
            && local_requires_memory(&function.signature.ret, layouts);
        let mut arg_count = function.body.arg_count;
        for decl in &function.body.locals {
            if let LocalKind::Arg(index) = decl.kind {
                arg_count = arg_count.max(index + 1);
            }
        }
        let wasm_arg_count = arg_count
            .checked_add(if has_sret { 1 } else { 0 })
            .ok_or_else(|| {
                Error::Codegen("function argument count exceeds WebAssembly limits".into())
            })?;
        let LocalPlan {
            locals,
            local_types,
            local_tys,
            value_types,
            representations,
            aggregate_allocations,
            return_local,
            block_local,
            temp_local,
            float_temp_local,
            double_temp_local,
            wide_temp_local,
            wide_temp_local_hi,
            stack_temp_local,
            stack_adjust_local,
            scratch_local,
            frame_local,
            frame_size,
        } = plan_locals(function, layouts, &address_taken, wasm_arg_count, has_sret)?;
        if std::env::var_os("CHIC_DEBUG_WASM_LOCALS").is_some()
            && (function.name.contains("MemoryTestHelpers::Alloc")
                || function.name.contains("MemoryTestHelpers::SpanFrom")
                || function
                    .name
                    .contains("Given_memory_copy_copies_middle_byte_When_executed_Then_memory_copy_copies_middle_byte"))
        {
            eprintln!(
                "[wasm-locals] func={} return_local={:?} frame_size={}",
                function.name, return_local, frame_size
            );
            for (idx, decl) in function.body.locals.iter().enumerate() {
                let repr = representations
                    .get(idx)
                    .copied()
                    .unwrap_or(LocalRepresentation::Scalar);
                let ty = local_tys
                    .get(idx)
                    .map(|ty| ty.canonical_name())
                    .unwrap_or_else(|| "<unknown>".into());
                let slot = locals.get(idx).copied().flatten();
                let alloc = aggregate_allocations.get(idx).copied().flatten();
                eprintln!(
                    "  local {idx} name={} kind={:?} ty={ty} repr={repr:?} slot={slot:?} alloc={alloc:?}",
                    decl.name.clone().unwrap_or_else(|| "_".into()),
                    decl.kind
                );
            }
        }
        if std::env::var_os("CHIC_DEBUG_WASM_ASYNC").is_some() && function.is_async {
            eprintln!(
                "[wasm-async] locals for {}: frame_size={}",
                function.name, frame_size
            );
            for (idx, decl) in function.body.locals.iter().enumerate() {
                let repr = representations
                    .get(idx)
                    .copied()
                    .unwrap_or(LocalRepresentation::Scalar);
                let ty = local_tys
                    .get(idx)
                    .map(|ty| ty.canonical_name())
                    .unwrap_or_else(|| "<unknown>".into());
                let slot = locals.get(idx).copied().flatten();
                let alloc = aggregate_allocations.get(idx).copied().flatten();
                eprintln!(
                    "  local {idx} name={} kind={:?} ty={ty} repr={repr:?} slot={slot:?} alloc={alloc:?}",
                    decl.name.clone().unwrap_or_else(|| "_".into()),
                    decl.kind
                );
            }
        }
        if std::env::var_os("CHIC_DEBUG_WASM_ENUM").is_some()
            && (function.name.contains("EnvironmentVariables::Enumerate")
                || function.name.contains("ProcessInfo::CommandLine")
                || function.name.contains("IoState::BorrowStdout"))
        {
            eprintln!(
                "[wasm-enum-debug] locals for {}: {:?}",
                function.name,
                function
                    .body
                    .locals
                    .iter()
                    .enumerate()
                    .map(|(idx, decl)| format!(
                        "{}:{} kind={:?} ty={} repr={:?}",
                        idx,
                        decl.name.clone().unwrap_or_else(|| "_".into()),
                        decl.kind,
                        local_tys
                            .get(idx)
                            .map(|ty| ty.canonical_name())
                            .unwrap_or_else(|| "<unknown>".into()),
                        representations.get(idx)
                    ))
                    .collect::<Vec<_>>()
            );
            for (block_idx, block) in function.body.blocks.iter().enumerate() {
                eprintln!("[wasm-enum-debug] block {block_idx}: {:?}", block);
            }
        }
        let block_map = Self::build_block_map(function);

        let string_literals = match string_literals {
            Some(map) => map,
            None => empty_string_literal_map(),
        };
        let signature_indices = match signature_indices {
            Some(map) => map,
            None => empty_signature_map(),
        };
        let trait_vtable_offsets = match trait_vtable_offsets {
            Some(map) => map,
            None => empty_trait_vtable_map(),
        };
        if std::env::var_os("CHIC_DEBUG_WASM_TRAIT").is_some() {
            eprintln!(
                "[wasm-trait] building {} with {} trait vtables",
                function.name,
                trait_vtables.len()
            );
        }
        let class_vtable_offsets = match class_vtable_offsets {
            Some(map) => map,
            None => empty_class_vtable_map(),
        };
        let async_vtable_offsets = match async_vtable_offsets {
            Some(map) => map,
            None => empty_async_vtable_map(),
        };
        let function_signatures = match function_signatures {
            Some(map) => map,
            None => empty_fn_signature_cache(),
        };
        let function_param_tys = match function_param_tys {
            Some(map) => map,
            None => empty_fn_param_ty_map(),
        };
        let statics = statics.unwrap_or_else(empty_static_slice);
        let static_offsets = match static_offsets {
            Some(map) => map,
            None => empty_static_offset_map(),
        };

        if function.name.contains("AsyncTestCancellation")
            || function.name.contains("RuntimeCancel")
        {
            let locals_debug: Vec<_> = function
                .body
                .locals
                .iter()
                .enumerate()
                .map(|(idx, decl)| {
                    let name = decl.name.clone().unwrap_or_else(|| format!("_{}", idx));
                    let ty = decl.ty.canonical_name();
                    let slot = locals.get(idx).copied().flatten();
                    let repr = representations
                        .get(idx)
                        .copied()
                        .unwrap_or(LocalRepresentation::Scalar);
                    (idx, name, ty, slot, repr)
                })
                .collect();
            eprintln!(
                "[wasm-async] locals for {}: return_local={:?} frame_size={}",
                function.name, return_local, frame_size
            );
            for (idx, name, ty, slot, repr) in locals_debug {
                eprintln!("  local #{idx} {name}: {ty} slot={slot:?} repr={:?}", repr);
            }
            if let Some(machine) = function.body.async_machine.as_ref() {
                eprintln!(
                    "[wasm-async] async machine for {}: result_ty={:?} result_local={:?}",
                    function.name, machine.result_ty, machine.result_local
                );
            } else {
                eprintln!(
                    "[wasm-async] async machine missing for {} (is_async={})",
                    function.name, function.is_async
                );
            }
        }

        let mut coverage_statement_indices = HashMap::new();
        if coverage_enabled {
            let mut statement_index: u32 = 0;
            for block in &function.body.blocks {
                for statement in &block.statements {
                    coverage_statement_indices
                        .insert(statement as *const _ as usize, statement_index);
                    statement_index = statement_index.wrapping_add(1);
                }
            }
        }

        Ok(Self {
            function,
            functions,
            function_return_tys,
            function_param_tys,
            signature_indices,
            layouts,
            string_literals,
            trait_vtable_offsets,
            class_vtable_offsets,
            async_vtable_offsets,
            function_signatures,
            trait_vtables,
            class_vtables,
            statics,
            static_offsets,
            locals,
            local_types,
            local_tys,
            local_value_types: value_types,
            representations,
            aggregate_allocations,
            block_map,
            return_local,
            block_local,
            temp_local,
            float_temp_local,
            double_temp_local,
            wide_temp_local,
            wide_temp_local_hi,
            stack_temp_local,
            stack_adjust_local,
            scratch_local,
            frame_local,
            frame_size,
            borrow_destinations,
            borrow_regions,
            initialised_borrow_locals: HashSet::new(),
            tracepoint,
            param_shift: if has_sret { 1 } else { 0 },
            coverage_enabled,
            coverage_function_index,
            coverage_statement_index: 0,
            coverage_statement_indices,
        })
    }

    fn build_block_map(function: &'a MirFunction) -> HashMap<BlockId, usize> {
        let mut block_map = HashMap::new();
        for (idx, block) in function.body.blocks.iter().enumerate() {
            block_map.insert(block.id, idx);
        }
        block_map
    }

    pub(super) fn lookup_struct_layout(&self, ty: &Ty) -> Option<&StructLayout> {
        let layout = lookup_layout(self.layouts, ty)?;
        match layout {
            TypeLayout::Struct(data) | TypeLayout::Class(data) => Some(data),
            _ => None,
        }
    }

    pub(super) fn lookup_enum_layout(&self, ty: &Ty) -> Option<&EnumLayout> {
        let layout = lookup_layout(self.layouts, ty)?;
        match layout {
            TypeLayout::Enum(data) => Some(data),
            _ => None,
        }
    }

    pub(super) fn lookup_union_layout(&self, ty: &Ty) -> Option<&UnionLayout> {
        let layout = lookup_layout(self.layouts, ty)?;
        match layout {
            TypeLayout::Union(data) => Some(data),
            _ => None,
        }
    }

    pub(super) fn lookup_struct_field(&self, ty: &Ty, index: u32) -> Option<&FieldLayout> {
        let layout = self.lookup_struct_layout(ty)?;
        layout.fields.iter().find(|field| field.index == index)
    }

    pub(super) fn lookup_struct_field_named(&self, ty: &Ty, name: &str) -> Option<&FieldLayout> {
        let layout = self.lookup_struct_layout(ty)?;
        layout.fields.iter().find(|field| field.matches_name(name))
    }

    pub(super) fn resolve_self_ty(&self, ty: &Ty) -> Ty {
        match ty {
            Ty::Named(named) if named.as_str() == "Self" => {
                self.receiver_ty().unwrap_or_else(|| ty.clone())
            }
            Ty::Named(named) => {
                if named.args.is_empty() {
                    return ty.clone();
                }
                let mut updated = named.clone();
                updated.args = named
                    .args
                    .iter()
                    .map(|arg| match arg {
                        GenericArg::Type(inner) => GenericArg::Type(self.resolve_self_ty(inner)),
                        other => other.clone(),
                    })
                    .collect();
                Ty::Named(updated)
            }
            Ty::Nullable(inner) => Ty::Nullable(Box::new(self.resolve_self_ty(inner))),
            Ty::Pointer(ptr) => {
                let mut updated: PointerTy = (**ptr).clone();
                updated.element = self.resolve_self_ty(&ptr.element);
                Ty::Pointer(Box::new(updated))
            }
            Ty::Ref(ptr) => {
                let mut updated: RefTy = (**ptr).clone();
                updated.element = self.resolve_self_ty(&ptr.element);
                Ty::Ref(Box::new(updated))
            }
            Ty::Array(array) => {
                let mut updated: ArrayTy = array.clone();
                updated.element = Box::new(self.resolve_self_ty(&array.element));
                Ty::Array(updated)
            }
            Ty::Vec(vec_ty) => {
                let mut updated = vec_ty.clone();
                updated.element = Box::new(self.resolve_self_ty(&vec_ty.element));
                Ty::Vec(updated)
            }
            Ty::Span(span_ty) => {
                let mut updated = span_ty.clone();
                updated.element = Box::new(self.resolve_self_ty(&span_ty.element));
                Ty::Span(updated)
            }
            Ty::ReadOnlySpan(span_ty) => {
                let mut updated = span_ty.clone();
                updated.element = Box::new(self.resolve_self_ty(&span_ty.element));
                Ty::ReadOnlySpan(updated)
            }
            Ty::Rc(rc_ty) => {
                let mut updated = rc_ty.clone();
                updated.element = Box::new(self.resolve_self_ty(&rc_ty.element));
                Ty::Rc(updated)
            }
            Ty::Arc(arc_ty) => {
                let mut updated = arc_ty.clone();
                updated.element = Box::new(self.resolve_self_ty(&arc_ty.element));
                Ty::Arc(updated)
            }
            Ty::Tuple(tuple) => {
                let mut updated: TupleTy = tuple.clone();
                updated.elements = tuple
                    .elements
                    .iter()
                    .map(|elem| self.resolve_self_ty(elem))
                    .collect();
                Ty::Tuple(updated)
            }
            Ty::Fn(fn_ty) => {
                let mut updated: FnTy = fn_ty.clone();
                updated.params = fn_ty
                    .params
                    .iter()
                    .map(|param| self.resolve_self_ty(param))
                    .collect();
                updated.ret = Box::new(self.resolve_self_ty(&fn_ty.ret));
                Ty::Fn(updated)
            }
            _ => ty.clone(),
        }
    }

    fn receiver_ty(&self) -> Option<Ty> {
        if let Some(param) = self.function.signature.params.first() {
            let is_self = param
                .as_named()
                .map(|named| named.as_str() == "Self")
                .unwrap_or(false);
            if !is_self {
                return Some(param.clone());
            }
        }
        self.function
            .name
            .rsplit_once("::")
            .map(|(owner, _)| Ty::named(owner.to_string()))
    }

    pub(super) fn ty_is_reference(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Named(name) => {
                let canonical = ty.canonical_name();
                let layout = lookup_layout(self.layouts, ty);
                let layout_ref = layout.map_or(false, |layout| match layout {
                    TypeLayout::Class(_) => true,
                    TypeLayout::Struct(layout) => layout.class.is_some(),
                    _ => false,
                });
                let has_value_layout = layout.is_some_and(|layout| {
                    matches!(
                        layout,
                        TypeLayout::Struct(_) | TypeLayout::Enum(_) | TypeLayout::Union(_)
                    )
                });
                let class_layout = self.layouts.class_layout_info(name.as_str()).is_some()
                    || self.layouts.class_layout_info(&canonical).is_some();
                let inferred_exception = (!layout_ref && !class_layout && !has_value_layout)
                    && (Self::is_exception_name(name.as_str())
                        || Self::is_exception_name(&canonical));
                if inferred_exception && std::env::var_os("CHIC_DEBUG_WASM_REF_TYPES").is_some() {
                    eprintln!(
                        "[wasm-ref] inferred exception for `{}` (canonical `{}`) in {}",
                        name.as_str(),
                        canonical,
                        self.function.name
                    );
                }
                let is_ref = layout_ref || class_layout || inferred_exception;
                if !is_ref && std::env::var_os("CHIC_DEBUG_WASM_REF_TYPES").is_some() {
                    eprintln!(
                        "[wasm-ref] missing class layout for `{}` (canonical `{}`) in {}",
                        name.as_str(),
                        canonical,
                        self.function.name
                    );
                }
                is_ref
            }
            _ => false,
        }
    }

    pub(super) fn ty_is_trait_object(&self, ty: &Ty) -> bool {
        match ty {
            Ty::TraitObject(_) => true,
            Ty::Named(name) => {
                if name.as_str() == "Std::Platform::Thread::ThreadStart"
                    || name.as_str().ends_with("ThreadStart")
                {
                    return true;
                }
                let canonical = ty.canonical_name();
                let matched = self.trait_vtables.iter().any(|table| {
                    Self::names_equivalent(&table.trait_name, name.as_str())
                        || Self::names_equivalent(&table.trait_name, &canonical)
                });
                if matched && std::env::var_os("CHIC_DEBUG_WASM_TRAIT").is_some() {
                    eprintln!(
                        "[wasm-trait] treating `{}` (canonical `{}`) as trait object in {}",
                        name.as_str(),
                        canonical,
                        self.function.name
                    );
                }
                matched
            }
            _ => false,
        }
    }

    pub(super) fn ty_is_trait_object_like(&self, ty: &Ty) -> bool {
        if self.ty_is_trait_object(ty) {
            return true;
        }
        if let Ty::Nullable(inner) = ty {
            return self.ty_is_trait_object_like(inner);
        }
        match ty {
            Ty::Str | Ty::String | Ty::Span(_) | Ty::ReadOnlySpan(_) => return false,
            _ => {}
        }
        if let Ty::Named(name) = ty {
            let canonical = ty.canonical_name();
            if self.trait_vtables.iter().any(|table| {
                Self::names_equivalent(&table.trait_name, name.as_str())
                    || Self::names_equivalent(&table.trait_name, &canonical)
            }) {
                return true;
            }
        }
        // Heuristic for fat-pointer structs that model `{ data_ptr, vtable_ptr }`.
        //
        // IMPORTANT: exclude `class` layouts. Class instances also start with a `$vtable` pointer
        // at offset 0 and may have a pointer field at offset 4, but those are not trait objects.
        if let Some(layout) = lookup_layout(self.layouts, ty) {
            if let TypeLayout::Struct(layout) = layout {
                if layout.fields.len() == 2 {
                    let has_offsets = layout
                        .fields
                        .iter()
                        .filter_map(|field| field.offset)
                        .collect::<std::collections::HashSet<_>>();
                    let all_ptrs = layout
                        .fields
                        .iter()
                        .all(|field| map_type(&field.ty) == ValueType::I32);
                    if has_offsets.contains(&0) && has_offsets.contains(&4) && all_ptrs {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub(super) fn names_equivalent(expected: &str, candidate: &str) -> bool {
        if expected == candidate {
            return true;
        }
        let normalised_expected = expected.replace("::", ".");
        let normalised_candidate = candidate.replace("::", ".");
        normalised_expected == candidate
            || normalised_candidate == expected
            || normalised_expected == normalised_candidate
            || expected.ends_with(candidate)
            || candidate.ends_with(expected)
            || normalised_expected.ends_with(&normalised_candidate)
            || normalised_candidate.ends_with(&normalised_expected)
    }

    fn is_exception_name(name: &str) -> bool {
        let trimmed = name.split('<').next().unwrap_or(name);
        let short = trimmed.rsplit("::").next().unwrap_or(trimmed);
        short.ends_with("Exception") || short.ends_with("Error")
    }

    pub(crate) fn lookup_function_index(&self, symbol: &str) -> Option<u32> {
        if let Some(index) = self.functions.get(symbol) {
            return Some(*index);
        }
        let canonical = symbol.replace('.', "::");
        let strip_signature = |name: &str| name.split('(').next().unwrap_or(name).to_string();
        let strip_instantiation = |name: &str| name.split('<').next().unwrap_or(name).to_string();
        let base_symbol = strip_instantiation(&strip_signature(symbol));
        let canonical_base = strip_instantiation(&strip_signature(&canonical));
        if let Some(index) = self.functions.get(&canonical) {
            return Some(*index);
        }

        // Primitive receiver normalization: extension methods on `string` can surface as
        // `string::Method` during MIR/codegen, but the symbol table typically records them under
        // the canonical `Std::String::Method` pseudo-type.
        if let Some(rest) = canonical_base.strip_prefix("string::") {
            if let Some(index) = self.functions.get(&format!("Std::String::{rest}")) {
                return Some(*index);
            }
            if let Some(index) = self.functions.get(&format!("Std::Strings::string::{rest}")) {
                return Some(*index);
            }
        }
        if let Some(rest) = canonical_base.strip_prefix("Std::Strings::string::") {
            if let Some(index) = self.functions.get(&format!("Std::String::{rest}")) {
                return Some(*index);
            }
        }
        let resolve_runtime_hook = |rest: &str| -> Option<u32> {
            if let Some(index) = self.functions.get(&format!("chic_rt::{rest}")) {
                return Some(*index);
            }
            let hook = match rest {
                "object_new" => RuntimeHook::ObjectNew,
                "alloc" => RuntimeHook::Alloc,
                "alloc_zeroed" => RuntimeHook::AllocZeroed,
                "realloc" => RuntimeHook::Realloc,
                "free" => RuntimeHook::Free,
                "memcpy" => RuntimeHook::Memcpy,
                "memmove" => RuntimeHook::Memmove,
                "memset" => RuntimeHook::Memset,
                "string_as_slice" => RuntimeHook::StringAsSlice,
                "string_try_copy_utf8" => RuntimeHook::StringTryCopyUtf8,
                "string_from_slice" => RuntimeHook::StringFromSlice,
                _ => return None,
            };
            self.runtime_hook_index(hook).ok()
        };

        if let Some(rest) = symbol.strip_prefix("chic_rt_") {
            if let Some(index) = resolve_runtime_hook(rest) {
                return Some(index);
            }
        }
        if let Some(tail) = symbol.rsplit("::").next() {
            if let Some(rest) = tail.strip_prefix("chic_rt_") {
                if let Some(index) = resolve_runtime_hook(rest) {
                    return Some(index);
                }
            }
        }
        let mut found: Option<(&str, u32)> = None;
        for (name, index) in self.functions {
            let name_no_sig = strip_signature(name);
            let base_name = strip_instantiation(&name_no_sig);
            let base_matches = base_name == base_symbol
                || base_name == canonical_base
                || Self::names_equivalent(&base_symbol, &base_name)
                || Self::names_equivalent(&canonical_base, &base_name);
            if name.ends_with(symbol)
                || name.ends_with(&canonical)
                || name_no_sig.ends_with(symbol)
                || name_no_sig.ends_with(&canonical)
                || base_matches
                || Self::names_equivalent(symbol, name)
            {
                match found {
                    None => found = Some((name.as_str(), *index)),
                    Some((existing_name, existing_index)) => {
                        let existing_std = existing_name.contains("Std::");
                        let new_std = name.contains("Std::");
                        if new_std && !existing_std {
                            found = Some((name.as_str(), *index));
                            continue;
                        }
                        if existing_std && !new_std {
                            continue;
                        }
                        let existing_startup = existing_name.contains("Std::Runtime::Startup::");
                        let new_startup = name.contains("Std::Runtime::Startup::");
                        if existing_startup && !new_startup {
                            found = Some((name.as_str(), *index));
                            continue;
                        }
                        if !existing_startup && new_startup {
                            continue;
                        }
                        found = Some((existing_name, existing_index));
                    }
                }
            }
        }
        found.map(|(_, idx)| idx)
    }

    pub(super) fn lookup_enum_variant_by_index<'b>(
        &'b self,
        layout: &'b EnumLayout,
        index: u32,
    ) -> Option<&'b EnumVariantLayout> {
        layout
            .variants
            .iter()
            .find(|variant| variant.index == index)
    }

    pub(super) fn lookup_variant_field_by_index<'b>(
        &'b self,
        variant: &'b EnumVariantLayout,
        index: u32,
    ) -> Option<&'b FieldLayout> {
        variant.fields.iter().find(|field| field.index == index)
    }

    pub(super) fn lookup_variant_field_named<'b>(
        &'b self,
        variant: &'b EnumVariantLayout,
        name: &str,
    ) -> Option<&'b FieldLayout> {
        variant.fields.iter().find(|field| field.name == name)
    }

    pub(super) fn lookup_union_field<'b>(
        &'b self,
        layout: &'b UnionLayout,
        index: Option<u32>,
        name: Option<&str>,
    ) -> Option<&'b UnionFieldLayout> {
        layout.views.iter().find(|field| {
            if let Some(idx) = index {
                if field.index != idx {
                    return false;
                }
            }
            if let Some(expected) = name {
                if field.name != expected {
                    return false;
                }
            }
            true
        })
    }

    pub(super) fn runtime_hook_index(&self, hook: RuntimeHook) -> Result<u32, Error> {
        let key = hook.qualified_name();
        self.functions.get(&key).copied().ok_or_else(|| {
            Error::Codegen(format!(
                "required runtime hook `{}` is not available in WASM module",
                key
            ))
        })
    }

    pub(super) fn resolve_field_by_index<'b>(
        &'b self,
        ty: &Ty,
        variant: Option<&'b EnumVariantLayout>,
        index: u32,
    ) -> Result<(&'b FieldLayout, usize), Error> {
        if let Some(current_variant) = variant {
            let field = self
                .lookup_variant_field_by_index(current_variant, index)
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "enum variant `{}` does not have field index {index}",
                        current_variant.name
                    ))
                })?;
            let offset = field.offset.ok_or_else(|| {
                Error::Codegen(
                    "type layout missing field offset information for WASM lowering".into(),
                )
            })?;
            return Ok((field, offset));
        }

        let field = self.lookup_struct_field(ty, index).ok_or_else(|| {
            Error::Codegen(format!(
                "unsupported projection: unable to resolve field index {index} on type {:?}",
                ty
            ))
        })?;
        let offset = field.offset.ok_or_else(|| {
            Error::Codegen("type layout missing field offset information for WASM lowering".into())
        })?;
        Ok((field, offset))
    }

    pub(super) fn resolve_field_by_name(
        &self,
        ty: &Ty,
        variant: Option<&EnumVariantLayout>,
        name: &str,
    ) -> Result<(FieldLayout, usize), Error> {
        let resolved_ty = self.resolve_self_ty(ty);
        let ty = &resolved_ty;
        if let Ty::Ref(inner) = ty {
            return self.resolve_field_by_name(&inner.element, variant, name);
        }
        if let Ty::Pointer(inner) = ty {
            return self.resolve_field_by_name(&inner.element, variant, name);
        }

        if let Some(current_variant) = variant {
            let field = self
                .lookup_variant_field_named(current_variant, name)
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "enum variant `{}` does not have field `{name}`",
                        current_variant.name
                    ))
                })?;
            let offset = field.offset.ok_or_else(|| {
                Error::Codegen(
                    "type layout missing field offset information for WASM lowering".into(),
                )
            })?;
            return Ok((field.clone(), offset));
        }

        if let Some(field) = self.lookup_struct_field_named(ty, name) {
            let offset = field.offset.ok_or_else(|| {
                Error::Codegen(
                    "type layout missing field offset information for WASM lowering".into(),
                )
            })?;
            return Ok((field.clone(), offset));
        }

        if let Some(alias) = match name {
            "ptr" => Some("Pointer"),
            "len" => Some("Length"),
            "cap" => Some("Capacity"),
            "elem_size" => Some("ElementSize"),
            "elem_align" => Some("ElementAlignment"),
            "drop_fn" => Some("DropCallback"),
            _ => None,
        } {
            if let Some(field) = self.lookup_struct_field_named(ty, alias) {
                let offset = field.offset.ok_or_else(|| {
                    Error::Codegen(
                        "type layout missing field offset information for WASM lowering".into(),
                    )
                })?;
                return Ok((field.clone(), offset));
            }
        }

        if matches!(name, "ptr" | "len") {
            if let Some(raw_field) = self.lookup_struct_field_named(ty, "Raw") {
                if let Some(raw_offset) = raw_field.offset {
                    if let Ok((field, inner_offset)) =
                        self.resolve_field_by_name(&raw_field.ty, None, name)
                    {
                        let offset = raw_offset.checked_add(inner_offset).ok_or_else(|| {
                            Error::Codegen(
                                "field offset computation overflowed while lowering to WASM".into(),
                            )
                        })?;
                        return Ok((field, offset));
                    }
                }
            }
            if name == "ptr" {
                for data_name in ["Data", "data"] {
                    if let Some(data_field) = self.lookup_struct_field_named(ty, data_name) {
                        if let Some(data_offset) = data_field.offset {
                            if let Ok((field, inner_offset)) =
                                self.resolve_field_by_name(&data_field.ty, None, "Pointer")
                            {
                                let offset = data_offset
                                    .checked_add(inner_offset)
                                    .ok_or_else(|| {
                                        Error::Codegen(
                                            "field offset computation overflowed while lowering to WASM"
                                                .into(),
                                        )
                                    })?;
                                return Ok((field, offset));
                            }
                        }
                    }
                }
            }
        }

        if let Ty::Nullable(inner) = ty {
            let value_field = self.lookup_struct_field_named(ty, "Value").ok_or_else(|| {
                Error::Codegen(format!(
                    "nullable layout missing Value field while resolving `{name}` on `{}` (func={})",
                    ty.canonical_name(),
	                    self.function.name
	                ))
	            })?;
            let value_offset = value_field.offset.ok_or_else(|| {
                Error::Codegen(
                    "type layout missing field offset information for WASM lowering".into(),
                )
            })?;
            let (field, field_offset) = self.resolve_field_by_name(inner, None, name)?;
            let offset = value_offset.checked_add(field_offset).ok_or_else(|| {
                Error::Codegen("field offset computation overflowed while lowering to WASM".into())
            })?;
            return Ok((field, offset));
        }

        if let Some(struct_layout) = self.lookup_struct_layout(ty) {
            if let Some(class_info) = struct_layout.class.as_ref() {
                for base in &class_info.bases {
                    let canonical = base.replace('.', "::");
                    let base_ty = Ty::named(canonical);
                    if let Ok((field, offset)) = self.resolve_field_by_name(&base_ty, None, name) {
                        return Ok((field, offset));
                    }
                }
            }
        }

        if let Ty::Named(named) = ty {
            let type_name = named.name.replace('.', "::");
            let short_name = type_name
                .rsplit("::")
                .next()
                .unwrap_or(type_name.as_str())
                .split_once('<')
                .map(|(base, _)| base)
                .unwrap_or_else(|| type_name.rsplit("::").next().unwrap_or(type_name.as_str()));

            let mut fallback_match: Option<(FieldLayout, usize)> = None;
            for (key, layout) in &self.layouts.types {
                if key.rsplit("::").next() != Some(short_name) {
                    continue;
                }
                let layout = match layout {
                    TypeLayout::Struct(data) | TypeLayout::Class(data) => data,
                    _ => continue,
                };
                let Some(field) = layout.fields.iter().find(|field| field.matches_name(name))
                else {
                    continue;
                };
                let Some(offset) = field.offset else {
                    continue;
                };
                if fallback_match.is_some() {
                    break;
                }
                fallback_match = Some((field.clone(), offset));
            }
            if let Some((field, offset)) = fallback_match {
                return Ok((field, offset));
            }
        }

        Err(Error::Codegen(format!(
            "unable to resolve field `{name}` on type `{}` (func={})",
            ty.canonical_name(),
            self.function.name
        )))
    }

    pub(crate) fn emit_runtime_panic_with_code(
        &self,
        buf: &mut Vec<u8>,
        code: i32,
    ) -> Result<(), Error> {
        let hook = self.runtime_hook_index(RuntimeHook::Panic)?;
        emit_instruction(buf, Op::I32Const(code));
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::Unreachable);
        Ok(())
    }

    pub(crate) fn emit_runtime_panic(&self, buf: &mut Vec<u8>) -> Result<(), Error> {
        self.emit_runtime_panic_with_code(buf, PANIC_EXIT_CODE)
    }

    pub(crate) fn emit_frame_teardown(&self, buf: &mut Vec<u8>) {
        emit_instruction(buf, Op::GlobalGet(STACK_POINTER_GLOBAL_INDEX));
        emit_instruction(buf, Op::LocalGet(self.stack_adjust_local));
        emit_instruction(buf, Op::I32Add);
        if self.frame_size > 0 {
            emit_instruction(buf, Op::I32Const(self.frame_size as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::GlobalSet(STACK_POINTER_GLOBAL_INDEX));
    }

    pub(crate) fn initialise_frame_allocations(
        &self,
        code: &mut Vec<u8>,
        frame_local: u32,
    ) -> Result<(), Error> {
        let memcpy_hook = self.runtime_hook_index(RuntimeHook::Memcpy)?;
        for (local_index, repr) in self.representations.iter().enumerate() {
            if !matches!(repr, LocalRepresentation::FrameAllocated) {
                continue;
            }
            let Some(pointer_local) = self.locals.get(local_index).and_then(|slot| *slot) else {
                continue;
            };
            let decl = self
                .function
                .body
                .locals
                .get(local_index)
                .unwrap_or_else(|| {
                    panic!(
                        "missing local decl for index {} while initialising frame allocations",
                        local_index
                    )
                });
            let Some(info) = self
                .aggregate_allocations
                .get(local_index)
                .and_then(|entry| *entry)
            else {
                continue;
            };
            let from_param = matches!(decl.kind, crate::mir::LocalKind::Arg(_));
            emit_instruction(code, Op::LocalGet(frame_local));
            if info.offset != 0 {
                emit_instruction(code, Op::I32Const(info.offset as i32));
                emit_instruction(code, Op::I32Add);
            }
            if from_param {
                emit_instruction(code, Op::LocalTee(pointer_local));
            } else {
                emit_instruction(code, Op::LocalSet(pointer_local));
            }
            if let crate::mir::LocalKind::Arg(param_idx) = decl.kind {
                let param_local = (param_idx as u32) + self.param_shift;
                let ty = &self.local_tys[local_index];
                if local_requires_memory(ty, self.layouts) {
                    emit_instruction(code, Op::LocalGet(pointer_local));
                    emit_instruction(code, Op::LocalGet(param_local));
                    emit_instruction(code, Op::I32Const(info.size as i32));
                    emit_instruction(code, Op::Call(memcpy_hook));
                } else {
                    let value_ty = map_type(ty);
                    emit_instruction(code, Op::LocalGet(param_local));
                    emit_instruction(
                        code,
                        match value_ty {
                            ValueType::I32 => Op::I32Store(0),
                            ValueType::I64 => Op::I64Store(0),
                            ValueType::F32 => Op::F32Store(0),
                            ValueType::F64 => Op::F64Store(0),
                        },
                    );
                }
            }
        }
        Ok(())
    }

    pub(crate) fn emit_body(&mut self) -> Result<Vec<u8>, Error> {
        let body_start = Instant::now();
        wasm_debug!(
            "    emit_body `{}`: declaring {} wasm locals",
            self.function.name,
            self.local_types.len()
        );
        let mut out = Vec::new();
        self.emit_local_decls(&mut out)?;

        let mut code = Vec::new();
        self.emit_prelude(&mut code)?;
        self.emit_trace_enter(&mut code)?;
        self.emit_blocks(&mut code)?;
        self.emit_epilogue(&mut code);

        out.extend_from_slice(&code);
        wasm_debug!(
            "    emit_body `{}`: finished ({} bytes) in {:?}",
            self.function.name,
            out.len(),
            body_start.elapsed()
        );
        Ok(out)
    }

    fn static_slot(&self, id: StaticId) -> Result<(&StaticVar, u32, ValueType), Error> {
        let var = self
            .statics
            .get(id.0)
            .ok_or_else(|| Error::Codegen(format!("static id {} is not registered", id.0)))?;
        let offset = self.static_offsets.get(&id).copied().ok_or_else(|| {
            Error::Codegen(format!(
                "static `{}` is missing a WebAssembly storage slot",
                var.qualified
            ))
        })?;
        let value_ty = map_type(&var.ty);
        Ok((var, offset, value_ty))
    }

    pub(super) fn emit_static_load(
        &mut self,
        buf: &mut Vec<u8>,
        id: StaticId,
    ) -> Result<ValueType, Error> {
        let (_var, offset, value_ty) = self.static_slot(id)?;
        let offset_i32 = i32::try_from(offset).map_err(|_| {
            Error::Codegen(format!(
                "static offset {offset} for id {} exceeds i32 range",
                id.0
            ))
        })?;
        emit_instruction(buf, Op::I32Const(offset_i32));
        emit_instruction(
            buf,
            match value_ty {
                ValueType::I32 => Op::I32Load(0),
                ValueType::I64 => Op::I64Load(0),
                ValueType::F32 => Op::F32Load(0),
                ValueType::F64 => Op::F64Load(0),
            },
        );
        Ok(value_ty)
    }

    pub(super) fn emit_static_ref(
        &mut self,
        buf: &mut Vec<u8>,
        id: StaticId,
    ) -> Result<ValueType, Error> {
        let (_var, offset, _) = self.static_slot(id)?;
        let offset_i32 = i32::try_from(offset).map_err(|_| {
            Error::Codegen(format!(
                "static offset {offset} for id {} exceeds i32 range",
                id.0
            ))
        })?;
        emit_instruction(buf, Op::I32Const(offset_i32));
        Ok(ValueType::I32)
    }

    pub(super) fn emit_static_store(
        &mut self,
        buf: &mut Vec<u8>,
        id: StaticId,
        value: &Operand,
    ) -> Result<(), Error> {
        let (_var, offset, value_ty) = self.static_slot(id)?;
        let offset_i32 = i32::try_from(offset).map_err(|_| {
            Error::Codegen(format!(
                "static offset {offset} for id {} exceeds i32 range",
                id.0
            ))
        })?;
        emit_instruction(buf, Op::I32Const(offset_i32));
        let mut operand_ty = self.emit_operand(buf, value)?;
        if operand_ty != value_ty {
            match (operand_ty, value_ty) {
                (ValueType::F64, ValueType::F32) => emit_instruction(buf, Op::F32DemoteF64),
                (ValueType::F32, ValueType::F64) => emit_instruction(buf, Op::F64PromoteF32),
                (ValueType::I64, ValueType::I32) => emit_instruction(buf, Op::I32WrapI64),
                (ValueType::I32, ValueType::I64) => emit_instruction(buf, Op::I64ExtendI32S),
                (ValueType::I32, ValueType::F32) => emit_instruction(buf, Op::F32ConvertI32S),
                (ValueType::I32, ValueType::F64) => emit_instruction(buf, Op::F64ConvertI32S),
                (ValueType::F32, ValueType::I32) => emit_instruction(buf, Op::I32TruncF32S),
                (ValueType::F64, ValueType::I32) => emit_instruction(buf, Op::I32TruncF64S),
                (ValueType::F32, ValueType::I64) => emit_instruction(buf, Op::I64TruncF32S),
                (ValueType::F64, ValueType::I64) => emit_instruction(buf, Op::I64TruncF64S),
                _ => {}
            }
            operand_ty = value_ty;
        }
        Self::ensure_operand_type(operand_ty, value_ty, "static store")?;
        emit_instruction(
            buf,
            match value_ty {
                ValueType::I32 => Op::I32Store(0),
                ValueType::I64 => Op::I64Store(0),
                ValueType::F32 => Op::F32Store(0),
                ValueType::F64 => Op::F64Store(0),
            },
        );
        Ok(())
    }

    fn emit_local_decls(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        write_u32(
            out,
            ensure_u32(
                self.local_types.len(),
                "local declaration count exceeds WebAssembly limits",
            )?,
        );
        for ty in &self.local_types {
            write_u32(out, 1);
            out.push(ty.to_byte());
        }
        Ok(())
    }

    fn emit_prelude(&self, code: &mut Vec<u8>) -> Result<(), Error> {
        if let Some(frame_local) = self.frame_local {
            emit_instruction(code, Op::GlobalGet(STACK_POINTER_GLOBAL_INDEX));
            if self.frame_size > 0 {
                emit_instruction(code, Op::I32Const(self.frame_size as i32));
                emit_instruction(code, Op::I32Sub);
            }
            emit_instruction(code, Op::LocalTee(frame_local));
            emit_instruction(code, Op::GlobalSet(STACK_POINTER_GLOBAL_INDEX));
            if self.frame_size > 0 {
                emit_instruction(code, Op::LocalGet(frame_local));
                emit_instruction(code, Op::I32Const(0));
                emit_instruction(code, Op::I32Const(self.frame_size as i32));
                emit_instruction(code, Op::MemoryFill);
            }
            self.initialise_frame_allocations(code, frame_local)?;
        }
        if self.coverage_enabled {
            let hook = self.runtime_hook_index(RuntimeHook::CoverageHit)?;
            let id = (self.coverage_function_index as u64) << 32;
            emit_instruction(code, Op::I64Const(id as i64));
            emit_instruction(code, Op::Call(hook));
        }
        emit_instruction(code, Op::I32Const(0));
        emit_instruction(code, Op::LocalSet(self.stack_adjust_local));
        emit_instruction(code, Op::I32Const(0));
        emit_instruction(code, Op::LocalSet(self.block_local));
        emit_instruction(code, Op::Block);
        emit_instruction(code, Op::Loop);
        Ok(())
    }
}
