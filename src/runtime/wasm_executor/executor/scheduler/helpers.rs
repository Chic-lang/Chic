use crate::runtime::wasm_executor::errors::WasmExecutionError;
use crate::runtime::wasm_executor::types::{Value, ValueType};
use std::convert::TryFrom;

pub(super) fn binary_i32<F>(
    stack: &mut Vec<Value>,
    constructor: fn(i32) -> Value,
    op: F,
) -> Result<(), WasmExecutionError>
where
    F: FnOnce(i32, i32) -> i32,
{
    let right = stack
        .pop()
        .ok_or_else(|| WasmExecutionError {
            message: "value stack underflow".into(),
        })?
        .as_i32()?;
    let left = stack
        .pop()
        .ok_or_else(|| WasmExecutionError {
            message: "value stack underflow".into(),
        })?
        .as_i32()?;
    stack.push(constructor(op(left, right)));
    Ok(())
}

pub(super) fn shift_amount(operand: i32) -> u32 {
    let masked = operand & 0x1F;
    u32::try_from(masked).unwrap_or_default()
}

pub(super) fn shift_amount_i64(operand: i64) -> u32 {
    let masked = operand & 0x3F;
    u32::try_from(masked).unwrap_or_default()
}

pub(super) fn binary_i64<F>(
    stack: &mut Vec<Value>,
    constructor: fn(i64) -> Value,
    op: F,
) -> Result<(), WasmExecutionError>
where
    F: FnOnce(i64, i64) -> i64,
{
    let right = stack
        .pop()
        .ok_or_else(|| WasmExecutionError {
            message: "value stack underflow".into(),
        })?
        .as_i64()?;
    let left = stack
        .pop()
        .ok_or_else(|| WasmExecutionError {
            message: "value stack underflow".into(),
        })?
        .as_i64()?;
    stack.push(constructor(op(left, right)));
    Ok(())
}

pub(super) fn pop_address(
    stack: &mut Vec<Value>,
    context: &str,
) -> Result<u32, WasmExecutionError> {
    let value = stack.pop().ok_or_else(|| WasmExecutionError {
        message: format!("value stack underflow on {context}"),
    })?;
    let addr = value.as_i32()?;
    u32::try_from(addr).map_err(|_| WasmExecutionError {
        message: format!("negative memory address on {context} (raw={value:?})"),
    })
}

pub(super) fn value_matches_type(value: Value, ty: ValueType) -> bool {
    match (value, ty) {
        (Value::I32(_), ValueType::I32)
        | (Value::I64(_), ValueType::I64)
        | (Value::F32(_), ValueType::F32)
        | (Value::F64(_), ValueType::F64) => true,
        _ => false,
    }
}
