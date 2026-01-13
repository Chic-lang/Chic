use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use chic::syntax::expr::{format_expression, parse_expression};

fn bench_parse_format(c: &mut Criterion) {
    let source = "async (Foo input) => Foo(ref input, name: out result, in data[3]) ??\n        Helper.Transform($\"{input.Name,-10}: {result:0.00}\")";

    c.bench_function("expr_parse", |b| {
        b.iter(|| {
            let parsed = parse_expression(black_box(source)).expect("expression should parse");
            black_box(parsed);
        })
    });

    let parsed = parse_expression(source).expect("expression should parse");
    c.bench_function("expr_format", |b| {
        b.iter(|| {
            let formatted = format_expression(black_box(&parsed));
            black_box(formatted);
        })
    });
}

criterion_group!(expr_pretty, bench_parse_format);
criterion_main!(expr_pretty);
