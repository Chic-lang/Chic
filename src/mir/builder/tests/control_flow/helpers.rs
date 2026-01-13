use crate::mir::ConstOperand;
use crate::mir::data::{
    BasicBlock, BlockId, ConstValue, LocalId, MatchArm, MirBody, Operand, Pattern, Place,
    Terminator,
};

/// Convenience wrapper that exposes structural assertions over a MIR body.
pub(super) struct GraphAssert<'a> {
    body: &'a MirBody,
}

impl<'a> GraphAssert<'a> {
    #[must_use]
    pub fn new(body: &'a MirBody) -> Self {
        Self { body }
    }

    fn block(&self, index: usize) -> &'a BasicBlock {
        self.body
            .blocks
            .get(index)
            .unwrap_or_else(|| panic!("MIR body does not contain block {index}"))
    }

    fn terminator(&self, index: usize) -> &'a Terminator {
        self.block(index)
            .terminator
            .as_ref()
            .unwrap_or_else(|| panic!("block {index} is missing a terminator"))
    }

    pub fn expect_return(&self, index: usize) {
        assert!(
            matches!(self.terminator(index), Terminator::Return),
            "expected block {index} to return, found {:?}",
            self.terminator(index)
        );
    }

    pub fn expect_goto(&self, index: usize) -> BlockId {
        match self.terminator(index) {
            Terminator::Goto { target } => *target,
            other => panic!("expected goto terminator in block {index}, found {other:?}"),
        }
    }

    pub fn expect_switch(&self, index: usize) -> SwitchAssert<'a> {
        match self.terminator(index) {
            Terminator::SwitchInt {
                discr,
                targets,
                otherwise,
            } => SwitchAssert {
                discr,
                targets,
                otherwise: *otherwise,
                index,
            },
            other => panic!("expected switch terminator in block {index}, found {other:?}"),
        }
    }

    pub fn expect_match(&self, index: usize) -> MatchAssert<'a> {
        match self.terminator(index) {
            Terminator::Match {
                arms, otherwise, ..
            } => MatchAssert {
                arms,
                otherwise: *otherwise,
                index,
            },
            other => panic!("expected match terminator in block {index}, found {other:?}"),
        }
    }

    pub fn successors(&self, index: usize) -> Vec<BlockId> {
        terminator_successors(self.terminator(index))
    }
}

pub(super) struct SwitchAssert<'a> {
    discr: &'a Operand,
    targets: &'a [(i128, BlockId)],
    otherwise: BlockId,
    index: usize,
}

impl<'a> SwitchAssert<'a> {
    pub fn expect_target_count(&self, expected: usize) -> &Self {
        assert_eq!(
            self.targets.len(),
            expected,
            "expected {expected} switch targets in block {}, found {}",
            self.index,
            self.targets.len()
        );
        self
    }

    pub fn assert_distinct_otherwise(&self) -> &Self {
        if let Some(target) = self.targets.first() {
            assert_ne!(
                target.1, self.otherwise,
                "switch block {} reuses `otherwise` target {:?}",
                self.index, self.otherwise
            );
        }
        self
    }

    pub fn otherwise(&self) -> BlockId {
        self.otherwise
    }

    pub fn discr(&self) -> &Operand {
        self.discr
    }
}

pub(super) struct MatchAssert<'a> {
    arms: &'a [MatchArm],
    otherwise: BlockId,
    index: usize,
}

impl<'a> MatchAssert<'a> {
    pub fn expect_arm_count(&self, expected: usize) -> &Self {
        assert_eq!(
            self.arms.len(),
            expected,
            "expected {expected} match arms in block {}, found {}",
            self.index,
            self.arms.len()
        );
        self
    }

    pub fn arms(&self) -> &'a [MatchArm] {
        self.arms
    }

    pub fn otherwise(&self) -> BlockId {
        self.otherwise
    }
}

fn terminator_successors(term: &Terminator) -> Vec<BlockId> {
    match term {
        Terminator::Goto { target } => vec![*target],
        Terminator::SwitchInt {
            targets, otherwise, ..
        } => targets
            .iter()
            .map(|(_, block)| *block)
            .chain(std::iter::once(*otherwise))
            .collect(),
        Terminator::Match {
            arms, otherwise, ..
        } => arms
            .iter()
            .map(|arm| arm.target)
            .chain(std::iter::once(*otherwise))
            .collect(),
        Terminator::Call { target, unwind, .. } => {
            let mut edges = vec![*target];
            if let Some(unwind) = unwind {
                edges.push(*unwind);
            }
            edges
        }
        Terminator::Yield { resume, drop, .. } | Terminator::Await { resume, drop, .. } => {
            vec![*resume, *drop]
        }
        Terminator::Pending(_) => Vec::new(),
        Terminator::Return
        | Terminator::Throw { .. }
        | Terminator::Panic
        | Terminator::Unreachable => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn switch_body() -> MirBody {
        let mut body = MirBody::new(0, None);
        body.blocks = vec![
            BasicBlock {
                id: BlockId(0),
                statements: Vec::new(),
                terminator: Some(Terminator::SwitchInt {
                    discr: Operand::Const(ConstOperand::new(ConstValue::Bool(true))),
                    targets: vec![(0, BlockId(1))],
                    otherwise: BlockId(2),
                }),
                span: None,
            },
            BasicBlock {
                id: BlockId(1),
                statements: Vec::new(),
                terminator: Some(Terminator::Return),
                span: None,
            },
            BasicBlock {
                id: BlockId(2),
                statements: Vec::new(),
                terminator: Some(Terminator::Return),
                span: None,
            },
        ];
        body
    }

    #[test]
    fn graph_assert_reports_switch_branch_count() {
        let body = switch_body();
        let graph = GraphAssert::new(&body);
        let switch = graph.expect_switch(0);
        switch.expect_target_count(1).assert_distinct_otherwise();
    }

    #[test]
    fn graph_assert_reports_successors() {
        let body = switch_body();
        let graph = GraphAssert::new(&body);
        let edges = graph.successors(0);
        assert_eq!(edges, vec![BlockId(1), BlockId(2)]);
        graph.expect_return(1);
        graph.expect_return(2);
    }

    fn match_body() -> MirBody {
        let mut body = MirBody::new(0, None);
        body.blocks = vec![
            BasicBlock {
                id: BlockId(0),
                statements: Vec::new(),
                terminator: Some(Terminator::Match {
                    value: Place::new(LocalId(0)),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            bindings: Vec::new(),
                            target: BlockId(1),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            bindings: Vec::new(),
                            target: BlockId(2),
                        },
                    ],
                    otherwise: BlockId(3),
                }),
                span: None,
            },
            BasicBlock {
                id: BlockId(1),
                statements: Vec::new(),
                terminator: Some(Terminator::Return),
                span: None,
            },
            BasicBlock {
                id: BlockId(2),
                statements: Vec::new(),
                terminator: Some(Terminator::Return),
                span: None,
            },
            BasicBlock {
                id: BlockId(3),
                statements: Vec::new(),
                terminator: Some(Terminator::Return),
                span: None,
            },
        ];
        body
    }

    #[test]
    fn graph_assert_handles_match_terminators() {
        let body = match_body();
        let graph = GraphAssert::new(&body);
        let matcher = graph.expect_match(0);
        matcher.expect_arm_count(2);
        assert_eq!(matcher.otherwise(), BlockId(3));
        matcher
            .arms()
            .iter()
            .for_each(|arm| assert!(arm.target.0 <= 3));
    }
}
