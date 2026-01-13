use std::path::PathBuf;

use crate::cli::commands::common::is_help_flag;
use crate::cli::{Cli, CliError, Command};
use crate::logging::LogOptions;

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.first().is_some_and(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("doc".to_string()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }

    let mut output = None;
    let mut scope = None;
    let mut template = None;
    let mut front_matter = None;
    let mut tag_handlers: Vec<String> = Vec::new();
    let mut link_resolver = None;
    let mut layout = None;
    let mut banner = None;
    let mut inputs = Vec::new();
    let mut idx = 0;
    while idx < args.len() {
        let value = &args[idx];
        if value.starts_with('-') {
            match value.as_str() {
                "-o" | "--output" => {
                    idx += 1;
                    let Some(path) = args.get(idx) else {
                        return Err(CliError::with_usage(
                            "expected path after --output/-o for chic doc",
                        ));
                    };
                    output = Some(PathBuf::from(path));
                }
                "--scope" => {
                    idx += 1;
                    let Some(raw) = args.get(idx) else {
                        return Err(CliError::with_usage(
                            "expected value after --scope (package|module|assembly)",
                        ));
                    };
                    scope = Some(raw.clone());
                }
                "--template" => {
                    idx += 1;
                    let Some(path) = args.get(idx) else {
                        return Err(CliError::with_usage(
                            "expected path after --template for chic doc",
                        ));
                    };
                    template = Some(PathBuf::from(path));
                }
                "--front-matter" => {
                    idx += 1;
                    let Some(path) = args.get(idx) else {
                        return Err(CliError::with_usage(
                            "expected path after --front-matter for chic doc",
                        ));
                    };
                    front_matter = Some(PathBuf::from(path));
                }
                "--tag-handler" => {
                    idx += 1;
                    let Some(name) = args.get(idx) else {
                        return Err(CliError::with_usage(
                            "expected handler name after --tag-handler",
                        ));
                    };
                    tag_handlers.push(name.clone());
                }
                "--link-resolver" => {
                    idx += 1;
                    let Some(name) = args.get(idx) else {
                        return Err(CliError::with_usage(
                            "expected resolver name after --link-resolver",
                        ));
                    };
                    link_resolver = Some(name.clone());
                }
                "--layout" => {
                    idx += 1;
                    let Some(value) = args.get(idx) else {
                        return Err(CliError::with_usage(
                            "expected layout after --layout (single|per-type)",
                        ));
                    };
                    layout = Some(value.clone());
                }
                "--no-banner" => {
                    banner = Some(false);
                }
                "--banner" => {
                    banner = Some(true);
                }
                flag if is_help_flag(flag) => {
                    return Ok(Cli {
                        command: Command::Help {
                            topic: Some("doc".to_string()),
                        },
                        log_options: LogOptions::from_env(),
                        error_format: None,
                    });
                }
                other => {
                    return Err(CliError::with_usage(format!(
                        "unsupported flag '{other}' for chic doc"
                    )));
                }
            }
            idx += 1;
            continue;
        }
        inputs.push(PathBuf::from(value));
        idx += 1;
    }

    if inputs.len() > 1 {
        return Err(CliError::with_usage(
            "provide at most one manifest.yaml or directory to chic doc",
        ));
    }

    Ok(Cli {
        command: Command::Doc {
            manifest: inputs.pop(),
            output,
            scope,
            template,
            front_matter,
            tag_handlers,
            link_resolver,
            layout,
            banner,
        },
        log_options: LogOptions::from_env(),
        error_format: None,
    })
}
