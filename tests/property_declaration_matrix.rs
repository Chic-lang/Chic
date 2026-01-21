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

const HEADER: &str = r#"
// SUPPORT TYPE USED BY SOME EXAMPLES
public class MyType { }

public class Dictionary<TKey, TValue>
{
    public TValue this[TKey key] { get; set; }
}

public class List<T> { }
"#;

const BASIC_HEADER: &str = r#"
public class MyType { }
"#;

const VALID_DECLARATIONS: &str = r#"
// VALID: Simple auto-implemented property with public get and set.
public class AutoPropertyExample
{
    public int Value { get; set; }
}

// VALID: Auto-implemented property with initializer.
public class AutoPropertyWithInitializer
{
    public int Value { get; set; } = 10;
}

// VALID: Auto-implemented property with private setter.
public class AutoPropertyPrivateSetter
{
    public string Name { get; private set; }
}

// VALID: Read-only auto-implemented property.
public class ReadOnlyAutoProperty
{
    public string Name { get; }
}

// VALID: Read-only auto-implemented property with initializer.
public class ReadOnlyAutoWithInitializer
{
    public int Id { get; } = 42;
}

// VALID: Auto-implemented property with init accessor.
public class InitOnlyProperty
{
    public string Name { get; init; }
}

// VALID: Required init property (must be set during initialization).
public class RequiredInitProperty
{
    public required string Name { get; init; }
}

// VALID: Static auto-implemented property.
public class StaticAutoProperty
{
    public static int InstanceCount { get; private set; }
}

// VALID: Property with explicit backing field and block-bodied accessors.
public class BackingFieldProperty
{
    private int _value;

    public int Value
    {
        get { return _value; }
        set { _value = value; }
    }
}

// VALID: Property with backing field and logic in setter.
public class ValidatingProperty
{
    private int _age;

    public int Age
    {
        get { return _age; }
        set
        {
            if (value < 0)
            {
                value = 0;
            }

            _age = value;
        }
    }
}

// VALID: Property with expression-bodied accessors.
public class ExpressionBodiedAccessors
{
    private int _value;

    public int Value
    {
        get => _value;
        set => _value = value;
    }
}

// VALID: Property with mixed accessor styles (expression-bodied get, block-bodied set).
public class MixedAccessorStyles
{
    private int _value;

    public int Value
    {
        get => _value;
        set { _value = value; }
    }
}

// VALID: Expression-bodied read-only property (no explicit backing field).
public class ExpressionBodiedProperty
{
    public int BaseValue { get; init; }

    public int Double => BaseValue * 2;
}

// VALID: Property with different accessor access levels.
public class DifferentAccessorAccessLevels
{
    public int Id { get; protected set; }
}

// VALID: Virtual property in base class.
public class VirtualPropertyBase
{
    public virtual string Description { get; set; }
}

// VALID: Override property in derived class.
public class VirtualPropertyDerived : VirtualPropertyBase
{
    public override string Description { get; set; }
}

// VALID: Abstract property in abstract base class.
public abstract class AbstractPropertyBase
{
    public abstract int Value { get; set; }
}

// VALID: Implementation of abstract property.
public class AbstractPropertyDerived : AbstractPropertyBase
{
    private int _value;

    public override int Value
    {
        get { return _value; }
        set { _value = value; }
    }
}

// VALID: Interface with property.
public interface IHasName
{
    string Name { get; set; }
}

// VALID: Explicit interface property implementation.
public class ExplicitInterfaceProperty : IHasName
{
    string IHasName.Name { get; set; }
}

// VALID: Indexer (property with a parameter) using backing array.
public class IndexerPropertyExample
{
    private readonly int[] _items = new int[10];

    public int this[int index]
    {
        get { return _items[index]; }
        set { _items[index] = value; }
    }
}

// VALID: Indexer with string key using backing Dictionary (pattern you mentioned).
public class MyClass
{
    private readonly Dictionary<string, MyType> collection = new();

    public MyType this[string name]
    {
        get { return collection[name]; }
        set { collection[name] = value; }
    }
}

// VALID: Expression-bodied indexer with string key.
public class ExpressionBodiedStringIndexer
{
    private readonly Dictionary<string, MyType> _collection = new();

    public MyType this[string name]
    {
        get => _collection[name];
        set => _collection[name] = value;
    }
}

// VALID: Read-only indexer.
public class ReadOnlyIndexer
{
    private readonly Dictionary<int, MyType> _collection = new();

    public MyType this[int index]
    {
        get { return _collection[index]; }
    }
}

// VALID: Indexer on a generic class.
public class GenericIndexer<TItem>
{
    private readonly Dictionary<string, TItem> _items = new();

    public TItem this[string key]
    {
        get => _items[key];
        set => _items[key] = value;
    }
}

// VALID: Indexer whose type is a constructed generic.
public class ConstructedGenericIndexer
{
    private readonly Dictionary<string, List<MyType>> _items = new();

    public List<MyType> this[string key]
    {
        get => _items[key];
        set => _items[key] = value;
    }
}

// VALID: Indexer with multiple parameters (e.g., 2D grid).
public class MultiParameterIndexer
{
    private readonly Dictionary<(int Row, int Column), MyType> _grid = new();

    public MyType this[int row, int column]
    {
        get => _grid[(row, column)];
        set => _grid[(row, column)] = value;
    }
}

// VALID: Indexer defined on an interface.
public interface IStringIndexer
{
    MyType this[string key] { get; set; }
}

// VALID: Explicit interface implementation of an indexer.
public class ExplicitIndexerImplementation : IStringIndexer
{
    private readonly Dictionary<string, MyType> _items = new();

    MyType IStringIndexer.this[string key]
    {
        get => _items[key];
        set => _items[key] = value;
    }
}

// VALID: Auto-implemented property in a struct.
public struct StructAutoProperty
{
    public int Value { get; set; }
}

// VALID: Read-only struct with init-only property.
public readonly struct ReadonlyStructProperty
{
    public int Value { get; init; }
}

// VALID: Generic class with generic property.
public class GenericClass<T>
{
    public T Item { get; set; }
}

// VALID: Generic class with two generic properties.
public class Pair<TKey, TValue>
{
    public TKey Key { get; set; }
    public TValue Value { get; set; }
}

// VALID: Non-generic property whose type is a constructed generic.
public class ConstructedGenericProperty
{
    public Dictionary<string, int> Map { get; set; }
}

// VALID: Property with nullable value type.
public class NullableProperty
{
    public int? OptionalNumber { get; set; }
}

// VALID: Property with tuple type.
public class TupleProperty
{
    public (int X, int Y) Coordinates { get; set; }
}

// VALID: Read-only computed property using other properties.
public class ComputedProperty
{
    public int Width { get; init; }
    public int Height { get; init; }

    public int Area => Width * Height;
}

// VALID: Property with only a getter and a private backing field (read-only).
public class ReadOnlyWithBackingField
{
    private int _id = 123;

    public int Id
    {
        get { return _id; }
    }
}
"#;

fn wrap(body: &str) -> String {
    format!("{HEADER}\n\n{body}")
}

fn wrap_basic(body: &str) -> String {
    format!("{BASIC_HEADER}\n\n{body}")
}

#[test]
fn property_declaration_matrix() {
    let cases: Vec<Case> = vec![
        Case {
            name: "all_valid_property_examples",
            source: wrap(VALID_DECLARATIONS),
            expectation: Expectation::Success,
        },
        Case {
            name: "invalid_set_only_auto_property",
            source: wrap_basic(
                r#"
// INVALID: Auto-implemented property must have a get accessor.
public class InvalidSetOnlyAutoProperty
{
    public int OnlySet { set; } // ERROR: Auto-implemented properties must have a get accessor.
}
"#,
            ),
            expectation: Expectation::Failure(&[
                "auto-implemented property must declare a `get` accessor",
            ]),
        },
        Case {
            name: "invalid_no_accessors",
            source: wrap_basic(
                r#"
// INVALID: Property must declare at least one accessor (get, set, or init).
public class InvalidNoAccessors
{
    public int BadProperty { } // ERROR: Expected accessor list.
}
"#,
            ),
            expectation: Expectation::Failure(&["at least one accessor"]),
        },
        Case {
            name: "invalid_expression_body_and_accessors",
            source: wrap_basic(
                r#"
// INVALID: Property cannot have both an expression body and an accessor list.
public class InvalidExpressionBodyAndAccessors
{
    private int _value;

    public int Bad => _value { get; set; } // ERROR: Cannot combine expression body with accessor list.
}
"#,
            ),
            expectation: Expectation::Failure(&["expression-bodied property", "accessor list"]),
        },
        Case {
            name: "invalid_property_with_parameters",
            source: wrap_basic(
                r#"
// INVALID: Property cannot have parameters like a method (only indexers use parameter lists).
public class InvalidPropertyWithParameters
{
    public int Wrong(int x) { get; set; } // ERROR: Properties cannot declare parameters this way.
}
"#,
            ),
            expectation: Expectation::Failure(&["properties cannot declare parameters"]),
        },
        Case {
            name: "invalid_void_property",
            source: wrap_basic(
                r#"
// INVALID: Property type cannot be void.
public class InvalidVoidProperty
{
    public void NoReturnProperty { get; set; } // ERROR: 'void' is not a valid property type.
}
"#,
            ),
            expectation: Expectation::Failure(&["void' is not a valid property type"]),
        },
        Case {
            name: "invalid_generic_property_usage",
            source: wrap_basic(
                r#"
// INVALID: Using generic type parameter T in property type without declaring T on the class.
public class InvalidGenericPropertyUsage
{
    public T Value { get; set; } // ERROR: The type or namespace name 'T' could not be found.
}
"#,
            ),
            expectation: Expectation::Failure(&["type `T`"]),
        },
        Case {
            name: "invalid_mixed_auto_and_block_accessor",
            source: wrap_basic(
                r#"
// INVALID: Mixed auto-accessor (semicolon) with block-bodied accessor in the same property.
public class InvalidMixedAutoAndBlockAccessor
{
    private int _value;

    public int Value
    {
        get;                     // Auto-accessor
        set { _value = value; }  // ERROR: Cannot mix auto-implemented accessor with block-bodied accessor.
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["auto-property accessors may not mix"]),
        },
        Case {
            name: "invalid_abstract_property_in_concrete_class",
            source: wrap_basic(
                r#"
// INVALID: Abstract property in a non-abstract class.
public class InvalidAbstractPropertyInConcreteClass
{
    public abstract int Value { get; set; } // ERROR: Only abstract classes can have abstract members.
}
"#,
            ),
            expectation: Expectation::Failure(&["must be declared `abstract`"]),
        },
        Case {
            name: "invalid_abstract_property_in_struct",
            source: wrap_basic(
                r#"
// INVALID: Property inside a struct cannot be abstract.
public struct InvalidAbstractPropertyInStruct
{
    public abstract int Value { get; set; } // ERROR: Structs cannot contain abstract members.
}
"#,
            ),
            expectation: Expectation::Failure(&[
                "`abstract` modifier is not supported on properties",
            ]),
        },
        Case {
            name: "invalid_named_indexer",
            source: wrap_basic(
                r#"
// INVALID: Indexer cannot have a name (must use 'this').
public class InvalidNamedIndexer
{
    public MyType Item[int index]   // ERROR: Indexer must be declared as 'this'.
    {
        get { return null!; }
        set { }
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["must be declared as 'this'"]),
        },
        Case {
            name: "invalid_static_indexer",
            source: wrap_basic(
                r#"
// INVALID: Indexer cannot be static.
public class InvalidStaticIndexer
{
    public static MyType this[int index]   // ERROR: Indexer cannot be static.
    {
        get { return null!; }
        set { }
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["indexer cannot be static"]),
        },
        Case {
            name: "invalid_parameterless_indexer",
            source: wrap_basic(
                r#"
// INVALID: Indexer must have at least one parameter.
public class InvalidParameterlessIndexer
{
    public MyType this[]   // ERROR: Indexer must declare at least one parameter.
    {
        get { return null!; }
        set { }
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["at least one parameter"]),
        },
        Case {
            name: "invalid_void_returning_indexer",
            source: wrap_basic(
                r#"
// INVALID: Indexer cannot return void.
public class InvalidVoidReturningIndexer
{
    public void this[int index]   // ERROR: 'void' is not a valid indexer return type.
    {
        get { }
        set { }
    }
}
"#,
            ),
            expectation: Expectation::Failure(&["void' is not a valid indexer return type"]),
        },
    ];

    for case in cases {
        const_eval_config::set_global(ConstEvalConfig::default());
        let parse = parse_module(case.source.as_str());
        match parse {
            Ok(parsed) => {
                if let Expectation::Success = &case.expectation {
                    let messages: Vec<String> = parsed
                        .diagnostics
                        .iter()
                        .map(|diag| diag.message.clone())
                        .collect();
                    assert!(
                        messages.is_empty(),
                        "case `{}` expected success but produced diagnostics: {:?}",
                        case.name,
                        messages
                    );
                    continue;
                }

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
