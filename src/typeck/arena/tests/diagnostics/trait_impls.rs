use super::fixtures::layouts_with_struct;
use super::{ArenaDiagnosticCase, ArenaDiagnosticFixture, Expectation, run_cases};
use crate::frontend::parser::parse_module;
use crate::mir::{AutoTraitOverride, AutoTraitSet};
use crate::typeck::arena::TypeCheckResult;

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::parsed(
        "class_missing_interface_method_emits_error",
        r#"
public interface IPrintable { void Print(in this); }

public class Report : IPrintable { }
"#,
        Expectation::contains(&["missing implementation for `Print`"]),
    ),
    ArenaDiagnosticCase::parsed(
        "trait_impl_missing_method_reports_error",
        r#"
namespace Demo;

public interface Printable
{
    string Display();
}

public class Widget { }

public class WidgetPrinter : Printable
{
}
"#,
        Expectation::contains(&["missing implementation for `Display`"]),
    ),
    ArenaDiagnosticCase::parsed(
        "trait_impl_skips_default_method",
        r#"
namespace Demo;

public interface Greeter
{
    string Hello() { return "hello"; }
}

public class Person { }

public class GreeterPerson : Greeter
{
}
"#,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::parsed(
        "trait_impl_missing_associated_type_reports_error",
        r#"
namespace Demo;

public interface Iterable
{
    type Item;
}

public class Numbers { }

        public class NumbersIterable : Iterable
        {
        }
        "#,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::lowered(
        "trait_impl_requires_thread_safe_when_trait_marked_thread_safe",
        r#"
namespace Demo;

@thread_safe
public interface Worker
{
    void Run();
}

@not_thread_safe
public struct Payload
{
    public void Run() { }
}

        public class PayloadRunner : Worker
        {
            public void Run() { }
        }
"#,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::lowered(
        "trait_impl_respects_trait_auto_trait_requirement_when_satisfied",
        r#"
namespace Demo;

@thread_safe
public interface Worker
{
    void Run();
}

@thread_safe
public struct Payload
{
    public void Run() { }
}

        public class PayloadRunner : Worker
        {
            public void Run() { }
        }
"#,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::parsed(
        "trait_object_requires_associated_type_defaults",
        r#"
namespace Demo;

public interface Iterable
{
    type Item;
}

public struct Holder
{
    public dyn Iterable Value;
}
"#,
        Expectation::contains(&["missing a default"]),
    ),
    ArenaDiagnosticCase::parsed(
        "trait_object_rejects_self_returning_methods",
        r#"
namespace Demo;

public interface Cloner
{
    Self Clone();
}

public struct Holder
{
    public dyn Cloner Value;
}
"#,
        Expectation::contains(&["returns `Self`"]),
    ),
    ArenaDiagnosticCase::custom(
        "repr_c_layout_mismatch_reports_spec_link",
        repr_c_layout_mismatch_reports_spec_link,
        Expectation::contains(&["`@repr(c)`", "SPEC.md#7-interop-unixmacos-focus"]),
    ),
    ArenaDiagnosticCase::parsed(
        "duplicate_trait_impl_reports_error",
        r#"
namespace Demo;

public interface Printable
{
    string Display();
}

public class Widget { }

public class WidgetPrinter : Printable
{
    string Display() { return "Widget"; }
}

public class WidgetPrinter2 : Printable
{
    string Display() { return "Duplicate"; }
}
"#,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::parsed(
        "generic_trait_impl_not_supported",
        r#"
namespace Demo;

public interface Formatter
{
    string Format(int value);
}

public class Logger<T> { }

public class LoggerFormatter<T> : Formatter
{
    string Format(int value) { return "generic"; }
}
"#,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::parsed(
        "inherent_impl_not_supported",
        r#"
namespace Demo;

public class Helper { }

public class HelperExtensions { public void Reset() { } }
"#,
        Expectation::clean(),
    ),
];

fn repr_c_layout_mismatch_reports_spec_link(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let parsed = parse_module(
        r#"
namespace Demo;

@repr(c)
public struct Broken { }
"#,
    )
    .expect("parse repr struct");
    let layouts = layouts_with_struct(
        "Demo::Broken",
        AutoTraitSet::all_yes(),
        AutoTraitOverride::default(),
    );
    fixture.check_module(&parsed.module, &[], &layouts)
}

#[test]
fn trait_impl_diagnostics() {
    run_cases("trait_impls", CASES);
}
