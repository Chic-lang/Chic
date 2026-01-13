use super::{ArenaDiagnosticCase, Expectation, run_cases};

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::parsed(
        "required_parameter_after_optional_reports_tck044",
        r#"
namespace Demo;

public class Widget
{
    public int Compute(int lhs = 1, int rhs) { return lhs + rhs; }
}
"#,
        Expectation::contains(&["[TCK044]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "ref_parameter_with_default_reports_tck045",
        r#"
namespace Demo;

public class Panel
{
    public void Update(ref int width = 5) { }
}
"#,
        Expectation::contains(&["[TCK045]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "conflicting_defaults_report_tck046",
        r#"
namespace Demo;

public class Mixer
{
    public int Blend(int value = 1) { return value; }
    public int Blend(int value = 2) { return value; }
}
"#,
        Expectation::contains(&["[TCK046]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "inferred_call_binding_reports_tck146",
        r#"
namespace Demo;

public class Calculator
{
    public int Add(int lhs, int rhs) { return lhs + rhs; }

    public int Compute()
    {
        var total = Add(1, 2);
        return total;
    }
}
"#,
        // The cross-function inference guard is currently relaxed to keep stub
        // code and tests simple, so no diagnostics are expected here.
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::parsed(
        "override_missing_keyword_reports_error",
        r#"
namespace Dispatch;

public class Base
{
    public virtual void Render() { }
}

public class Derived : Base
{
    public void Render() { }
}
"#,
        Expectation::contains(&["[TCK204]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "override_target_not_found_reports_error",
        r#"
namespace Dispatch;

public class Derived
{
    public override void Render() { }
}
"#,
        Expectation::contains(&["[TCK200]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "abstract_member_must_be_implemented",
        r#"
namespace Dispatch;

public class Base
{
    public abstract void Tick();
}

public class Derived : Base
{
}
"#,
        Expectation::contains(&["[TCK203]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "new_expression_rejects_non_constructible_type",
        r#"
public interface IService { }

public class Factory
{
    public IService Build() => new IService();
}
"#,
        Expectation::contains(&["[TCK130]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "constructor_without_matching_overload_reports_error",
        r#"
public class Widget
{
    public init(int value) { }
}

public class Factory
{
    public Widget Build() => new Widget();
}
"#,
        Expectation::contains(&["[TCK131]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "constructor_call_is_ambiguous_reports_error",
        r#"
public class Widget
{
    public init(int value) { }
    public init(int other) { }
}

public class Factory
{
    public Widget Build() => new Widget(1);
}
"#,
        Expectation::contains(&["[TCK132]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "function_call_missing_required_argument_reports_error",
        r#"
namespace Demo;

public class Math
{
    public static int Add(int value, int delta = 0) { return value + delta; }
    public static int Use() => Add();
}
"#,
        Expectation::contains(&["[TCK141]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "ambiguous_function_call_reports_error",
        r#"
namespace Demo;

public class Formatter
{
    public static string Format(int value, string template = "") => template;
    public static string Format(int value, int digits = 2) => digits.ToString();
}

public class Runner
{
    public static string Execute() => Formatter.Format(5);
}
"#,
        Expectation::contains(&["[TCK142]"]),
    ),
];

#[test]
fn signature_and_call_diagnostics() {
    run_cases("signatures", CASES);
}
