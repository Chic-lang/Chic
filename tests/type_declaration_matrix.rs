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
fn type_declaration_matrix() {
    let cases: &[Case] = &[
        Case {
            name: "simple_inheritance",
            source: r#"
// VALID: Simple base class and derived class.
public class SimpleBase { }
public class SimpleDerived : SimpleBase { }
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "implements_multiple_interfaces",
            source: r#"
// VALID: Class implementing multiple interfaces.
public interface IReadable { }
public interface IWritable { }

public class StreamAdapter : IReadable, IWritable { }
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "abstract_base_concrete_derived",
            source: r#"
// VALID: Abstract base class with a concrete subclass.
public abstract class AnimalBase { }
public class Cat : AnimalBase { }
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_base_closed_child",
            source: r#"
// VALID: Generic base class with non-generic derived class.
public class GenericBase<T> { }
public class IntGenericDerived : GenericBase<int> { }
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "interface_constraint_on_generic",
            source: r#"
// VALID: Generic class constrained to an interface.
public interface IEntityMarker { }

public class Repository<T> where T : IEntityMarker { }
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_interface_concrete_impl",
            source: r#"
// VALID: Generic interface and concrete implementation using specific type arguments.
public interface IService<TRequest, TResponse> { }

public class LoginService : IService<string, bool> { }
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "interface_inherits_multiple",
            source: r#"
// VALID: Interface inheriting from multiple other interfaces.
public interface IHasId { }
public interface IHasName { }

public interface IEntityInfo : IHasId, IHasName { }
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_class_implements_generic_interface",
            source: r#"
// VALID: Generic class implementing a generic interface.
public interface IRepository<T> { }

public class Order { }

public class OrderRepository : IRepository<Order> { }
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "sealed_class_implements_interface",
            source: r#"
// VALID: Sealed class implementing an interface (no inheritance beyond this point).
public interface ILoggable { }

public sealed class FileLogger : ILoggable { }
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "internal_inheritance_chain",
            source: r#"
// VALID: Internal base class with internal derived class (accessibility is consistent).
internal class InternalBase { }
internal class InternalDerived : InternalBase { }
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "reject_multiple_base_classes",
            source: r#"
// INVALID: Class cannot inherit from multiple base *classes*.
public class BaseA { }
public class BaseB { }

public class InvalidMultipleBaseClasses : BaseA, BaseB { }  // ERROR
"#,
            expectation: Expectation::Failure(&["multiple base"]),
        },
        Case {
            name: "reject_inaccessible_base",
            source: r#"
// INVALID: Public class cannot have a less accessible (internal) base class.
internal class InternalOnlyBase { }

public class InvalidAccessDerived : InternalOnlyBase { }    // ERROR: inconsistent accessibility
"#,
            expectation: Expectation::Failure(&["InternalOnlyBase"]),
        },
        Case {
            name: "reject_inaccessible_interface",
            source: r#"
// INVALID: Public class cannot implement a less accessible (internal) interface.
internal interface IInternalFeature { }

public class InvalidAccessImplements : IInternalFeature { } // ERROR: inconsistent accessibility
"#,
            expectation: Expectation::Failure(&["IInternalFeature"]),
        },
        Case {
            name: "interface_cannot_inherit_class",
            source: r#"
// INVALID: Interface cannot inherit from a class.
public class NonInterfaceBase { }

public interface InvalidInterfaceInheritance : NonInterfaceBase { } // ERROR
"#,
            expectation: Expectation::Failure(&["NonInterfaceBase"]),
        },
        Case {
            name: "reject_sealed_base",
            source: r#"
// INVALID: Cannot derive from a sealed class.
public sealed class ClosedBase { }

public class InvalidSealedInheritance : ClosedBase { } // ERROR
"#,
            expectation: Expectation::Failure(&["ClosedBase"]),
        },
        Case {
            name: "reject_abstract_sealed_class",
            source: r#"
// INVALID: Class cannot be both abstract and sealed.
public abstract sealed class InvalidAbstractSealed { } // ERROR
"#,
            expectation: Expectation::Failure(&["abstract and sealed"]),
        },
        Case {
            name: "reject_static_base",
            source: r#"
// INVALID: Cannot inherit from a static class.
public static class StaticUtilityBase { }

public class InvalidStaticInheritance : StaticUtilityBase { } // ERROR
"#,
            expectation: Expectation::Failure(&["StaticUtilityBase"]),
        },
        Case {
            name: "generic_multiple_class_constraints",
            source: r#"
// INVALID: Generic type parameter cannot have more than one class-type constraint.
public class BaseOne { }
public class BaseTwo { }

public class InvalidGenericBaseConstraints<T> where T : BaseOne, BaseTwo { } // ERROR
"#,
            expectation: Expectation::Failure(&["BaseTwo"]),
        },
        Case {
            name: "new_constraint_ordering",
            source: r#"
// INVALID: The 'new()' constraint must be last in the constraint list.
public class InvalidGenericConstraintOrder<T> where T : new(), class { } // ERROR
"#,
            expectation: Expectation::Failure(&["`new()` constraint must be the final constraint"]),
        },
        Case {
            name: "interface_cycle",
            source: r#"
// INVALID: Interfaces cannot form an inheritance cycle.
public interface ICircularA : ICircularB { } // ERROR (part of cycle)
public interface ICircularB : ICircularA { } // ERROR (part of cycle)
"#,
            expectation: Expectation::Failure(&["ICircularA", "ICircularB"]),
        },
        Case {
            name: "generic_repository_constraint_order_valid",
            source: r#"
// VALID: Generic repository implementing generic interface with proper constraint order.
public interface IRepository<T> { }

public class Repository<T> : IRepository<T>
    where T : class, new()
{
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_repository_constraint_order_invalid",
            source: r#"
// INVALID: Constraint order is wrong (`new()` must be last).
public interface IRepository<T> { }

public class InvalidRepositoryConstraintOrder<T> : IRepository<T>
    where T : new(), class
{
}
"#,
            expectation: Expectation::Failure(&["`new()` constraint must be the final constraint"]),
        },
        Case {
            name: "derived_adds_compatible_constraints",
            source: r#"
// VALID: Generic base with constraint; derived adds extra constraints (still consistent).
public class BaseRepository<T>
    where T : class
{
}

public class DerivedRepository<T> : BaseRepository<T>
    where T : class, System.IDisposable, new()
{
}

namespace System
{
    public interface IDisposable { }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "derived_conflicts_with_base_constraints",
            source: r#"
// INVALID: Derived generic class conflicts with base constraints (no type can satisfy both).
public class ConflictingBase<T>
    where T : class
{
}

public class ConflictingDerived<T> : ConflictingBase<T>
    where T : struct
{
}
"#,
            expectation: Expectation::Failure(&["ConflictingBase"]),
        },
        Case {
            name: "multi_constraint_repository_valid",
            source: r#"
// VALID: Multiple type parameters with different constraints.
public class MultiConstraintRepository<TKey, TValue>
    where TKey : struct
    where TValue : class
{
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "generic_param_class_and_struct",
            source: r#"
// INVALID: Generic parameter cannot have both `class` and `struct` constraints.
public class InvalidClassAndStruct<T>
    where T : class, struct
{
}
"#,
            expectation: Expectation::Failure(&["struct` and `class`"]),
        },
        Case {
            name: "open_generic_base_closed_child",
            source: r#"
// VALID: Open generic base, closed generic derived.
public class OpenBase<TFirst, TSecond>
{
}

public class ClosedDerived : OpenBase<int, string>
{
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "invalid_generic_arity",
            source: r#"
// INVALID: Wrong generic arity when specifying base type.
public class OpenBase<TFirst, TSecond>
{
}

public class InvalidGenericArity : OpenBase<int>
{
}
"#,
            expectation: Expectation::Failure(&["type `OpenBase` expects"]),
        },
        Case {
            name: "nested_generic_constraint_outer_reference",
            source: r#"
// VALID: Nested generic class with constraint referencing outer type parameter.
public class Outer<T>
{
    public class Inner<U>
        where U : T
    {
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "covariant_interface_producer",
            source: r#"
// VALID: Covariant interface and implementing class.
public interface IProducer<out T> { }

public class Producer<T> : IProducer<T>
{
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "interface_with_constraint_repeated_on_impl",
            source: r#"
// VALID: Interface with constraint; implementing class repeats compatible constraint.
public interface IFactory<T>
    where T : new()
{
}

public class DefaultFactory<T> : IFactory<T>
    where T : new()
{
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "interface_constraint_missing_on_impl",
            source: r#"
// INVALID: Implementing class does not satisfy interface’s generic constraint.
public interface IStrictFactory<TStrict>
    where TStrict : new()
{
}

public class InvalidFactory<TStrict> : IStrictFactory<TStrict>
{
}
"#,
            expectation: Expectation::Failure(&["IStrictFactory"]),
        },
        Case {
            name: "combined_constraints_reference_interface",
            source: r#"
// VALID: Generic class with multiple constraints: reference type, interface, and `new()`.
public interface IEntity { }

public interface IRepository<T> { }

public class ConstrainedEntityRepository<T> : IRepository<T>
    where T : class, IEntity, new()
{
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "multiple_base_class_constraints",
            source: r#"
// INVALID: Multiple base-class constraints on a single type parameter.
public class BaseOne { }
public class BaseTwo { }

public class InvalidMultipleBaseClassConstraints<T>
    where T : BaseOne, BaseTwo
{
}
"#,
            expectation: Expectation::Failure(&["BaseTwo"]),
        },
        Case {
            name: "interface_inheritance_and_generic_impl",
            source: r#"
// VALID: Interface inheritance combined with generic implementation.
public interface IReadable { }
public interface IWritable { }

public interface IReadableWritable : IReadable, IWritable { }

public class StreamAdapter : IReadableWritable
{
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "public_class_internal_generic_interface",
            source: r#"
// INVALID: Public class implementing less-accessible generic interface.
internal interface IInternalInterface<T> { }

public class InvalidAccessGeneric<T> : IInternalInterface<T>
{
}
"#,
            expectation: Expectation::Failure(&["IInternalInterface"]),
        },
        Case {
            name: "internal_class_public_interface",
            source: r#"
// VALID: Internal class implementing public interface (access level is not weakened).
public interface IVisible<T> { }

internal class InternalVisibleImpl<T> : IVisible<T>
{
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "sealed_generic_base_rejected",
            source: r#"
// INVALID: Sealed generic class used as base.
public sealed class SealedBase<T>
{
}

public class InvalidSealedGenericInheritance<T> : SealedBase<T>
{
}
"#,
            expectation: Expectation::Failure(&["SealedBase"]),
        },
        Case {
            name: "struct_constraint_with_interface",
            source: r#"
// VALID: Generic constraint combining `struct` with interface.
public interface IValueMarker { }

public class ValueContainer<T>
    where T : struct, IValueMarker
{
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
