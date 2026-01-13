use chic::mir::MemoryPlan;

#[test]
fn memory_plan_packs_buffers_without_overlap() {
    let buffers = vec![("a", 16u64), ("b", 8), ("c", 4)];
    let plan = MemoryPlan::from_sizes(&buffers);
    assert!(plan.validate(), "plan should be non-overlapping");
    assert_eq!(plan.buffers[0].offset, 0);
    assert_eq!(plan.buffers[1].offset, 16);
    assert_eq!(plan.buffers[2].offset, 24);
}
