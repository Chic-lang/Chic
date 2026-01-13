use super::common::RequireExt;
use super::*;
use crate::mir::MirFunction;
use crate::mir::builder::module_lowering::driver::LoweringResult;

pub(crate) fn lower(source: &str) -> LoweringResult {
    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    if std::env::var_os("DUMP_LOWER").is_some() {
        for func in &lowering.module.functions {
            eprintln!("function {}:", func.name);
            for (index, block) in func.body.blocks.iter().enumerate() {
                eprintln!("  block {index}: {:#?}", block.statements);
                eprintln!("  terminator: {:#?}", block.terminator);
            }
        }
    }
    lowering
}

pub(crate) fn lower_no_diagnostics(source: &str) -> LoweringResult {
    let lowering = lower(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    lowering
}

pub(crate) fn find_function<'a>(lowering: &'a LoweringResult, name: &str) -> &'a MirFunction {
    lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == name || func.name.ends_with(name))
        .require(&format!("missing {} lowering", name))
}
