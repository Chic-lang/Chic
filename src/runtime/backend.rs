/// Active runtime backend. Chic is the only supported option.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeBackend {
    Chic,
}

/// Returns the active backend (defaults to Chic).
pub fn runtime_backend() -> RuntimeBackend {
    RuntimeBackend::Chic
}

/// Returns the active backend name for diagnostics and logging.
pub fn runtime_backend_name() -> &'static str {
    "chic"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_is_fixed_to_chic() {
        assert_eq!(runtime_backend(), RuntimeBackend::Chic);
        assert_eq!(runtime_backend_name(), "chic");
        assert_eq!(runtime_backend(), RuntimeBackend::Chic);
    }
}
