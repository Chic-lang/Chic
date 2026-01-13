use crate::frontend::diagnostics::Span;
use crate::mir::borrow::BorrowChecker;
use crate::mir::borrow::context::base::{
    BorrowState, InitState, LoanInfo, LocalFacts, Presence, UnionActiveState, UnionLocalInfo,
    enable_trace_for_tests, resolve_union_layout, union_field_index,
};
use crate::mir::data::{
    BasicBlock, BlockId, BorrowId, BorrowKind, LocalDecl, LocalId, LocalKind, MirBody, Place,
    ProjectionElem, RegionVar, Terminator,
};
use crate::mir::layout::{
    AutoTraitOverride, AutoTraitSet, TypeLayout, TypeLayoutTable, TypeRepr, UnionFieldLayout,
    UnionFieldMode, UnionLayout,
};
use crate::mir::{FnSig, FunctionKind, MirFunction, Ty};

fn sample_union_layout() -> UnionLayout {
    UnionLayout {
        name: "Demo::UnionValue".into(),
        repr: TypeRepr::Default,
        packing: None,
        views: vec![UnionFieldLayout {
            name: "Data".into(),
            ty: Ty::named("int"),
            index: 0,
            mode: UnionFieldMode::Value,
            span: None,
            is_nullable: false,
        }],
        size: Some(4),
        align: Some(4),
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
    }
}

#[test]
fn resolve_union_layout_prefers_exact_matches_and_detects_ambiguity() {
    let mut layouts = TypeLayoutTable::default();
    let layout = sample_union_layout();
    layouts
        .types
        .insert("Demo::UnionValue".into(), TypeLayout::Union(layout.clone()));

    let exact = resolve_union_layout(&layouts, &Ty::named("Demo::UnionValue"));
    assert!(
        exact.is_some(),
        "expected to resolve fully-qualified union name"
    );

    let short = resolve_union_layout(&layouts, &Ty::named("UnionValue"));
    assert!(
        short.is_some(),
        "single candidate short names should resolve"
    );

    layouts
        .types
        .insert("Other::UnionValue".into(), TypeLayout::Union(layout));
    let ambiguous = resolve_union_layout(&layouts, &Ty::named("UnionValue"));
    assert!(
        ambiguous.is_none(),
        "ambiguous short names should not resolve to a layout"
    );
}

#[test]
fn union_field_index_extracts_projection_indices() {
    let mut place = Place::new(LocalId(3));
    place.projection.push(ProjectionElem::UnionField {
        index: 2,
        name: "View".into(),
    });
    assert_eq!(
        union_field_index(&place),
        Some(2),
        "expected union projection index to be recovered"
    );
    place.projection.clear();
    place.projection.push(ProjectionElem::Field(0));
    assert!(
        union_field_index(&place).is_none(),
        "non-union projections should return None"
    );
}

#[test]
fn borrow_state_merges_active_loans_and_downgrades_presence() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let layouts = TypeLayoutTable::default();

    let mut left = BorrowState::new(&body, &layouts);
    let mut right = BorrowState::new(&body, &layouts);

    let place = Place::new(LocalId(0));
    let loan = LoanInfo {
        borrow_id: BorrowId(0),
        kind: BorrowKind::Shared,
        place: place.clone(),
        region: RegionVar(0),
        origin_span: None,
    };
    left.record_borrow(loan.clone());
    right.record_borrow(loan);

    if let Some(active) = left.active_loans.get_mut(&BorrowId(0)) {
        active.presence = Presence::Present;
    }
    if let Some(active) = right.active_loans.get_mut(&BorrowId(0)) {
        active.presence = Presence::Maybe;
    }

    assert!(
        left.merge_active_loans(&right),
        "merge should report a changed state"
    );
    assert!(
        matches!(
            left.active_loans
                .get(&BorrowId(0))
                .map(|loan| loan.presence),
            Some(Presence::Maybe)
        ),
        "presence should downgrade to Maybe when one branch loses the loan"
    );
}

#[test]
fn borrow_state_merge_union_active_state_resolves_conflicts() {
    let layout = sample_union_layout();
    let mut lhs = UnionLocalInfo {
        layout: &layout,
        active: Some(UnionActiveState::Field { index: 0 }),
    };
    let rhs = UnionLocalInfo {
        layout: &layout,
        active: Some(UnionActiveState::Field { index: 1 }),
    };
    assert!(
        BorrowState::merge_union_active_state(&mut lhs, &rhs),
        "merging disjoint active fields should mark state as unknown"
    );
    assert!(matches!(lhs.active, Some(UnionActiveState::Unknown)));
}

#[test]
fn borrow_state_remove_loans_for_place_clears_match() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let layouts = TypeLayoutTable::default();

    let mut state = BorrowState::new(&body, &layouts);
    let place = Place::new(LocalId(0));
    let loan = LoanInfo {
        borrow_id: BorrowId(11),
        kind: BorrowKind::Unique,
        place: place.clone(),
        region: RegionVar(2),
        origin_span: None,
    };
    state.record_borrow(loan);

    let removed = state.remove_loans_for_place(&place);
    assert_eq!(removed.len(), 1, "expected the matching loan to be removed");
    assert!(
        state.active_loans.is_empty(),
        "state should no longer track any loans for the place"
    );
}

#[test]
fn borrow_state_remove_loans_for_view_releases_associated_loans() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("array".into()),
        Ty::Array(crate::mir::data::ArrayTy::new(
            Box::new(Ty::named("int")),
            1,
        )),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("view".into()),
        Ty::ReadOnlySpan(crate::mir::data::ReadOnlySpanTy::new(Box::new(Ty::named(
            "int",
        )))),
        true,
        None,
        LocalKind::Local,
    ));
    let layouts = TypeLayoutTable::default();

    let mut state = BorrowState::new(&body, &layouts);
    let root_place = Place::new(LocalId(1));
    let loan = LoanInfo {
        borrow_id: BorrowId(22),
        kind: BorrowKind::Shared,
        place: root_place.clone(),
        region: RegionVar(3),
        origin_span: None,
    };
    state.record_borrow(loan);
    if let Some(active) = state.active_loans.get_mut(&BorrowId(22)) {
        active.associated_view = Some(LocalId(2));
    } else {
        panic!("expected loan to be recorded");
    }

    let removed = state.remove_loans_for_view(LocalId(2));
    assert_eq!(removed.len(), 1, "expected associated loan to be removed");
    assert!(
        state.active_loans.is_empty(),
        "state should clear tracked loan after view release"
    );
}

#[test]
fn borrow_state_merge_unsafe_depth_chooses_minimum() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let layouts = TypeLayoutTable::default();

    let mut left = BorrowState::new(&body, &layouts);
    let mut right = BorrowState::new(&body, &layouts);
    left.unsafe_depth = 3;
    right.unsafe_depth = 1;
    assert!(
        left.merge_unsafe_depth(&right),
        "merging different unsafe depths should report changes"
    );
    assert_eq!(left.unsafe_depth, 1);
}

#[test]
fn local_facts_merge_from_tracks_assignments_and_init_state() {
    let decl = LocalDecl::new(Some("tmp".into()), Ty::Unit, true, None, LocalKind::Local);
    let mut lhs = LocalFacts::from_decl(&decl);
    let mut rhs = LocalFacts::from_decl(&decl);

    lhs.init = InitState::Uninit;
    rhs.init = InitState::Init;
    rhs.assignment_count = 2;
    rhs.last_assignment = Some(Span::new(4, 8));
    assert!(lhs.merge_from(&rhs), "merge should report changes");
    assert_eq!(lhs.init, InitState::Maybe);
    assert_eq!(lhs.assignment_count, 2);
    assert!(matches!(lhs.last_assignment, Some(_)));
}

#[test]
fn borrow_checker_trace_logging_handles_multiple_blocks() {
    let _guard = enable_trace_for_tests();
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));

    for idx in 0..16 {
        let mut block = BasicBlock::new(BlockId(idx), None);
        block.terminator = Some(Terminator::Goto {
            target: BlockId(idx + 1),
        });
        body.blocks.push(block);
    }
    body.blocks.push({
        let mut block = BasicBlock::new(BlockId(16), None);
        block.terminator = Some(Terminator::Return);
        block
    });

    let function = MirFunction {
        name: "Trace::Run".into(),
        kind: FunctionKind::Function,
        signature: FnSig::empty(),
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    };

    let layouts = TypeLayoutTable::default();
    let mut checker = BorrowChecker::new(&function, &layouts);
    checker.run();
}
