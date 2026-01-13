use super::fixtures::{
    layouts_with_struct, module_with_struct, requires_auto_trait_constraint, simple_struct_layout,
    with_constraints,
};
use super::{ArenaDiagnosticCase, ArenaDiagnosticFixture, Expectation, run_cases};
use crate::frontend::parser::parse_module;
use crate::mir::{AutoTraitOverride, AutoTraitSet, AutoTraitStatus, TypeLayoutTable, lower_module};
use crate::threading::{self, ThreadRuntimeMode};
use crate::typeck::arena::{
    AutoTraitConstraintOrigin, AutoTraitKind, ConstraintKind, TypeCheckResult, TypeConstraint,
};

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::custom(
        "thread_spawn_auto_trait_origin_reports_mm0102",
        thread_spawn_auto_trait_origin_reports_mm0102,
        Expectation::contains(&["[MM0102]"]),
    ),
    ArenaDiagnosticCase::custom(
        "thread_spawn_backend_constraint_reports_mm0101",
        thread_spawn_backend_constraint_reports_mm0101,
        Expectation::contains(&["[MM0101]"]),
    ),
    ArenaDiagnosticCase::custom(
        "requires_thread_safe_trait_for_async_locals",
        requires_thread_safe_trait_for_async_locals,
        Expectation::contains(&[
            "NotSend",
            "ThreadSafe",
            "SPEC.md#2-7-1-concurrency-guarantees",
        ]),
    ),
    ArenaDiagnosticCase::custom(
        "thread_safe_diagnostic_suggests_std_sync_guard",
        thread_safe_diagnostic_suggests_std_sync_guard,
        Expectation::contains(&["std.sync::Mutex"]),
    ),
    ArenaDiagnosticCase::custom(
        "requires_shareable_trait_for_async_borrows",
        requires_shareable_trait_for_async_borrows,
        Expectation::contains(&["NotShareable", "Shareable"]),
    ),
    ArenaDiagnosticCase::custom(
        "auto_trait_constraint_rejects_non_thread_safe_argument",
        auto_trait_constraint_rejects_non_thread_safe_argument,
        Expectation::with(&["[TCK035]"], &["SafeWorker"]),
    ),
    ArenaDiagnosticCase::custom(
        "auto_trait_constraint_respected_for_generic_arguments",
        auto_trait_constraint_respected_for_generic_arguments,
        Expectation::lacks(&["[TCK035]"]),
    ),
    ArenaDiagnosticCase::custom(
        "auto_trait_constraint_respected_for_async_generic_context",
        auto_trait_constraint_respected_for_async_generic_context,
        Expectation::lacks(&["[TCK035]", "[TCK037]"]),
    ),
    ArenaDiagnosticCase::custom(
        "auto_trait_constraint_requires_generic_annotation_when_missing",
        auto_trait_constraint_requires_generic_annotation_when_missing,
        Expectation::contains(&["[TCK037]"]),
    ),
];

fn thread_spawn_auto_trait_origin_reports_mm0102(
    fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let constraints = [TypeConstraint::new(
        ConstraintKind::RequiresAutoTrait {
            function: "Demo::Start".into(),
            target: "payload".into(),
            ty: "Demo::Worker".into(),
            trait_kind: AutoTraitKind::ThreadSafe,
            origin: AutoTraitConstraintOrigin::ThreadSpawn,
        },
        None,
    )];
    let module = module_with_struct("Demo::Worker");
    fixture.check_module(&module, &constraints, &TypeLayoutTable::default())
}

fn thread_spawn_backend_constraint_reports_mm0101(
    fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    threading::configure_thread_runtime(ThreadRuntimeMode::Unsupported { backend: "wasm" });
    let constraints = [TypeConstraint::new(
        ConstraintKind::ThreadingBackendAvailable {
            function: "Demo::Start".into(),
            backend: "wasm".into(),
            call: "Std::Thread::Thread::Spawn".into(),
        },
        None,
    )];
    let module = module_with_struct("Demo::Worker");
    let result = fixture.check_module(&module, &constraints, &TypeLayoutTable::default());
    threading::configure_thread_runtime(ThreadRuntimeMode::Supported);
    result
}

fn requires_thread_safe_trait_for_async_locals(
    _fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let layouts = layouts_with_struct(
        "Demo::NotSend",
        AutoTraitSet::new(
            AutoTraitStatus::No,
            AutoTraitStatus::Yes,
            AutoTraitStatus::No,
        ),
        AutoTraitOverride {
            thread_safe: Some(false),
            shareable: Some(true),
            copy: None,
        },
    );
    let constraints = [requires_auto_trait_constraint(
        "Demo::Work",
        "state",
        "Demo::NotSend",
        AutoTraitKind::ThreadSafe,
    )];
    let module = module_with_struct("Demo::NotSend");
    with_constraints(constraints, &module, &layouts)
}

fn thread_safe_diagnostic_suggests_std_sync_guard(
    _fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let layouts = layouts_with_struct(
        "Demo::NotSend",
        AutoTraitSet::new(
            AutoTraitStatus::No,
            AutoTraitStatus::Yes,
            AutoTraitStatus::No,
        ),
        AutoTraitOverride {
            thread_safe: Some(false),
            shareable: Some(true),
            copy: None,
        },
    );
    let constraints = [requires_auto_trait_constraint(
        "Demo::Work",
        "state",
        "Demo::NotSend",
        AutoTraitKind::ThreadSafe,
    )];
    let module = module_with_struct("Demo::NotSend");
    with_constraints(constraints, &module, &layouts)
}

fn requires_shareable_trait_for_async_borrows(
    _fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let layouts = layouts_with_struct(
        "Demo::NotShareable",
        AutoTraitSet::new(
            AutoTraitStatus::Yes,
            AutoTraitStatus::No,
            AutoTraitStatus::No,
        ),
        AutoTraitOverride {
            thread_safe: Some(true),
            shareable: Some(false),
            copy: None,
        },
    );
    let constraints = [requires_auto_trait_constraint(
        "Demo::Read",
        "data",
        "Demo::NotShareable",
        AutoTraitKind::Shareable,
    )];
    let module = module_with_struct("Demo::NotShareable");
    with_constraints(constraints, &module, &layouts)
}

fn auto_trait_constraint_rejects_non_thread_safe_argument(
    _fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let source = r#"
namespace Demo;

public struct Worker { }
public struct SafeWorker { }

public class Holder<T>
    where T : @thread_safe
{
    public T Value { get; set; }
}

public class Uses
{
    private Holder<Worker> _bad;
    private Holder<SafeWorker> _good;
}
"#;
    let parsed = parse_module(source).unwrap_or_else(|err| {
        panic!("parse failed: {:?}", err.diagnostics());
    });
    let module = parsed.module;
    let lowering = lower_module(&module);
    let mut layouts = TypeLayoutTable::default();
    layouts.types.insert(
        "Demo::Worker".into(),
        simple_struct_layout(
            "Demo::Worker",
            AutoTraitSet::new(
                AutoTraitStatus::No,
                AutoTraitStatus::Yes,
                AutoTraitStatus::Yes,
            ),
            AutoTraitOverride::default(),
        ),
    );
    layouts.types.insert(
        "Demo::SafeWorker".into(),
        simple_struct_layout(
            "Demo::SafeWorker",
            AutoTraitSet::all_yes(),
            AutoTraitOverride::default(),
        ),
    );
    with_constraints(lowering.constraints, &module, &layouts)
}

fn auto_trait_constraint_respected_for_generic_arguments(
    _fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let source = r#"
namespace Demo;

public class Holder<T>
    where T : @thread_safe
{
    public T Value { get; set; }
}

public class Uses<T>
    where T : @thread_safe
{
    private Holder<T> _holder;
}
"#;
    let parsed = parse_module(source).unwrap_or_else(|err| {
        panic!("parse failed: {:?}", err.diagnostics());
    });
    let module = parsed.module;
    let lowering = lower_module(&module);
    with_constraints(lowering.constraints, &module, &TypeLayoutTable::default())
}

fn auto_trait_constraint_respected_for_async_generic_context(
    _fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let source = r#"
namespace Demo;

public class Runner<T>
    where T : @thread_safe
{
    public void Run(T payload) { }
}
"#;
    let parsed = parse_module(source).unwrap_or_else(|err| {
        panic!("parse failed: {:?}", err.diagnostics());
    });
    let module = parsed.module;
    let constraints = [TypeConstraint::new(
        ConstraintKind::RequiresAutoTrait {
            function: "Demo::Runner::Run".into(),
            target: "payload".into(),
            ty: "T".into(),
            trait_kind: AutoTraitKind::ThreadSafe,
            origin: AutoTraitConstraintOrigin::AsyncSuspend,
        },
        None,
    )];
    with_constraints(constraints, &module, &TypeLayoutTable::default())
}

fn auto_trait_constraint_requires_generic_annotation_when_missing(
    _fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let source = r#"
namespace Demo;

public class Runner<T>
{
    public void Run(T payload) { }
}
"#;
    let parsed = parse_module(source).unwrap_or_else(|err| {
        panic!("parse failed: {:?}", err.diagnostics());
    });
    let module = parsed.module;
    let constraints = [TypeConstraint::new(
        ConstraintKind::RequiresAutoTrait {
            function: "Demo::Runner::Run".into(),
            target: "payload".into(),
            ty: "T".into(),
            trait_kind: AutoTraitKind::ThreadSafe,
            origin: AutoTraitConstraintOrigin::AsyncSuspend,
        },
        None,
    )];
    with_constraints(constraints, &module, &TypeLayoutTable::default())
}

#[test]
fn auto_trait_diagnostics() {
    run_cases("auto_traits", CASES);
}
