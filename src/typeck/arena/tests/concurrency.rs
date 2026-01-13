#![cfg(test)]

use super::fixtures::{parse_and_check, result_contains, simple_struct_layout};
use crate::frontend::parser::parse_module;
use crate::mir::{AutoTraitOverride, AutoTraitSet, AutoTraitStatus, TypeLayoutTable};
use crate::typeck::arena::check_module;

const RUNTIME_STUB: &str = r"
namespace Std.Sync
{
    public enum MemoryOrder
    {
        Relaxed,
        Acquire,
        Release,
        AcqRel,
        SeqCst,
    }

    public class Atomic<T>
    {
        public bool CompareExchange(T expected, T desired, MemoryOrder success, MemoryOrder failure)
        {
            return true;
        }

        public void Store(T value, MemoryOrder order) { }
    }
}
";

#[test]
fn atomic_ordering_requires_memory_order_enum() {
    let source = format!(
        "{runtime}
namespace Demo
{{
    public class Counter
    {{
        private Std.Sync.Atomic<int> _value;

        public void Update()
        {{
            atomic(42)
            {{
                _value.Store(1, Std.Sync.MemoryOrder.SeqCst);
            }}
        }}
    }}
}}
",
        runtime = RUNTIME_STUB
    );

    let (_module, report) = parse_and_check(&source);
    assert!(
        result_contains(&report, "[MM0001]"),
        "expected MM0001 diagnostic, found {:?}",
        report.diagnostics
    );
}

#[test]
fn compare_exchange_rejects_stronger_failure_order() {
    let source = format!(
        "{runtime}
namespace Demo
{{
    public class Counter
    {{
        private Std.Sync.Atomic<int> _value;

        public void Update()
        {{
            atomic(Std.Sync.MemoryOrder.Acquire)
            {{
                _value.CompareExchange(
                    0,
                    1,
                    Std.Sync.MemoryOrder.Acquire,
                    Std.Sync.MemoryOrder.Release
                );
            }}
        }}
    }}
}}
",
        runtime = RUNTIME_STUB
    );

    let (_module, report) = parse_and_check(&source);
    assert!(
        result_contains(&report, "[MM0002]"),
        "expected MM0002 diagnostic, found {:?}",
        report.diagnostics
    );
}

#[test]
fn atomic_inner_type_must_be_threadsafe() {
    let source = format!(
        "{runtime}
namespace Demo
{{
    public struct NotSend {{ }}

    public class Holder
    {{
        private Std.Sync.Atomic<Demo.NotSend> _value;
    }}
}}
",
        runtime = RUNTIME_STUB
    );

    let parsed = parse_module(&source).expect("parse module");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );

    let module = parsed.module;
    let mut layouts = TypeLayoutTable::default();
    layouts.types.insert(
        "Demo::NotSend".into(),
        simple_struct_layout(
            "Demo::NotSend",
            AutoTraitSet::new(
                AutoTraitStatus::No,
                AutoTraitStatus::No,
                AutoTraitStatus::No,
            ),
            AutoTraitOverride::default(),
        ),
    );

    let report = check_module(&module, &[], &layouts);
    assert!(
        result_contains(&report, "[MM0003]"),
        "expected MM0003 diagnostic, found {:?}",
        report.diagnostics
    );
}
