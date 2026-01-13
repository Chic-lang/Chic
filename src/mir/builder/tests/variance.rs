use super::common::RequireExt;
use super::*;
use crate::type_metadata::TypeVariance;

fn variance_for<'a>(
    variances: &'a std::collections::HashMap<String, Vec<TypeVariance>>,
    name: &str,
) -> &'a [TypeVariance] {
    variances
        .get(name)
        .map(|values| values.as_slice())
        .unwrap_or_else(|| panic!("missing variance entry for `{name}`; map = {variances:?}"))
}

#[test]
fn type_variance_records_interface_annotations() {
    let source = r#"
namespace Sample;

public interface IProducer<out TResult>
{
    TResult Produce();
}

public interface IConsumer<in TValue>
{
    void Consume(TValue value);
}

public interface ITransducer<out TOutput, in TInput> : IProducer<TOutput>
{
    void Accept(TInput value);
}

public class Relay<T>
{
    public T Value;
}
"#;

    let parsed = parse_module(source).require("parse variance module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:#?}",
        lowering.diagnostics
    );

    let variance_map = &lowering.module.type_variance;
    let producer = variance_for(variance_map, "Sample::IProducer");
    assert_eq!(
        producer,
        [TypeVariance::Covariant],
        "`IProducer` should mark TResult covariant"
    );

    let consumer = variance_for(variance_map, "Sample::IConsumer");
    assert_eq!(
        consumer,
        [TypeVariance::Contravariant],
        "`IConsumer` should mark TValue contravariant"
    );

    let transducer = variance_for(variance_map, "Sample::ITransducer");
    assert_eq!(
        transducer,
        [TypeVariance::Covariant, TypeVariance::Contravariant],
        "`ITransducer` should propagate variance for both parameters"
    );

    let relay = variance_for(variance_map, "Sample::Relay");
    assert_eq!(
        relay,
        [TypeVariance::Invariant],
        "classes remain invariant even when interfaces use variance"
    );
}

#[test]
fn type_variance_records_delegate_annotations() {
    let source = r#"
namespace Sample;

public delegate TResult Converter<in T, out TResult>(T value);
public delegate void Action();
"#;

    let parsed = parse_module(source).require("parse delegate variance module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:#?}",
        lowering.diagnostics
    );

    let variance_map = &lowering.module.type_variance;
    let converter = variance_for(variance_map, "Sample::Converter");
    assert_eq!(
        converter,
        [TypeVariance::Contravariant, TypeVariance::Covariant],
        "`Converter` should propagate variance for both parameters"
    );

    let action = variance_map
        .get("Sample::Action")
        .map(|list| list.as_slice())
        .unwrap_or(&[]);
    assert!(
        action.is_empty(),
        "`Action` should not record variance entries"
    );
}
