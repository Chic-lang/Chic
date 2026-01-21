use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn try_emit_startup_runtime_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: &CallLowering<'_>,
    ) -> Result<bool, Error> {
        let canonical = match call.func {
            Operand::Const(constant) => match &constant.value {
                ConstValue::Symbol(name) => canonical_symbol_name(name),
                _ => return Ok(false),
            },
            _ => return Ok(false),
        };
        if !canonical.ends_with("::chic_rt_startup_exit") && canonical != "chic_rt_startup_exit" {
            return Ok(false);
        }
        if call.destination.is_some() {
            return Err(Error::Codegen(
                "startup exit call cannot assign to a destination in WASM backend".into(),
            ));
        }
        if call.args.len() != 1 {
            return Err(Error::Codegen(
                "startup exit call expects exactly one argument".into(),
            ));
        }
        let value_ty = self.emit_operand(buf, &call.args[0])?;
        if value_ty != ValueType::I32 {
            return Err(Error::Codegen(
                "startup exit argument must lower to i32 in WASM backend".into(),
            ));
        }
        let hook = self.runtime_hook_index(RuntimeHook::Abort)?;
        emit_instruction(buf, Op::Call(hook));
        self.release_call_borrows(buf, call.args, call.modes)?;
        self.emit_goto(buf, call.target);
        Ok(true)
    }

    pub(super) fn try_emit_decimal_fast_runtime_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: &CallLowering<'_>,
    ) -> Result<bool, Error> {
        let canonical = match call.func {
            Operand::Const(constant) => match &constant.value {
                ConstValue::Symbol(name) => canonical_symbol_name(name),
                _ => return Ok(false),
            },
            _ => return Ok(false),
        };
        let (hook, returns_struct) = match canonical.as_str() {
            "Std::Async::RuntimeIntrinsics::chic_rt_async_token_new"
            | "chic_rt_async_token_new" => (RuntimeHook::AsyncTokenNew, false),
            "Std::Async::RuntimeIntrinsics::chic_rt_async_token_cancel"
            | "chic_rt_async_token_cancel" => (RuntimeHook::AsyncTokenCancel, false),
            "Std::Async::RuntimeIntrinsics::chic_rt_async_task_header"
            | "chic_rt_async_task_header" => (RuntimeHook::AsyncTaskHeader, false),
            "Std::Async::RuntimeIntrinsics::chic_rt_async_spawn_local"
            | "chic_rt_async_spawn_local" => (RuntimeHook::AsyncSpawnLocal, false),
            "Std::Async::RuntimeIntrinsics::chic_rt_async_scope" | "chic_rt_async_scope" => {
                (RuntimeHook::AsyncScope, false)
            }
            "Std::Async::RuntimeIntrinsics::chic_rt_async_spawn" | "chic_rt_async_spawn" => {
                (RuntimeHook::AsyncSpawn, false)
            }
            "Std::Async::RuntimeIntrinsics::chic_rt_async_block_on" | "chic_rt_async_block_on" => {
                (RuntimeHook::AsyncScope, false)
            }
            "Std::Numeric::Decimal::RuntimeIntrinsics::chic_rt_decimal_sum"
            | "chic_rt_decimal_sum" => (RuntimeHook::DecimalSum, true),
            "Std::Numeric::Decimal::RuntimeIntrinsics::chic_rt_decimal_dot"
            | "chic_rt_decimal_dot" => (RuntimeHook::DecimalDot, true),
            "Std::Numeric::Decimal::RuntimeIntrinsics::chic_rt_decimal_matmul"
            | "chic_rt_decimal_matmul" => (RuntimeHook::DecimalMatMul, false),
            "chic_rt_closure_env_alloc" => (RuntimeHook::ClosureEnvAlloc, false),
            "chic_rt_closure_env_clone" => (RuntimeHook::ClosureEnvClone, false),
            "chic_rt_closure_env_free" => (RuntimeHook::ClosureEnvFree, false),
            _ => return Ok(false),
        };

        if returns_struct {
            let destination = call.destination.ok_or_else(|| {
                Error::Codegen(
                    "decimal runtime call must assign its result in the WASM backend".into(),
                )
            })?;
            let result_access = self.resolve_memory_access(destination)?;
            self.emit_pointer_expression(buf, &result_access)?;
            for (index, arg) in call.args.iter().enumerate() {
                let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                self.emit_call_argument_for_mode(buf, arg, mode)?;
            }
            let hook_index = self.runtime_hook_index(hook)?;
            emit_instruction(buf, Op::Call(hook_index));
            self.release_call_borrows(buf, call.args, call.modes)?;
            self.emit_goto(buf, call.target);
            return Ok(true);
        }

        for (index, arg) in call.args.iter().enumerate() {
            let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
            self.emit_call_argument_for_mode(buf, arg, mode)?;
        }
        let hook_index = self.runtime_hook_index(hook)?;
        emit_instruction(buf, Op::Call(hook_index));
        self.release_call_borrows(buf, call.args, call.modes)?;
        if let Some(place) = call.destination {
            self.store_call_result(buf, place)?;
        } else {
            let signature = hook.signature();
            match signature.results.len() {
                0 => {}
                1 => emit_instruction(buf, Op::Drop),
                n => {
                    for _ in 0..n {
                        emit_instruction(buf, Op::Drop);
                    }
                }
            }
        }
        self.emit_goto(buf, call.target);
        Ok(true)
    }
}
