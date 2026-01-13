// External bindings to the Chic-native float env helpers.
#[allow(unsafe_code)]
unsafe extern "C" {
    pub fn chic_rt_float_flags_read() -> u32;
    pub fn chic_rt_float_flags_clear();
    pub fn chic_rt_set_rounding_mode(mode: i32) -> i32;
    pub fn chic_rt_rounding_mode() -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::RoundingMode;

    #[test]
    fn native_rounding_mode_roundtrips() {
        unsafe {
            assert_eq!(chic_rt_set_rounding_mode(0), 0);
            assert_eq!(chic_rt_rounding_mode(), 0);
            assert_eq!(
                chic_rt_set_rounding_mode(RoundingMode::TowardNegative as i32),
                0
            );
            assert_eq!(chic_rt_rounding_mode(), RoundingMode::TowardNegative as i32);
        }
    }
}
