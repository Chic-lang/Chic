use super::super::{Cli, CliError};
use super::build_like::{CommandKind, parse_build_like};

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    parse_build_like(args, CommandKind::Profile)
}
