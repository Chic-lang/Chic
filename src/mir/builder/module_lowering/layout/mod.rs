mod auto_traits;
mod enums;
mod mmio;
mod structs;
mod unions;

use std::collections::HashSet;

use super::super::Visibility;
use super::driver::{LoweringDiagnostic, ModuleLowering, TypeDeclInfo, visibility_keyword};
use crate::frontend::diagnostics::Span;

impl ModuleLowering {
    pub(super) fn record_type_visibility(
        &mut self,
        name: &str,
        visibility: Visibility,
        namespace: Option<&str>,
        enclosing_type: Option<&str>,
    ) {
        self.type_visibilities
            .entry(name.to_string())
            .or_insert(TypeDeclInfo {
                visibility,
                namespace: namespace.map(ToOwned::to_owned),
                enclosing_type: enclosing_type.map(ToOwned::to_owned),
                package: None,
            });
    }

    fn ensure_type_accessible_resolved(
        &mut self,
        ty_name: &str,
        namespace: Option<&str>,
        context_type: Option<&str>,
        usage: &str,
        span: Option<Span>,
    ) {
        let Some(info) = self.type_visibilities.get(ty_name) else {
            return;
        };
        if self.is_type_accessible(namespace, context_type, ty_name, info) {
            return;
        }
        let message = format!(
            "{usage} references inaccessible type `{ty_name}` ({})",
            visibility_keyword(info.visibility)
        );
        self.diagnostics.push(LoweringDiagnostic { message, span });
    }

    fn is_type_accessible(
        &self,
        namespace: Option<&str>,
        context_type: Option<&str>,
        target_name: &str,
        info: &TypeDeclInfo,
    ) -> bool {
        match info.visibility {
            Visibility::Public => true,
            Visibility::Internal => true,
            Visibility::Protected => self.is_protected_access(context_type, target_name, info),
            Visibility::Private => {
                if let Some(enclosing) = &info.enclosing_type {
                    context_type.is_some_and(|ctx| ctx == enclosing)
                } else {
                    context_type.is_some_and(|ctx| ctx == target_name)
                }
            }
            Visibility::ProtectedInternal => {
                self.is_internal_access(namespace, info.namespace.as_deref())
                    || self.is_protected_access(context_type, target_name, info)
            }
            Visibility::PrivateProtected => {
                self.is_internal_access(namespace, info.namespace.as_deref())
                    && self.is_protected_access(context_type, target_name, info)
            }
        }
    }

    fn is_internal_access(&self, _namespace: Option<&str>, _target_ns: Option<&str>) -> bool {
        true
    }

    fn is_protected_access(
        &self,
        context_type: Option<&str>,
        target_name: &str,
        info: &TypeDeclInfo,
    ) -> bool {
        if context_type.is_some_and(|ctx| ctx == target_name) {
            return true;
        }
        let base = info.enclosing_type.as_deref().unwrap_or(target_name);
        self.is_derived_from(context_type, base)
    }

    fn is_derived_from(&self, context_type: Option<&str>, base: &str) -> bool {
        let Some(root) = context_type else {
            return false;
        };
        if root == base {
            return true;
        }

        let mut visited = HashSet::new();
        let mut stack = vec![root.to_string()];

        while let Some(candidate) = stack.pop() {
            if candidate == base {
                return true;
            }
            if !visited.insert(candidate.clone()) {
                continue;
            }
            if let Some(parents) = self.class_bases.get(&candidate) {
                for parent in parents {
                    if !visited.contains(parent) {
                        stack.push(parent.clone());
                    }
                }
            }
        }

        false
    }
}
