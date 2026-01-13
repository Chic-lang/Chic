use super::common::assert_no_pending;
use super::*;
use crate::mir::{AtomicFenceScope, AtomicOrdering};

fn lower_source(source: &str) -> LoweringResult {
    let parsed = parse_module(source).expect("module should parse");
    lower_module(&parsed.module)
}

fn find_function<'a>(lowering: &'a LoweringResult, name: &str) -> &'a MirFunction {
    lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn atomic_fences(func: &MirFunction) -> Vec<(AtomicOrdering, AtomicFenceScope)> {
    func.body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter_map(|stmt| match &stmt.kind {
            StatementKind::AtomicFence { order, scope } => Some((*order, *scope)),
            _ => None,
        })
        .collect()
}

#[test]
fn unchecked_block_suppresses_lossy_cast_warning() {
    let source = r#"
namespace Sample {
    public int Convert(long input) {
        unchecked
        {
            return (int)input;
        }
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unchecked block should suppress lossy-cast diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = find_function(&lowering, "Sample::Convert");
    assert_no_pending(&func.body);

    let cast_count = func
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter(|stmt| {
            matches!(
                &stmt.kind,
                StatementKind::Assign {
                    value: Rvalue::Cast { .. },
                    ..
                }
            )
        })
        .count();
    assert!(
        cast_count >= 1,
        "expected unchecked block to lower lossy cast statement"
    );
}

#[test]
fn unchecked_block_suppresses_float_precision_warnings() {
    let source = r#"
namespace Sample {
    public double Convert(long input, double fractional) {
        unchecked
        {
            let widened = (double)input;
            let narrowed = (int)fractional;
            return widened + (double)narrowed;
        }
    }
}
"#;

    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unchecked block should suppress int<->float lossy cast diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = find_function(&lowering, "Sample::Convert");
    assert_no_pending(&func.body);
}

#[test]
fn checked_block_reenables_lossy_casts_inside_unchecked() {
    let source = r#"
namespace Sample {
    public int Convert(long first, long second) {
        var checkedValue = 0;
        var uncheckedValue = 0;
        unchecked
        {
            checked { checkedValue = (int)first; }
            uncheckedValue = (int)second;
        }
        return checkedValue + uncheckedValue;
    }
}
"#;
    let lowering = lower_source(source);
    let truncation_diags: Vec<_> = lowering
        .diagnostics
        .iter()
        .filter(|diag| {
            diag.message.contains("C-style cast") && diag.message.contains("may truncate or wrap")
        })
        .collect();
    assert_eq!(
        truncation_diags.len(),
        1,
        "checked block should re-enable truncation diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = find_function(&lowering, "Sample::Convert");
    assert_no_pending(&func.body);
    let cast_count = func
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter(|stmt| {
            matches!(
                &stmt.kind,
                StatementKind::Assign {
                    value: Rvalue::Cast { .. },
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        cast_count, 2,
        "both casts inside checked/unchecked regions should be lowered"
    );
}

#[test]
fn atomic_block_defaults_to_seq_cst_and_emits_fences() {
    let source = r#"
namespace Sample {
    public class Counter {
        public void Block() {
            atomic {
                let value = 1;
            }
        }
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics for atomic block: {:?}",
        lowering.diagnostics
    );
    let func = find_function(&lowering, "Sample::Counter::Block");
    assert_no_pending(&func.body);

    let fences = atomic_fences(func);
    assert_eq!(
        fences.len(),
        2,
        "atomic block should emit enter/exit fences"
    );
    assert_eq!(
        fences[0],
        (AtomicOrdering::SeqCst, AtomicFenceScope::BlockEnter)
    );
    assert_eq!(
        fences[1],
        (AtomicOrdering::SeqCst, AtomicFenceScope::BlockExit)
    );
}

#[test]
fn atomic_block_reports_invalid_ordering_and_still_emits_fences() {
    let source = r#"
namespace Sample {
    public class Counter {
        public void Invalid() {
            atomic(42) { }
        }
    }
}
"#;
    let lowering = lower_source(source);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("MemoryOrder")),
        "expected atomic ordering diagnostic, found {:?}",
        lowering.diagnostics
    );
    let func = find_function(&lowering, "Sample::Counter::Invalid");
    assert_no_pending(&func.body);

    let fences = atomic_fences(func);
    assert_eq!(
        fences.len(),
        2,
        "atomic block should emit fences even on error"
    );
    for (order, scope) in fences {
        assert_eq!(
            order,
            AtomicOrdering::SeqCst,
            "invalid order defaults to SeqCst"
        );
        assert!(
            matches!(
                scope,
                AtomicFenceScope::BlockEnter | AtomicFenceScope::BlockExit
            ),
            "unexpected fence scope {scope:?}"
        );
    }
}
