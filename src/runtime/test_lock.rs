#![cfg(test)]

use std::sync::{Mutex, MutexGuard, OnceLock};

static RUNTIME_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[inline]
pub fn runtime_test_guard() -> MutexGuard<'static, ()> {
    RUNTIME_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner())
}
