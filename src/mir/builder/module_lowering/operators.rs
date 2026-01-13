use super::super::{Item, qualify};
use super::driver::ModuleLowering;
use crate::frontend::ast::{
    ClassDecl, ClassMember, ExtensionDecl, ExtensionMember, FunctionDecl, InterfaceDecl,
    InterfaceMember,
};
use crate::frontend::import_resolver::Resolution as ImportResolution;
use crate::frontend::type_utils::{extension_method_symbol, instantiate_extension_method};

impl ModuleLowering {
    pub(crate) fn collect_operator_overloads(&mut self, items: &[Item], namespace: Option<&str>) {
        for item in items {
            match item {
                Item::Namespace(ns) => {
                    let nested = qualify(namespace, &ns.name);
                    self.collect_operator_overloads(&ns.items, Some(&nested));
                }
                Item::Struct(strct) => {
                    let struct_ns = qualify(namespace, &strct.name);
                    self.collect_struct_operator_overloads(&strct.methods, namespace, &struct_ns);
                    if !strct.nested_types.is_empty() {
                        self.collect_operator_overloads(&strct.nested_types, Some(&struct_ns));
                    }
                }
                Item::Class(class) => {
                    let class_ns = qualify(namespace, &class.name);
                    self.collect_class_operator_overloads(class, namespace, &class_ns);
                    if !class.nested_types.is_empty() {
                        self.collect_operator_overloads(&class.nested_types, Some(&class_ns));
                    }
                }
                Item::Interface(iface) => {
                    let iface_ns = qualify(namespace, &iface.name);
                    self.collect_interface_operator_overloads(iface, namespace, &iface_ns);
                }
                Item::Extension(ext) => {
                    self.collect_extension_operator_overloads(ext, namespace);
                }
                _ => {}
            }
        }
    }

    fn collect_struct_operator_overloads(
        &mut self,
        methods: &[FunctionDecl],
        namespace: Option<&str>,
        struct_ns: &str,
    ) {
        for method in methods {
            if method.operator.is_none() {
                continue;
            }
            let method_name = format!("{struct_ns}::{}", method.name);
            self.register_operator_overload(
                struct_ns,
                namespace,
                Some(struct_ns),
                method,
                &method_name,
            );
        }
    }

    fn collect_class_operator_overloads(
        &mut self,
        class: &ClassDecl,
        namespace: Option<&str>,
        class_ns: &str,
    ) {
        for member in &class.members {
            let ClassMember::Method(method) = member else {
                continue;
            };
            if method.operator.is_none() {
                continue;
            }
            let method_name = format!("{class_ns}::{}", method.name);
            self.register_operator_overload(
                class_ns,
                namespace,
                Some(class_ns),
                method,
                &method_name,
            );
        }
    }

    fn collect_interface_operator_overloads(
        &mut self,
        iface: &InterfaceDecl,
        namespace: Option<&str>,
        iface_ns: &str,
    ) {
        for member in &iface.members {
            let InterfaceMember::Method(method) = member else {
                continue;
            };
            if method.operator.is_none() {
                continue;
            }
            let method_name = format!("{iface_ns}::{}", method.name);
            self.register_operator_overload(
                iface_ns,
                namespace,
                Some(iface_ns),
                method,
                &method_name,
            );
        }
    }

    fn collect_extension_operator_overloads(
        &mut self,
        ext: &ExtensionDecl,
        namespace: Option<&str>,
    ) {
        let target_name = match self.resolve_type_for_expr(&ext.target, namespace, None) {
            ImportResolution::Found(resolved) => resolved,
            ImportResolution::Ambiguous(_) | ImportResolution::NotFound => {
                qualify(namespace, &ext.target.name)
            }
        };

        for member in &ext.members {
            let ExtensionMember::Method(method) = member;
            if method.function.operator.is_none() {
                continue;
            }
            let method_name = extension_method_symbol(
                target_name.as_str(),
                namespace,
                &method.function.name,
                method.is_default,
            );
            let instantiated = instantiate_extension_method(&method.function, &ext.target);
            self.register_operator_overload(
                &target_name,
                namespace,
                Some(target_name.as_str()),
                &instantiated,
                &method_name,
            );
        }
    }
}
