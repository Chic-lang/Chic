use super::*;
use crate::mir::builder::support::resolve_type_layout_name;
use crate::syntax::expr::NameOfOperand;
use std::collections::HashSet;

body_builder_impl! {
    pub(super) fn lower_nameof_expr(
        &mut self,
        operand: NameOfOperand,
        span: Option<Span>,
            ) -> Option<Operand> {
        match self.resolve_nameof_operand(&operand) {
            Ok(name) => {
                let value = self.normalise_const(ConstValue::RawStr(name), span);
                Some(Operand::Const(ConstOperand::new(value)))
            }
            Err(message) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message,
                    span: operand.span.or(span),
                                    });
                None
            }
        }
    }

    fn resolve_nameof_operand(&self, operand: &NameOfOperand) -> Result<String, String> {
        if operand.segments.is_empty() {
            return Err("`nameof` requires an operand".to_string());
        }

        let display = operand.display().to_string();

        if operand.segments.len() == 1 {
            let name = &operand.segments[0];
            if self.lookup_name(name).is_some() {
                return Ok(name.clone());
            }

            if let Some(current_type) = self.current_type_context() {
                if self.symbol_index.has_field(current_type, name) {
                    return Ok(name.clone());
                }
                if self.property_exists(current_type, name) {
                    return Ok(name.clone());
                }
                if let Some(count) = self.symbol_index.method_count(current_type, name) {
                    return Self::method_count_result(name, count, &display);
                }
                if self.symbol_index.has_enum_variant(current_type, name) {
                    return Ok(name.clone());
                }
            }

            if let Some(type_name) = self.resolve_type_segments(&operand.segments) {
                return Ok(simple_name(&type_name).to_string());
            }

            if let Some((func_name, count)) = self.resolve_function_segments(&operand.segments) {
                return Self::method_count_result(&func_name, count, &display);
            }

            return Err(format!("cannot resolve symbol `{display}` for `nameof`"));
        }

        if let Some(type_name) = self.resolve_type_segments(&operand.segments) {
            return Ok(simple_name(&type_name).to_string());
        }

        let (base, member) = operand.segments.split_at(operand.segments.len() - 1);
        let member_name = &member[0];

        if let Some(type_name) = self.resolve_type_segments(base) {
            if self.symbol_index.has_field(&type_name, member_name) {
                return Ok(member_name.clone());
            }
            if self.symbol_index.has_enum_variant(&type_name, member_name) {
                return Ok(member_name.clone());
            }
            if self.property_exists(&type_name, member_name) {
                return Ok(member_name.clone());
            }
            if let Some(count) = self.symbol_index.method_count(&type_name, member_name) {
                return Self::method_count_result(member_name, count, &display);
            }
            return Err(format!(
                "type `{}` does not contain member `{}` referenced by `nameof` operand `{}`",
                type_name,
                member_name,
                display,
            ));
        }

        if let Some((func_name, count)) = self.resolve_function_segments(&operand.segments) {
            return Self::method_count_result(&func_name, count, &display);
        }

        Err(format!("cannot resolve symbol `{display}` for `nameof`"))
    }

    fn resolve_type_segments(&self, segments: &[String]) -> Option<String> {
        if segments.is_empty() {
            return None;
        }

        let namespace = self.namespace.as_deref();
        for candidate in candidate_names(namespace, segments) {
            if self.type_layouts.types.contains_key(&candidate)
                || self.symbol_index.type_names().contains(&candidate)
            {
                return Some(candidate);
            }
        }

        if segments.len() == 1 {
            let name = &segments[0];
            if self
                .primitive_registry
                .descriptor_for_name(name)
                .is_some()
            {
                return Some(name.clone());
            }
            if let Some(namespace) = namespace {
                if let Some(resolved) = resolve_type_layout_name(
                    self.type_layouts,
                    Some(self.import_resolver),
                    Some(namespace),
                    None,
                    name,
                ) {
                    return Some(resolved);
                }
            }
        }

        None
    }

    fn resolve_function_segments(&self, segments: &[String]) -> Option<(String, usize)> {
        if segments.is_empty() {
            return None;
        }
        let namespace = self.namespace.as_deref();
        for candidate in candidate_names(namespace, segments) {
            if let Some(count) = self.symbol_index.function_count(&candidate) {
                let simple = segments.last().cloned().unwrap_or_default();
                return Some((simple, count));
            }
        }
        None
    }

    fn current_type_context(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    fn method_count_result(name: &str, count: usize, operand: &str) -> Result<String, String> {
        if count == 1 {
            Ok(name.to_string())
        } else {
            Err(format!(
                "`nameof` operand `{operand}` resolves to {count} overloads of `{name}`"
            ))
        }
    }

    fn property_exists(&self, type_name: &str, name: &str) -> bool {
        self.symbol_index.property(type_name, name).is_some()
    }
}

fn candidate_names(namespace: Option<&str>, segments: &[String]) -> Vec<String> {
    let joined = segments.join("::");
    let mut seen = HashSet::new();
    let mut results = Vec::new();

    push_candidate(&joined, &mut seen, &mut results);

    if let Some(ns) = namespace {
        let mut current = Some(ns);
        while let Some(prefix) = current {
            let candidate = format!("{prefix}::{joined}");
            push_candidate(&candidate, &mut seen, &mut results);
            current = prefix.rfind("::").map(|idx| &prefix[..idx]);
        }
    }

    results
}

fn push_candidate(candidate: &str, seen: &mut HashSet<String>, results: &mut Vec<String>) {
    if seen.insert(candidate.to_string()) {
        results.push(candidate.to_string());
    }
}

fn simple_name(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}
