use super::*;

impl<'a> Executor<'a> {
    pub(super) fn invoke_int128_import(
        &mut self,
        name: &str,
        params: &[Value],
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "i128_add" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_add expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_add out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_add lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_add rhs")?;
                let result = self.read_i128(lhs)?.wrapping_add(self.read_i128(rhs)?);
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_sub" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_sub expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_sub out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_sub lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_sub rhs")?;
                let result = self.read_i128(lhs)?.wrapping_sub(self.read_i128(rhs)?);
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_mul" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_mul expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_mul out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_mul lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_mul rhs")?;
                let result = self.read_i128(lhs)?.wrapping_mul(self.read_i128(rhs)?);
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_div" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_div expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_div out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_div lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_div rhs")?;
                let rhs_value = self.read_i128(rhs)?;
                let Some(result) = self.read_i128(lhs)?.checked_div(rhs_value) else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_div division error".into(),
                    });
                };
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_rem" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_rem expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_rem out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_rem lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_rem rhs")?;
                let rhs_value = self.read_i128(rhs)?;
                let Some(result) = self.read_i128(lhs)?.checked_rem(rhs_value) else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_rem division error".into(),
                    });
                };
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_eq" => {
                let [Value::I32(lhs_ptr), Value::I32(rhs_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_eq expects (i32, i32) arguments".into(),
                    });
                };
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_eq lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_eq rhs")?;
                let result = i32::from(self.read_i128(lhs)? == self.read_i128(rhs)?);
                Ok(Some(Value::I32(result)))
            }
            "i128_cmp" => {
                let [Value::I32(lhs_ptr), Value::I32(rhs_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_cmp expects (i32, i32) arguments".into(),
                    });
                };
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_cmp lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_cmp rhs")?;
                let lhs_value = self.read_i128(lhs)?;
                let rhs_value = self.read_i128(rhs)?;
                let result = match lhs_value.cmp(&rhs_value) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                Ok(Some(Value::I32(result)))
            }
            "i128_neg" => {
                let [Value::I32(out_ptr), Value::I32(value_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_neg expects (i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_neg out")?;
                let value = self.checked_ptr(*value_ptr, "chic_rt.i128_neg value")?;
                let result = self.read_i128(value)?.wrapping_neg();
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_not" => {
                let [Value::I32(out_ptr), Value::I32(value_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_not expects (i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_not out")?;
                let value = self.checked_ptr(*value_ptr, "chic_rt.i128_not value")?;
                let result = !self.read_i128(value)?;
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_and" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_and expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_and out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_and lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_and rhs")?;
                let result = self.read_i128(lhs)? & self.read_i128(rhs)?;
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_or" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_or expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_or out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_or lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_or rhs")?;
                let result = self.read_i128(lhs)? | self.read_i128(rhs)?;
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_xor" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_xor expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_xor out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_xor lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_xor rhs")?;
                let result = self.read_i128(lhs)? ^ self.read_i128(rhs)?;
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_shl" => {
                let [Value::I32(out_ptr), Value::I32(lhs_ptr), Value::I32(amount)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_shl expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_shl out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_shl lhs")?;
                let shift = *amount as u32;
                let result = self.read_i128(lhs)?.wrapping_shl(shift);
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_shr" => {
                let [Value::I32(out_ptr), Value::I32(lhs_ptr), Value::I32(amount)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_shr expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_shr out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_shr lhs")?;
                let shift = *amount as u32;
                let result = self.read_i128(lhs)?.wrapping_shr(shift);
                self.write_i128(out, result)?;
                Ok(None)
            }
            "u128_add" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_add expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_add out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_add lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_add rhs")?;
                let result = self.read_u128(lhs)?.wrapping_add(self.read_u128(rhs)?);
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_sub" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_sub expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_sub out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_sub lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_sub rhs")?;
                let result = self.read_u128(lhs)?.wrapping_sub(self.read_u128(rhs)?);
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_mul" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_mul expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_mul out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_mul lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_mul rhs")?;
                let result = self.read_u128(lhs)?.wrapping_mul(self.read_u128(rhs)?);
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_div" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_div expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_div out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_div lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_div rhs")?;
                let rhs_value = self.read_u128(rhs)?;
                let Some(result) = self.read_u128(lhs)?.checked_div(rhs_value) else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_div division error".into(),
                    });
                };
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_rem" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_rem expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_rem out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_rem lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_rem rhs")?;
                let rhs_value = self.read_u128(rhs)?;
                let Some(result) = self.read_u128(lhs)?.checked_rem(rhs_value) else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_rem division error".into(),
                    });
                };
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_eq" => {
                let [Value::I32(lhs_ptr), Value::I32(rhs_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_eq expects (i32, i32) arguments".into(),
                    });
                };
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_eq lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_eq rhs")?;
                let lhs_value = self.read_u128(lhs)?;
                let rhs_value = self.read_u128(rhs)?;
                let result = i32::from(lhs_value == rhs_value);
                if std::env::var_os("CHIC_DEBUG_WASM_U128").is_some() {
                    eprintln!(
                        "[wasm-u128] eq lhs=0x{lhs_value:032x} rhs=0x{rhs_value:032x} -> {result} caller={}",
                        self.current_wasm_context()
                    );
                }
                Ok(Some(Value::I32(result)))
            }
            "u128_cmp" => {
                let [Value::I32(lhs_ptr), Value::I32(rhs_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_cmp expects (i32, i32) arguments".into(),
                    });
                };
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_cmp lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_cmp rhs")?;
                let lhs_value = self.read_u128(lhs)?;
                let rhs_value = self.read_u128(rhs)?;
                let result = match lhs_value.cmp(&rhs_value) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                Ok(Some(Value::I32(result)))
            }
            "u128_not" => {
                let [Value::I32(out_ptr), Value::I32(value_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_not expects (i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_not out")?;
                let value = self.checked_ptr(*value_ptr, "chic_rt.u128_not value")?;
                let result = !self.read_u128(value)?;
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_and" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_and expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_and out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_and lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_and rhs")?;
                let lhs_value = self.read_u128(lhs)?;
                let rhs_value = self.read_u128(rhs)?;
                let result = lhs_value & rhs_value;
                if std::env::var_os("CHIC_DEBUG_WASM_U128").is_some() {
                    eprintln!(
                        "[wasm-u128] and lhs=0x{lhs_value:032x} rhs=0x{rhs_value:032x} -> 0x{result:032x} caller={}",
                        self.current_wasm_context()
                    );
                }
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_or" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_or expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_or out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_or lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_or rhs")?;
                let result = self.read_u128(lhs)? | self.read_u128(rhs)?;
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_xor" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_xor expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_xor out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_xor lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_xor rhs")?;
                let result = self.read_u128(lhs)? ^ self.read_u128(rhs)?;
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_shl" => {
                let [Value::I32(out_ptr), Value::I32(lhs_ptr), Value::I32(amount)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_shl expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_shl out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_shl lhs")?;
                let shift = *amount as u32;
                let lhs_value = self.read_u128(lhs)?;
                let result = lhs_value.wrapping_shl(shift);
                if std::env::var_os("CHIC_DEBUG_WASM_U128").is_some() {
                    eprintln!(
                        "[wasm-u128] shl lhs=0x{lhs_value:032x} shift={shift} -> 0x{result:032x} caller={}",
                        self.current_wasm_context()
                    );
                }
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_shr" => {
                let [Value::I32(out_ptr), Value::I32(lhs_ptr), Value::I32(amount)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_shr expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_shr out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_shr lhs")?;
                let shift = *amount as u32;
                let lhs_value = self.read_u128(lhs)?;
                let result = lhs_value.wrapping_shr(shift);
                if std::env::var_os("CHIC_DEBUG_WASM_U128").is_some() {
                    eprintln!(
                        "[wasm-u128] shr lhs=0x{lhs_value:032x} shift={shift} -> 0x{result:032x} caller={}",
                        self.current_wasm_context()
                    );
                }
                self.write_u128(out, result)?;
                Ok(None)
            }
            _ => Err(WasmExecutionError {
                message: format!("unsupported chic_rt int128 import `{name}`"),
            }),
        }
    }

    fn checked_ptr(&self, value: i32, context: &str) -> Result<u32, WasmExecutionError> {
        u32::try_from(value).map_err(|_| WasmExecutionError {
            message: format!("{context} received negative pointer"),
        })
    }

    fn read_u128(&self, ptr: u32) -> Result<u128, WasmExecutionError> {
        let lo = self.read_u64(ptr)?;
        let hi = self.read_u64(ptr + 8)?;
        Ok((u128::from(hi) << 64) | u128::from(lo))
    }

    fn write_u128(&mut self, ptr: u32, value: u128) -> Result<(), WasmExecutionError> {
        let lo = value as u64;
        let hi = (value >> 64) as u64;
        self.write_u64(ptr, lo)?;
        self.write_u64(ptr + 8, hi)?;
        Ok(())
    }
}
