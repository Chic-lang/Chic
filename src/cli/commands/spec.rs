use crate::logging::LogOptions;

use super::super::{Cli, CliError, Command};
use super::common::is_help_flag;

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.iter().any(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("spec".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    if !args.is_empty() {
        return Err(CliError::with_usage(
            "chic spec does not accept additional arguments",
        ));
    }
    Ok(Cli {
        command: Command::ShowSpec,
        log_options: LogOptions::from_env(),
        error_format: None,
    })
}
