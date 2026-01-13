#![deny(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all, clippy::pedantic)]
#![deny(clippy::unwrap_used, clippy::expect_used)]

use chic::lsp;
use lsp_server::Connection;
use serde_json::Value;
use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(err) = try_main() {
        eprintln!("impact-lsp failed to start: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn try_main() -> Result<(), String> {
    let (connection, io_threads) = Connection::stdio();
    let init_result = lsp::capabilities();
    let init_value: Value = serde_json::to_value(&init_result)
        .map_err(|err| format!("failed to serialise capabilities: {err}"))?;
    let _client_init = connection
        .initialize(init_value)
        .map_err(|err| format!("initialization failed: {err}"))?;
    lsp::run(connection, init_result);
    let _ = io_threads.join();
    Ok(())
}
