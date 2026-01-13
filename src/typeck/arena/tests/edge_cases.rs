#![cfg(test)]

use super::fixtures::{layouts_with_struct, module_with_struct, parse_and_check, result_contains};
use crate::mir::{
    AutoTraitOverride, AutoTraitSet, AutoTraitStatus, StructLayout, TypeLayout, TypeLayoutTable,
    TypeRepr,
};
use crate::typeck::arena::check_module;

#[test]
fn rc_auto_traits_reported_via_layouts() {
    let layouts = TypeLayoutTable::default();
    let traits = layouts.resolve_auto_traits("Rc<int>");
    assert_eq!(traits.thread_safe, AutoTraitStatus::No);
    assert_eq!(traits.shareable, AutoTraitStatus::Yes);
}

#[test]
fn arc_auto_traits_follow_inner_type() {
    let mut layouts = TypeLayoutTable::default();
    layouts.types.insert(
        "Demo::Unsafe".into(),
        TypeLayout::Struct(StructLayout {
            name: "Demo::Unsafe".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size: Some(4),
            align: Some(4),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::new(
                AutoTraitStatus::No,
                AutoTraitStatus::No,
                AutoTraitStatus::No,
            ),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
    layouts.finalize_auto_traits();
    let traits = layouts.resolve_auto_traits("Arc<Demo::Unsafe>");
    assert_eq!(traits.thread_safe, AutoTraitStatus::No);
    assert_eq!(traits.shareable, AutoTraitStatus::No);
}

#[test]
fn std_sync_mutex_is_trait_safe() {
    let layouts = TypeLayoutTable::default();
    let traits = layouts.resolve_auto_traits("std::sync::Mutex<int>");
    assert_eq!(traits.thread_safe, AutoTraitStatus::Yes);
    assert_eq!(traits.shareable, AutoTraitStatus::Yes);

    let guard_traits = layouts.resolve_auto_traits("std::sync::MutexGuard<int>");
    assert_eq!(guard_traits.thread_safe, AutoTraitStatus::Yes);
    assert_eq!(guard_traits.shareable, AutoTraitStatus::Yes);

    let std_traits = layouts.resolve_auto_traits("Std::Sync::Mutex<int>");
    assert_eq!(std_traits.thread_safe, AutoTraitStatus::Yes);
    assert_eq!(std_traits.shareable, AutoTraitStatus::Yes);

    let std_guard_traits = layouts.resolve_auto_traits("Std::Sync::MutexGuard<int>");
    assert_eq!(std_guard_traits.thread_safe, AutoTraitStatus::Yes);
    assert_eq!(std_guard_traits.shareable, AutoTraitStatus::Yes);
}

#[test]
fn std_sync_rwlock_is_trait_safe() {
    let layouts = TypeLayoutTable::default();
    let lock_traits = layouts.resolve_auto_traits("Std::Sync::RwLock<int>");
    assert_eq!(lock_traits.thread_safe, AutoTraitStatus::Yes);
    assert_eq!(lock_traits.shareable, AutoTraitStatus::Yes);

    let read_guard_traits = layouts.resolve_auto_traits("Std::Sync::RwLockReadGuard<int>");
    assert_eq!(read_guard_traits.thread_safe, AutoTraitStatus::Yes);
    assert_eq!(read_guard_traits.shareable, AutoTraitStatus::Yes);

    let write_guard_traits = layouts.resolve_auto_traits("Std::Sync::RwLockWriteGuard<int>");
    assert_eq!(write_guard_traits.thread_safe, AutoTraitStatus::Yes);
    assert_eq!(write_guard_traits.shareable, AutoTraitStatus::Yes);
}

#[test]
fn std_sync_condvar_and_once_are_trait_safe() {
    let layouts = TypeLayoutTable::default();
    let condvar_traits = layouts.resolve_auto_traits("Std::Sync::Condvar");
    assert_eq!(condvar_traits.thread_safe, AutoTraitStatus::Yes);
    assert_eq!(condvar_traits.shareable, AutoTraitStatus::Yes);

    let once_traits = layouts.resolve_auto_traits("Std::Sync::Once");
    assert_eq!(once_traits.thread_safe, AutoTraitStatus::Yes);
    assert_eq!(once_traits.shareable, AutoTraitStatus::Yes);

    let callback_traits = layouts.resolve_auto_traits("Std::Sync::OnceCallback");
    assert_eq!(callback_traits.thread_safe, AutoTraitStatus::Yes);
    assert_eq!(callback_traits.shareable, AutoTraitStatus::Yes);
}

#[test]
fn di_optional_injection_skips_missing_registration() {
    let (_module, result) = parse_and_check(
        r#"
@service
public class Consumer
{
    @inject
    public init(@inject(optional: true) Dependency dep) { }
}

public class Dependency { }
"#,
    );
    assert!(
        !result_contains(&result, "DI0001"),
        "unexpected DI0001 diagnostic for optional injection: {:?}",
        result.diagnostics
    );
}

#[test]
fn accepts_lock_guard_when_traits_satisfied() {
    let layouts = layouts_with_struct(
        "std::sync::Mutex",
        AutoTraitSet::all_yes(),
        AutoTraitOverride {
            thread_safe: Some(true),
            shareable: Some(true),
            copy: None,
        },
    );

    let constraints = Vec::new();
    let module = module_with_struct("std::sync::Mutex");
    let result = check_module(&module, &constraints, &layouts);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}
