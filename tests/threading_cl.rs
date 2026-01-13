use chic::frontend::parser::parse_module;
use chic::mir::TypeLayoutTable;
use chic::threading::{ThreadRuntimeMode, configure_thread_runtime, thread_runtime_mode};
use chic::typeck::{
    AutoTraitConstraintOrigin, AutoTraitKind, ConstraintKind, TypeCheckResult, TypeConstraint,
    check_module,
};

fn parse_dummy_module() -> TypeCheckResultContext {
    let parsed = parse_module(
        r#"
namespace Samples;

public class Worker { }
"#,
    )
    .expect("threading sample parses");
    TypeCheckResultContext {
        module: parsed.module,
        layouts: TypeLayoutTable::default(),
    }
}

struct TypeCheckResultContext {
    module: chic::frontend::ast::Module,
    layouts: TypeLayoutTable,
}

struct ThreadModeGuard {
    previous: ThreadRuntimeMode,
}

impl ThreadModeGuard {
    fn install(mode: ThreadRuntimeMode) -> Self {
        let previous = thread_runtime_mode();
        configure_thread_runtime(mode);
        Self { previous }
    }
}

impl Drop for ThreadModeGuard {
    fn drop(&mut self) {
        configure_thread_runtime(self.previous);
    }
}

fn diag_contains(report: &TypeCheckResult, needle: &str) -> bool {
    report
        .diagnostics
        .iter()
        .any(|diag| diag.message.contains(needle))
}

#[test]
fn thread_spawn_requires_threadsafe_payload() {
    let ctx = parse_dummy_module();
    let constraints = [TypeConstraint::new(
        ConstraintKind::RequiresAutoTrait {
            function: "Samples::Spawner".into(),
            target: "payload".into(),
            ty: "Samples::Worker".into(),
            trait_kind: AutoTraitKind::ThreadSafe,
            origin: AutoTraitConstraintOrigin::ThreadSpawn,
        },
        None,
    )];

    let report = check_module(&ctx.module, &constraints, &ctx.layouts);
    assert!(
        diag_contains(&report, "[MM0102]"),
        "expected MM0102 diagnostic, found {:?}",
        report.diagnostics
    );
}

#[test]
fn thread_spawn_reports_mm0101_when_backend_disabled() {
    let _guard = ThreadModeGuard::install(ThreadRuntimeMode::Unsupported { backend: "wasm" });
    let ctx = parse_dummy_module();
    let constraints = [TypeConstraint::new(
        ConstraintKind::ThreadingBackendAvailable {
            function: "Samples::Spawner".into(),
            backend: "wasm".into(),
            call: "Std::Thread::Thread::Spawn".into(),
        },
        None,
    )];

    let report = check_module(&ctx.module, &constraints, &ctx.layouts);
    assert!(
        diag_contains(&report, "[MM0101]"),
        "expected MM0101 diagnostic, found {:?}",
        report.diagnostics
    );
}
