#![cfg(test)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::chic_kind::ChicKind;
use crate::codegen::wasm::STACK_POINTER_GLOBAL_INDEX;
use crate::codegen::wasm::emitter::function::FunctionEmitter;
use crate::codegen::wasm::module_builder::{FunctionSignature, ModuleBuilder};
use crate::codegen::wasm::runtime_hooks::ALL_RUNTIME_HOOKS;
use crate::error::Error;
use crate::mir::{MirFunction, MirModule, StaticId, TypeLayoutTable};
use crate::perf::PerfMetadata;

use super::{contains_bytes, leb_u32, module_with_functions};

const GOLDEN_ENV: &str = "UPDATE_WASM_GOLDENS";
const GOLDEN_DIR: &str = "tests/golden/wasm";

/// Shared harness for WebAssembly function and module tests.
///
/// The harness owns a [`MirModule`] and computed layout context so tests can
/// obtain [`FunctionEmitter`] and [`ModuleBuilder`] instances without copying
/// scaffolding. It also exposes deterministic golden comparison utilities to
/// keep module snapshots consistent across suites.
pub(crate) struct WasmFunctionHarness {
    module: MirModule,
    layouts: TypeLayoutTable,
    function_indices: HashMap<String, u32>,
    runtime_hook_indices: HashMap<String, u32>,
    function_signatures: HashMap<String, FunctionSignature>,
    static_offsets: HashMap<StaticId, u32>,
    class_vtable_offsets: HashMap<String, u32>,
    perf: PerfMetadata,
}

impl WasmFunctionHarness {
    /// Construct a harness from an existing MIR module.
    pub(crate) fn from_module(module: MirModule) -> Self {
        Self::from_module_with_layouts(module, super::wasm_layouts())
    }

    /// Construct a harness from individual MIR functions.
    pub(crate) fn from_functions(functions: Vec<MirFunction>) -> Self {
        Self::from_module(module_with_functions(functions))
    }

    /// Construct a harness for a single MIR function.
    pub(crate) fn from_function(function: MirFunction) -> Self {
        Self::from_functions(vec![function])
    }

    /// Construct a harness from an existing module with explicit layouts.
    pub(crate) fn from_module_with_layouts(module: MirModule, layouts: TypeLayoutTable) -> Self {
        let function_indices = module
            .functions
            .iter()
            .enumerate()
            .map(|(idx, func)| (func.name.clone(), idx as u32))
            .collect();
        let runtime_hook_indices = runtime_hook_indices();
        let function_signatures = module
            .functions
            .iter()
            .map(|func| {
                (
                    func.name.clone(),
                    FunctionSignature::from_mir(func, &layouts),
                )
            })
            .collect();
        let perf = PerfMetadata::default();
        let (static_offsets, class_vtable_offsets) = match ModuleBuilder::new(
            &module,
            None,
            ChicKind::Executable,
            &[],
            &[],
            &[],
            None,
            &perf,
            false,
        ) {
            Ok(builder) => (
                builder.static_offsets().clone(),
                builder.class_vtable_offsets().clone(),
            ),
            Err(_) => (HashMap::new(), HashMap::new()),
        };
        Self {
            module,
            layouts,
            function_indices,
            runtime_hook_indices,
            function_signatures,
            static_offsets,
            class_vtable_offsets,
            perf,
        }
    }

    /// Override the layout table for an existing harness.
    pub(crate) fn with_layouts(mut self, layouts: TypeLayoutTable) -> Self {
        self.layouts = layouts;
        self
    }

    /// Borrow the underlying module.
    pub(crate) fn module(&self) -> &MirModule {
        &self.module
    }

    pub(crate) fn runtime_hooks(&self) -> &HashMap<String, u32> {
        &self.runtime_hook_indices
    }

    pub(crate) fn function_index_map(&self) -> HashMap<String, u32> {
        let mut indices = self.function_indices.clone();
        indices.extend(self.runtime_hook_indices.clone());
        indices
    }

    /// Produce a [`FunctionEmitter`] for the requested function name.
    /// Emit the WebAssembly body bytes for the given function using optional index overrides.
    pub(crate) fn emit_body_with(
        &self,
        name: &str,
        configure: impl FnOnce(&mut HashMap<String, u32>) -> Option<HashMap<FunctionSignature, u32>>,
    ) -> Result<Vec<u8>, Error> {
        let mut indices = self.function_index_map();
        let signature_indices = configure(&mut indices);
        let function_return_tys = HashMap::new();
        let mut emitter = FunctionEmitter::new(
            self.function(name),
            &indices,
            &function_return_tys,
            None,
            &self.layouts,
            None,
            None,
            Some(&self.class_vtable_offsets),
            signature_indices.as_ref(),
            Some(&self.function_signatures),
            &self.module.trait_vtables,
            &self.module.class_vtables,
            Some(&self.module.statics),
            Some(&self.static_offsets),
            None,
        )?;
        emitter.emit_body()
    }

    pub(crate) fn with_emitter<R>(
        &self,
        name: &str,
        configure: impl FnOnce(&mut HashMap<String, u32>) -> Option<HashMap<FunctionSignature, u32>>,
        action: impl FnOnce(&mut FunctionEmitter<'_>) -> R,
    ) -> Result<R, Error> {
        let mut indices = self.function_index_map();
        let signature_indices = configure(&mut indices);
        let function_return_tys = HashMap::new();
        let mut emitter = FunctionEmitter::new(
            self.function(name),
            &indices,
            &function_return_tys,
            None,
            &self.layouts,
            None,
            None,
            Some(&self.class_vtable_offsets),
            signature_indices.as_ref(),
            Some(&self.function_signatures),
            &self.module.trait_vtables,
            &self.module.class_vtables,
            Some(&self.module.statics),
            Some(&self.static_offsets),
            None,
        )?;
        Ok(action(&mut emitter))
    }

    /// Create a [`ModuleBuilder`] configured for the harness module.
    pub(crate) fn module_builder(
        &self,
        entry: Option<&str>,
        kind: ChicKind,
    ) -> Result<ModuleBuilder<'_>, Error> {
        ModuleBuilder::new(
            &self.module,
            entry.map(|name| name.to_string()),
            kind,
            &[],
            &[],
            &[],
            None,
            &self.perf,
            false,
        )
    }

    /// Emit a full module as bytes using the provided entry point and build kind.
    #[allow(dead_code)]
    pub(crate) fn emit_module(
        &self,
        entry: Option<&str>,
        kind: ChicKind,
    ) -> Result<Vec<u8>, Error> {
        let builder = self.module_builder(entry, kind)?;
        builder.emit()
    }

    /// Assert that `actual` bytes match the recorded golden snapshot named `slug`.
    ///
    /// Setting `UPDATE_WASM_GOLDENS=1` in the environment rewrites the snapshot.
    pub(crate) fn assert_golden(&self, slug: &str, actual: &[u8]) {
        let path = golden_path(slug);
        if std::env::var_os(GOLDEN_ENV).is_some() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create golden directory");
            }
            fs::write(&path, encode_hex(actual)).expect("write golden bytes");
        }

        let expected =
            fs::read_to_string(&path).unwrap_or_else(|err| panic!("read golden {path:?}: {err}"));
        let expected = expected.trim_end();
        let actual_hex = encode_hex(actual);
        assert_eq!(
            expected, actual_hex,
            "golden mismatch for `{slug}` — rerun with {GOLDEN_ENV}=1 to update"
        );
    }

    pub(crate) fn function(&self, name: &str) -> &MirFunction {
        self.module
            .functions
            .iter()
            .find(|func| func.name == name)
            .unwrap_or_else(|| panic!("no function named `{name}` in harness module"))
    }
}

/// Encode bytes as lowercase hexadecimal without whitespace.
fn encode_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from_digit(u32::from(byte >> 4), 16).unwrap());
        encoded.push(char::from_digit(u32::from(byte & 0xF), 16).unwrap());
    }
    encoded
}

fn golden_path(slug: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(GOLDEN_DIR)
        .join(format!("{slug}.hex"))
}

fn runtime_hook_indices() -> HashMap<String, u32> {
    ALL_RUNTIME_HOOKS
        .iter()
        .filter(|hook| !matches!(hook, crate::codegen::wasm::RuntimeHook::CoverageHit))
        .enumerate()
        .map(|(idx, hook)| (hook.qualified_name(), idx as u32))
        .collect()
}

/// Helper ensuring stack pointer prologue sequences appear in emitted bodies.
pub(crate) fn assert_stack_frame_initialised(body: &[u8]) {
    let mut prologue = vec![0x23];
    prologue.extend(leb_u32(STACK_POINTER_GLOBAL_INDEX));
    prologue.extend([0x41, 0x08]);
    assert!(
        contains_bytes(body, &prologue),
        "expected stack pointer prologue sequence"
    );
}

pub(crate) fn harness_with_layouts(
    function: MirFunction,
    layouts: TypeLayoutTable,
) -> (WasmFunctionHarness, String) {
    let name = function.name.clone();
    let harness = WasmFunctionHarness::from_function(function).with_layouts(layouts);
    (harness, name)
}
