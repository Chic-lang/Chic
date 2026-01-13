use super::common::{RequireExt, assert_no_pending};
use super::*;
use crate::language::{LanguageFeatures, set_language_features};

#[test]
fn first_class_span_conversions_and_inference() {
    set_language_features(LanguageFeatures {
        first_class_spans: true,
    });
    let source = r#"
import Std.Span;

namespace Sample;

public static class MemoryExtensions
{
    public static bool StartsWith<T>(this ReadOnlySpan<T> span, T value)
    {
        return true;
    }
}

public static class Harness
{
    public static bool Use(int[] input, string text)
    {
        return TakesSpan(input)
            && TakesReadOnly(input)
            && TakesReadOnly(ToSpan(input))
            && TakesUtf8(text)
            && MemoryExtensions.StartsWith(input, 1);
    }

    public static Span<int> ToSpan(Span<int> span) => span;
    public static bool TakesSpan(Span<int> span) => span.Length >= 0;
    public static bool TakesReadOnly(ReadOnlySpan<int> span) => span.Length >= 0;
    public static bool TakesUtf8(ReadOnlySpan<byte> span) => span.Length >= 0;

    public static bool AnyEqual<T>(ReadOnlySpan<T> span, T value)
    {
        return true;
    }

    public static bool Inference(int[] items)
    {
        return AnyEqual(items, 2);
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("Use"))
        .expect("missing Use function");
    assert_no_pending(&use_fn.body);

    let inference_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("Inference"))
        .expect("missing Inference function");
    assert_no_pending(&inference_fn.body);
}
