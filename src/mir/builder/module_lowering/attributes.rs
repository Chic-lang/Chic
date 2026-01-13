use super::driver::ModuleLowering;
use crate::frontend::ast::{Attribute, Module, NamespaceDecl};
use crate::frontend::attributes::{
    AttributeError, GlobalAllocatorAttr, collect_export_attributes, extract_global_allocator,
    extract_no_std, extract_suppress_startup_descriptor,
};
use crate::frontend::diagnostics::Span;
use crate::mir::module_metadata::{
    Export, GlobalAllocator, LinkLibrary, StdProfile, StdProfileSource,
};

impl ModuleLowering {
    // attribute_eval NOTE: Module-scope attribute scanning now lives in the
    // dedicated attributes module for clarity and testing.
    pub(super) fn collect_module_attributes(&mut self, module: &Module) {
        self.apply_crate_std_profile(module.crate_attributes.std_setting);
        self.apply_crate_main_setting(module.crate_attributes.main_setting);

        let (no_std_span, errors) = extract_no_std(&module.namespace_attributes);
        self.push_attribute_errors(errors);
        if let Some(span) = no_std_span {
            self.record_std_profile(
                StdProfile::NoStd,
                Some(span),
                StdProfileSource::NamespaceAttribute,
            );
        }

        let (allocator, errors) = extract_global_allocator(&module.namespace_attributes);
        self.push_attribute_errors(errors);
        if let Some(attr) = allocator {
            self.diagnostics.push(super::LoweringDiagnostic {
                message: "`@global_allocator` is not supported at namespace scope".to_string(),
                span: attr.span,
            });
        }

        let (suppress_span, errors) =
            extract_suppress_startup_descriptor(&module.namespace_attributes);
        self.push_attribute_errors(errors);
        self.record_suppress_startup_descriptor(suppress_span);
    }

    // attribute_eval NOTE: Namespace-level attribute handling continues the
    // extraction of diagnostics and bookkeeping.
    pub(super) fn collect_namespace_attributes(&mut self, namespace: &NamespaceDecl) {
        let (no_std_span, errors) = extract_no_std(&namespace.attributes);
        self.push_attribute_errors(errors);
        if let Some(span) = no_std_span {
            self.record_std_profile(
                StdProfile::NoStd,
                Some(span),
                StdProfileSource::NamespaceAttribute,
            );
        }

        let (allocator, errors) = extract_global_allocator(&namespace.attributes);
        self.push_attribute_errors(errors);
        if let Some(attr) = allocator {
            self.diagnostics.push(super::LoweringDiagnostic {
                message: "`@global_allocator` is only supported on type declarations".to_string(),
                span: attr.span,
            });
        }

        let (suppress_span, errors) = extract_suppress_startup_descriptor(&namespace.attributes);
        self.push_attribute_errors(errors);
        self.record_suppress_startup_descriptor(suppress_span);
    }

    pub(super) fn record_suppress_startup_descriptor(&mut self, span: Option<Span>) {
        if span.is_none() {
            return;
        }
        self.module_attributes.suppress_startup_descriptor = true;
    }

    // attribute_eval NOTE: Global allocator bookkeeping is centralised here so
    // cache invalidation can hook in later.
    pub(super) fn record_global_allocator(&mut self, type_name: String, attr: GlobalAllocatorAttr) {
        if let Some(existing) = &self.module_attributes.global_allocator {
            self.diagnostics.push(super::LoweringDiagnostic {
                message: format!(
                    "multiple `@global_allocator` declarations (`{}` and `{}`)",
                    existing.type_name, type_name
                ),
                span: attr.span,
            });
            return;
        }
        self.module_attributes.global_allocator = Some(GlobalAllocator {
            type_name,
            target: attr.target,
            span: attr.span,
        });
    }

    // attribute_eval NOTE: Export collection owns deduplication and diagnostics.
    pub(super) fn collect_exports_for(&mut self, function_name: &str, attributes: &[Attribute]) {
        let (exports, errors) = collect_export_attributes(attributes);
        self.push_attribute_errors(errors);
        for export in exports {
            if !self.exported_symbols.insert(export.symbol.clone()) {
                self.diagnostics.push(super::LoweringDiagnostic {
                    message: format!("duplicate export symbol `{}`", export.symbol),
                    span: export.span,
                });
                continue;
            }
            if std::env::var_os("CHIC_DEBUG_EXPORTS").is_some()
                && function_name.contains("ThreadRuntimeExports")
            {
                eprintln!(
                    "[chic-debug exports] record {} -> {}",
                    function_name, export.symbol
                );
            }
            self.exports.push(Export {
                function: function_name.to_string(),
                symbol: export.symbol,
                span: export.span,
            });
        }
    }

    pub(super) fn push_attribute_errors(&mut self, errors: Vec<AttributeError>) {
        for error in errors {
            self.diagnostics.push(super::LoweringDiagnostic {
                message: error.message,
                span: error.span,
            });
        }
    }

    pub(super) fn collect_link_library(&mut self, link_library: Option<&str>) {
        let Some(name) = link_library else {
            return;
        };
        if name.is_empty() {
            return;
        }
        if !self.linked_libraries.insert(name.to_string()) {
            return;
        }
        self.module_attributes.link_libraries.push(LinkLibrary {
            name: name.to_string(),
            span: None,
        });
    }

    fn apply_crate_main_setting(&mut self, setting: crate::frontend::ast::CrateMainSetting) {
        match setting {
            crate::frontend::ast::CrateMainSetting::Unspecified => {}
            crate::frontend::ast::CrateMainSetting::NoMain { span } => {
                self.module_attributes.no_main = true;
                self.module_attributes.no_main_span = span;
                self.module_attributes.suppress_startup_descriptor = true;
            }
        }
    }

    fn apply_crate_std_profile(&mut self, setting: crate::frontend::ast::CrateStdSetting) {
        match setting {
            crate::frontend::ast::CrateStdSetting::Unspecified => {}
            crate::frontend::ast::CrateStdSetting::Std { span } => {
                self.record_std_profile(StdProfile::Std, span, StdProfileSource::CrateAttribute)
            }
            crate::frontend::ast::CrateStdSetting::NoStd { span } => {
                self.record_std_profile(StdProfile::NoStd, span, StdProfileSource::CrateAttribute)
            }
        }
    }

    fn record_std_profile(
        &mut self,
        profile: StdProfile,
        span: Option<Span>,
        source: StdProfileSource,
    ) {
        if self.module_attributes.std_profile == profile {
            if matches!(
                self.module_attributes.std_profile_source,
                StdProfileSource::Default
            ) {
                self.module_attributes.std_profile_source = source;
                self.module_attributes.std_profile_span = span;
            }
            return;
        }

        if !matches!(
            self.module_attributes.std_profile_source,
            StdProfileSource::Default
        ) {
            let message = match self.module_attributes.std_profile {
                StdProfile::Std => {
                    "conflicting standard library mode: crate already marked `#![std]`".to_string()
                }
                StdProfile::NoStd => {
                    "conflicting standard library mode: crate already marked `#![no_std]`"
                        .to_string()
                }
            };
            self.diagnostics
                .push(super::LoweringDiagnostic { message, span });
            return;
        }

        self.module_attributes.std_profile = profile;
        self.module_attributes.std_profile_span = span;
        self.module_attributes.std_profile_source = source;
    }
}
