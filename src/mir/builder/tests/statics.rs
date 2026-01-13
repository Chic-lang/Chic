use super::*;
use crate::mir::builder::tests::common::RequireExt;
use crate::mir::data::{ConstValue, Operand, PendingOperand, Rvalue, StatementKind, Terminator};

#[test]
fn static_field_access_emits_static_ops() {
    let source = r"
namespace Demo;

public class Config
{
    public static int Version;

    public static void Set(int value)
    {
        Config.Version = value;
    }

    public static int Read()
    {
        return Config.Version;
    }
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let set_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Config::Set")
        .expect("missing Config::Set");
    assert!(
        set_fn
            .body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .any(|stmt| matches!(stmt.kind, StatementKind::StaticStore { .. })),
        "expected static store in Config::Set"
    );

    let read_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Config::Read")
        .expect("missing Config::Read");
    assert!(
        read_fn
            .body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .any(|stmt| matches!(
                stmt.kind,
                StatementKind::Assign {
                    value: Rvalue::StaticLoad { .. },
                    ..
                }
            )),
        "expected static load in Config::Read"
    );
}

#[test]
fn static_fields_are_accessible_by_simple_name() {
    let source = r#"
namespace Demo;

public class Parser
{
    private static readonly string Space = " ";

    public static string Read()
    {
        return Space;
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let read_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Parser::Read")
        .expect("missing Parser::Read");

    let has_static_load = read_fn.body.blocks.iter().any(|block| {
        block.statements.iter().any(|stmt| {
            matches!(
                stmt.kind,
                StatementKind::Assign {
                    value: Rvalue::StaticLoad { .. },
                    ..
                }
            )
        })
    });

    assert!(has_static_load, "Parser::Read should load the static field");
}

#[test]
fn static_properties_are_accessible_by_simple_name() {
    let source = r#"
namespace Demo;

public class Parser
{
    public static string Name { get; set; }

    public static string Copy(string value)
    {
        Name = value;
        return Name;
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let copy_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Parser::Copy")
        .expect("missing Parser::Copy");

    let mut setter_args = None;
    let mut getter_args = None;

    for block in &copy_fn.body.blocks {
        if let Some(Terminator::Call { func, args, .. }) = &block.terminator {
            if matches!(
                func,
                Operand::Pending(PendingOperand { repr, .. })
                if repr == "Demo::Parser::set_Name"
            ) {
                setter_args = Some(args.len());
            } else if matches!(
                func,
                Operand::Pending(PendingOperand { repr, .. })
                if repr == "Demo::Parser::get_Name"
            ) {
                getter_args = Some(args.len());
            }
        }
    }

    assert_eq!(
        setter_args,
        Some(1),
        "setter should only receive the value argument"
    );
    assert_eq!(getter_args, Some(0), "getter should not receive a receiver");
}

#[test]
fn static_property_calls_accessors_without_receiver() {
    let source = r"
namespace Demo;

public class Config
{
    public static string Name { get; set; }

    public static string CurrentName()
    {
        return Config.Name;
    }

    public static void SetName(string value)
    {
        Config.Name = value;
    }
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let getter_call = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Config::CurrentName")
        .and_then(|func| {
            func.body
                .blocks
                .iter()
                .find_map(|block| match &block.terminator {
                    Some(Terminator::Call { func, args, .. })
                        if matches!(
                            func,
                            Operand::Pending(PendingOperand { repr, .. })
                            if repr == "Demo::Config::get_Name"
                        ) =>
                    {
                        Some(args.len())
                    }
                    _ => None,
                })
        })
        .expect("expected getter call in CurrentName");
    assert_eq!(
        getter_call, 0,
        "static property getter should not pass a receiver"
    );

    let setter_call = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Config::SetName")
        .and_then(|func| {
            func.body
                .blocks
                .iter()
                .find_map(|block| match &block.terminator {
                    Some(Terminator::Call { func, args, .. })
                        if matches!(
                            func,
                            Operand::Pending(PendingOperand { repr, .. })
                            if repr == "Demo::Config::set_Name"
                        ) =>
                    {
                        Some(args.len())
                    }
                    _ => None,
                })
        })
        .expect("expected setter call in SetName");
    assert_eq!(
        setter_call, 1,
        "static property setter should only receive the value argument"
    );
}

#[test]
fn using_static_field_ambiguity_reports_diagnostic() {
    let source = r#"
import static Demo.Alpha;
import static Demo.Beta;

namespace Demo;

public class Alpha
{
    public static int Value = 1;
}

public class Beta
{
    public static int Value = 2;
}

public class Consumer
{
    public int Read()
    {
        return Value;
    }
}
"#;

    let parsed = parse_module(source).require("parse static using ambiguity");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("ambiguous")),
        "expected ambiguity diagnostic, got {:?}",
        lowering.diagnostics
    );
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("Alpha::Value")),
        "expected diagnostic to cite Alpha::Value, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn using_static_method_ambiguity_reports_diagnostic() {
    let source = r#"
import static Demo.Alpha;
import static Demo.Beta;

namespace Demo;

public class Alpha
{
    public static int Build()
    {
        return 1;
    }
}

public class Beta
{
    public static int Build()
    {
        return 2;
    }
}

public class Consumer
{
    public int Read()
    {
        return Build();
    }
}
"#;

    let parsed = parse_module(source).require("parse static using ambiguity for methods");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("ambiguous")),
        "expected ambiguity diagnostic, got {:?}",
        lowering.diagnostics
    );
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("Alpha::Build")),
        "expected diagnostic to cite Alpha::Build, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn using_static_const_resolves_without_receiver() {
    let source = r#"
import static Demo.Constants;

namespace Demo;

public static class Constants
{
    public const int Answer = 42;
}

public class Consumer
{
    public int Read()
    {
        return Answer;
    }
}
"#;

    let parsed = parse_module(source).require("parse static using const");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let read_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Consumer::Read")
        .expect("missing Consumer::Read");

    let has_const_answer = read_fn
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .any(|stmt| match &stmt.kind {
            StatementKind::Assign { value, .. } => matches!(
                value,
                Rvalue::Use(Operand::Const(constant))
                    if matches!(constant.value, ConstValue::Int(42))
            ),
            _ => false,
        });

    assert!(
        has_const_answer,
        "expected Consumer::Read to lower import static const reference into a const operand"
    );
}

#[test]
fn module_level_statics_lower_to_static_ops() {
    let source = r#"
namespace Demo;

static const int Answer = 7;
static mut int Counter = 1;

public static int Read()
{
    return Answer;
}

public static void Write(int value)
{
    unsafe {
        Counter = value;
    }
}
"#;

    let parsed = parse_module(source).require("parse module statics");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    assert_eq!(lowering.module.statics.len(), 2);
    let answer = lowering
        .module
        .statics
        .iter()
        .find(|var| var.qualified == "Demo::Answer")
        .expect("missing Answer static");
    assert!(answer.is_readonly, "Answer should be immutable");
    assert!(matches!(answer.initializer, Some(ConstValue::Int(7))));

    let counter = lowering
        .module
        .statics
        .iter()
        .find(|var| var.qualified == "Demo::Counter")
        .expect("missing Counter static");
    assert!(!counter.is_readonly, "Counter should be mutable");
    assert!(matches!(counter.initializer, Some(ConstValue::Int(1))));

    let read_body = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Read")
        .expect("missing Read function");
    let has_static_load = read_body.body.blocks.iter().any(|block| {
        block.statements.iter().any(|stmt| {
            matches!(
                stmt.kind,
                StatementKind::Assign {
                    value: Rvalue::StaticLoad { .. },
                    ..
                }
            )
        })
    });
    assert!(has_static_load, "Read should load from Answer static");

    let write_body = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Write")
        .expect("missing Write function");
    let has_static_store = write_body.body.blocks.iter().any(|block| {
        block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, StatementKind::StaticStore { .. }))
    });
    assert!(has_static_store, "Write should store into Counter static");
}
