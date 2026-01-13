use super::arena::{FunctionSignature, OperatorSignatureInfo, SignatureId, TypeChecker};
use super::diagnostics::codes;
use super::helpers::base_type_name;
use crate::frontend::ast::{
    BinaryOperator, ConversionKind, FunctionDecl, OperatorKind, UnaryOperator, Visibility,
};
use std::collections::HashMap;

impl<'a> TypeChecker<'a> {
    pub(super) fn validate_operator(&mut self, owner: &str, method: &FunctionDecl) {
        let Some(operator) = method.operator.as_ref() else {
            return;
        };
        let span = method.body.as_ref().and_then(|body| body.span);
        if method.visibility != Visibility::Public {
            self.emit_error(
                codes::OPERATOR_SIGNATURE_INVALID,
                span,
                format!(
                    "{} in `{owner}` must be declared `public`",
                    operator_display(&operator.kind)
                ),
            );
        }
        let param_count = method.signature.parameters.len();
        match operator.kind {
            OperatorKind::Unary(_) | OperatorKind::Conversion(_) if param_count != 1 => {
                self.emit_error(
                    codes::OPERATOR_SIGNATURE_INVALID,
                    span,
                    format!(
                        "{} in `{owner}` must declare exactly one parameter",
                        operator_display(&operator.kind)
                    ),
                );
            }
            OperatorKind::Binary(_) if param_count != 2 => {
                self.emit_error(
                    codes::OPERATOR_SIGNATURE_INVALID,
                    span,
                    format!(
                        "{} in `{owner}` must declare exactly two parameters",
                        operator_display(&operator.kind)
                    ),
                );
            }
            _ => {}
        }

        if matches!(operator.kind, OperatorKind::Conversion(_))
            && method.signature.return_type.name == "void"
        {
            self.emit_error(
                codes::OPERATOR_SIGNATURE_INVALID,
                span,
                format!(
                    "{} in `{owner}` cannot return `void`",
                    operator_display(&operator.kind)
                ),
            );
        }

        if method.is_async {
            self.emit_error(
                codes::OPERATOR_SIGNATURE_INVALID,
                span,
                format!(
                    "{} in `{owner}` cannot be marked async",
                    operator_display(&operator.kind)
                ),
            );
        }
        if let OperatorKind::Binary(op) = operator.kind {
            if matches!(
                op,
                BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::LessThan
                    | BinaryOperator::LessThanOrEqual
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::GreaterThanOrEqual
            ) {
                let return_base =
                    base_type_name(&method.signature.return_type.name).to_ascii_lowercase();
                if return_base != "bool" {
                    self.emit_error(
                        codes::OPERATOR_SIGNATURE_INVALID,
                        span,
                        format!(
                            "{} in `{owner}` must return `bool`",
                            operator_display(&operator.kind)
                        ),
                    );
                }
            }
        }

        self.operator_signatures
            .entry(owner.to_string())
            .or_default()
            .push(OperatorSignatureInfo {
                kind: operator.kind.clone(),
                return_type: method.signature.return_type.name.clone(),
                span,
            });

        let owner_short = short_type_fragment(owner);

        match operator.kind {
            OperatorKind::Unary(_) if param_count == 1 => {
                let operand_matches = method
                    .signature
                    .parameters
                    .first()
                    .is_some_and(|param| matches_owner_type_name(&param.ty.name, owner));
                if !operand_matches {
                    self.emit_error(
                        codes::OPERATOR_SIGNATURE_INVALID,
                        span,
                        format!(
                            "{} in `{owner}` must take `{owner_short}` as its operand",
                            operator_display(&operator.kind)
                        ),
                    );
                }
            }
            OperatorKind::Binary(_) if param_count == 2 => {
                let owns_param = method
                    .signature
                    .parameters
                    .iter()
                    .any(|param| matches_owner_type_name(&param.ty.name, owner));
                if !owns_param {
                    self.emit_error(
                        codes::OPERATOR_SIGNATURE_INVALID,
                        span,
                                                format!(
                            "{} in `{owner}` must have at least one parameter of type `{owner_short}`",
                            operator_display(&operator.kind)
                        ),
                    );
                }
            }
            OperatorKind::Conversion(_) if param_count == 1 => {
                let source_matches = method
                    .signature
                    .parameters
                    .first()
                    .is_some_and(|param| matches_owner_type_name(&param.ty.name, owner));
                let return_matches =
                    matches_owner_type_name(&method.signature.return_type.name, owner);
                if !(source_matches || return_matches) {
                    self.emit_error(
                        codes::OPERATOR_SIGNATURE_INVALID,
                        span,
                        format!(
                            "{} in `{owner}` must convert to or from `{owner_short}`",
                            operator_display(&operator.kind)
                        ),
                    );
                }
            }
            _ => {}
        }
    }

    pub(super) fn check_overloads(&mut self) {
        let function_keys: Vec<String> = self.functions.keys().cloned().collect();
        for name in function_keys {
            if let Some(ids) = self.functions.get(&name).cloned() {
                self.check_signature_conflicts(&name, &ids);
            }
        }
        let method_keys: Vec<String> = self.methods.keys().cloned().collect();
        for ty in method_keys {
            if let Some(ids) = self.methods.get(&ty).cloned() {
                self.check_signature_conflicts(&ty, &ids);
            }
        }
        self.check_operator_pairs();
    }

    pub(super) fn check_signature_conflicts(&mut self, scope: &str, signatures: &[SignatureId]) {
        let mut seen: HashMap<(String, Vec<String>, usize), SignatureId> = HashMap::new();
        for &id in signatures {
            let sig = self.signatures.get(id);
            let key = (
                sig.name.clone(),
                sig.param_types.clone(),
                self.signature_generic_arity(sig, id),
            );
            if let Some(existing_id) = seen.get(&key) {
                let existing = self.signatures.get(*existing_id);
                self.emit_error(
                    codes::OVERLOAD_CONFLICT,
                    sig.span.or(existing.span),
                    format!(
                        "duplicate overload `{}` with parameter types {:?} in `{scope}`",
                        sig.name, sig.param_types
                    ),
                );
            } else {
                seen.insert(key, id);
            }
        }
    }

    pub(super) fn check_operator_pairs(&mut self) {
        let operator_sets: Vec<(String, Vec<OperatorSignatureInfo>)> = self
            .operator_signatures
            .iter()
            .map(|(owner, ops)| (owner.clone(), ops.clone()))
            .collect();

        for (owner, operators) in operator_sets {
            let mut binary_ops: HashMap<BinaryOperator, &OperatorSignatureInfo> = HashMap::new();
            for info in &operators {
                if let OperatorKind::Binary(kind) = info.kind {
                    binary_ops.insert(kind, info);
                }
            }

            let mut require_pair = |first: BinaryOperator, second: BinaryOperator| {
                if binary_ops.contains_key(&first) && !binary_ops.contains_key(&second) {
                    let first_display = operator_display(&OperatorKind::Binary(first)).to_string();
                    let second_display =
                        operator_display(&OperatorKind::Binary(second)).to_string();
                    let span = binary_ops.get(&first).and_then(|info| info.span);
                    self.emit_error(
                        codes::OPERATOR_SIGNATURE_INVALID,
                        span,
                        format!(
                            "{first_display} in `{owner}` requires a matching {second_display}"
                        ),
                    );
                }
            };

            require_pair(BinaryOperator::Equal, BinaryOperator::NotEqual);
            require_pair(BinaryOperator::NotEqual, BinaryOperator::Equal);
            require_pair(BinaryOperator::LessThan, BinaryOperator::GreaterThan);
            require_pair(BinaryOperator::GreaterThan, BinaryOperator::LessThan);
            require_pair(
                BinaryOperator::LessThanOrEqual,
                BinaryOperator::GreaterThanOrEqual,
            );
            require_pair(
                BinaryOperator::GreaterThanOrEqual,
                BinaryOperator::LessThanOrEqual,
            );
        }
    }

    pub(super) fn signature_generic_arity(
        &self,
        signature: &FunctionSignature,
        id: SignatureId,
    ) -> usize {
        self.signature_generics
            .get(&id)
            .map(|params| params.len())
            .or_else(|| {
                self.function_generics
                    .get(&signature.name)
                    .map(|params| params.len())
            })
            .unwrap_or(0)
    }
}

fn short_type_fragment(name: &str) -> &str {
    name.split([':', '.'])
        .filter(|segment| !segment.is_empty())
        .next_back()
        .unwrap_or(name)
}

fn matches_owner_type_name(ty: &str, owner: &str) -> bool {
    let ty = strip_nullable_suffix(ty);
    if ty == "Self" {
        return true;
    }
    let ty_base = base_type_name(ty);
    if ty_base == owner || ty == owner {
        return true;
    }
    let owner_short = short_type_fragment(owner);
    let ty_short = short_type_fragment(ty_base);
    ty_short == owner_short
}

fn strip_nullable_suffix(ty: &str) -> &str {
    ty.strip_suffix('?').unwrap_or(ty)
}

fn operator_display(kind: &OperatorKind) -> &'static str {
    match kind {
        OperatorKind::Unary(op) => match op {
            UnaryOperator::Negate => "operator -",
            UnaryOperator::UnaryPlus => "operator +",
            UnaryOperator::LogicalNot => "operator !",
            UnaryOperator::OnesComplement => "operator ~",
            UnaryOperator::Increment => "operator ++",
            UnaryOperator::Decrement => "operator --",
        },
        OperatorKind::Binary(op) => match op {
            BinaryOperator::Add => "operator +",
            BinaryOperator::Subtract => "operator -",
            BinaryOperator::Multiply => "operator *",
            BinaryOperator::Divide => "operator /",
            BinaryOperator::Remainder => "operator %",
            BinaryOperator::BitAnd => "operator &",
            BinaryOperator::BitOr => "operator |",
            BinaryOperator::BitXor => "operator ^",
            BinaryOperator::ShiftLeft => "operator <<",
            BinaryOperator::ShiftRight => "operator >>",
            BinaryOperator::Equal => "operator ==",
            BinaryOperator::NotEqual => "operator !=",
            BinaryOperator::LessThan => "operator <",
            BinaryOperator::LessThanOrEqual => "operator <=",
            BinaryOperator::GreaterThan => "operator >",
            BinaryOperator::GreaterThanOrEqual => "operator >=",
        },
        OperatorKind::Conversion(ConversionKind::Implicit) => "implicit operator",
        OperatorKind::Conversion(ConversionKind::Explicit) => "explicit operator",
    }
}
