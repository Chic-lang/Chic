//! Cost-model normalisation and validation pass.

use std::collections::HashSet;

use crate::mir::MirModule;
use crate::perf::PerfMetadata;

/// Normalise collected `@cost` and `@trace` metadata to match the lowered MIR module.
///
/// - Removes entries for functions that were eliminated during lowering,
/// - Propagates declared budgets onto tracepoints when absent,
/// - Sorts metadata deterministically to keep emitted sidecars stable.
pub fn normalise_cost_model(perf: &mut PerfMetadata, mir: &MirModule) {
    let valid_functions: HashSet<&str> = mir
        .functions
        .iter()
        .map(|func| func.name.as_str())
        .collect();
    perf.costs
        .retain(|cost| valid_functions.contains(cost.function.as_str()));
    perf.tracepoints
        .retain(|trace| valid_functions.contains(trace.function.as_str()));

    for trace in &mut perf.tracepoints {
        if trace.budget.is_none() {
            if let Some(cost) = perf
                .costs
                .iter()
                .find(|cost| cost.function == trace.function)
            {
                trace.budget = Some(cost.clone());
            }
        }
    }

    perf.costs
        .sort_by(|lhs, rhs| lhs.function.cmp(&rhs.function));
    perf.tracepoints
        .sort_by(|lhs, rhs| lhs.function.cmp(&rhs.function));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        Abi, BasicBlock, BlockId, FnSig, FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction,
        MirModule, Terminator, Ty, TypeLayoutTable,
    };
    use crate::perf::{CostModel, TraceLevel, Tracepoint, trace_id};

    fn dummy_function(name: &str) -> MirFunction {
        let mut body = MirBody::new(0, None);
        body.locals.push(LocalDecl::new(
            Some("_ret".into()),
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        body.blocks.push(BasicBlock {
            id: BlockId(0),
            statements: Vec::new(),
            terminator: Some(Terminator::Return),
            span: None,
        });
        MirFunction {
            name: name.into(),
            kind: FunctionKind::Function,
            signature: FnSig {
                params: Vec::new(),
                ret: Ty::Unit,
                abi: Abi::Chic,
                effects: Vec::new(),
                lends_to_return: None,
                variadic: false,
            },
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        }
    }

    #[test]
    fn propagates_costs_to_tracepoints() {
        let mut perf = PerfMetadata::default();
        perf.costs.push(CostModel {
            function: "Demo::trace_me".into(),
            cpu_budget_us: Some(5),
            gpu_budget_us: None,
            mem_budget_bytes: None,
            span: None,
        });
        perf.tracepoints.push(Tracepoint {
            function: "Demo::trace_me".into(),
            label: "trace".into(),
            label_id: None,
            level: TraceLevel::Perf,
            trace_id: trace_id("Demo::trace_me", "trace"),
            span: None,
            budget: None,
        });

        let mut module = MirModule::default();
        module.type_layouts = TypeLayoutTable::default();
        module.functions.push(dummy_function("Demo::trace_me"));

        normalise_cost_model(&mut perf, &module);
        assert_eq!(perf.costs.len(), 1);
        assert_eq!(perf.tracepoints.len(), 1);
        assert!(perf.tracepoints[0].budget.is_some());
    }
}
