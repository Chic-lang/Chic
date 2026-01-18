use std::path::PathBuf;

use crate::frontend::ast::{CrateAttributes, CrateStdSetting};
use crate::frontend::diagnostics::{Diagnostic, Span};
use crate::manifest::Manifest;

use super::FrontendModuleState;

pub(super) fn resolve_workspace_crate_attributes(
    modules: &mut [FrontendModuleState],
    root_manifest: Option<&Manifest>,
) -> CrateAttributes {
    let mut resolved = CrateAttributes::default();
    let root_manifest_path = root_manifest.and_then(|manifest| manifest.path().map(PathBuf::from));

    for module in modules.iter_mut().filter(|module| !module.is_stdlib) {
        if let Some(root_path) = &root_manifest_path {
            if let Some(module_manifest) = module
                .manifest
                .as_ref()
                .and_then(|manifest| manifest.path())
                .map(PathBuf::from)
            {
                if &module_manifest != root_path {
                    continue;
                }
            }
        }

        let setting = {
            let module_ref = module.parse.module_ref();
            module_ref.crate_attributes.std_setting
        };
        merge_std_setting(&mut resolved, setting, module);
        let main_setting = {
            let module_ref = module.parse.module_ref();
            module_ref.crate_attributes.main_setting
        };
        merge_main_setting(&mut resolved, main_setting, module);
        if matches!(setting, CrateStdSetting::Unspecified) {
            if let Some(span) = module_declares_no_std_attr(module) {
                merge_std_setting(
                    &mut resolved,
                    CrateStdSetting::NoStd { span: Some(span) },
                    module,
                );
            }
        }
    }

    resolved
}

fn merge_main_setting(
    resolved: &mut CrateAttributes,
    setting: crate::frontend::ast::CrateMainSetting,
    module: &mut FrontendModuleState,
) {
    use crate::frontend::ast::CrateMainSetting;
    if matches!(setting, CrateMainSetting::Unspecified) {
        return;
    }
    match resolved.main_setting {
        CrateMainSetting::Unspecified => resolved.main_setting = setting,
        existing if existing == setting => {}
        existing => {
            let message = match existing {
                CrateMainSetting::NoMain { .. } => {
                    "conflicting crate attributes: crate already marked `#![no_main]`".to_string()
                }
                CrateMainSetting::Unspecified => unreachable!("handled above"),
            };
            module
                .parse
                .diagnostics
                .push(Diagnostic::error(message, setting.span()));
        }
    }
}

fn merge_std_setting(
    resolved: &mut CrateAttributes,
    setting: CrateStdSetting,
    module: &mut FrontendModuleState,
) {
    if matches!(setting, CrateStdSetting::Unspecified) {
        return;
    }

    match resolved.std_setting {
        CrateStdSetting::Unspecified => resolved.std_setting = setting,
        existing if existing == setting => {}
        existing => {
            let message = match existing {
                CrateStdSetting::Std { .. } => {
                    "conflicting crate attributes: crate already marked `#![std]`".to_string()
                }
                CrateStdSetting::NoStd { .. } => {
                    "conflicting crate attributes: crate already marked `#![no_std]`".to_string()
                }
                CrateStdSetting::Unspecified => unreachable!("handled above"),
            };
            module
                .parse
                .diagnostics
                .push(Diagnostic::error(message, setting.span()));
        }
    }
}

fn module_declares_no_std_attr(module: &FrontendModuleState) -> Option<Span> {
    let module_ref = module.parse.module_ref();
    module_ref
        .namespace_attributes
        .iter()
        .find(|attr| {
            attr.name.eq_ignore_ascii_case("no_std") || attr.name.eq_ignore_ascii_case("nostd")
        })
        .and_then(|attr| attr.span)
}
