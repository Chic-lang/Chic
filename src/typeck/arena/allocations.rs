//! Allocation budgeting and tracking helpers for the type checker arena.

use crate::frontend::ast::Module;
use crate::frontend::ast::{
    ClassDecl, ClassMember, ExtensionDecl, ExtensionMember, InterfaceDecl, InterfaceMember, Item,
    StructDecl, TraitDecl, TraitMember,
};

const ALLOCATION_CATEGORY_COUNT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AllocationCategory {
    Signatures = 0,
    TypeInfos = 1,
    TraitInfos = 2,
}

impl AllocationCategory {
    const fn as_index(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct ArenaAllocationStats {
    pub signatures: usize,
    pub type_infos: usize,
    pub trait_infos: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ArenaAllocations {
    counts: [usize; ALLOCATION_CATEGORY_COUNT],
    budgets: [usize; ALLOCATION_CATEGORY_COUNT],
}

impl ArenaAllocations {
    pub fn with_budgets(budgets: ArenaAllocationBudgets) -> Self {
        Self {
            counts: [0; ALLOCATION_CATEGORY_COUNT],
            budgets: [budgets.signatures, budgets.type_infos, budgets.trait_infos],
        }
    }

    pub fn record(&mut self, category: AllocationCategory) {
        let idx = category.as_index();
        self.counts[idx] += 1;
        let budget = self.budgets[idx];
        if budget == 0 {
            return;
        }
        debug_assert!(
            self.counts[idx] <= budget,
            "arena allocation budget for {:?} exceeded: {} allocations (budget {})",
            category,
            self.counts[idx],
            budget
        );
    }

    #[cfg(test)]
    pub fn snapshot(&self) -> ArenaAllocationStats {
        ArenaAllocationStats {
            signatures: self.counts[AllocationCategory::Signatures.as_index()],
            type_infos: self.counts[AllocationCategory::TypeInfos.as_index()],
            trait_infos: self.counts[AllocationCategory::TraitInfos.as_index()],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct ArenaAllocationBudgets {
    pub signatures: usize,
    pub type_infos: usize,
    pub trait_infos: usize,
}

impl ArenaAllocationBudgets {
    pub fn from_module(module: &Module) -> Self {
        let mut counters = AllocationCounters::default();
        visit_items_for_budgets(&module.items, &mut counters);
        Self {
            signatures: with_slack(counters.signatures),
            type_infos: with_slack(counters.type_infos),
            trait_infos: with_slack(counters.trait_infos),
        }
    }
}

#[derive(Default)]
struct AllocationCounters {
    signatures: usize,
    type_infos: usize,
    trait_infos: usize,
}

fn with_slack(count: usize) -> usize {
    if count == 0 {
        0
    } else {
        count + (count / 2).max(4)
    }
}

fn visit_items_for_budgets(items: &[Item], counters: &mut AllocationCounters) {
    for item in items {
        visit_item_for_budgets(item, counters);
    }
}

fn visit_item_for_budgets(item: &Item, counters: &mut AllocationCounters) {
    match item {
        Item::Function(_) => counters.signatures += 1,
        Item::Struct(decl) => add_struct_budgets(decl, counters),
        Item::Union(_) | Item::Enum(_) => counters.type_infos += 1,
        Item::Class(decl) => add_class_budgets(decl, counters),
        Item::Interface(decl) => add_interface_budgets(decl, counters),
        Item::Trait(decl) => add_trait_budgets(decl, counters),
        Item::Extension(decl) => add_extension_budgets(decl, counters),
        Item::Delegate(_) => {
            counters.type_infos += 1;
            counters.signatures += 1;
        }
        Item::Namespace(ns) => visit_items_for_budgets(&ns.items, counters),
        Item::TestCase(_)
        | Item::Impl(_)
        | Item::Import(_)
        | Item::Const(_)
        | Item::Static(_)
        | Item::TypeAlias(_) => {}
    }
}

fn add_struct_budgets(decl: &StructDecl, counters: &mut AllocationCounters) {
    counters.type_infos += 1;
    counters.signatures += decl.methods.len();
    counters.signatures += decl.constructors.len();
    visit_items_for_budgets(&decl.nested_types, counters);
}

fn add_class_budgets(decl: &ClassDecl, counters: &mut AllocationCounters) {
    counters.type_infos += 1;
    for member in &decl.members {
        match member {
            ClassMember::Method(_) => counters.signatures += 1,
            ClassMember::Constructor(_) => counters.signatures += 1,
            ClassMember::Field(_) | ClassMember::Property(_) | ClassMember::Const(_) => {}
        }
    }
}

fn add_interface_budgets(decl: &InterfaceDecl, counters: &mut AllocationCounters) {
    counters.type_infos += 1;
    counters.signatures += decl
        .members
        .iter()
        .filter(|member| matches!(member, InterfaceMember::Method(_)))
        .count();
}

fn add_trait_budgets(decl: &TraitDecl, counters: &mut AllocationCounters) {
    counters.type_infos += 1;
    counters.trait_infos += 1;
    counters.signatures += decl
        .members
        .iter()
        .filter(|member| matches!(member, TraitMember::Method(_)))
        .count();
}

fn add_extension_budgets(decl: &ExtensionDecl, counters: &mut AllocationCounters) {
    counters.signatures += decl
        .members
        .iter()
        .filter(|member| matches!(member, ExtensionMember::Method(_)))
        .count();
}
