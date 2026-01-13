use super::{ArenaDiagnosticCase, Expectation, run_cases};

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::parsed(
        "missing_interface_property_accessor_reports_error",
        r#"
namespace Demo;

public interface IHasValue
{
    public int Value { get; set; }
}

public class Impl : IHasValue
{
    public int Value { get; }
}
"#,
        Expectation::contains(&["missing a `set` accessor"]),
    ),
    ArenaDiagnosticCase::parsed(
        "interface_inline_default_satisfies_requirement",
        r#"
namespace Geometry;

public interface IDrawable
{
    void Draw(in this) { }
}

public class Circle : IDrawable { }
"#,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::parsed(
        "interface_extension_default_applies_when_condition_satisfied",
        r#"
namespace Geometry;

public interface IRenderable { void Draw(in this); }
public interface IVisible { }

public extension IRenderable when Self : IVisible
{
    public default void Draw(in this) { }
}

public class Sprite : IRenderable, IVisible { }
"#,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::parsed(
        "default_extension_requires_interface_target",
        r#"
public struct Helper { }

public extension Helper
{
    public default void DoThing(in this) { }
}
"#,
        Expectation::contains(&["[DIM0001]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "conflicting_extension_defaults_emit_diagnostic",
        r#"
public interface IRenderable { void Draw(in this); }

namespace A
{
    public extension IRenderable
    {
        public default void Draw(in this) { }
    }
}

namespace B
{
    public extension IRenderable
    {
        public default void Draw(in this) { }
    }
}

public class Shape : IRenderable { }
"#,
        Expectation::contains(&["[DIM0003]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "retroactive_extension_default_satisfies_requirement",
        r#"
public interface IRenderable { void Draw(in this); }

public class Canvas : IRenderable { }

namespace Defaults
{
    public extension IRenderable
    {
        public default void Draw(in this) { }
    }
}
"#,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::parsed(
        "retroactive_extension_default_conflict_reports_dim0003",
        r#"
public interface IRenderable { void Draw(in this); }

public class Canvas : IRenderable { }

namespace Defaults
{
    public extension IRenderable
    {
        public default void Draw(in this) { }
    }
}

namespace Library
{
    public extension IRenderable
    {
        public default void Draw(in this) { }
    }
}
"#,
        Expectation::contains(&["[DIM0003]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "interface_method_signature_mismatch_reports_tck066",
        r#"
namespace Demo;

public class Base { }
public class Derived : Base { }

public interface IConsumer<in T>
{
    void Consume(T value);
}

public class BadConsumer : IConsumer<Base>
{
    public void Consume(Derived value) { }
}
"#,
        Expectation::contains(&["[TCK066]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "interface_property_type_mismatch_applies_substitution",
        r#"
namespace Demo;

public interface IBox<T>
{
    T Value { get; }
}

public class BadBox : IBox<int>
{
    public string Value { get; }
}
"#,
        Expectation::contains(&["requires `int`"]),
    ),
    ArenaDiagnosticCase::parsed(
        "static_property_mismatch_reports_error",
        r#"
public interface IConfig
{
    public string Name { get; }
}

public class Config : IConfig
{
    public static string Name { get; }
}
"#,
        Expectation::contains(&["must be declared instance"]),
    ),
];

#[test]
fn interface_diagnostics() {
    run_cases("interfaces", CASES);
}
