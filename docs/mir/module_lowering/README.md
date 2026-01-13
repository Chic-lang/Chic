# Module lowering driver phases

The driver now separates planning and execution:

1. **Planner (`driver/planner.rs`)** – validates/records per-item translation units and prepares a lowering plan (currently the item unit map).
2. **Executor (`driver/executor.rs`)** – owns `ModuleLowering`, runs the `LoweringPipeline`, and collects diagnostics/metrics for reporting.
3. **Driver orchestrator (`driver.rs`)** – wires planner → executor and records pass metrics.

Extend the pipeline by:
- Adding scheduling logic in `planner.rs` (e.g., cancellation or custom unit mapping).
- Leaving pass execution/reporting in `executor.rs`.
- Updating this doc with new phases and coverage commands (`cargo llvm-cov --lib --json --output-path coverage/mir_module_lowering_driver.json -- mir::builder::module_lowering::driver::planner::tests:: mir::builder::module_lowering::driver::executor::tests::`).
