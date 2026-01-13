use crate::runtime::wasm_executor::errors::WasmExecutionError;
use crate::runtime::wasm_executor::module::Module;
use crate::runtime::wasm_executor::types::Value;

use super::Executor;
use super::options::{WasmExecutionOptions, WasmExecutionTrace};

/// Abstraction over a WebAssembly executor capable of evaluating Chic programs.
pub trait WasmExecutor<'module> {
    /// Invoke a function within the module and capture its return value alongside the execution trace.
    fn call_with_trace(
        &mut self,
        func_index: u32,
        args: &[Value],
    ) -> Result<(Option<Value>, WasmExecutionTrace), WasmExecutionError>;
}

/// Factory for producing executor instances, enabling alternate runtimes during testing or embedding.
pub trait WasmExecutorFactory {
    type Executor<'module>: WasmExecutor<'module>;

    fn create<'module>(
        &self,
        module: &'module Module,
        options: &WasmExecutionOptions,
    ) -> Result<Self::Executor<'module>, WasmExecutionError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultExecutorFactory;

impl WasmExecutorFactory for DefaultExecutorFactory {
    type Executor<'module> = Executor<'module>;

    fn create<'module>(
        &self,
        module: &'module Module,
        options: &WasmExecutionOptions,
    ) -> Result<Self::Executor<'module>, WasmExecutionError> {
        Executor::with_options(module, options)
    }
}

impl<'module> WasmExecutor<'module> for Executor<'module> {
    fn call_with_trace(
        &mut self,
        func_index: u32,
        args: &[Value],
    ) -> Result<(Option<Value>, WasmExecutionTrace), WasmExecutionError> {
        Executor::call_with_trace(self, func_index, args)
    }
}
