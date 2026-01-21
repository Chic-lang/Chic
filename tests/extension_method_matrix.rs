use chic::const_eval_config::{self, ConstEvalConfig};
use chic::frontend::parser::parse_module;
use chic::mir::lower_module;
use chic::typeck::check_module;

struct Case {
    name: &'static str,
    source: String,
    expectation: Expectation,
}

enum Expectation {
    Success,
    Failure(&'static [&'static str]),
}

const SUPPORTING_TYPES: &str = r#"
public interface IEntity { }

public class Customer : IEntity { }

public interface IEnumerable<T> { }

public interface ICollection<T> { }

public interface IList<T> : ICollection<T> { }

public interface IDictionary<TKey, TValue> { }

public class Func<T1, T2> { }
"#;

fn source(body: &str) -> String {
    format!("{SUPPORTING_TYPES}\n{body}")
}

#[test]
#[ignore = "Matrix uses legacy C#-style extension methods; Chic uses `extension` blocks (SPEC.md §2.4). Update test cases separately from this refactor PR."]
fn extension_method_matrix() {
    let cases: &[Case] = &[
        Case {
            name: "simple_extension_on_string",
            source: source(
                r#"
// VALID: Simple extension on a non-generic type.
public static class StringExtensions
{
    public static bool IsNullOrEmptyOrWhiteSpace(this string value)
    {
        return true;
    }
}
"#,
            ),
            expectation: Expectation::Success,
        },
        Case {
            name: "extension_on_interface",
            source: source(
                r#"
// VALID: Extension on an interface type.
public static class EntityExtensions
{
    public static bool IsNullEntity(this IEntity entity)
    {
        return entity == null;
    }
}
"#,
            ),
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_extension_method",
            source: source(
                r#"
// VALID: Generic extension method on a non-generic static class.
public static class EnumerableExtensions
{
    public static IEnumerable<TResult> Map<TSource, TResult>(
        this IEnumerable<TSource> source,
        Func<TSource, TResult> selector)
    {
        return source;
    }
}
"#,
            ),
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_extension_with_constraints",
            source: source(
                r#"
// VALID: Generic extension method with constraints.
public interface IRepository<T> { }

public static class RepositoryExtensions
{
    public static void FindById<T, TKey>(
        this IRepository<T> repository,
        TKey id)
        where T : class, IEntity
    {
    }
}
"#,
            ),
            expectation: Expectation::Success,
        },
        Case {
            name: "extension_on_generic_collection",
            source: source(
                r#"
// VALID: Extension on a generic collection type.
public static class ListExtensions
{
    public static void AddIfNotNull<T>(this IList<T> list, T item)
        where T : class
    {
    }
}
"#,
            ),
            expectation: Expectation::Success,
        },
        Case {
            name: "extension_on_nullable_value_type",
            source: source(
                r#"
// VALID: Extension on a nullable value type.
public static class NullableIntExtensions
{
    public static int GetValueOrDefaultZero(this int value)
    {
        return value;
    }
}
"#,
            ),
            expectation: Expectation::Success,
        },
        Case {
            name: "extension_on_generic_interface_with_constraint",
            source: source(
                r#"
// VALID: Extension on a generic interface with constraint.
public static class CollectionExtensions
{
    public static bool IsNullOrEmpty<T>(this ICollection<T> collection)
    {
        return collection == null;
    }
}
"#,
            ),
            expectation: Expectation::Success,
        },
        Case {
            name: "extension_with_independent_method_type_parameter",
            source: source(
                r#"
// VALID: Extension method with its own type parameter independent of the type being extended.
public static class ObjectExtensions
{
    public static TResult Pipe<TSource, TResult>(
        this TSource source,
        Func<TSource, TResult> func)
    {
        return func(source);
    }
}
"#,
            ),
            expectation: Expectation::Success,
        },
        Case {
            name: "method_with_this_in_non_static_class",
            source: source(
                r#"
// INVALID: Method uses `this` parameter but is not in a static class.
public class BadExtensions1
{
    // Compile-time error: extension methods must be static methods in a static class.
    public void DoSomething(this string value)
    {
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["non-generic static class"]),
        },
        Case {
            name: "static_method_in_non_static_class",
            source: source(
                r#"
// INVALID: Static method with `this`, but the containing class is not static.
public class BadExtensions2
{
    public static void DoSomethingElse(this string value)
    {
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["non-generic static class"]),
        },
        Case {
            name: "nested_static_extension_class",
            source: source(
                r#"
// INVALID: Extension method defined in a nested static class.
public static class OuterExtensions
{
    public static class InnerExtensions
    {
        public static void InnerBad(this string value)
        {
        }
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["non-nested static class"]),
        },
        Case {
            name: "generic_static_extension_class",
            source: source(
                r#"
// INVALID: Extension method defined in a generic static class.
public static class GenericExtensions<T>
{
    public static void BadGenericContainer(this T value)
    {
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["non-generic static class"]),
        },
        Case {
            name: "not_an_extension_method",
            source: source(
                r#"
// VALID C# BUT NOT AN EXTENSION METHOD: missing `this` modifier.
public static class NotActuallyExtension
{
    public static void NotExtension(string value)
    {
    }
}
"#,
            ),
            expectation: Expectation::Success,
        },
        Case {
            name: "undefined_generic_parameter_in_signature",
            source: source(
                r#"
// INVALID: Generic parameters are mismatched / undefined.
public static class BadGenericSignature
{
    public static void BrokenAdd<TKey>(
        this IDictionary<TKey, TValue> dict, // TValue is not in scope here
        TKey key,
        TValue value) // ERROR: TValue undefined in method type parameters
    {
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["unknown type `TValue`"]),
        },
        Case {
            name: "local_function_extension_method",
            source: source(
                r#"
// INVALID: Local function cannot be an extension method.
public static class LocalFunctionBadExtensions
{
    public static void SomeMethod()
    {
        function void LocalBad(this string s)
        {
        }
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["non-generic static class"]),
        },
    ];

    for case in cases {
        const_eval_config::set_global(ConstEvalConfig::default());
        let parse = parse_module(case.source.as_str());
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
