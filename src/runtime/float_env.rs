use std::cell::RefCell;

use crate::mir::{FloatStatusFlags, RoundingMode};

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; float environment runtime lives in the native runtime."
);

mod native;
#[allow(unused_imports)]
pub use native::*;

#[derive(Debug, Clone)]
pub struct FloatEnv {
    pub rounding_mode: RoundingMode,
    pub flags: FloatStatusFlags,
}

impl Default for FloatEnv {
    fn default() -> Self {
        Self {
            rounding_mode: RoundingMode::NearestTiesToEven,
            flags: FloatStatusFlags::default(),
        }
    }
}

thread_local! {
    static ENV: RefCell<FloatEnv> = RefCell::new(FloatEnv::default());
}

pub fn rounding_mode() -> RoundingMode {
    ENV.with(|env| env.borrow().rounding_mode)
}

pub fn set_rounding_mode(mode: RoundingMode) {
    ENV.with(|env| env.borrow_mut().rounding_mode = mode);
}

pub fn read_flags() -> FloatStatusFlags {
    ENV.with(|env| env.borrow().flags)
}

pub fn clear_flags() {
    ENV.with(|env| env.borrow_mut().flags.clear());
}

pub fn record_flags(flags: FloatStatusFlags) {
    if !flags.any() {
        return;
    }
    ENV.with(|env| {
        env.borrow_mut().flags.merge(flags);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_round_trip() {
        clear_flags();
        assert!(!read_flags().any());

        let mut flags = FloatStatusFlags::default();
        flags.invalid = true;
        record_flags(flags);
        let snapshot = read_flags();
        assert!(snapshot.invalid);
        assert!(!snapshot.div_by_zero);

        clear_flags();
        let cleared = read_flags();
        assert!(!cleared.any());
    }
}
