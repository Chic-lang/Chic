#![deny(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all, clippy::pedantic)]
#![deny(clippy::unwrap_used, clippy::expect_used)]

use chic::lsp;
use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(err) = try_main() {
        eprintln!("impact-lsp failed to start: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn try_main() -> Result<(), String> {
    let init_result = lsp::capabilities();
    lsp::run_stdio(init_result)
}
