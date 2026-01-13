use super::common::RequireExt;
use super::*;

#[test]
fn lowers_pointer_operations_inside_unsafe_blocks() {
    let source = r#"
namespace Sample;

public int Read(int value)
{
    unsafe
    {
        let ptr = &value;
        return *ptr;
    }
}

public void Write(int value)
{
    unsafe
    {
        var mutableValue = value;
        let ptr = &mut mutableValue;
        *ptr = 10;
    }
}
"#;

    let parsed = parse_module(source).require("parse pointer module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let read_fn = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Read"))
        .require("Sample::Read");

    let mut saw_read_address = false;
    let mut saw_read_deref = false;
    for block in &read_fn.body.blocks {
        for stmt in &block.statements {
            if let MirStatementKind::Assign { value, .. } = &stmt.kind {
                match value {
                    Rvalue::AddressOf { mutability, .. } => {
                        saw_read_address = true;
                        assert_eq!(*mutability, Mutability::Immutable);
                    }
                    Rvalue::Unary {
                        op: UnOp::Deref, ..
                    } => saw_read_deref = true,
                    _ => {}
                }
            }
        }
    }
    assert!(saw_read_address, "expected address-of in Sample::Read body");
    assert!(saw_read_deref, "expected dereference in Sample::Read body");

    let write_fn = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Write"))
        .require("Sample::Write");

    let mut saw_write_address = false;
    for block in &write_fn.body.blocks {
        for stmt in &block.statements {
            if let MirStatementKind::Assign { value, .. } = &stmt.kind {
                if let Rvalue::AddressOf { mutability, .. } = value {
                    saw_write_address = true;
                    assert_eq!(*mutability, Mutability::Mutable);
                }
            }
        }
    }
    assert!(
        saw_write_address,
        "expected mutable address-of in Sample::Write body"
    );

    verify_body(&read_fn.body).require("verify read body");
    verify_body(&write_fn.body).require("verify write body");
}

#[test]
fn pointer_operations_require_unsafe_blocks() {
    let source = r#"
namespace Sample;

public int Read(int value)
{
    let ptr = &value;
    return *ptr;
}
"#;

    let parsed = parse_module(source).require("parse pointer safety module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("requires an `unsafe` block")),
        "expected unsafe diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn unsafe_function_allows_pointer_operations_without_block() {
    let source = r#"
namespace Sample;

public unsafe int Read(int value)
{
    let ptr = &value;
    return *ptr;
}
"#;

    let parsed = parse_module(source).require("parse unsafe function module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn calling_unsafe_function_requires_unsafe_context() {
    let source = r#"
namespace Sample;

public unsafe void Dangerous(int* ptr)
{
    *ptr = 42;
}

public void Caller(int* ptr)
{
    Dangerous(ptr);
}

public void SafeCaller(int* ptr)
{
    unsafe
    {
        Dangerous(ptr);
    }
}

public unsafe void UnsafeCaller(int* ptr)
{
    Dangerous(ptr);
}
"#;

    let parsed = parse_module(source).require("parse unsafe call module");
    let lowering = lower_module(&parsed.module);
    let messages = lowering
        .diagnostics
        .iter()
        .map(|diag| diag.message.clone())
        .collect::<Vec<_>>();

    assert!(
        messages.iter().any(|msg| msg
            .contains("call to unsafe function `Sample::Dangerous` requires an `unsafe` block")),
        "expected unsafe call diagnostic, found {:?}",
        messages
    );

    // Ensure only the safe caller triggers the diagnostic.
    assert_eq!(
        messages
            .iter()
            .filter(|msg| msg.contains("call to unsafe function"))
            .count(),
        1,
        "expected exactly one unsafe call diagnostic"
    );
}

#[test]
fn pointer_integer_cast_requires_expose_address() {
    let source = r#"
namespace Sample;

public unsafe ulong Addr(*mut byte ptr)
{
    unsafe
    {
        return ptr as ulong;
    }
}
"#;
    let parsed = parse_module(source).require("parse pointer cast module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("@expose_address")),
        "expected diagnostic about missing @expose_address, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn pointer_integer_cast_prompts_using_helpers_even_with_expose_address() {
    let source = r#"
namespace Sample;

public unsafe ulong Addr(*mut @expose_address byte ptr)
{
    unsafe
    {
        return ptr as ulong;
    }
}
"#;
    let parsed = parse_module(source).require("parse pointer cast ok module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("prefer dedicated pointer APIs")),
        "expected helper hint diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn uint_ptr_from_pointer_accepts_expose_address() {
    let source = r#"
namespace Std.Numeric
{
    public struct UIntPtr
    {
        private usize raw;

        public static UIntPtr From(nuint value)
        {
            var result = new UIntPtr();
            result.raw = 0;
            return result;
        }
    }

    public static class Pointer
    {
        public static UIntPtr HandleFrom<T>(*mut @expose_address T pointer)
        {
            return UIntPtr.From(0);
        }
    }
}

namespace Sample
{
    import Std.Numeric;

    public class Program
    {
        public unsafe void Run(*mut @expose_address byte ptr)
        {
            unsafe
            {
                var handle = Pointer.HandleFrom<byte>(ptr);
            }
        }
    }
}
"#;
    let parsed = parse_module(source).require("parse uint_ptr success module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
}
