#![cfg(test)]

use super::super::allocations::{
    AllocationCategory, ArenaAllocationBudgets, ArenaAllocationStats, ArenaAllocations,
};
use crate::frontend::parser::parse_module;

#[test]
fn budgets_include_nested_items_and_apply_slack() {
    let parsed = parse_module(
        r#"
        namespace Demo {
            public struct A { public void M1() { } public void M2() { } }
            public class B {
                public init() { }
                public void Do() { }
            }
            public interface IPrintable { void Print(in this); }
            public interface Iterable { void Next(in this); }
            public func Free() { }
        }
    "#,
    )
    .expect("parse failed");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let budgets = ArenaAllocationBudgets::from_module(&parsed.module);

    // Raw counts: type_infos = 4 (struct/class/interfaces), trait_infos = 0,
    // signatures = 7 (Free + 2 struct methods + class ctor + class method + interface methods).
    // Slack adds max(type_infos, count/2).
    assert_eq!(budgets.signatures, 11);
    assert_eq!(budgets.type_infos, 8);
    assert_eq!(budgets.trait_infos, 0);
}

#[test]
fn allocation_recorder_tracks_categories() {
    let budgets = ArenaAllocationBudgets {
        signatures: 2,
        type_infos: 1,
        trait_infos: 1,
    };
    let mut allocations = ArenaAllocations::with_budgets(budgets);
    allocations.record(AllocationCategory::Signatures);
    allocations.record(AllocationCategory::Signatures);
    allocations.record(AllocationCategory::TypeInfos);
    allocations.record(AllocationCategory::TraitInfos);

    assert_eq!(
        allocations.snapshot(),
        ArenaAllocationStats {
            signatures: 2,
            type_infos: 1,
            trait_infos: 1
        }
    );
}
