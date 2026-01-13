use std::path::PathBuf;

use crate::logging::LogOptions;

use super::super::{Cli, CliError, Command};
use super::common::is_help_flag;

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.first().is_some_and(|value| is_help_flag(value)) {
        return help();
    }

    let mut template = "app".to_string();
    let mut output = None;
    let mut name = None;

    let mut iter = args.into_iter().peekable();
    while let Some(arg) = iter.next() {
        if is_help_flag(&arg) {
            return help();
        }
        match arg.as_str() {
            "--template" | "-t" => {
                let Some(value) = iter.next() else {
                    return Err(CliError::with_usage(
                        "expected value after --template/-t for chic init",
                    ));
                };
                template = value;
            }
            "--name" => {
                let Some(value) = iter.next() else {
                    return Err(CliError::with_usage(
                        "expected value after --name for chic init",
                    ));
                };
                if value.trim().is_empty() {
                    return Err(CliError::with_usage(
                        "--name requires a non-empty project name",
                    ));
                }
                name = Some(value);
            }
            "--output" | "-o" => {
                let Some(value) = iter.next() else {
                    return Err(CliError::with_usage(
                        "expected path after --output/-o for chic init",
                    ));
                };
                output = Some(PathBuf::from(value));
            }
            _ if arg.starts_with('-') => {
                return Err(CliError::with_usage(format!(
                    "unsupported option '{arg}' for chic init; expected --template, --name, or --output"
                )));
            }
            _ => {
                if output.is_some() {
                    return Err(CliError::with_usage(
                        "chic init accepts at most one output directory argument",
                    ));
                }
                output = Some(PathBuf::from(arg));
            }
        }
    }

    Ok(Cli {
        command: Command::Init {
            template,
            output,
            name,
        },
        log_options: LogOptions::from_env(),
        error_format: None,
    })
}

fn help() -> Result<Cli, CliError> {
    Ok(Cli {
        command: Command::Help {
            topic: Some("init".into()),
        },
        log_options: LogOptions::from_env(),
        error_format: None,
    })
}
