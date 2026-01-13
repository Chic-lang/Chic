use super::{ArenaDiagnosticCase, Expectation, run_cases};

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::parsed(
        "object_initializer_unknown_member_reports_error",
        r#"
public class Widget
{
    public int Value { get; set; }
}

public class Factory
{
    public Widget Build() => new Widget { Missing = 1 };
}
"#,
        Expectation::contains(&["[TCK133]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "object_initializer_private_field_reports_error",
        r#"
public class Widget
{
    private int Hidden;
}

public class Factory
{
    public Widget Build() => new Widget { Hidden = 1 };
}
"#,
        Expectation::contains(&["[TCK134]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "object_initializer_internal_member_from_other_namespace_reports_error",
        r#"
namespace Models
{
    public class Counter
    {
        internal int Value;
    }
}

namespace Builders
{
    public class Factory
    {
        public Models.Counter Build() => new Models.Counter { Value = 1 };
    }
}
"#,
        Expectation::contains(&["[TCK134]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "object_initializer_get_only_property_reports_error",
        r#"
public class Widget
{
    public int Value { get; }
}

public class Factory
{
    public Widget Build() => new Widget { Value = 42 };
}
"#,
        Expectation::contains(&["[TCK135]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "struct_initializer_missing_required_members_reports_error",
        r#"
public struct Point
{
    public required int X;
    public required int Y;
}

public class Factory
{
    public Point Build() => new Point { X = 1 };
}
"#,
        Expectation::contains(&["[TCK136]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "object_initializer_duplicate_entries_report_error",
        r#"
public class Widget
{
    public int Value { get; set; }
}

public class Factory
{
    public Widget Build() => new Widget { Value = 1, Value = 2 };
}
"#,
        Expectation::contains(&["[TCK137]"]),
    ),
];

#[test]
fn initializer_diagnostics() {
    run_cases("initializers", CASES);
}
