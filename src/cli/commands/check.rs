use crate::logging::LogOptions;

use super::super::{Cli, CliError, Command};
use super::common::{is_help_flag, parse_target_and_kind, partition_inputs_and_flags};

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.first().is_some_and(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("check".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    let (inputs, rest) = partition_inputs_and_flags(args, "check requires <file> argument")?;
    if rest.iter().any(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("check".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    let (
        target,
        kind,
        const_eval_fuel,
        trace_pipeline,
        trait_solver_metrics,
        defines,
        log_settings,
        error_format,
    ) = parse_target_and_kind(rest.into_iter())?;
    Ok(Cli {
        command: Command::Check {
            inputs,
            target,
            kind,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
            defines,
        },
        log_options: log_settings.merged_with_env(),
        error_format,
    })
}
