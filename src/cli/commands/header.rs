use crate::logging::LogOptions;

use super::super::{Cli, CliError, Command};
use super::common::{is_help_flag, parse_header_options, require_path};

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.first().is_some_and(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("header".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    let mut iter = args.into_iter();
    let input = require_path(iter.next(), "header requires <file> argument")?;
    let rest = iter.collect::<Vec<_>>();
    if rest.iter().any(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("header".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    let (output, include_guard, error_format) = parse_header_options(rest.into_iter())?;
    Ok(Cli {
        command: Command::Header {
            input,
            output,
            include_guard,
        },
        log_options: LogOptions::from_env(),
        error_format,
    })
}
