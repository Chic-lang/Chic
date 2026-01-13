pub(crate) mod common;

mod build;
pub(crate) mod build_like;
mod cc1;
mod check;
mod clean;
mod coverage;
mod doc;
mod extern_bind;
mod extern_cmd;
mod format;
mod header;
mod init;
mod lint;
mod mir_dump;
mod perf_report;
mod profile;
mod run;
mod seed;
mod spec;
mod test;

use super::{CommandDescriptor, CommandFeature};
use build::parse as parse_build_command;
use cc1::parse as parse_cc1_command;
use check::parse as parse_check_command;
use clean::parse as parse_clean_command;
use coverage::parse as parse_coverage_command;
use format::parse as parse_format_command;
use header::parse as parse_header_command;
use init::parse as parse_init_command;
use lint::parse as parse_lint_command;
use mir_dump::parse as parse_mir_dump_command;
use perf_report::parse as parse_perf_report_command;
use profile::parse as parse_profile_command;
use run::parse as parse_run_command;
use seed::parse as parse_seed_command;
use spec::parse as parse_spec_command;
use test::parse as parse_test_command;

const COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        name: "check",
        aliases: &[],
        parser: parse_check_command,
        feature: None,
    },
    CommandDescriptor {
        name: "lint",
        aliases: &[],
        parser: parse_lint_command,
        feature: None,
    },
    CommandDescriptor {
        name: "build",
        aliases: &["publish", "pack"],
        parser: parse_build_command,
        feature: None,
    },
    CommandDescriptor {
        name: "clean",
        aliases: &[],
        parser: parse_clean_command,
        feature: None,
    },
    CommandDescriptor {
        name: "init",
        aliases: &[],
        parser: parse_init_command,
        feature: None,
    },
    CommandDescriptor {
        name: "doc",
        aliases: &["docs"],
        parser: doc::parse,
        feature: None,
    },
    CommandDescriptor {
        name: "run",
        aliases: &[],
        parser: parse_run_command,
        feature: None,
    },
    CommandDescriptor {
        name: "profile",
        aliases: &[],
        parser: parse_profile_command,
        feature: None,
    },
    CommandDescriptor {
        name: "test",
        aliases: &[],
        parser: parse_test_command,
        feature: None,
    },
    CommandDescriptor {
        name: "coverage",
        aliases: &[],
        parser: parse_coverage_command,
        feature: None,
    },
    CommandDescriptor {
        name: "format",
        aliases: &["cleanup"],
        parser: parse_format_command,
        feature: None,
    },
    CommandDescriptor {
        name: "mir-dump",
        aliases: &[],
        parser: parse_mir_dump_command,
        feature: None,
    },
    CommandDescriptor {
        name: "header",
        aliases: &[],
        parser: parse_header_command,
        feature: None,
    },
    CommandDescriptor {
        name: "cc1",
        aliases: &[],
        parser: parse_cc1_command,
        feature: Some(CommandFeature::Cc1),
    },
    CommandDescriptor {
        name: "spec",
        aliases: &["show-spec"],
        parser: parse_spec_command,
        feature: None,
    },
    CommandDescriptor {
        name: "extern",
        aliases: &[],
        parser: extern_cmd::parse,
        feature: Some(CommandFeature::ExternBind),
    },
    CommandDescriptor {
        name: "perf",
        aliases: &["perf-report"],
        parser: parse_perf_report_command,
        feature: None,
    },
    CommandDescriptor {
        name: "seed",
        aliases: &[],
        parser: parse_seed_command,
        feature: None,
    },
];

pub(crate) fn descriptors() -> &'static [CommandDescriptor] {
    COMMANDS
}
