use super::module_lowering::driver::ModuleLoweringDriver;
use super::*;
use std::fs;

fn print_metrics(label: &str, source: &str) {
    let parsed = parse_module(source).expect("module should parse");
    let mut driver = ModuleLoweringDriver::new();
    let first = driver.lower_with_units(&parsed.module, None, None);
    let second = driver.lower_with_units(&parsed.module, None, None);
    println!(
        "{label}: first run hits={}, misses={}; second run hits={}, misses={}",
        first.cache_metrics.hits,
        first.cache_metrics.misses,
        second.cache_metrics.hits,
        second.cache_metrics.misses
    );
}

#[test]
#[ignore]
fn cache_metrics_stdlib_io() {
    let source = fs::read_to_string("packages/std/src/io.ch").expect("read packages/std/src/io.ch");
    print_metrics("packages/std/src/io.ch", &source);
}

#[test]
#[ignore]
fn cache_metrics_tmp_loop() {
    let source = fs::read_to_string("tmp_loop.ch").expect("read tmp_loop.ch");
    print_metrics("tmp_loop.ch", &source);
}

#[test]
fn caching_hits_on_subsequent_lowerings() {
    let source = r#"
namespace Math {
    public int Add(int a, int b) {
        return a + b;
    }
}
"#;
    let parsed = parse_module(source).expect("module should parse");
    let mut driver = ModuleLoweringDriver::new();

    let first = driver.lower_with_units(&parsed.module, None, None);
    assert_eq!(first.cache_metrics.hits, 0);
    assert!(first.cache_metrics.misses > 0);

    let second = driver.lower_with_units(&parsed.module, None, None);
    assert!(second.cache_metrics.hits >= first.cache_metrics.misses);
    assert_eq!(second.cache_metrics.misses, 0);
    assert_eq!(second.module.functions.len(), first.module.functions.len());
}

#[test]
fn cache_miss_on_source_change() {
    let source = r#"
namespace Math {
    public int Add(int a, int b) {
        return a + b;
    }
}
"#;
    let parsed = parse_module(source).expect("module should parse");
    let mut driver = ModuleLoweringDriver::new();

    let baseline = driver.lower_with_units(&parsed.module, None, None);
    assert!(baseline.cache_metrics.misses > 0);

    let modified_source = r#"
namespace Math {
    public int Add(int a, int b) {
        let total = a + b;
        return total;
    }
}
"#;
    let modified = parse_module(modified_source).expect("module should parse");
    let rerun = driver.lower_with_units(&modified.module, None, None);
    assert!(rerun.cache_metrics.misses > 0);
}
