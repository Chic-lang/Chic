use super::common::{
    RequireExt, assert_drop_sequence, assert_no_defer_drop, deinit_index, drop_index,
    storage_dead_index,
};
use super::*;
use std::collections::HashSet;

#[test]
fn locals_drop_on_explicit_and_implicit_return() {
    let source = r"
namespace Cleanup
{
    public struct Disposable { public void dispose(ref this) { } }

    public void Explicit()
    {
        var d = new Disposable();
        return;
    }

    public void Implicit()
    {
        var d = new Disposable();
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let explicit = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Explicit"))
        .require("Explicit function");
    let implicit = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Implicit"))
        .require("Implicit function");

    let explicit_local = explicit
        .body
        .locals
        .iter()
        .enumerate()
        .find_map(|(idx, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name == "d")
                .then(|| LocalId(idx))
        })
        .require("explicit local `d`");
    let implicit_local = implicit
        .body
        .locals
        .iter()
        .enumerate()
        .find_map(|(idx, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name == "d")
                .then(|| LocalId(idx))
        })
        .require("implicit local `d`");

    assert_drop_sequence(&explicit.body, explicit_local, "explicit local drop", true);

    assert_drop_sequence(&implicit.body, implicit_local, "implicit local drop", true);
}

#[test]
fn lock_statement_emits_guard_drop_sequence() {
    let source = r"
namespace Std.Sync
{
    public struct LockGuard
    {
        public void dispose(ref this) { }
    }

    public struct Lock
    {
        public LockGuard Enter()
        {
            return new LockGuard();
        }
    }
}

namespace Sync
{
    public void Critical() { }

    public void Use(Std.Sync.Lock mutex)
    {
    lock (mutex)
    {
        Critical();
    }
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Use"))
        .require("Use function");
    assert!(!func.is_generator);
    assert!(func.body.generator.is_none());
    assert_no_defer_drop(&func.body);

    let guard_locals: HashSet<LocalId> = func
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter_map(|stmt| match &stmt.kind {
            MirStatementKind::Drop { place, .. } => Some(place.local),
            _ => None,
        })
        .collect();
    assert!(
        !guard_locals.is_empty(),
        "lock lowering should emit a drop for the guard"
    );
    for local in guard_locals {
        let label = format!("lock guard {local:?}");
        assert_drop_sequence(&func.body, local, &label, false);
    }
}

#[test]
fn lock_with_break_drops_guard() {
    let source = r"
namespace Std.Sync
{
    public struct LockGuard
    {
        public void dispose(ref this) { }
    }

    public struct Lock
    {
        public LockGuard Enter()
        {
            return new LockGuard();
        }
    }
}

namespace Sample
{
    public void Loop(Std.Sync.Lock gate)
    {
        while (true)
        {
            lock (gate)
            {
                break;
            }
        }
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Loop"))
        .require("Loop function");

    let mut guard_dropped = false;
    for block in &func.body.blocks {
        if matches!(block.terminator, Some(Terminator::Goto { .. }))
            && block
                .statements
                .iter()
                .any(|stmt| matches!(stmt.kind, MirStatementKind::Drop { .. }))
        {
            guard_dropped = true;
            break;
        }
    }
    assert!(
        guard_dropped,
        "lock should drop its guard even when exiting via break"
    );
}

#[test]
fn lowers_lock_with_early_return_drops_guard_before_return() {
    let source = r"
namespace Std.Sync
{
    public struct LockGuard
    {
        public void dispose(ref this) { }
    }

    public struct Lock
    {
        public LockGuard Enter()
        {
            return new LockGuard();
        }
    }
}

namespace Sync
{
    public int Use(Std.Sync.Lock mutex)
    {
    lock (mutex)
    {
        return 1;
    }
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Use"))
        .require("Use function");
    assert_no_defer_drop(&func.body);

    let return_block = func
        .body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::Return)))
        .require("expected return block");

    let drop_position = return_block
        .statements
        .iter()
        .enumerate()
        .find_map(|(idx, stmt)| match &stmt.kind {
            MirStatementKind::Drop { place, .. } => Some((idx, place)),
            _ => None,
        })
        .require("return block should drop the lock guard before returning");
    let drop_local = drop_position.1.local;
    let label = format!("lock guard early-return {drop_local:?}");
    assert_drop_sequence(&func.body, drop_local, &label, false);
    let has_deinit = deinit_index(&func.body, drop_local).is_some();
    let deinit_before_drop = return_block.statements.iter().take(drop_position.0).any(
        |stmt| matches!(&stmt.kind, MirStatementKind::Deinit(place) if place.local == drop_local),
    );
    if has_deinit {
        assert!(
            deinit_before_drop,
            "lock guard drop in the return block should be preceded by a deinit"
        );
    }
    assert!(
        drop_position.1.projection.is_empty(),
        "drop should target the guard local directly"
    );
}

#[test]
fn using_expression_emits_drop_sequence() {
    let source = r"
namespace Lifetime;

public class Disposable { public void Dispose() { } }

public void Run(Disposable disposable)
{
using (disposable)
{
    disposable.Dispose();
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Run"))
        .require("Run function");
    assert_no_defer_drop(&func.body);

    let mut drop_locals = HashSet::new();
    for block in &func.body.blocks {
        for stmt in &block.statements {
            if let MirStatementKind::Drop { ref place, .. } = stmt.kind {
                drop_locals.insert(place.local);
            }
        }
    }
    assert!(
        !drop_locals.is_empty(),
        "using lowering should inject an explicit drop for the disposable"
    );
    for local in drop_locals {
        let label = format!("using expression resource {local:?}");
        assert_drop_sequence(&func.body, local, &label, false);
    }
}

#[test]
fn using_declaration_emits_drop_sequence() {
    let source = r"
namespace Sample;

public class Disposable { }

public void Critical() { }

public void Use(Disposable d)
{
using var handle = d;
Critical();
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Use"))
        .require("Use function");
    assert_no_defer_drop(&func.body);

    let mut drop_locals = HashSet::new();
    for block in &func.body.blocks {
        for stmt in &block.statements {
            if let MirStatementKind::Drop { ref place, .. } = stmt.kind {
                drop_locals.insert(place.local);
            }
        }
    }
    assert!(
        !drop_locals.is_empty(),
        "using declaration should produce a drop for the resource"
    );
    for local in drop_locals {
        let label = format!("using declaration resource {local:?}");
        assert_drop_sequence(&func.body, local, &label, false);
    }
}

#[test]
fn using_scope_drops_before_goto() {
    let source = r"
namespace Cleanup;

public struct Disposable { public void dispose(ref this) { } }

public void UsingGoto(bool flag)
{
    using (new Disposable())
    {
        if (flag)
        {
            goto after;
        }
    }
after:
    ;
}
";
    let parsed = parse_module(source).require("parse");

    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::UsingGoto"))
        .require("UsingGoto function");
    assert_no_defer_drop(&func.body);

    let handle_local = func
        .body
        .locals
        .iter()
        .enumerate()
        .find_map(|(idx, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name.starts_with("__using_resource_"))
                .then(|| LocalId(idx))
        })
        .require("using handle local");
    assert_drop_sequence(&func.body, handle_local, "using goto resource", true);
}

#[test]
fn using_scope_drops_before_throw_dispatch() {
    let source = r"
namespace Cleanup;

public struct Disposable { public void dispose(ref this) { } }
public class Exception { }
public class MyException : Exception { }

public void UsingThrow()
{
    try
    {
        using var handle = new Disposable();
        throw new MyException();
    }
    catch (MyException ex)
    {
    }
}
";
    let parsed = parse_module(source).require("parse");

    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::UsingThrow"))
        .require("UsingThrow function");
    assert_no_defer_drop(&func.body);

    let handle_local = func
        .body
        .locals
        .iter()
        .enumerate()
        .find_map(|(idx, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name == "handle")
                .then(|| LocalId(idx))
        })
        .require("using handle local");
    assert_drop_sequence(&func.body, handle_local, "using throw resource", true);
}

#[test]
fn lowers_unsafe_block_into_marker_statements() {
    let source = r"
namespace Unsafe;

public void Work()
{
unsafe
{
    DoSomething();
}
}

public void DoSomething() { }
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Work"))
        .require("Work function");

    let mut enter = 0;
    let mut exit = 0;
    for block in &func.body.blocks {
        for stmt in &block.statements {
            match stmt.kind {
                MirStatementKind::EnterUnsafe => enter += 1,
                MirStatementKind::ExitUnsafe => exit += 1,
                _ => {}
            }
        }
    }

    assert_eq!(enter, 1, "expected one EnterUnsafe statement");
    assert_eq!(exit, 1, "expected one ExitUnsafe statement");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "integration-style MIR lowering test requires full control-flow fixture"
)]
fn lowers_fixed_statement_with_unique_borrow_and_pinned_guard() {
    let source = r"
namespace Memory;

public struct Buffer { public int Bytes; }

public void Pin(Buffer buffer)
{
fixed (let ptr = (byte*)buffer.Bytes)
{
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Pin"))
        .require("Pin function");
    assert_no_defer_drop(&func.body);

    let mut saw_unique_borrow = false;
    let mut saw_address_of = false;
    for block in &func.body.blocks {
        for stmt in &block.statements {
            match &stmt.kind {
                MirStatementKind::Borrow { kind, .. } => {
                    if *kind == BorrowKind::Unique {
                        saw_unique_borrow = true;
                    }
                }
                MirStatementKind::Assign { value, .. } => {
                    if matches!(value, Rvalue::AddressOf { .. }) {
                        saw_address_of = true;
                    }
                }
                _ => {}
            }
        }
    }
    assert!(
        saw_unique_borrow,
        "fixed should emit a unique borrow for the pinned place"
    );
    assert!(
        saw_address_of,
        "fixed should materialise an address-of for the pointer binding"
    );

    let guard_local = func
        .body
        .locals
        .iter()
        .enumerate()
        .find_map(|(idx, decl)| decl.is_pinned.then_some(LocalId(idx)))
        .require("fixed lowering should create a pinned guard local");
    let guard_decl = func
        .body
        .locals
        .get(guard_local.0)
        .require("guard local declaration");
    assert!(
        guard_decl.is_pinned,
        "guard local must be marked as pinned for borrow checking"
    );

    let guard_storage_dead = storage_dead_index(&func.body, guard_local).is_some();
    assert!(
        guard_storage_dead,
        "fixed guard should be dropped in the lexical fallthrough path"
    );

    let label = format!("fixed guard {guard_local:?}");
    assert_drop_sequence(&func.body, guard_local, &label, false);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "integration-style MIR lowering test exercises nested resource scopes"
)]
fn lowers_nested_using_and_fixed_drop_order() {
    let source = r"
namespace Lifetime;

public class Disposable { }

public struct Buffer { public int Bytes; }

public void Touch() { }

public void Manage(Disposable disposable, Buffer buffer)
{
using (disposable)
{
    fixed (let ptr = (byte*)buffer.Bytes)
    {
        Touch();
    }
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Manage"))
        .require("Manage function");
    let body = &func.body;
    assert_no_defer_drop(body);

    let ptr_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| decl.name.as_deref() == Some("ptr"))
        .map(|(idx, _)| LocalId(idx))
        .require("expected fixed pointer local");
    let guard_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name.starts_with("__fixed_guard_"))
        })
        .map(|(idx, _)| LocalId(idx))
        .require("expected fixed guard local");
    let using_local = body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name.starts_with("__using_resource_"))
        })
        .map(|(idx, _)| LocalId(idx))
        .require("expected using resource local");

    let pointer_dead_idx =
        storage_dead_index(body, ptr_local).require("fixed pointer should be StorageDead");
    let guard_dead_idx =
        storage_dead_index(body, guard_local).require("fixed guard should be StorageDead");
    let using_dead_idx =
        storage_dead_index(body, using_local).require("using resource should be StorageDead");
    let guard_drop_idx = drop_index(body, guard_local).require("fixed guard should emit a drop");
    let using_drop_idx = drop_index(body, using_local).require("using resource should emit a drop");

    let guard_label = format!("fixed guard {guard_local:?}");
    assert_drop_sequence(body, guard_local, &guard_label, false);
    let using_label = format!("using resource {using_local:?}");
    assert_drop_sequence(body, using_local, &using_label, false);

    assert!(
        pointer_dead_idx < guard_dead_idx,
        "pointer binding should be released before the guard"
    );
    assert!(
        guard_dead_idx < using_dead_idx,
        "fixed guard should be dropped before the using resource"
    );
    assert!(
        guard_drop_idx < using_drop_idx,
        "guard drop should execute before the enclosing using resource drop"
    );
}

#[test]
fn using_expression_without_body_emits_storage_dead() {
    let source = r"
namespace Lifetime;

public class Disposable { }

public void Run(Disposable disposable)
{
using (disposable);
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Run"))
        .require("Run function");
    assert_no_defer_drop(&func.body);

    let resource_local = func
        .body
        .locals
        .iter()
        .enumerate()
        .find(|(_, decl)| {
            decl.name
                .as_deref()
                .is_some_and(|name| name.contains("__using_resource"))
        })
        .map(|(idx, _)| LocalId(idx))
        .require("using lowering should create a resource local");

    let has_storage_dead = storage_dead_index(&func.body, resource_local).is_some();
    assert!(
        has_storage_dead,
        "using without body should still drop the resource local"
    );
    let label = format!("using statement resource {resource_local:?}");
    assert_drop_sequence(&func.body, resource_local, &label, false);
}

#[test]
fn using_declaration_without_initializer_reports_pending() {
    let source = r"
namespace Lifetime;

public class Disposable { }

public void Run()
{
using var handle;
}
";
    let parse_result = parse_module(source);
    assert!(
        parse_result.is_err(),
        "parser should reject using declarations without an initializer"
    );
    let err = parse_result.err().unwrap();
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("initializer required for this declaration")),
        "expected diagnostic about missing initializer, got {:#?}",
        err.diagnostics()
    );
}
