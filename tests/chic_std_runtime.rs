use assert_cmd::cargo::cargo_bin_cmd;

const TEST_CASES: &[&str] = &[
    "packages/std/src/random/rng.ch",
    "packages/std/src/accelerator/streams.ch",
    "packages/std.core/src/memory/pinned.ch",
    "packages/std/src/distributed/actor.ch",
    "packages/std/src/distributed/collectives.ch",
    "packages/std/src/diagnostics/cost.ch",
    "packages/std/src/diagnostics/trace.ch",
    "packages/std.async/src/async/cancel.ch",
    "packages/std.async/src/async/scope.ch",
    "packages/std/src/probability/dist.ch",
    "tests/runtime/dispose_hook.ch",
];

fn run_chic_test(path: &str) {
    let mut cmd = cargo_bin_cmd!("chic");
    cmd.args(["test", path, "--log-format", "text"]);
    cmd.assert().success();
}

#[test]
#[ignore = "Stdlib native testcase runner currently segfaults (SIGSEGV) in this environment; run manually via `chic test` once the native runner is stable again"]
fn chic_std_runtime_replacements_pass() {
    for case in TEST_CASES {
        run_chic_test(case);
    }
}
