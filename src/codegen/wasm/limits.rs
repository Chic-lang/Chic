use std::convert::TryFrom;

use crate::error::Error;

/// Convert a usize into a u32 while reporting context-specific overflow errors.
pub(crate) fn ensure_u32(value: usize, context: &str) -> Result<u32, Error> {
    u32::try_from(value).map_err(|_| Error::Codegen(context.into()))
}

#[cfg(test)]
mod tests {
    use super::ensure_u32;

    #[test]
    fn ensure_u32_accepts_in_range_values() {
        let result = ensure_u32(1234, "should not overflow").expect("value within range");
        assert_eq!(result, 1234u32);
    }

    #[test]
    fn ensure_u32_reports_context_on_overflow() {
        let err = ensure_u32(usize::MAX, "custom overflow context")
            .expect_err("value should overflow conversion");
        let message = format!("{err}");
        assert!(
            message.contains("custom overflow context"),
            "error message should include provided context, message = {message}"
        );
    }
}
