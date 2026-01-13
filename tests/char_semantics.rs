#![cfg(not(chic_native_runtime))]

use chic::runtime::{
    CharError, chic_rt_char_from_codepoint, chic_rt_char_is_digit, chic_rt_char_is_letter,
    chic_rt_char_is_scalar, chic_rt_char_is_whitespace, chic_rt_char_status, chic_rt_char_to_lower,
    chic_rt_char_to_upper, chic_rt_char_value,
};

fn status(result: u64) -> CharError {
    match unsafe { chic_rt_char_status(result) } {
        0 => CharError::Success,
        1 => CharError::InvalidScalar,
        2 => CharError::NullPointer,
        3 => CharError::ComplexMapping,
        other => panic!("unexpected status {other}"),
    }
}

fn value(result: u64) -> u16 {
    unsafe { chic_rt_char_value(result) }
}

#[test]
fn char_runtime_behaviour() {
    unsafe {
        // Upper casing of basic ASCII
        let upper = chic_rt_char_to_upper('a' as u16);
        assert_eq!(status(upper), CharError::Success);
        assert_eq!(value(upper), 'A' as u16);

        // Upper casing of scalar with direct mapping
        let cedilla = chic_rt_char_to_upper('\u{00E7}' as u16);
        assert_eq!(status(cedilla), CharError::Success);
        assert_eq!(value(cedilla), '\u{00C7}' as u16);

        // Complex mapping should report status and preserve original scalar
        let sharp_s = chic_rt_char_to_upper('\u{00DF}' as u16);
        assert_eq!(status(sharp_s), CharError::ComplexMapping);
        assert_eq!(value(sharp_s), 'S' as u16);

        // Lower casing
        let lower = chic_rt_char_to_lower('\u{00C7}' as u16);
        assert_eq!(status(lower), CharError::Success);
        assert_eq!(value(lower), '\u{00E7}' as u16);

        // Invalid scalar should be rejected
        let surrogate = chic_rt_char_to_upper(0xD800);
        assert_eq!(status(surrogate), CharError::InvalidScalar);

        // Code point conversion success for BMP values
        let cedilla_cp = chic_rt_char_from_codepoint(0x00E7);
        assert_eq!(status(cedilla_cp), CharError::Success);
        assert_eq!(value(cedilla_cp), 0x00E7);

        // Code point conversion failure for unsupported value
        let smile = chic_rt_char_from_codepoint(0x1F60A);
        assert_eq!(status(smile), CharError::InvalidScalar);

        // Code point conversion failure for invalid value
        let invalid = chic_rt_char_from_codepoint(0x110000);
        assert_eq!(status(invalid), CharError::InvalidScalar);

        // Classification helpers
        assert!(chic_rt_char_is_digit('9' as u16) == 1);
        assert!(chic_rt_char_is_digit('A' as u16) == 0);
        assert!(chic_rt_char_is_letter('A' as u16) == 1);
        assert!(chic_rt_char_is_letter('1' as u16) == 0);
        assert!(chic_rt_char_is_scalar('\u{00E7}' as u16) == 1);
        assert!(chic_rt_char_is_scalar(0xD800) == 0);
        assert!(chic_rt_char_is_whitespace(' ' as u16) == 1);
        assert!(chic_rt_char_is_whitespace('X' as u16) == 0);

        // String conversion is covered elsewhere; ensure classification agrees with runtime flags.
    }
}
