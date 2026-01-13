use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::frontend::diagnostics::Span;
use crate::mir::AutoTraitOverride;

use super::arena::{ImplInfo, TypeChecker};
use super::diagnostics::codes;
use super::helpers::canonical_type_name;
use super::traits::AutoTraitCheck;
use super::{AutoTraitConstraintOrigin, AutoTraitKind};

#[derive(Clone, Debug)]
pub struct TraitSolverMetrics {
    pub impls_checked: usize,
    pub overlaps_detected: usize,
    pub traits_checked: usize,
    pub cycles_detected: usize,
    pub elapsed: Duration,
}

impl Default for TraitSolverMetrics {
    fn default() -> Self {
        Self {
            impls_checked: 0,
            overlaps_detected: 0,
            traits_checked: 0,
            cycles_detected: 0,
            elapsed: Duration::from_secs(0),
        }
    }
}

pub(super) struct TraitSolver<'a, 'tcx> {
    checker: &'a mut TypeChecker<'tcx>,
    metrics: TraitSolverMetrics,
}

impl<'a, 'tcx> TraitSolver<'a, 'tcx> {
    pub(super) fn run(checker: &'a mut TypeChecker<'tcx>) -> TraitSolverMetrics {
        let start = Instant::now();
        let mut solver = Self {
            checker,
            metrics: TraitSolverMetrics::default(),
        };
        solver.check_trait_constraints();
        solver.check_impls();
        solver.metrics.elapsed = start.elapsed();
        solver.metrics.clone()
    }

    fn check_trait_constraints(&mut self) {
        self.check_trait_cycles();
    }

    fn check_trait_cycles(&mut self) {
        let trait_names: Vec<String> = self.checker.traits.keys().cloned().collect();
        self.metrics.traits_checked = trait_names.len();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        let mut stack = Vec::new();
        for trait_name in trait_names {
            self.visit_trait_for_cycles(&trait_name, &mut visited, &mut visiting, &mut stack);
        }
    }

    fn visit_trait_for_cycles(
        &mut self,
        trait_name: &str,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        stack: &mut Vec<String>,
    ) {
        if visited.contains(trait_name) {
            return;
        }
        if !self.checker.traits.contains_key(trait_name) {
            return;
        }
        stack.push(trait_name.to_string());
        visiting.insert(trait_name.to_string());
        let super_traits: Vec<String> = self
            .checker
            .traits
            .get(trait_name)
            .map(|info| {
                info.super_traits
                    .iter()
                    .map(canonical_type_name)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for super_name in super_traits {
            if !self.checker.traits.contains_key(&super_name) {
                continue;
            }
            if visiting.contains(super_name.as_str()) {
                self.metrics.cycles_detected += 1;
                let cycle = self.describe_cycle(super_name.as_str(), stack);
                let span = self
                    .trait_span(trait_name)
                    .or_else(|| self.trait_span(super_name.as_str()));
                self.emit_error(
                    codes::TRAIT_CYCLE_DETECTED,
                    span,
                    format!("trait `{trait_name}` participates in a cycle: {cycle}"),
                );
                continue;
            }
            self.visit_trait_for_cycles(&super_name, visited, visiting, stack);
        }
        visiting.remove(trait_name);
        stack.pop();
        visited.insert(trait_name.to_string());
    }

    fn describe_cycle(&self, start: &str, stack: &[String]) -> String {
        let start_index = stack
            .iter()
            .position(|name| name == start)
            .unwrap_or_default();
        let mut path = stack[start_index..].to_vec();
        path.push(start.to_string());
        path.join(" -> ")
    }

    fn check_impls(&mut self) {
        let mut seen: HashMap<(String, String), Option<Span>> = HashMap::new();
        let impls = self.checker.impls.clone();
        for impl_info in &impls {
            self.check_impl(impl_info, &mut seen);
        }
    }

    fn check_impl(
        &mut self,
        impl_info: &ImplInfo,
        seen: &mut HashMap<(String, String), Option<Span>>,
    ) {
        self.metrics.impls_checked += 1;
        let span = impl_info.span;
        let Some(trait_name) = impl_info.trait_name.as_ref() else {
            self.emit_error(
                codes::TRAIT_FEATURE_UNAVAILABLE,
                span,
                "inherent `impl` blocks are not supported yet",
            );
            return;
        };

        if impl_info
            .generics
            .as_ref()
            .is_some_and(|params| !params.params.is_empty())
        {
            // `registry` already emits TCK095 for blanket impls; skip solver work.
            return;
        }

        let Some(trait_overrides) = self
            .checker
            .traits
            .get(trait_name)
            .map(|info| info.auto_trait_overrides)
        else {
            self.emit_error(
                codes::TRAIT_NOT_IMPLEMENTED,
                span,
                format!("trait `{trait_name}` is not defined in this module"),
            );
            return;
        };

        let target_name = canonical_type_name(&impl_info.target);
        if !self.checker.is_local_type(&target_name)
            && !self.checker.traits.contains_key(trait_name)
        {
            self.emit_error(
                codes::TRAIT_ORPHAN_RULE,
                span,
                format!(
                    "impl of `{trait_name}` for `{target_name}` violates the orphan rule (either the trait or the type must be defined in this module)"
                ),
            );
            return;
        }

        let key = (trait_name.clone(), target_name.clone());
        if let Some(existing_span) = seen.get(&key) {
            self.metrics.overlaps_detected += 1;
            self.emit_error(
                codes::TRAIT_IMPL_OVERLAP,
                span,
                format!(
                    "multiple implementations of `{trait_name}` for `{target_name}` found (previous implementation here: {:?})",
                    existing_span
                ),
            );
            return;
        }
        seen.insert(key, span);

        self.enforce_trait_auto_traits(trait_name, trait_overrides, &target_name, span);
    }

    fn emit_error(&mut self, code: &'static str, span: Option<Span>, message: impl Into<String>) {
        self.checker.emit_error(code, span, message);
    }

    fn trait_span(&self, trait_name: &str) -> Option<Span> {
        self.checker
            .traits
            .get(trait_name)
            .and_then(|info| info.span)
    }

    fn enforce_trait_auto_traits(
        &mut self,
        trait_name: &str,
        overrides: AutoTraitOverride,
        target_name: &str,
        span: Option<Span>,
    ) {
        if overrides.thread_safe.unwrap_or(false) {
            self.emit_auto_trait_requirement(
                trait_name,
                target_name,
                AutoTraitKind::ThreadSafe,
                span,
            );
        }
        if overrides.shareable.unwrap_or(false) {
            self.emit_auto_trait_requirement(
                trait_name,
                target_name,
                AutoTraitKind::Shareable,
                span,
            );
        }
        if overrides.copy.unwrap_or(false) {
            self.emit_auto_trait_requirement(trait_name, target_name, AutoTraitKind::Copy, span);
        }
    }

    fn emit_auto_trait_requirement(
        &mut self,
        trait_name: &str,
        target_name: &str,
        kind: AutoTraitKind,
        span: Option<Span>,
    ) {
        let target_desc = format!("impl of {trait_name}");
        self.checker.ensure_auto_trait(AutoTraitCheck {
            function: trait_name,
            target: &target_desc,
            ty: target_name,
            kind,
            origin: AutoTraitConstraintOrigin::Generic,
            span,
        });
    }
}
