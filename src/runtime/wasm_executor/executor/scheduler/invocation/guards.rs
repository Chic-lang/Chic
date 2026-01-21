use super::super::Executor;
use crate::runtime::wasm_executor::types::{Value, ValueType};

pub(super) struct CallDepthGuard<'a> {
    exec: *mut Executor<'a>,
    prev_func: Option<u32>,
    pushed: bool,
    frame_depth: u32,
}

impl<'a> CallDepthGuard<'a> {
    pub(super) fn new(exec: &mut Executor<'a>, func_index: u32) -> Self {
        let prev_func = exec.current_function;
        exec.current_function = Some(func_index);
        exec.call_stack.push(func_index);
        let frame_depth = exec.call_depth.min(u32::MAX as usize) as u32;
        Self {
            exec: exec as *mut Executor<'a>,
            prev_func,
            pushed: true,
            frame_depth,
        }
    }
}

impl<'a> Drop for CallDepthGuard<'a> {
    fn drop(&mut self) {
        unsafe {
            if self.pushed {
                let _ = (*self.exec).call_stack.pop();
            }
            (*self.exec).call_depth = (*self.exec).call_depth.saturating_sub(1);
            let frame_depth = self.frame_depth;
            (*self.exec)
                .borrow_records
                .retain(|key, _| key.frame_depth != frame_depth);
            (*self.exec).current_function = self.prev_func;
        }
    }
}

pub(super) struct StackPointerGuard<'a> {
    exec: *mut Executor<'a>,
    saved: Option<i32>,
}

impl<'a> StackPointerGuard<'a> {
    pub(super) fn new(exec: &mut Executor<'a>, stack_base: i32) -> Self {
        let mut saved = None;
        if let Some(global) = exec.globals.get_mut(0) {
            if global.mutable && matches!(global.ty, ValueType::I32) {
                let current = match global.value {
                    Value::I32(v) => v,
                    _ => 0,
                };
                let sanitized = if current <= 0 { stack_base } else { current };
                if sanitized != current && std::env::var_os("CHIC_DEBUG_WASM_SP").is_some() {
                    eprintln!("[wasm-sp] reset global0 from {} to {}", current, sanitized);
                }
                global.value = Value::I32(sanitized);
                saved = Some(sanitized);
            }
        }
        Self {
            exec: exec as *mut Executor<'a>,
            saved,
        }
    }
}

impl<'a> Drop for StackPointerGuard<'a> {
    fn drop(&mut self) {
        unsafe {
            if let Some(saved) = self.saved {
                let exec = &mut *self.exec;
                if let Some(global) = exec.globals.get_mut(0) {
                    if global.mutable && matches!(global.ty, ValueType::I32) {
                        global.value = Value::I32(saved);
                    }
                }
            }
        }
    }
}
