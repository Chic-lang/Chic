use super::super::{Cli, CliError, Command};
use crate::logging::LogOptions;
use std::path::PathBuf;

#[derive(Default)]
struct BindOptionState {
    header: Option<PathBuf>,
    output: Option<PathBuf>,
    namespace: Option<String>,
    library: Option<String>,
    binding: String,
    convention: String,
    optional: bool,
}

impl BindOptionState {
    fn consume_flag(&mut self, args: &[String]) -> Result<usize, CliError> {
        let Some(flag) = args.first() else {
            return Err(CliError::with_usage(
                "missing argument for extern bind command",
            ));
        };
        match flag.as_str() {
            "--header" => {
                let value = Self::next_value(args, "expected path after --header")?;
                self.header = Some(PathBuf::from(value));
                Ok(2)
            }
            "--output" | "-o" => {
                let value = Self::next_value(args, "expected path after --output/-o")?;
                self.output = Some(PathBuf::from(value));
                Ok(2)
            }
            "--namespace" => {
                let value = Self::next_value(args, "expected namespace after --namespace")?;
                self.namespace = Some(value.to_string());
                Ok(2)
            }
            "--library" => {
                let value = Self::next_value(args, "expected library name after --library")?;
                self.library = Some(value.to_string());
                Ok(2)
            }
            "--binding" => {
                let value = Self::next_value(args, "expected binding mode after --binding")?;
                let lower = value.to_ascii_lowercase();
                if !matches!(lower.as_str(), "lazy" | "eager" | "static") {
                    return Err(CliError::with_usage(
                        "binding must be one of lazy|eager|static",
                    ));
                }
                self.binding = lower;
                Ok(2)
            }
            "--convention" => {
                let value =
                    Self::next_value(args, "expected calling convention after --convention")?;
                self.convention = value.to_string();
                Ok(2)
            }
            "--optional" => {
                self.optional = true;
                Ok(1)
            }
            other => Err(CliError::with_usage(format!(
                "unsupported option '{other}' for extern bind"
            ))),
        }
    }

    fn finish(self) -> Result<BindOptions, CliError> {
        let header = self
            .header
            .ok_or_else(|| CliError::with_usage("--header <path> is required for extern bind"))?;
        let output = self
            .output
            .ok_or_else(|| CliError::with_usage("--output <path> is required for extern bind"))?;
        let namespace = self.namespace.ok_or_else(|| {
            CliError::with_usage("--namespace <name> is required for extern bind")
        })?;
        let library = self
            .library
            .ok_or_else(|| CliError::with_usage("--library <name> is required for extern bind"))?;
        Ok(BindOptions {
            header,
            output,
            namespace,
            library,
            binding: if self.binding.is_empty() {
                "lazy".into()
            } else {
                self.binding
            },
            convention: if self.convention.is_empty() {
                "system".into()
            } else {
                self.convention
            },
            optional: self.optional,
        })
    }

    fn next_value<'a>(args: &'a [String], message: &str) -> Result<&'a str, CliError> {
        args.get(1)
            .map(String::as_str)
            .ok_or_else(|| CliError::with_usage(message))
    }
}

#[derive(Debug)]
struct BindOptions {
    header: PathBuf,
    output: PathBuf,
    namespace: String,
    library: String,
    binding: String,
    convention: String,
    optional: bool,
}

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args
        .first()
        .is_some_and(|value| matches!(value.as_str(), "--help" | "-h"))
    {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("extern-bind".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }

    let mut state = BindOptionState {
        binding: "lazy".into(),
        convention: "system".into(),
        ..BindOptionState::default()
    };

    let mut idx = 0;
    while idx < args.len() {
        let consumed = state.consume_flag(&args[idx..])?;
        idx += consumed;
    }
    let opts = state.finish()?;

    Ok(Cli {
        command: Command::ExternBind {
            header: opts.header,
            output: opts.output,
            namespace: opts.namespace,
            library: opts.library,
            binding: opts.binding,
            convention: opts.convention,
            optional: opts.optional,
        },
        log_options: LogOptions::from_env(),
        error_format: None,
    })
}
