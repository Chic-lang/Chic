use super::common::RequireExt;
use super::*;

#[test]
fn lowers_trait_impl_and_builds_vtable() {
    let source = r#"
namespace Demo;

public interface Formatter
{
    string Format();
}

public class Widget { }

public class WidgetFormatter : Formatter
{
    string Format() { return "widget"; }
}
"#;

    let parsed = parse_module(source).require("parse trait impl module");
    let lowering = lower_module(&parsed.module);

    assert_eq!(lowering.module.trait_vtables.len(), 1);
    let vtable = &lowering.module.trait_vtables[0];
    assert_eq!(
        vtable.symbol,
        "__vtable_Demo__Formatter__Demo__WidgetFormatter"
    );
    assert_eq!(vtable.trait_name, "Demo::Formatter");
    assert_eq!(vtable.impl_type, "Demo::WidgetFormatter");
    assert_eq!(vtable.slots.len(), 1);
    assert_eq!(vtable.slots[0].method, "Format");
    assert_eq!(vtable.slots[0].symbol, "Demo::WidgetFormatter::Format");

    let method_names: Vec<_> = lowering
        .module
        .functions
        .iter()
        .map(|func| func.name.as_str())
        .collect();
    assert!(
        method_names.contains(&"Demo::WidgetFormatter::Format"),
        "trait impl method missing from function list: {:?}",
        method_names
    );
}

#[test]
fn lowers_trait_object_call_with_dispatch_metadata() {
    let source = r#"
namespace Demo;

public interface Formatter
{
    void Touch();
}

public class Widget { }

public class WidgetFormatter : Formatter
{
    void Touch() { }
}

public void Render(dyn Formatter fmt)
{
    fmt.Touch();
}
"#;

    let parsed = parse_module(source).require("parse trait object call module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let render = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Render"))
        .expect("Render function");
    let entry = &render.body.blocks[0];
    let Terminator::Call { dispatch, .. } =
        entry.terminator.as_ref().expect("Render entry terminator")
    else {
        panic!("Render entry terminator is not a call");
    };
    let dispatch = dispatch
        .as_ref()
        .and_then(|entry| match entry {
            crate::mir::CallDispatch::Trait(info) => Some(info),
            _ => None,
        })
        .expect("dyn call dispatch metadata");
    assert_eq!(dispatch.trait_name, "Demo::Formatter");
    assert_eq!(dispatch.method, "Touch");
    assert_eq!(dispatch.slot_index, 0);
    assert_eq!(dispatch.slot_count, 1);
    assert_eq!(dispatch.receiver_index, 0);
}

#[test]
fn diagnoses_async_trait_impl_mismatch() {
    let source = r#"
namespace Demo;
import Std.Async;

public interface Worker
{
    async Task Run();
}

public class Runner { }

public class RunnerImpl : Worker
{
    void Run() { }
}
"#;

    let parsed = parse_module(source).require("parse async trait module");
    let lowering = lower_module(&parsed.module);
    let messages: Vec<_> = lowering
        .diagnostics
        .iter()
        .map(|diag| diag.message.clone())
        .collect();
    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("mismatches asyncness of `Run`")),
        "expected async mismatch diagnostic, got {messages:?}"
    );
}
