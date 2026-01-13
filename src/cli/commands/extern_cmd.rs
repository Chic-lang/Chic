use super::super::{Cli, CliError, Command};
use super::common::is_help_flag;
use crate::logging::LogOptions;

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.first().is_some_and(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("extern".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }

    let Some(subcommand) = args.first() else {
        return Err(CliError::with_usage(
            "chic extern requires a subcommand such as `bind`",
        ));
    };
    match subcommand.as_str() {
        "bind" => super::extern_bind::parse(args.into_iter().skip(1).collect()),
        other => Err(CliError::with_usage(format!(
            "unknown extern subcommand '{other}'"
        ))),
    }
}
