use std::hint::black_box;
use std::time::Instant;

use chic::syntax::expr::{format_expression, parse_expression};

fn main() {
    let source =
        "async (in int count, string label) => label + count * (count - 1) ?? Logger.Format(count)";
    let iterations = 20_000;

    let mut parse_total = 0f64;
    for _ in 0..iterations {
        let start = Instant::now();
        let expr = parse_expression(source).expect("expression should parse");
        parse_total += start.elapsed().as_secs_f64();
        black_box(expr);
    }
    let parse_avg = parse_total / iterations as f64;

    let parsed = parse_expression(source).expect("expression should parse");
    let mut format_total = 0f64;
    for _ in 0..iterations {
        let start = Instant::now();
        let formatted = format_expression(&parsed);
        format_total += start.elapsed().as_secs_f64();
        black_box(formatted);
    }
    let format_avg = format_total / iterations as f64;

    println!("parse average: {:.3}µs", parse_avg * 1e6);
    println!("format average: {:.3}µs", format_avg * 1e6);
}
