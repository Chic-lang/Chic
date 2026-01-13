use crate::logging::LogOptions;

use super::super::{Cli, CliError, Command};
use super::common::{is_help_flag, parse_cc1_options, require_cc1_inputs};

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.first().is_some_and(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("cc1".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    let (input, rest) = require_cc1_inputs(args)?;
    if rest.iter().any(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("cc1".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    let (output, target, extra_args) = parse_cc1_options(rest.into_iter())?;
    Ok(Cli {
        command: Command::Cc1 {
            input,
            output,
            target,
            extra_args,
        },
        log_options: LogOptions::from_env(),
        error_format: None,
    })
}
