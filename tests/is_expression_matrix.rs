use chic::const_eval_config::{self, ConstEvalConfig};
use chic::frontend::parser::parse_module;
use chic::mir::lower_module;
use chic::typeck::check_module;

struct Case {
    name: &'static str,
    source: &'static str,
    expectation: Expectation,
}

enum Expectation {
    Success,
    Failure(&'static [&'static str]),
}

#[test]
fn is_expression_matrix() {
    let cases: &[Case] = &[
        Case {
            name: "basic_is_results",
            source: r#"
// VALID: Basic runtime true/false cases.
public class Basics
{
    public void Run()
    {
        var o1 = "hello";
        var o2 = 123;

        var v1 = o1 is string;
        var v2 = o2 is int;
        var v3 = o1 is int;
        var v4 = o2 is string;

        if (o1 is string s)
        {
        }

        if (o2 is string s2)
        {
        }
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "inheritance_and_interfaces",
            source: r#"
// VALID: Class inheritance and interface checks.
public class Animal { }
public class Dog : Animal { }
public class Cat : Animal { }

public interface IWalk { }
public interface IMeow { }

public class WalkingDog : Dog, IWalk { }
public class HouseCat : Cat, IMeow { }

public class Examples
{
    public void Run()
    {
        var a = new WalkingDog();

        var e1 = a is Animal;
        var e2 = a is Dog;
        var e3 = a is IWalk;
        var e4 = a is IMeow;
        var e5 = a is HouseCat;
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "value_types_and_boxing",
            source: r#"
// VALID: Value types and boxing interactions.
public class ValueIsExamples
{
    public void Run()
    {
        var boxedInt = 42;
        var boxedDouble = 42.0;

        var b1 = boxedInt is int;
        var b2 = boxedInt is long;
        var b3 = boxedDouble is int;

        var x = 10;

        var b4 = x is int;
        var b5 = x is null;
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "null_checks_and_overloaded_equals",
            source: r#"
// VALID: `is null` bypasses operator overloading.
public sealed class Weird
{
    public static bool operator ==(Weird a, Weird b) => true;
    public static bool operator !=(Weird a, Weird b) => !(a == b);
}

public class NullExamples
{
    public void Run()
    {
        var w = new Weird();
        var eq1 = (w == null);
        var eq2 = w is null;
        var eq3 = w is not null;
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_null_check",
            source: r#"
// VALID: Generic `is null` works for unconstrained type parameters.
public class GenericNulls
{
    public static bool IsNull<T>(T value)
    {
        return value is null;
    }

    public void Run()
    {
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "patterns_and_relational_tests",
            source: r#"
// VALID: Pattern variables, property patterns, and relational patterns.
public class PatternExamples
{
    public void Run()
    {
        var value = 12;

        if (value is int number)
        {
        }

        var numberCheck = 10;
        var p1 = numberCheck is > 0;
        var p2 = numberCheck is < 0;
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "always_true_or_false_constructs",
            source: r#"
// VALID: Odd but legal `is` usages.
public class AlwaysCases
{
    public void Run()
    {
        var o = 123;
        var a = o is object;

        var value = 10;
        var b = value is null;

        var maybe = 1;
        var c = maybe is null;
        var d = maybe is int;
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "is_not_shorthand",
            source: r#"
// VALID: `is not` mirrors `is`.
public class NotPatterns
{
    public void Run()
    {
        var maybeObj = "hi";

        if (maybeObj is not null)
        {
        }

        if (maybeObj is not int)
        {
        }
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "invalid_void_in_is",
            source: r#"
// INVALID: `void` cannot appear in an `is` expression.
public class InvalidVoid
{
    public void Run()
    {
        var o2 = new object();
        var bad1 = o2 is void; // ERROR
    }
}
"#,
            expectation: Expectation::Failure(&["void"]),
        },
        Case {
            name: "invalid_open_generic_is",
            source: r#"
// INVALID: open generic type in `is` expression.
namespace System.Collections.Generic
{
    public class List<T> { }
}

public class InvalidOpenGeneric
{
    public void Run()
    {
        var o2 = new System.Collections.Generic.List<int>();
        var bad2 = o2 is System.Collections.Generic.List<>; // ERROR
    }
}
"#,
            expectation: Expectation::Failure(&["unknown identifier"]),
        },
        Case {
            name: "invalid_type_parameter_pattern",
            source: r#"
// INVALID: Type parameters cannot be used directly in `is` outside a generic scope.
namespace System.Collections.Generic
{
    public class List<T> { }
}

public class InvalidTypeParameterUse
{
    public void Run()
    {
        var o2 = new System.Collections.Generic.List<int>();
        var bad3 = o2 is System.Collections.Generic.List<T>; // ERROR
    }
}
"#,
            expectation: Expectation::Failure(&["T"]),
        },
        Case {
            name: "invalid_pointer_is_pattern",
            source: r#"
// INVALID: `is` pattern matching is not valid on pointers.
public unsafe class PointerInvalid
{
    public static int* MakePtr() { return null; }

    public void Run()
    {
        var ptr = MakePtr();
        var bad4 = ptr is string; // ERROR
    }
}
"#,
            expectation: Expectation::Failure(&["pointer"]),
        },
    ];

    for case in cases {
        const_eval_config::set_global(ConstEvalConfig::default());
        let parse = parse_module(case.source);
        match parse {
            Ok(parsed) => {
                let mut messages: Vec<String> = parsed
                    .diagnostics
                    .iter()
                    .map(|diag| diag.message.clone())
                    .collect();
                let module = parsed.module;
                let lowering = lower_module(&module);
                messages.extend(lowering.diagnostics.iter().map(|diag| diag.message.clone()));
                let typeck = check_module(
                    &module,
                    &lowering.constraints,
                    &lowering.module.type_layouts,
                );
                messages.extend(typeck.diagnostics.iter().map(|diag| diag.message.clone()));

                match &case.expectation {
                    Expectation::Success => {
                        assert!(
                            messages.is_empty(),
                            "case `{}` expected success but produced diagnostics: {:?}",
                            case.name,
                            messages
                        );
                    }
                    Expectation::Failure(snippets) => {
                        assert!(
                            !messages.is_empty(),
                            "case `{}` expected diagnostics but none were produced",
                            case.name
                        );
                        for snippet in *snippets {
                            assert!(
                                messages.iter().any(|msg| msg.contains(snippet)),
                                "case `{}` expected diagnostic containing `{snippet}`, found {:?}",
                                case.name,
                                messages
                            );
                        }
                    }
                }
            }
            Err(err) => {
                let messages: Vec<String> = err
                    .diagnostics()
                    .iter()
                    .map(|diag| diag.message.clone())
                    .collect();
                match &case.expectation {
                    Expectation::Success => panic!(
                        "case `{}` failed during parse with diagnostics: {:?}",
                        case.name, messages
                    ),
                    Expectation::Failure(snippets) => {
                        for snippet in *snippets {
                            assert!(
                                messages.iter().any(|msg| msg.contains(snippet)),
                                "case `{}` expected diagnostic containing `{snippet}` during parse, found {:?}",
                                case.name,
                                messages
                            );
                        }
                    }
                }
            }
        }
    }
}
