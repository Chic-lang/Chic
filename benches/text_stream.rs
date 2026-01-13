use std::io;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use chic::chic_kind::ChicKind;
use chic::codegen::{TextStreamConfig, generate_text, stream_module};
use chic::frontend::parser::parse_module;
use chic::target::Target;

fn large_module_source(functions: usize) -> String {
    let mut out = String::from("namespace Benchmark;\n\n");
    for idx in 0..functions {
        out.push_str(&format!(
            "public double Fn{idx}(double x)\n{{\n    return x + {idx}.0;\n}}\n\n"
        ));
    }
    out
}

fn large_module(functions: usize) -> Arc<chic::frontend::ast::Module> {
    let parsed = parse_module(&large_module_source(functions)).expect("large module");
    Arc::new(parsed.module)
}

fn bench_text_stream(c: &mut Criterion) {
    let module = large_module(400);
    let target = Target::host();

    let mut group = c.benchmark_group("text_stream_large_module");

    group.bench_function("generate_text", |b| {
        let module = Arc::clone(&module);
        let target = target.clone();
        b.iter(move || {
            let text = generate_text(module.as_ref(), &target, ChicKind::Executable);
            black_box(text.len());
        });
    });

    group.bench_function("stream_module_sink", |b| {
        let module = Arc::clone(&module);
        let target = target.clone();
        b.iter(move || {
            let config = TextStreamConfig {
                buffer_capacity: 64 * 1024,
                on_flush: None,
            };
            let (_sink, metrics) = stream_module(
                io::sink(),
                module.as_ref(),
                &target,
                ChicKind::Executable,
                config,
            )
            .expect("stream");
            black_box(metrics.bytes_written);
        });
    });

    group.finish();
}

criterion_group!(text_stream, bench_text_stream);
criterion_main!(text_stream);
