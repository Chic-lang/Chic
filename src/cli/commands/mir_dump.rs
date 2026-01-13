use crate::logging::{LogFormat, LogLevel, LogOptions, LogSettings};

use super::super::{Cli, CliError, Command};
use super::common::{is_help_flag, parse_const_eval_fuel, parse_error_format, require_path};

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.first().is_some_and(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("mir-dump".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    let mut iter = args.into_iter();
    let input = require_path(iter.next(), "mir-dump requires <file> argument")?;
    let rest = iter.collect::<Vec<_>>();
    if rest.iter().any(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("mir-dump".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    let mut idx = 0usize;
    let mut const_eval_fuel = None;
    let mut trace_pipeline = false;
    let mut trait_solver_metrics = false;
    let mut log_settings = LogSettings::default();
    let mut error_format = None;
    while idx < rest.len() {
        match rest[idx].as_str() {
            "--consteval-fuel" => {
                idx += 1;
                let value = rest
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| CliError::with_usage("expected value after --consteval-fuel"))?;
                const_eval_fuel = Some(parse_const_eval_fuel(&value)?);
                idx += 1;
            }
            "--trace-pipeline" => {
                trace_pipeline = true;
                idx += 1;
            }
            "--trait-solver-metrics" => {
                trait_solver_metrics = true;
                idx += 1;
            }
            "--log-format" => {
                idx += 1;
                let value = rest
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| CliError::with_usage("expected value after --log-format"))?;
                let Some(format) = LogFormat::parse(&value) else {
                    return Err(CliError::with_usage(format!(
                        "invalid log format '{value}'; supported values: auto, text, json"
                    )));
                };
                log_settings.apply_format(format);
                idx += 1;
            }
            "--log-level" => {
                idx += 1;
                let value = rest
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| CliError::with_usage("expected value after --log-level"))?;
                let Some(level) = LogLevel::parse(&value) else {
                    return Err(CliError::with_usage(format!(
                        "invalid log level '{value}'; supported values: error, warn, info, debug, trace"
                    )));
                };
                log_settings.apply_level(level);
                idx += 1;
            }
            "--error-format" => {
                idx += 1;
                let value = rest
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| CliError::with_usage("expected value after --error-format"))?;
                error_format = Some(parse_error_format(&value)?);
                idx += 1;
            }
            other => {
                return Err(CliError::with_usage(format!(
                    "unsupported option '{other}' for command"
                )));
            }
        }
    }
    Ok(Cli {
        command: Command::MirDump {
            input,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
        },
        log_options: log_settings.merged_with_env(),
        error_format,
    })
}
