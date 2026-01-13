use std::path::PathBuf;

use crate::logging::LogOptions;

use super::super::{Cli, CliError, Command};
use super::common::is_help_flag;

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.first().is_some_and(|value| is_help_flag(value)) {
        return Ok(help_cli());
    }
    let mut inputs = Vec::new();
    let mut config = None;
    let mut check = false;
    let mut diff = false;
    let mut write = true;
    let mut stdin = false;
    let mut stdout = false;
    let mut iter = args.into_iter().peekable();
    while let Some(arg) = iter.next() {
        if is_help_flag(&arg) {
            return Ok(help_cli());
        }
        match arg.as_str() {
            "--config" => {
                let Some(path) = iter.next() else {
                    return Err(CliError::with_usage("expected path after --config"));
                };
                config = Some(PathBuf::from(path));
            }
            "--check" => {
                check = true;
                write = false;
            }
            "--diff" => {
                diff = true;
                write = false;
            }
            "--write" => {
                write = true;
                check = false;
                diff = false;
            }
            "--stdin" => stdin = true,
            "--stdout" => stdout = true,
            other if other.starts_with('-') => {
                return Err(CliError::with_usage(format!(
                    "unsupported option '{other}' for chic format"
                )));
            }
            other => inputs.push(PathBuf::from(other)),
        }
    }

    if stdin && !inputs.is_empty() {
        return Err(CliError::with_usage(
            "cannot combine --stdin with file or directory inputs",
        ));
    }

    Ok(Cli {
        command: Command::Format {
            inputs,
            config,
            check,
            diff,
            write,
            stdin,
            stdout,
        },
        log_options: LogOptions::from_env(),
        error_format: None,
    })
}

fn help_cli() -> Cli {
    Cli {
        command: Command::Help {
            topic: Some("format".into()),
        },
        log_options: LogOptions::from_env(),
        error_format: None,
    }
}
