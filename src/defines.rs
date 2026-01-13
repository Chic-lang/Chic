//! Shared structures for CLI/compiler conditional defines.

/// Raw define collected from the CLI or build configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefineFlag {
    pub name: String,
    pub value: Option<String>,
}

impl DefineFlag {
    #[must_use]
    pub fn new(name: impl Into<String>, value: Option<String>) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}
