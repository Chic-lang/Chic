use super::{ArenaDiagnosticCase, Expectation, run_cases};

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::lowered(
        "returning_ref_parameter_emits_cl0031",
        r#"
namespace Borrow;

public class Samples
{
    public string ReturnRef(ref string value)
    {
        return value;
    }
}
"#,
        Expectation::contains(&["[CL0031]", "[CLL0001]"]),
    ),
    ArenaDiagnosticCase::lowered(
        "storing_ref_parameter_to_field_emits_cl0031",
        r#"
namespace Borrow;

public struct Holder
{
    public string Value;
}

public class Cache
{
    public void Remember(ref string source)
    {
        var holder = new Holder();
        holder.Value = source;
    }
}
"#,
        Expectation::contains(&["[CL0031]"]),
    ),
    ArenaDiagnosticCase::lowered(
        "storing_in_parameter_to_local_emits_cl0031",
        r#"
namespace Borrow;

public class Aliasings
{
    public void Alias(in string value)
    {
        let copy = value;
    }
}
"#,
        Expectation::contains(&["[CL0031]", "[CLL0001]"]),
    ),
    ArenaDiagnosticCase::lowered(
        "capturing_ref_parameter_in_closure_emits_cl0031",
        r#"
import Std;

namespace Borrow;

public class Closures
{
    public void Capture(ref string value)
    {
        let closure = () => value;
    }
}
"#,
        Expectation::contains(&["[CL0031]", "[CLL0001]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "lends_requires_view_return_and_targets_exist",
        r#"
namespace Borrow;

public str MissingView(in view str src) lends(src);
public view str MissingParam(in view str src) lends(other);
public view str ValueParam(string src) lends(src);
"#,
        Expectation::contains(&["[TCK181]", "[TCK180]", "[TCK182]", "[TCK183]"]),
    ),
];

#[test]
fn borrow_diagnostics() {
    run_cases("borrows", CASES);
}
