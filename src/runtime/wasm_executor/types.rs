use std::convert::TryFrom;

use super::errors::WasmExecutionError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    I32,
    I64,
    F32,
    F64,
}

impl ValueType {
    pub fn default_value(self) -> Value {
        match self {
            ValueType::I32 => Value::I32(0),
            ValueType::I64 => Value::I64(0),
            ValueType::F32 => Value::F32(0.0),
            ValueType::F64 => Value::F64(0.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FuncType {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

#[derive(Debug, Clone, Copy)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl Value {
    pub fn as_i32(self) -> Result<i32, WasmExecutionError> {
        match self {
            Value::I32(v) => Ok(v),
            Value::I64(v) => Ok(v as i32),
            _ => Err(WasmExecutionError {
                message: "expected i32 value".into(),
            }),
        }
    }

    pub fn as_i64(self) -> Result<i64, WasmExecutionError> {
        match self {
            Value::I64(v) => Ok(v),
            Value::I32(v) => Ok(i64::from(v)),
            _ => Err(WasmExecutionError {
                message: "expected i64 value".into(),
            }),
        }
    }

    pub fn as_f32(self) -> Result<f32, WasmExecutionError> {
        match self {
            Value::F32(v) => Ok(v),
            Value::F64(v) => Ok(v as f32),
            _ => Err(WasmExecutionError {
                message: "expected f32 value".into(),
            }),
        }
    }

    pub fn as_f64(self) -> Result<f64, WasmExecutionError> {
        match self {
            Value::F64(v) => Ok(v),
            Value::F32(v) => Ok(f64::from(v)),
            _ => Err(WasmExecutionError {
                message: "expected f64 value".into(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl From<WasmValue> for Value {
    fn from(value: WasmValue) -> Self {
        match value {
            WasmValue::I32(v) => Value::I32(v),
            WasmValue::I64(v) => Value::I64(v),
            WasmValue::F32(v) => Value::F32(v),
            WasmValue::F64(v) => Value::F64(v),
        }
    }
}

impl From<Value> for WasmValue {
    fn from(value: Value) -> Self {
        match value {
            Value::I32(v) => WasmValue::I32(v),
            Value::I64(v) => WasmValue::I64(v),
            Value::F32(v) => WasmValue::F32(v),
            Value::F64(v) => WasmValue::F64(v),
        }
    }
}

impl WasmValue {
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Wasm numeric conversions intentionally truncate toward zero."
    )]
    pub fn as_i32(self) -> Option<i32> {
        match self {
            WasmValue::I32(v) => Some(v),
            WasmValue::I64(v) => i32::try_from(v).ok(),
            WasmValue::F32(v) => Some(v as i32),
            WasmValue::F64(v) => Some(v as i32),
        }
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Wasm numeric conversions intentionally truncate toward zero."
    )]
    pub fn as_i64(self) -> Option<i64> {
        match self {
            WasmValue::I32(v) => Some(i64::from(v)),
            WasmValue::I64(v) => Some(v),
            WasmValue::F32(v) => Some(v as i64),
            WasmValue::F64(v) => Some(v as i64),
        }
    }

    #[must_use]
    pub fn as_bool(self) -> Option<bool> {
        self.as_i32().map(|v| v != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_value_matches_variant() {
        let i32_default = ValueType::I32.default_value();
        let i64_default = ValueType::I64.default_value();
        let f32_default = ValueType::F32.default_value();
        let f64_default = ValueType::F64.default_value();

        assert!(
            matches!(i32_default, Value::I32(0)),
            "unexpected value: {i32_default:?}"
        );
        assert!(
            matches!(i64_default, Value::I64(0)),
            "unexpected value: {i64_default:?}"
        );
        assert!(
            matches!(f32_default, Value::F32(0.0)),
            "unexpected value: {f32_default:?}"
        );
        assert!(
            matches!(f64_default, Value::F64(0.0)),
            "unexpected value: {f64_default:?}"
        );
    }

    #[test]
    fn value_as_i32_success_and_failure() {
        let result = Value::I32(42).as_i32();
        let value = match result {
            Ok(num) => num,
            Err(err) => panic!("expected Ok result, found Err: {err:?}"),
        };
        assert_eq!(value, 42);

        let result_err = Value::F64(1.5).as_i32();
        match result_err {
            Ok(num) => panic!("expected Err, found Ok: {num}"),
            Err(err) => assert!(err.message.contains("expected i32")),
        }
    }

    #[test]
    fn wasm_value_integer_views() {
        assert_eq!(WasmValue::I32(5).as_i32(), Some(5));
        assert_eq!(WasmValue::I64(7).as_i32(), Some(7));
        assert_eq!(WasmValue::I64(i64::from(i32::MAX) + 1).as_i32(), None);
        assert_eq!(WasmValue::F32(2.5).as_i32(), Some(2));
        assert_eq!(WasmValue::F64(-3.4).as_i32(), Some(-3));

        assert_eq!(WasmValue::I32(9).as_i64(), Some(9));
        assert_eq!(WasmValue::I64(-11).as_i64(), Some(-11));
        assert_eq!(WasmValue::F32(6.7).as_i64(), Some(6));
        assert_eq!(WasmValue::F64(-8.2).as_i64(), Some(-8));
    }

    #[test]
    fn wasm_value_boolean_view() {
        assert_eq!(WasmValue::I32(0).as_bool(), Some(false));
        assert_eq!(WasmValue::I32(12).as_bool(), Some(true));
        assert_eq!(WasmValue::F64(0.0).as_bool(), Some(false));
    }
}
