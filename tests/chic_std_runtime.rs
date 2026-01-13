use assert_cmd::Command;

const TEST_CASES: &[&str] = &[
    "packages/std/src/random/rng.cl",
    "packages/std/src/accelerator/streams.cl",
    "packages/std.core/src/memory/pinned.cl",
    "packages/std/src/distributed/actor.cl",
    "packages/std/src/distributed/collectives.cl",
    "packages/std/src/diagnostics/cost.cl",
    "packages/std/src/diagnostics/trace.cl",
    "packages/std.async/src/async/cancel.cl",
    "packages/std.async/src/async/scope.cl",
    "packages/std/src/probability/dist.cl",
    "tests/runtime/dispose_hook.cl",
];

fn run_chic_test(path: &str) {
    let mut cmd = Command::cargo_bin("chic").expect("chic binary");
    cmd.args(["test", path, "--log-format", "text"]);
    cmd.assert().success();
}

#[test]
fn chic_std_runtime_replacements_pass() {
    for case in TEST_CASES {
        run_chic_test(case);
    }
}
