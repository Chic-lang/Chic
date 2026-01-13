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
fn method_declaration_matrix() {
    let cases: &[Case] = &[
        Case {
            name: "simple_methods",
            source: r#"
// VALID: Simple non-generic methods in a class.
public class SimpleMethods
{
    public void DoWork()
    {
    }

    private int Add(int a, int b)
    {
        return a + b;
    }

    // Expression-bodied ("fat arrow") method.
    public int Multiply(int a, int b) => a * b;
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "abstract_override_valid",
            source: r#"
// VALID: Abstract method and overrides.
public abstract class AbstractBase
{
    public abstract void DoSomething();

    public virtual void Describe()
    {
    }
}

public class AbstractDerived : AbstractBase
{
    // VALID override of abstract method
    public override void DoSomething()
    {
    }

    // VALID override of virtual method
    public override void Describe()
    {
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "abstract_in_concrete_invalid",
            source: r#"
// INVALID: Abstract method in a non-abstract class.
public class InvalidAbstractInConcrete
{
    // INVALID: CS0513 - 'InvalidAbstractInConcrete.DoSomething()' is abstract
    // but it is contained in non-abstract class 'InvalidAbstractInConcrete'.
    public abstract void DoSomething();
}
"#,
            expectation: Expectation::Failure(&["must be declared `abstract`"]),
        },
        Case {
            name: "abstract_method_with_body_invalid",
            source: r#"
// INVALID: Abstract method with a body.
public abstract class InvalidAbstractWithBody
{
    // INVALID: CS0806 - Abstract method cannot have a body.
    public abstract void DoSomething()
    {
        // Not allowed for abstract methods.
    }
}
"#,
            expectation: Expectation::Failure(&["cannot declare a body"]),
        },
        Case {
            name: "sealed_override_chain_valid",
            source: r#"
// VALID: Virtual, override, and sealed override in a chain.
public class VirtualBase
{
    public virtual void Process()
    {
    }
}

public class VirtualMid : VirtualBase
{
    // VALID: override + sealed to prevent further overrides.
    public sealed override void Process()
    {
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "sealed_override_rejected",
            source: r#"
public class VirtualBase
{
    public virtual void Process()
    {
    }
}

public class VirtualMid : VirtualBase
{
    public sealed override void Process()
    {
    }
}

public class InvalidOverrideSealed : VirtualMid
{
    // INVALID: CS0239 - cannot override sealed member 'VirtualMid.Process()'.
    public override void Process()
    {
    }
}
"#,
            expectation: Expectation::Failure(&["sealed member"]),
        },
        Case {
            name: "override_non_virtual_invalid",
            source: r#"
// INVALID: override when base method is not virtual/abstract.
public class NonVirtualBase
{
    public void Execute()
    {
    }
}

public class InvalidOverrideNonVirtual : NonVirtualBase
{
    // INVALID: CS0506 - 'Execute': cannot override inherited member
    // 'NonVirtualBase.Execute()' because it is not marked virtual, abstract, or override.
    public override void Execute()
    {
    }
}
"#,
            expectation: Expectation::Failure(&["no matching virtual member"]),
        },
        Case {
            name: "new_hides_base_valid",
            source: r#"
// VALID: Hiding a base method with 'new'.
public class HideBaseMethod
{
    public void Show()
    {
    }
}

public class HiddenMethod : HideBaseMethod
{
    // VALID: 'new' hides the inherited method instead of overriding.
    public new void Show()
    {
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_methods_valid",
            source: r#"
// VALID: Generic methods in a non-generic class.
public class GenericMethods
{
    // Simple generic method.
    public T Echo<T>(T value)
    {
        return value;
    }

    // Multiple type parameters with constraints.
    public TResult Combine<T1, T2, TResult>(T1 first, T2 second)
        where T1 : class
        where T2 : struct
        where TResult : new()
    {
    }

    // Expression-bodied generic method.
    public T GetDefault<T>()
    {
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_methods_in_generic_class",
            source: r#"
// VALID: Generic methods inside a generic class.
public class Container<TItem>
{
    private TItem _item;

    public init(TItem item)
    {
        _item = item;
    }

    // Uses the class-level generic parameter.
    public TItem GetItem()
    {
        return _item;
    }

    // Method-level generic parameter independent of class TItem.
    public TMethod EchoMethodType<TMethod>(TMethod value)
    {
        return value;
    }

    // Method-level generic parameter constrained by the class-level parameter.
    public TDerived CreateDerived<TDerived>()
        where TDerived : TItem, new()
    {
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "constraint_ordering_valid",
            source: r#"
// VALID: Generic constraint ordering (class/struct and new() last).
public class ConstraintOrderingValid
{
    // 'class' first, then interfaces/base types, then 'new()' last.
    public T CreateReference<T>()
        where T : class, new()
    {
    }

    // 'struct' first, then interfaces, 'new()' last (implicitly required for struct).
    public T CreateValue<T>()
        where T : struct
    {
    }

    // Multiple type parameters, each with its own constraints.
    public TResult Transform<TSource, TResult>(TSource source)
        where TSource : class
        where TResult : TSource, new()
    {
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "constraint_ordering_invalid",
            source: r#"
// INVALID: Generic constraint ordering (new() not last).
public class ConstraintOrderingInvalid
{
    // INVALID: 'new()' must be the last constraint.
    // CS0450 / CS0080 depending on compiler version.
    public T CreateBad<T>()
        where T : new(), class
    {
    }
}
"#,
            expectation: Expectation::Failure(&["`new()` constraint must be the final constraint"]),
        },
        Case {
            name: "shadowed_type_parameter_valid",
            source: r#"
// VALID (but confusing): Method-level type parameter shadows class-level type parameter.
public class ShadowedTypeParameter<T>
{
    // VALID: The method-level T hides the class-level T inside this method.
    // This is allowed but usually discouraged.
    public T MethodGeneric<T>(T value)
    {
        return value;
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "constraint_on_undeclared_type_param",
            source: r#"
// INVALID: Generic constraint on undeclared type parameter.
public class InvalidConstraintUndeclared
{
    // INVALID: CS0246 / CS0308 - 'U' is not a type parameter on this method.
    public void DoSomething<T>()
        where U : class
    {
    }
}
"#,
            expectation: Expectation::Failure(&["unknown type parameter `U`"]),
        },
        Case {
            name: "generic_overload_valid",
            source: r#"
// VALID: Overloading generic methods by different type-parameter counts.
public class GenericOverloadValid
{
    public void Configure<T>()
    {
    }

    public void Configure<T1, T2>()
    {
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_overload_invalid",
            source: r#"
// INVALID: Duplicate generic methods differing only in type parameter names.
public class GenericOverloadInvalid
{
    public void Setup<T>()
    {
    }

    // INVALID: CS0111 - Type parameter names do not contribute to the signature,
    // so this conflicts with 'Setup<T>()' above.
    public void Setup<U>()
    {
    }
}
"#,
            expectation: Expectation::Failure(&["duplicate overload"]),
        },
        Case {
            name: "static_virtual_invalid",
            source: r#"
// INVALID: Static virtual method in a class.
public class InvalidStaticVirtual
{
    // INVALID: 'static' and 'virtual' cannot be combined in a class method.
    public static virtual void DoWork()
    {
    }
}
"#,
            expectation: Expectation::Failure(&[
                "`virtual` modifier is not supported on static methods",
            ]),
        },
        Case {
            name: "static_abstract_invalid",
            source: r#"
// INVALID: Abstract static method in a class.
public class InvalidStaticAbstract
{
    // INVALID: 'static' and 'abstract' cannot be combined in a class method.
    public static abstract void DoWork();
}
"#,
            expectation: Expectation::Failure(&[
                "`abstract` modifier is not supported on static methods",
            ]),
        },
        Case {
            name: "generic_override_valid",
            source: r#"
// VALID: Generic override matching base generic method.
public class GenericBase
{
    public virtual T Identity<T>(T value)
    {
        return value;
    }
}

public class GenericOverride : GenericBase
{
    // VALID: Same generic parameter list and constraints as base method.
    public override T Identity<T>(T value)
    {
        return value;
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_override_constraint_mismatch_invalid",
            source: r#"
// INVALID: Override with different generic signature than base.
public class GenericBase2
{
    public virtual T Identity<T>(T value)
        where T : class
    {
        return value;
    }
}

public class InvalidGenericOverride : GenericBase2
{
    // INVALID: Signature does not match because the override removes the constraint.
    // CS0508 - return type must be 'T' and constraints must match base definition.
    public override T Identity<T>(T value)
    {
        return value;
    }
}
"#,
            expectation: Expectation::Failure(&["generic constraints", "GenericBase2::Identity"]),
        },
        Case {
            name: "type_parameter_used_without_declaration_invalid",
            source: r#"
// INVALID: Using a type parameter in a non-generic, non-generic-class context.
public class InvalidUseOfTypeParameter
{
    // INVALID: CS0246 - The type or namespace name 'T' could not be found.
    public void UseT(T value)
    {
    }
}
"#,
            expectation: Expectation::Failure(&["unknown type `T`"]),
        },
        Case {
            name: "class_type_parameter_usage_valid",
            source: r#"
// VALID: Using class-level type parameter inside methods.
public class ValidUseOfClassTypeParameter<T>
{
    public T Echo(T value)
    {
        return value;
    }

    public void CreateOrDefault(bool createNew)
    {
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "ref_out_generic_methods_valid",
            source: r#"
// VALID: Out/ref parameters with generics on method level.
public class RefOutGenericMethods
{
    public void Swap<T>(ref T left, ref T right)
    {
    }

    public void TryParseInt<T>(string value, out T result)
        where T : struct
    {
    }
}
"#,
            expectation: Expectation::Success,
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
