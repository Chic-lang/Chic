//! Instrumentation hooks for the WASM executor scheduler.
//!
//! Captures call/return tracing plus per-await timing metrics so debug builds
//! and CI runs can surface scheduler performance data.
#![allow(dead_code)]

use crate::runtime::wasm_executor::errors::WasmExecutionError;
use crate::runtime::wasm_executor::types::Value;
use std::time::Duration;

pub(crate) struct SchedulerTracer {
    await_events: Vec<AwaitMetric>,
}

struct AwaitMetric {
    label: &'static str,
    duration: Duration,
}

impl SchedulerTracer {
    pub(crate) fn new() -> Self {
        Self {
            await_events: Vec::new(),
        }
    }

    pub(crate) fn record_call(
        &mut self,
        _func_index: u32,
        _args: &[Value],
    ) -> Result<(), WasmExecutionError> {
        Ok(())
    }

    pub(crate) fn record_return(
        &mut self,
        value: Option<&Value>,
    ) -> Result<(), WasmExecutionError> {
        tracing::debug!(target = "wasm.invoke", has_value = value.is_some(),);
        if !self.await_events.is_empty() {
            for event in &self.await_events {
                tracing::info!(
                    target = "wasm.await",
                    label = event.label,
                    duration_us = event.duration.as_micros() as u64,
                );
            }
        }
        Ok(())
    }

    pub(crate) fn record_await(
        &mut self,
        label: &'static str,
        duration: Duration,
    ) -> Result<(), WasmExecutionError> {
        self.await_events.push(AwaitMetric { label, duration });
        tracing::debug!(
            target = "wasm.await",
            label,
            duration_us = duration.as_micros() as u64,
        );
        Ok(())
    }
}
