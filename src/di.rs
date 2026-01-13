use crate::frontend::ast::DiLifetime;
use crate::frontend::ast::{
    Attribute, ClassDecl, ClassMember, ConstructorDecl, Module, PropertyDecl,
};
use crate::frontend::diagnostics::Span;

#[derive(Debug, Clone, Default)]
pub struct DiManifest {
    pub services: Vec<DiService>,
    pub modules: Vec<DiModule>,
}

#[derive(Debug, Clone)]
pub struct DiModule {
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct DiService {
    pub name: String,
    pub lifetime: DiLifetime,
    pub named: Option<String>,
    pub span: Option<Span>,
    pub dependencies: Vec<DiDependency>,
}

#[derive(Debug, Clone)]
pub struct DiDependency {
    pub target: String,
    pub optional: bool,
    pub requested_lifetime: Option<DiLifetime>,
    pub requested_name: Option<String>,
    pub site: DiInjectionSite,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub enum DiInjectionSite {
    ConstructorParameter {
        constructor: String,
        parameter: String,
    },
    Property {
        property: String,
    },
}

impl DiManifest {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

pub fn collect_di_manifest(module: &Module) -> DiManifest {
    let mut manifest = DiManifest::default();
    let namespace = module.namespace.as_deref();
    collect_items(&mut manifest, namespace, &module.items);
    manifest
}

fn collect_items(
    manifest: &mut DiManifest,
    namespace: Option<&str>,
    items: &[crate::frontend::ast::Item],
) {
    use crate::frontend::ast::Item;
    for item in items {
        match item {
            Item::Class(class) => collect_class(manifest, namespace, class),
            Item::Namespace(ns) => {
                let nested = qualify(namespace, &ns.name);
                collect_items(manifest, Some(nested.as_str()), &ns.items);
            }
            _ => {}
        }
    }
}

fn collect_class(manifest: &mut DiManifest, namespace: Option<&str>, class: &ClassDecl) {
    let full_name = qualify(namespace, &class.name);

    if class.di_module {
        let span = find_attribute_span(&class.attributes, "module");
        manifest.modules.push(DiModule {
            name: full_name.clone(),
            span,
        });
    }

    let Some(service_attr) = class.di_service.clone() else {
        return;
    };

    let service_span = find_attribute_span(&class.attributes, "service");
    let lifetime = service_attr.lifetime.unwrap_or(DiLifetime::Transient);
    let mut service = DiService {
        name: full_name.clone(),
        lifetime,
        named: service_attr.named.clone(),
        span: service_span,
        dependencies: Vec::new(),
    };

    let mut ctor_index = 0usize;
    for member in &class.members {
        if let ClassMember::Constructor(ctor) = member {
            collect_constructor_dependencies(&mut service, &full_name, ctor, ctor_index);
            ctor_index += 1;
        } else if let ClassMember::Property(property) = member {
            collect_property_dependency(&mut service, &full_name, property);
        }
    }

    manifest.services.push(service);
}

fn collect_constructor_dependencies(
    service: &mut DiService,
    service_name: &str,
    ctor: &ConstructorDecl,
    ctor_index: usize,
) {
    let ctor_attr = ctor.di_inject.clone();
    let ctor_has_inject = ctor_attr.is_some()
        || ctor
            .parameters
            .iter()
            .any(|param| param.di_inject.is_some());
    if !ctor_has_inject {
        return;
    }

    let ctor_span = find_attribute_span(&ctor.attributes, "inject").or(ctor.span);
    let constructor_name = format!("{service_name}::init#{ctor_index}");

    for param in &ctor.parameters {
        let param_attr = param.di_inject.clone();
        if param_attr.is_none() && ctor_attr.is_none() {
            continue;
        }

        let effective_attr = param_attr.clone().or_else(|| ctor_attr.clone());
        let optional = effective_attr.as_ref().map_or(false, |attr| attr.optional);
        let requested_lifetime = effective_attr.as_ref().and_then(|attr| attr.lifetime);
        let requested_name = effective_attr.as_ref().and_then(|attr| attr.named.clone());
        let span = param_attr
            .as_ref()
            .and_then(|_| find_attribute_span(&param.attributes, "inject"))
            .or(ctor_span);

        service.dependencies.push(DiDependency {
            target: param.ty.name.clone(),
            optional,
            requested_lifetime,
            requested_name,
            site: DiInjectionSite::ConstructorParameter {
                constructor: constructor_name.clone(),
                parameter: param.name.clone(),
            },
            span,
        });
    }
}

fn collect_property_dependency(
    service: &mut DiService,
    service_name: &str,
    property: &PropertyDecl,
) {
    let Some(attr) = property.di_inject.clone() else {
        return;
    };
    let span = find_attribute_span(&property.attributes, "inject").or(property.span);

    service.dependencies.push(DiDependency {
        target: property.ty.name.clone(),
        optional: attr.optional,
        requested_lifetime: attr.lifetime,
        requested_name: attr.named.clone(),
        site: DiInjectionSite::Property {
            property: format!("{service_name}::{}", property.name),
        },
        span,
    });
}

fn find_attribute_span(attributes: &[Attribute], name: &str) -> Option<Span> {
    attributes
        .iter()
        .find(|attr| attr.name.eq_ignore_ascii_case(name))
        .and_then(|attr| attr.span)
}

fn qualify(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(prefix) if !prefix.is_empty() => {
            let mut prefix_parts: Vec<String> = prefix
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();
            let name_parts: Vec<String> = name
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();

            if !prefix_parts.is_empty()
                && name_parts.len() >= prefix_parts.len()
                && name_parts[..prefix_parts.len()] == prefix_parts[..]
            {
                name_parts.join("::")
            } else if name_parts.is_empty() {
                prefix_parts.join("::")
            } else {
                prefix_parts.extend(name_parts);
                prefix_parts.join("::")
            }
        }
        _ => name.to_string(),
    }
}
