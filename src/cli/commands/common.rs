use std::path::PathBuf;

use crate::chic_kind::{ChicKind, ChicKindError};
use crate::defines::DefineFlag;
use crate::diagnostics::ErrorFormat;
use crate::logging::{LogFormat, LogLevel, LogSettings};
use crate::target::{Target, TargetError};

use super::super::CliError;

pub(crate) fn is_help_flag(value: &str) -> bool {
    matches!(value, "-h" | "--help")
}

pub(crate) fn partition_inputs_and_flags(
    args: Vec<String>,
    missing_message: &str,
) -> Result<(Vec<PathBuf>, Vec<String>), CliError> {
    if args.is_empty() {
        return Err(CliError::with_usage(missing_message));
    }

    let mut inputs = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let value = &args[index];
        if value.starts_with('-') {
            break;
        }
        if value.trim().is_empty() {
            return Err(CliError::with_usage("input path must not be empty"));
        }
        inputs.push(PathBuf::from(value));
        index += 1;
    }

    if inputs.is_empty() {
        return Err(CliError::with_usage(missing_message));
    }

    let rest = args.into_iter().skip(index).collect();
    Ok((inputs, rest))
}

pub(crate) fn require_path<T>(value: Option<T>, message: &str) -> Result<PathBuf, CliError>
where
    T: Into<String>,
{
    let Some(raw) = value else {
        return Err(CliError::with_usage(message));
    };
    Ok(PathBuf::from(raw.into()))
}

pub(crate) fn parse_target_and_kind<I, T>(
    args: I,
) -> Result<
    (
        Target,
        ChicKind,
        Option<usize>,
        bool,
        bool,
        Vec<DefineFlag>,
        LogSettings,
        Option<ErrorFormat>,
    ),
    CliError,
>
where
    I: Iterator<Item = T>,
    T: Into<String>,
{
    let iter = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let mut idx = 0;
    let mut target = None;
    let mut kind = None;
    let mut const_eval_fuel = None;
    let mut trace_pipeline = false;
    let mut trait_solver_metrics = false;
    let mut defines = Vec::new();
    let mut log_settings = LogSettings::default();
    let mut error_format = None;

    while idx < iter.len() {
        let flag = &iter[idx];
        idx += 1;
        match flag.as_str() {
            "-t" | "--target" => {
                let Some(spec) = iter.get(idx) else {
                    return Err(CliError::with_usage("expected triple after -t/--target"));
                };
                idx += 1;
                target = Some(parse_target(spec)?);
            }
            "--crate-type" | "--kind" => {
                let Some(value) = iter.get(idx) else {
                    return Err(CliError::with_usage(
                        "expected value after --crate-type/--kind",
                    ));
                };
                idx += 1;
                kind = Some(parse_chic_kind(value)?);
            }
            "--consteval-fuel" => {
                let Some(value) = iter.get(idx) else {
                    return Err(CliError::with_usage(
                        "expected value after --consteval-fuel",
                    ));
                };
                idx += 1;
                const_eval_fuel = Some(parse_const_eval_fuel(value)?);
            }
            "--trace-pipeline" => {
                trace_pipeline = true;
            }
            "--trait-solver-metrics" => {
                trait_solver_metrics = true;
            }
            "-D" | "--define" => {
                let Some(value) = iter.get(idx) else {
                    return Err(CliError::with_usage("expected value after --define/-D"));
                };
                idx += 1;
                let flag = parse_define_flag(value)?;
                defines.push(flag);
            }
            "--log-format" => {
                let Some(value) = iter.get(idx) else {
                    return Err(CliError::with_usage("expected value after --log-format"));
                };
                idx += 1;
                let Some(format) = LogFormat::parse(value) else {
                    return Err(CliError::with_usage(format!(
                        "invalid log format '{value}'; supported values: auto, text, json"
                    )));
                };
                log_settings.apply_format(format);
            }
            "--log-level" => {
                let Some(value) = iter.get(idx) else {
                    return Err(CliError::with_usage("expected value after --log-level"));
                };
                idx += 1;
                let Some(level) = LogLevel::parse(value) else {
                    return Err(CliError::with_usage(format!(
                        "invalid log level '{value}'; supported values: error, warn, info, debug, trace"
                    )));
                };
                log_settings.apply_level(level);
            }
            "--error-format" => {
                let Some(value) = iter.get(idx) else {
                    return Err(CliError::with_usage("expected value after --error-format"));
                };
                idx += 1;
                error_format = Some(parse_error_format(value)?);
            }
            other => {
                return Err(CliError::with_usage(format!(
                    "unsupported option '{other}' for command"
                )));
            }
        }
    }

    Ok((
        target.unwrap_or_else(Target::host),
        kind.unwrap_or_default(),
        const_eval_fuel,
        trace_pipeline,
        trait_solver_metrics,
        defines,
        log_settings,
        error_format,
    ))
}

pub(crate) fn parse_target(spec: &str) -> Result<Target, CliError> {
    Target::parse(spec).map_err(|err| match err {
        TargetError::Empty => CliError::with_usage("target triple must not be empty"),
        TargetError::UnsupportedArch(_) => CliError::with_usage(format!("invalid target: {err}")),
    })
}

pub(crate) fn parse_chic_kind(spec: &str) -> Result<ChicKind, CliError> {
    ChicKind::parse(spec).map_err(|err| match err {
        ChicKindError::Empty => CliError::with_usage("crate type must not be empty"),
        ChicKindError::Unsupported(_) => CliError::with_usage(format!("invalid crate type: {err}")),
    })
}

pub(crate) fn parse_const_eval_fuel(spec: &str) -> Result<usize, CliError> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_usage(
            "--consteval-fuel requires a positive integer value",
        ));
    }
    let value = trimmed.parse::<usize>().map_err(|_| {
        CliError::with_usage(format!(
            "invalid --consteval-fuel value '{trimmed}'; expected decimal integer"
        ))
    })?;
    if value == 0 {
        return Err(CliError::with_usage(
            "--consteval-fuel must be greater than zero",
        ));
    }
    Ok(value)
}

pub(crate) fn parse_error_format(spec: &str) -> Result<ErrorFormat, CliError> {
    let value = spec.trim().to_ascii_lowercase();
    let format = match value.as_str() {
        "human" => ErrorFormat::Human,
        "json" => ErrorFormat::Json,
        "toon" => ErrorFormat::Toon,
        "short" => ErrorFormat::Short,
        _ => {
            return Err(CliError::with_usage(format!(
                "invalid --error-format '{spec}'; supported values: human, json, toon, short"
            )));
        }
    };
    Ok(format)
}

pub(crate) fn parse_header_options<I, T>(
    args: I,
) -> Result<(Option<PathBuf>, Option<String>, Option<ErrorFormat>), CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    let iter = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let mut idx = 0;
    let mut output = None;
    let mut include_guard = None;
    let mut error_format = None;

    while idx < iter.len() {
        let flag = &iter[idx];
        idx += 1;
        match flag.as_str() {
            "-o" | "--output" => {
                let value = iter
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| CliError::with_usage("expected path after -o/--output"))?;
                idx += 1;
                output = Some(PathBuf::from(value));
            }
            "--include-guard" => {
                let value = iter
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| CliError::with_usage("expected value after --include-guard"))?;
                idx += 1;
                include_guard = Some(value);
            }
            "--error-format" => {
                let value = iter
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| CliError::with_usage("expected value after --error-format"))?;
                idx += 1;
                error_format = Some(parse_error_format(&value)?);
            }
            other => {
                return Err(CliError::with_usage(format!(
                    "unsupported option '{other}' for command"
                )));
            }
        }
    }

    Ok((output, include_guard, error_format))
}

pub(crate) fn require_cc1_inputs(args: Vec<String>) -> Result<(PathBuf, Vec<String>), CliError> {
    if args.is_empty() {
        return Err(CliError::with_usage("cc1 requires <file> argument"));
    }
    let input = require_path(Some(args[0].clone()), "cc1 requires <file> argument")?;
    let rest = args.into_iter().skip(1).collect();
    Ok((input, rest))
}

pub(crate) fn parse_define_flag(raw: &str) -> Result<DefineFlag, CliError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_usage(
            "--define/-D requires NAME or NAME=VALUE",
        ));
    }
    if let Some(eq) = trimmed.find('=') {
        let name = trimmed[..eq].trim();
        let value = trimmed[eq + 1..].to_string();
        if name.is_empty() {
            return Err(CliError::with_usage("--define/-D requires NAME before '='"));
        }
        Ok(DefineFlag::new(name, Some(value)))
    } else {
        Ok(DefineFlag::new(trimmed, None))
    }
}

pub(crate) fn parse_cc1_options<I, T>(
    args: I,
) -> Result<(Option<PathBuf>, Target, Vec<String>), CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    let iter = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let mut idx = 0;
    let mut output = None;
    let mut target = None;
    let mut extra_args = Vec::new();

    while idx < iter.len() {
        let flag = &iter[idx];
        idx += 1;
        match flag.as_str() {
            "-o" | "--output" => {
                let value = iter
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| CliError::with_usage("expected path after -o/--output"))?;
                idx += 1;
                output = Some(PathBuf::from(value));
            }
            "-t" | "--target" => {
                let value = iter
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| CliError::with_usage("expected triple after -t/--target"))?;
                idx += 1;
                target = Some(parse_target(&value)?);
            }
            "--cc1-arg" => {
                let value = iter
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| CliError::with_usage("expected value after --cc1-arg"))?;
                idx += 1;
                extra_args.push(value);
            }
            other => {
                return Err(CliError::with_usage(format!(
                    "unsupported option '{other}' for command"
                )));
            }
        }
    }

    Ok((output, target.unwrap_or_else(Target::host), extra_args))
}
