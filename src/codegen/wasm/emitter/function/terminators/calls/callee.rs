use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn resolve_callee(&self, operand: &Operand, args: &[Operand]) -> Result<u32, Error> {
        match operand {
            Operand::Pending(pending) => {
                if let Some(info) = &pending.info {
                    let PendingOperandInfo::FunctionGroup { candidates, .. } = info.as_ref();
                    let arg_types: Vec<Option<ValueType>> = args
                        .iter()
                        .map(|arg| match arg {
                            Operand::Const(constant) => match constant.value() {
                                ConstValue::Bool(_) => Some(ValueType::I32),
                                ConstValue::Int(value) | ConstValue::Int32(value) => {
                                    let bits = constant
                                        .literal()
                                        .and_then(|meta| match meta.literal_type {
                                            NumericLiteralType::Signed(width)
                                            | NumericLiteralType::Unsigned(width) => {
                                                Some(width.bit_width(self.pointer_width_bits()))
                                            }
                                            _ => None,
                                        })
                                        .unwrap_or_else(|| {
                                            if *value >= i32::MIN as i128
                                                && *value <= i32::MAX as i128
                                            {
                                                32
                                            } else {
                                                64
                                            }
                                        });
                                    Some(if bits <= 32 {
                                        ValueType::I32
                                    } else {
                                        ValueType::I64
                                    })
                                }
                                ConstValue::UInt(value) => {
                                    let bits = constant
                                        .literal()
                                        .and_then(|meta| match meta.literal_type {
                                            NumericLiteralType::Unsigned(width)
                                            | NumericLiteralType::Signed(width) => {
                                                Some(width.bit_width(self.pointer_width_bits()))
                                            }
                                            _ => None,
                                        })
                                        .unwrap_or_else(|| {
                                            if *value <= u32::MAX as u128 { 32 } else { 64 }
                                        });
                                    Some(if bits <= 32 {
                                        ValueType::I32
                                    } else {
                                        ValueType::I64
                                    })
                                }
                                ConstValue::Char(_) => Some(ValueType::I32),
                                ConstValue::Float(value) => Some(if value.width.bits() <= 32 {
                                    ValueType::F32
                                } else {
                                    ValueType::F64
                                }),
                                ConstValue::Null => Some(ValueType::I32),
                                ConstValue::Str { .. } => Some(ValueType::I64),
                                ConstValue::RawStr(_) => Some(ValueType::I32),
                                ConstValue::Unit => Some(ValueType::I32),
                                _ => None,
                            },
                            other => self
                                .operand_ty(other)
                                .map(|ty| map_type(&self.resolve_self_ty(&ty))),
                        })
                        .collect();

                    let mut best: Option<(u32, usize)> = None;
                    for candidate in candidates {
                        let Some(index) = self.lookup_function_index(&candidate.qualified) else {
                            continue;
                        };
                        let canonical = candidate.qualified.replace('.', "::");
                        let signature = self.function_signatures.get(&canonical).or_else(|| {
                            canonical
                                .split('<')
                                .next()
                                .and_then(|base| self.function_signatures.get(base))
                        });
                        let Some(signature) = signature else {
                            continue;
                        };
                        let offset = if signature.params.len() == arg_types.len() {
                            0
                        } else if signature.params.len() == arg_types.len() + 1
                            && matches!(signature.params.first(), Some(ValueType::I32))
                            && signature.results.len() == 1
                            && signature.results[0] == ValueType::I32
                        {
                            1
                        } else {
                            continue;
                        };

                        let mut score = 0usize;
                        let mut mismatch = false;
                        for (idx, arg_ty) in arg_types.iter().enumerate() {
                            let Some(arg_ty) = arg_ty else {
                                continue;
                            };
                            let Some(expected) = signature.params.get(idx + offset) else {
                                mismatch = true;
                                break;
                            };
                            if *expected == *arg_ty {
                                score += 1;
                            } else {
                                mismatch = true;
                                break;
                            }
                        }
                        if mismatch {
                            continue;
                        }
                        if best.map_or(true, |(_, best_score)| score > best_score) {
                            best = Some((index, score));
                        }
                    }
                    if let Some((index, _)) = best {
                        return Ok(index);
                    }
                    if let Some(index) = candidates
                        .iter()
                        .find_map(|candidate| self.lookup_function_index(&candidate.qualified))
                    {
                        return Ok(index);
                    }
                }
                let repr = pending.repr.replace('.', "::");
                if !repr.ends_with("::init#super") && !repr.ends_with("::init#self") {
                    if let Some(idx) = self.lookup_function_index(&repr) {
                        return Ok(idx);
                    }
                }
                if repr.ends_with("::init#super") {
                    if let Some((owner, _)) = repr.rsplit_once("::init#super") {
                        if let Some(class) = self.layouts.class_layout_info(owner) {
                            if let Some(base) = class.bases.first() {
                                let base_key = base.replace('.', "::");
                                let canonical_base = self
                                    .layouts
                                    .resolve_type_key(base_key.as_str())
                                    .unwrap_or(base_key.as_str())
                                    .to_string();
                                let expected = args.len();
                                let matches_base = |name: &str| {
                                    name.starts_with(&format!("{base_key}::init#"))
                                        || name.starts_with(&format!("{canonical_base}::init#"))
                                };
                                if let Some((_, idx)) = self.functions.iter().find(|(name, _)| {
                                    matches_base(name)
                                        && self
                                            .function_signatures
                                            .get(*name)
                                            .map(|sig| sig.params.len() == expected)
                                            .unwrap_or(false)
                                }) {
                                    return Ok(*idx);
                                }
                                if let Some((_, idx)) =
                                    self.functions.iter().find(|(name, _)| matches_base(name))
                                {
                                    return Ok(*idx);
                                }
                                if let Some(idx) =
                                    self.functions.get(&format!("{canonical_base}::init"))
                                {
                                    return Ok(*idx);
                                }
                            }
                        }
                    }
                }
                if repr.ends_with("::init#self") {
                    if let Some((owner, _)) = repr.rsplit_once("::init#self") {
                        let expected = args.len();
                        if let Some((_, idx)) = self.functions.iter().find(|(name, _)| {
                            name.starts_with(&format!("{owner}::init#"))
                                && self
                                    .function_signatures
                                    .get(*name)
                                    .map(|sig| sig.params.len() == expected)
                                    .unwrap_or(false)
                        }) {
                            return Ok(*idx);
                        }
                        if let Some((_, idx)) = self
                            .functions
                            .iter()
                            .find(|(name, _)| name.starts_with(&format!("{owner}::init#")))
                        {
                            return Ok(*idx);
                        }
                    }
                }
                if let Some(idx) = self.functions.iter().find_map(|(name, index)| {
                    if name == &repr || name.ends_with(&format!("::{repr}")) {
                        Some(*index)
                    } else {
                        None
                    }
                }) {
                    Ok(idx)
                } else if let Some(method) = repr.rsplit("::").next() {
                    if let Some(idx) = self.functions.iter().find_map(|(name, index)| {
                        if name.ends_with(&format!("::{method}")) {
                            Some(*index)
                        } else {
                            None
                        }
                    }) {
                        Ok(idx)
                    } else {
                        Err(Error::Codegen(format!(
                            "unable to resolve call target '{repr}' in WASM backend"
                        )))
                    }
                } else {
                    Err(Error::Codegen(format!(
                        "unable to resolve call target '{repr}' in WASM backend"
                    )))
                }
            }
            Operand::Copy(place) | Operand::Move(place) => {
                let ty = self
                    .local_tys
                    .get(place.local.0)
                    .map(|ty| ty.canonical_name())
                    .unwrap_or_else(|| "<unknown>".into());
                Err(Error::Codegen(format!(
                    "first-class function values are not yet supported by the WASM backend (local {} type {} projection {:?})",
                    place.local.0, ty, place.projection
                )))
            }
            Operand::Const(constant) => match &constant.value {
                ConstValue::Symbol(name) => {
                    if let Some(index) = self.lookup_function_index(name) {
                        Ok(index)
                    } else if name.contains("AsUtf8Span") || name.contains("AsUtf8") {
                        // Treat missing UTF-8 span helpers as runtime string slice accessors.
                        self.runtime_hook_index(RuntimeHook::StringAsSlice)
                    } else if name.contains("TryCopyUtf8") {
                        self.runtime_hook_index(RuntimeHook::StringTryCopyUtf8)
                    } else if name.contains("AsSpan") {
                        // Treat missing char-span helpers as runtime string UTF-16 views.
                        if name.starts_with("str::") || name.contains("::str::") {
                            self.runtime_hook_index(RuntimeHook::StrAsChars)
                        } else {
                            self.runtime_hook_index(RuntimeHook::StringAsChars)
                        }
                    } else {
                        Err(Error::Codegen(format!(
                            "unable to resolve function `{name}` in WASM backend"
                        )))
                    }
                }
                _ => Err(Error::Codegen(
                    "unsupported constant call operand for WASM backend".into(),
                )),
            },
            _ => Err(Error::Codegen(
                "unsupported call operand for WASM backend".into(),
            )),
        }
    }
}
