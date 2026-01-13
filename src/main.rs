#![deny(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::all, clippy::pedantic)]
#![deny(clippy::unwrap_used, clippy::expect_used)]

use chic::cli::{Cli, dispatch};
use chic::driver::CompilerDriver;
use chic::error::Result;
use std::process::ExitCode;

fn main() -> ExitCode {
    run_with_args(std::env::args().skip(1))
}

fn run_with_args<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match try_main(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            dispatch::report_error(&err);
            ExitCode::FAILURE
        }
    }
}

fn try_main<I, S>(args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let cli = Cli::parse_from(args.into_iter())?;
    let driver = CompilerDriver::new();
    dispatch::run(&driver, cli)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chic::error::Error;

    #[test]
    fn run_with_args_returns_success_for_help() {
        let exit = run_with_args(["help"]);
        assert_eq!(exit, ExitCode::SUCCESS);
    }

    #[test]
    fn run_with_args_reports_error_on_missing_command() {
        let exit = run_with_args(std::iter::empty::<String>());
        assert_eq!(exit, ExitCode::FAILURE);
    }

    #[test]
    fn try_main_with_args_forwards_parse_errors() {
        let err = try_main(std::iter::empty::<String>())
            .expect_err("expected parse failure for missing args");
        match err {
            Error::Cli(_) => {}
            other => panic!("expected CLI error, found {other:?}"),
        }
    }
}
