use std::fmt;

/// Error emitted when WASM execution or parsing fails.
#[derive(Debug)]
pub struct WasmExecutionError {
    pub message: String,
}

impl fmt::Display for WasmExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for WasmExecutionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_message() {
        let err = WasmExecutionError {
            message: "example".into(),
        };
        assert_eq!(format!("{err}"), "example");
    }
}
