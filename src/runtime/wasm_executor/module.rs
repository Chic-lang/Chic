use std::collections::HashMap;

use super::errors::WasmExecutionError;
use super::executor::{
    DefaultExecutorFactory, WasmExecutionOptions, WasmExecutionTrace, WasmExecutor,
    WasmExecutorFactory,
};
use super::instructions::Instruction;
use super::parser::parse_module;
use super::types::{FuncType, Value, ValueType, WasmValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableElementType {
    FuncRef,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub element_type: TableElementType,
    pub min: u32,
    pub max: Option<u32>,
    pub elements: Vec<Option<u32>>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub type_index: u32,
    pub locals: Vec<ValueType>,
    pub code: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub module: String,
    pub name: String,
    pub type_index: u32,
}

#[derive(Debug, Clone)]
pub struct Global {
    pub ty: ValueType,
    pub mutable: bool,
    pub initial: Value,
}

#[derive(Debug, Clone)]
pub struct DataSegment {
    pub offset: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub types: Vec<FuncType>,
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
    pub function_names: Vec<String>,
    pub tables: Vec<Table>,
    pub exports: HashMap<String, u32>,
    pub memory_min_pages: Option<u32>,
    pub globals: Vec<Global>,
    pub data_segments: Vec<DataSegment>,
    pub interface_defaults: Vec<InterfaceDefault>,
    pub type_metadata: Vec<TypeMetadataRecord>,
    pub hash_glue: Vec<GlueRecord>,
    pub eq_glue: Vec<GlueRecord>,
}

impl Module {
    pub fn function_name(&self, func_index: u32) -> Option<&str> {
        self.function_names
            .get(func_index as usize)
            .map(String::as_str)
    }
}

#[derive(Clone)]
pub struct WasmProgram {
    module: Module,
}

pub struct WasmProgramExportOutcome {
    pub value: Option<WasmValue>,
    pub trace: WasmExecutionTrace,
}

#[derive(Debug, Clone)]
pub struct InterfaceDefault {
    pub implementer: String,
    pub interface: String,
    pub method: String,
    pub function_index: u32,
}

#[derive(Debug, Clone)]
pub struct GlueRecord {
    pub type_id: u64,
    pub function_index: u32,
}

#[derive(Debug, Clone)]
pub struct TypeMetadataRecord {
    pub type_id: u64,
    pub size: u32,
    pub align: u32,
    pub variance: Vec<TypeVarianceRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeVarianceRecord {
    Invariant,
    Covariant,
    Contravariant,
}

impl WasmProgram {
    /// Parse raw WebAssembly bytes into an executable program.
    ///
    /// # Errors
    /// Returns `WasmExecutionError` when the payload is not a valid Chic-compatible WASM module.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WasmExecutionError> {
        let module = parse_module(bytes)?;
        Ok(Self { module })
    }

    #[must_use]
    pub fn has_export(&self, export: &str) -> bool {
        self.module.exports.contains_key(export)
    }

    #[must_use]
    pub fn export_index(&self, export: &str) -> Option<u32> {
        self.module.exports.get(export).copied()
    }

    pub fn export_names(&self) -> impl Iterator<Item = &String> {
        self.module.exports.keys()
    }

    #[must_use]
    pub fn interface_defaults(&self) -> &[InterfaceDefault] {
        &self.module.interface_defaults
    }

    /// Execute an exported function with the provided arguments.
    ///
    /// # Errors
    /// Returns `WasmExecutionError` if the export is missing or the program traps during execution.
    pub fn execute_export(
        &self,
        name: &str,
        args: &[super::types::Value],
    ) -> Result<Option<WasmValue>, WasmExecutionError> {
        self.execute_export_with_options(name, args, &WasmExecutionOptions::default())
            .map(|outcome| outcome.value)
    }

    pub fn execute_export_with_options(
        &self,
        name: &str,
        args: &[super::types::Value],
        options: &WasmExecutionOptions,
    ) -> Result<WasmProgramExportOutcome, WasmExecutionError> {
        self.execute_export_with_options_using(name, args, options, &DefaultExecutorFactory)
    }

    pub fn execute_export_with_options_using<F: WasmExecutorFactory>(
        &self,
        name: &str,
        args: &[super::types::Value],
        options: &WasmExecutionOptions,
        factory: &F,
    ) -> Result<WasmProgramExportOutcome, WasmExecutionError> {
        let export_index = *self
            .module
            .exports
            .get(name)
            .ok_or_else(|| WasmExecutionError {
                message: format!("export `{name}` not found"),
            })?;
        let mut executor = factory.create(&self.module, options)?;
        let (value, trace) = executor.call_with_trace(export_index, args)?;
        Ok(WasmProgramExportOutcome {
            value: value.map(Into::into),
            trace,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::wasm_executor::errors::WasmExecutionError;
    use crate::runtime::wasm_executor::executor::{
        Executor, WasmExecutionOptions, WasmExecutionTrace, WasmExecutor, WasmExecutorFactory,
    };
    use crate::runtime::wasm_executor::tests::simple_module;
    use crate::runtime::wasm_executor::types::Value;
    use std::cell::Cell;

    fn expect_ok<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
        match result {
            Ok(value) => value,
            Err(err) => panic!("{context}: {err}"),
        }
    }

    fn expect_err<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> E {
        match result {
            Ok(_) => panic!("{context}: expected error"),
            Err(err) => err,
        }
    }

    #[test]
    fn missing_export_reports_error() {
        let wasm = simple_module(1);
        let program = expect_ok(WasmProgram::from_bytes(&wasm), "parse ok");
        let err = expect_err(program.execute_export("missing", &[]), "missing export");
        assert!(err.message.contains("export `missing` not found"));
    }

    #[test]
    fn has_export_and_names_iterate() {
        let wasm = simple_module(2);
        let program = expect_ok(WasmProgram::from_bytes(&wasm), "parse ok");
        assert!(program.has_export("chic_main"));
        let exports: Vec<_> = program.export_names().cloned().collect();
        assert_eq!(exports, vec!["chic_main".to_string()]);
    }

    #[test]
    fn execute_export_supports_custom_executor_factory() {
        struct RecordingFactory {
            constructed: Cell<bool>,
        }

        impl RecordingFactory {
            fn new() -> Self {
                Self {
                    constructed: Cell::new(false),
                }
            }

            fn constructed(&self) -> bool {
                self.constructed.get()
            }
        }

        struct RecordingExecutor<'module> {
            inner: Executor<'module>,
        }

        impl<'module> WasmExecutor<'module> for RecordingExecutor<'module> {
            fn call_with_trace(
                &mut self,
                func_index: u32,
                args: &[Value],
            ) -> Result<(Option<Value>, WasmExecutionTrace), WasmExecutionError> {
                self.inner.call_with_trace(func_index, args)
            }
        }

        impl WasmExecutorFactory for RecordingFactory {
            type Executor<'module> = RecordingExecutor<'module>;

            fn create<'module>(
                &self,
                module: &'module Module,
                options: &WasmExecutionOptions,
            ) -> Result<Self::Executor<'module>, WasmExecutionError> {
                self.constructed.set(true);
                let inner = Executor::with_options(module, options)?;
                Ok(RecordingExecutor { inner })
            }
        }

        let wasm = simple_module(7);
        let program = expect_ok(WasmProgram::from_bytes(&wasm), "parse ok");
        let factory = RecordingFactory::new();
        let outcome = expect_ok(
            program.execute_export_with_options_using(
                "chic_main",
                &[],
                &WasmExecutionOptions::default(),
                &factory,
            ),
            "execute with custom executor factory",
        );
        assert_eq!(outcome.value, Some(WasmValue::I32(7)));
        assert!(factory.constructed());
    }
}
