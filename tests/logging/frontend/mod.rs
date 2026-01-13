use crate::harness::{CommandKind, FilterKind};

const COMMANDS: &[CommandKind] = &[
    CommandKind::Check,
    CommandKind::Build,
    CommandKind::Test,
    CommandKind::Run,
];

pub(super) fn frontend_filter() -> FilterKind {
    FilterKind::stage("frontend.")
}

mod json;
mod text;
