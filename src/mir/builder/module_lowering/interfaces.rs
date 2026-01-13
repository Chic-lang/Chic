use super::super::functions::lower_function;
use super::super::{
    ConstraintKind, ExtensionDecl, ExtensionMember, FunctionKind, InterfaceDecl, InterfaceMember,
    TypeConstraint, qualify,
};
use super::driver::{LoweringDiagnostic, ModuleLowering};
use crate::frontend::import_resolver::Resolution as ImportResolution;
use crate::frontend::type_utils::{extension_method_symbol, instantiate_extension_method};
use crate::mir::builder::module_lowering::traits::{TraitLoweringInfo, TraitMethodLoweringInfo};

impl ModuleLowering {
    pub(super) fn lower_interface(&mut self, iface: &InterfaceDecl, namespace: Option<&str>) {
        let iface_ns = qualify(namespace, &iface.name);
        self.record_type_visibility(&iface_ns, iface.visibility, namespace, None);
        self.type_layouts.ensure_interface_layout(&iface_ns);
        let mut trait_methods: Vec<TraitMethodLoweringInfo> = Vec::new();
        for member in &iface.members {
            match member {
                InterfaceMember::Method(method) => {
                    let method_name = format!("{iface_ns}::{}", method.name);
                    self.check_signature(
                        &method.signature,
                        Some(iface_ns.as_str()),
                        Some(iface_ns.as_str()),
                        &method_name,
                    );
                    if method.body.is_some() {
                        let perf_diags =
                            self.record_perf_attributes(&method_name, &method.attributes, None);
                        self.diagnostics.extend(perf_diags);
                        let lowered = lower_function(
                            method,
                            &method_name,
                            FunctionKind::Method,
                            Some(iface_ns.as_str()),
                            self.current_package.as_deref(),
                            iface.generics.as_ref(),
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
                        let _ = self.record_lowered_function(lowered);
                    }
                    trait_methods.push(TraitMethodLoweringInfo {
                        name: method.name.clone(),
                        signature: method.signature.clone(),
                        default_impl: None,
                        is_async: method.is_async,
                    });
                }
                InterfaceMember::Property(property) => {
                    self.ensure_type_expr_accessible(
                        &property.ty,
                        namespace,
                        Some(iface_ns.as_str()),
                        &format!("interface property `{iface_ns}::{}`", property.name),
                        property.span,
                    );
                }
                InterfaceMember::Const(_) | InterfaceMember::AssociatedType(_) => {}
            }
        }
        if !trait_methods.is_empty() && !self.trait_decls.contains_key(&iface_ns) {
            self.trait_decls.insert(
                iface_ns.clone(),
                TraitLoweringInfo {
                    methods: trait_methods,
                },
            );
        }
    }

    pub(super) fn lower_extension(&mut self, ext: &ExtensionDecl, namespace: Option<&str>) {
        let target_name = match self.resolve_type_for_expr(&ext.target, namespace, None) {
            ImportResolution::Found(resolved) => resolved,
            ImportResolution::Ambiguous(candidates) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "extension target `{}` resolves to multiple types: {}",
                        ext.target.name,
                        candidates.join(", ")
                    ),
                    span: None,
                });
                qualify(namespace, &ext.target.name)
            }
            ImportResolution::NotFound => qualify(namespace, &ext.target.name),
        };
        self.ensure_type_expr_accessible(
            &ext.target,
            namespace,
            Some(target_name.as_str()),
            &format!("extension target `{target_name}`"),
            None,
        );
        self.constraints.push(TypeConstraint::new(
            ConstraintKind::ExtensionTarget {
                extension: target_name.clone(),
                target: ext.target.name.clone(),
            },
            None,
        ));
        for member in &ext.members {
            match member {
                ExtensionMember::Method(method) => {
                    let method_name = extension_method_symbol(
                        target_name.as_str(),
                        namespace,
                        &method.function.name,
                        method.is_default,
                    );
                    self.symbol_index
                        .register_method(&target_name, &method.function.name);
                    self.collect_exports_for(&method_name, &method.function.attributes);
                    self.collect_link_library(method.function.link_library.as_deref());
                    let instantiated = instantiate_extension_method(&method.function, &ext.target);
                    self.check_signature(
                        &instantiated.signature,
                        Some(target_name.as_str()),
                        Some(target_name.as_str()),
                        &method_name,
                    );
                    let perf_diags =
                        self.record_perf_attributes(&method_name, &instantiated.attributes, None);
                    self.diagnostics.extend(perf_diags);
                    let lowered = lower_function(
                        &instantiated,
                        &method_name,
                        FunctionKind::Method,
                        Some(target_name.as_str()),
                        self.current_package.as_deref(),
                        ext.generics.as_ref(),
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
                    let _ = self.record_lowered_function(lowered);
                }
            }
        }
    }
}
