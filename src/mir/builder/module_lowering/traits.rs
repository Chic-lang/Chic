use super::super::functions::lower_function;
use super::super::{FunctionKind, Item, qualify};
use super::driver::{LoweringDiagnostic, ModuleLowering};
use crate::frontend::ast::Signature;
use crate::frontend::ast::{
    FunctionDecl, ImplDecl, ImplMember, InterfaceDecl, InterfaceMember, TraitDecl, TraitMember,
};
use crate::frontend::import_resolver::Resolution as ImportResolution;
use crate::frontend::type_utils::instantiate_extension_method;
use crate::mir::data::{TraitVTable, VTableSlot};
use crate::mir::trait_vtable_symbol_name;
use std::collections::HashMap;

impl ModuleLowering {
    pub(super) fn collect_traits(&mut self, items: &[Item], namespace: Option<&str>) {
        for item in items {
            match item {
                Item::Trait(trait_decl) => {
                    self.register_trait_decl(trait_decl, namespace);
                }
                Item::Interface(iface) => {
                    self.register_interface_stub(iface, namespace);
                }
                Item::Namespace(ns) => {
                    let nested = qualify(namespace, &ns.name);
                    self.collect_traits(&ns.items, Some(&nested));
                }
                _ => {}
            }
        }
    }

    fn register_trait_decl(&mut self, trait_decl: &TraitDecl, namespace: Option<&str>) {
        let trait_name = qualify(namespace, &trait_decl.name);
        self.record_type_visibility(&trait_name, trait_decl.visibility, namespace, None);
        let mut methods = Vec::new();
        for member in &trait_decl.members {
            if let TraitMember::Method(method) = member {
                methods.push(TraitMethodLoweringInfo {
                    name: method.name.clone(),
                    signature: method.signature.clone(),
                    default_impl: method.body.as_ref().map(|_| method.clone()),
                    is_async: method.is_async,
                });
            }
        }
        let info = TraitLoweringInfo { methods };
        self.trait_decls.insert(trait_name, info);
    }

    fn register_interface_stub(&mut self, iface: &InterfaceDecl, namespace: Option<&str>) {
        let iface_name = qualify(namespace, &iface.name);
        if self.trait_decls.contains_key(&iface_name) {
            return;
        }
        let mut methods = Vec::new();
        for member in &iface.members {
            if let InterfaceMember::Method(method) = member {
                methods.push(TraitMethodLoweringInfo {
                    name: method.name.clone(),
                    signature: method.signature.clone(),
                    default_impl: None,
                    is_async: method.is_async,
                });
            }
        }
        if !methods.is_empty() {
            self.trait_decls
                .insert(iface_name, TraitLoweringInfo { methods });
        }
    }

    pub(super) fn lower_impl(&mut self, impl_decl: &ImplDecl, namespace: Option<&str>) {
        if impl_decl.trait_ref.is_none() {
            self.diagnostics.push(LoweringDiagnostic {
                message: "inherent impl blocks are not supported during lowering".into(),
                span: impl_decl.span,
            });
            return;
        }
        self.lower_trait_impl(impl_decl, namespace);
    }

    fn lower_trait_impl(&mut self, impl_decl: &ImplDecl, namespace: Option<&str>) {
        let Some(trait_ref) = &impl_decl.trait_ref else {
            return;
        };
        let trait_name = match self.resolve_type_for_expr(trait_ref, namespace, None) {
            ImportResolution::Found(name) => name,
            ImportResolution::Ambiguous(candidates) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "trait `{}` resolves to multiple candidates: {}",
                        trait_ref.name,
                        candidates.join(", ")
                    ),
                    span: impl_decl.span,
                });
                return;
            }
            ImportResolution::NotFound => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!("trait `{}` not found", trait_ref.name),
                    span: impl_decl.span,
                });
                return;
            }
        };

        let Some(trait_info) = self.trait_decls.get(&trait_name).cloned() else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "trait `{}` has not been declared in this module",
                    trait_name
                ),
                span: impl_decl.span,
            });
            return;
        };

        let impl_name = qualify(namespace, &impl_decl.target.name);
        let mut provided = ImplMethodMap::new();
        for member in &impl_decl.members {
            if let ImplMember::Method(method) = member {
                provided.insert(method.name.clone(), method.clone());
            }
        }

        let mut slots = Vec::new();
        for method in &trait_info.methods {
            let method_decl = if let Some(impl_method) = provided.get(&method.name) {
                impl_method.clone()
            } else if let Some(default_impl) = &method.default_impl {
                instantiate_extension_method(default_impl, &impl_decl.target)
            } else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "impl for `{impl_name}` must provide method `{}`",
                        method.name
                    ),
                    span: impl_decl.span,
                });
                continue;
            };

            if method_decl.is_async != method.is_async {
                let expectation = if method.is_async {
                    "must be marked async to satisfy the trait method"
                } else {
                    "must not be async to satisfy the trait method"
                };
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "impl for `{impl_name}` mismatches asyncness of `{}`: {expectation}",
                        method.name
                    ),
                    span: impl_decl.span,
                });
            }

            let trait_label = trait_name.rsplit("::").next().unwrap_or(&trait_name);
            let lowered_name = format!("{impl_name}::{trait_label}::{}", method.name);
            self.check_signature(
                &method_decl.signature,
                Some(impl_name.as_str()),
                Some(impl_name.as_str()),
                &lowered_name,
            );
            let perf_diags =
                self.record_perf_attributes(&lowered_name, &method_decl.attributes, None);
            self.diagnostics.extend(perf_diags);
            let lowered = lower_function(
                &method_decl,
                &lowered_name,
                FunctionKind::Method,
                Some(impl_name.as_str()),
                self.current_package.as_deref(),
                impl_decl.generics.as_ref(),
                &mut self.type_layouts,
                &self.type_visibilities,
                &self.primitive_registry,
                self.default_arguments.clone(),
                &self.function_packages,
                &self.operator_registry,
                &mut self.string_interner,
                &self.symbol_index,
                &self.import_resolver,
                &self.static_registry,
                &self.class_bases,
                &self.class_virtual_slots,
                &self.trait_decls,
                self.generic_specializations.clone(),
            );
            let final_name = self.record_lowered_function(lowered);
            slots.push(VTableSlot {
                method: method.name.clone(),
                symbol: final_name,
            });
        }

        if !slots.is_empty() {
            let symbol = trait_vtable_symbol_name(&trait_name, &impl_name);
            self.trait_vtables.push(TraitVTable {
                symbol,
                trait_name,
                impl_type: impl_name,
                slots,
            });
        }
    }
}

type ImplMethodMap = HashMap<String, FunctionDecl>;

#[derive(Clone, Default)]
pub(crate) struct TraitLoweringInfo {
    pub methods: Vec<TraitMethodLoweringInfo>,
}

#[derive(Clone)]
pub(crate) struct TraitMethodLoweringInfo {
    pub name: String,
    pub signature: Signature,
    pub default_impl: Option<FunctionDecl>,
    pub is_async: bool,
}
