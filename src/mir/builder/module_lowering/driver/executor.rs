use super::super::super::Module;
use super::super::pipeline::LoweringPipeline;
use super::planner::LowerPlan;
use super::{LoweringResult, ModuleLowering};

pub(crate) struct LowerExecutor<'a> {
    lowering: &'a mut ModuleLowering,
}

impl<'a> LowerExecutor<'a> {
    pub fn new(lowering: &'a mut ModuleLowering) -> Self {
        Self { lowering }
    }

    pub fn run(self, module: &Module, plan: &LowerPlan) -> LoweringResult {
        LoweringPipeline::new(self.lowering).run(module, plan.item_units.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::parser::parse_module;

    #[test]
    fn executes_lowering_with_plan() {
        let parsed = parse_module(
            r#"
public void Test() { }
"#,
        )
        .expect("parse module");
        let mut lowering = ModuleLowering::default();
        let plan = LowerPlan { item_units: None };
        let result = LowerExecutor::new(&mut lowering).run(&parsed.module, &plan);
        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics {:?}",
            result.diagnostics
        );
        assert_eq!(result.module.functions.len(), 1);
    }

    #[test]
    fn executes_with_item_units() {
        let parsed = parse_module("public void Test() { }").expect("parse module");
        let mut lowering = ModuleLowering::default();
        let plan = LowerPlan {
            item_units: Some(vec![0]),
        };
        let result = LowerExecutor::new(&mut lowering).run(&parsed.module, &plan);
        assert_eq!(result.module.functions.len(), 1);
        assert!(result.unit_slices.is_empty() || result.unit_slices.len() == 1);
    }
}
