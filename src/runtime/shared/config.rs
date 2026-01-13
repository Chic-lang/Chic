use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeakStrategy {
    Ignore,
    Warn,
    Panic,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub max_allocation_bytes: usize,
    pub track_shared_allocations: bool,
    pub leak_strategy: LeakStrategy,
    pub resource_table_capacity: usize,
}

impl RuntimeConfig {
    pub fn builder() -> RuntimeConfigBuilder {
        RuntimeConfigBuilder::new()
    }

    fn validate(&self) -> Result<(), RuntimeConfigError> {
        if self.max_allocation_bytes == 0 {
            return Err(RuntimeConfigError::Invalid(
                "max_allocation_bytes must be greater than zero",
            ));
        }
        if self.resource_table_capacity == 0 {
            return Err(RuntimeConfigError::Invalid(
                "resource_table_capacity must be greater than zero",
            ));
        }
        Ok(())
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_allocation_bytes: 32 * 1024 * 1024,
            track_shared_allocations: false,
            leak_strategy: LeakStrategy::Warn,
            resource_table_capacity: 8_192,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfigBuilder {
    inner: RuntimeConfig,
}

impl RuntimeConfigBuilder {
    pub fn new() -> Self {
        Self {
            inner: RuntimeConfig::default(),
        }
    }

    pub fn max_allocation_bytes(mut self, bytes: usize) -> Self {
        self.inner.max_allocation_bytes = bytes;
        self
    }

    pub fn track_shared_allocations(mut self, enabled: bool) -> Self {
        self.inner.track_shared_allocations = enabled;
        self
    }

    pub fn leak_strategy(mut self, strategy: LeakStrategy) -> Self {
        self.inner.leak_strategy = strategy;
        self
    }

    pub fn resource_table_capacity(mut self, capacity: usize) -> Self {
        self.inner.resource_table_capacity = capacity;
        self
    }

    pub fn build(self) -> Result<RuntimeConfig, RuntimeConfigError> {
        self.inner.validate()?;
        Ok(self.inner)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeConfigError {
    Invalid(&'static str),
}

struct RuntimeConfigState {
    current: RwLock<RuntimeConfig>,
    generation: AtomicU64,
}

impl RuntimeConfigState {
    fn new() -> Self {
        Self {
            current: RwLock::new(RuntimeConfig::default()),
            generation: AtomicU64::new(0),
        }
    }
}

static CONFIG: OnceLock<RuntimeConfigState> = OnceLock::new();

fn state() -> &'static RuntimeConfigState {
    CONFIG.get_or_init(RuntimeConfigState::new)
}

pub fn install_runtime_config(config: RuntimeConfig) -> Result<(), RuntimeConfigError> {
    config.validate()?;
    let state = state();
    {
        let mut guard = state
            .current
            .write()
            .expect("runtime config poisoned while installing");
        *guard = config;
    }
    state.generation.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

pub fn runtime_config_snapshot() -> RuntimeConfig {
    let state = state();
    state
        .current
        .read()
        .expect("runtime config poisoned while reading")
        .clone()
}

pub fn with_runtime_config<R>(f: impl FnOnce(&RuntimeConfig) -> R) -> R {
    let state = state();
    let guard = state
        .current
        .read()
        .expect("runtime config poisoned while reading");
    f(&guard)
}

pub fn runtime_config_generation() -> u64 {
    let state = state();
    state.generation.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::test_lock::runtime_test_guard;

    struct ConfigReset(RuntimeConfig);

    impl ConfigReset {
        fn capture() -> Self {
            ConfigReset(runtime_config_snapshot())
        }
    }

    impl Drop for ConfigReset {
        fn drop(&mut self) {
            let _ = install_runtime_config(self.0.clone());
        }
    }

    #[test]
    fn builder_validates_limits() {
        let _guard = runtime_test_guard();
        assert!(matches!(
            RuntimeConfig::builder()
                .max_allocation_bytes(0)
                .build()
                .unwrap_err(),
            RuntimeConfigError::Invalid(_)
        ));
        assert!(matches!(
            RuntimeConfig::builder()
                .resource_table_capacity(0)
                .build()
                .unwrap_err(),
            RuntimeConfigError::Invalid(_)
        ));
    }

    #[test]
    fn install_updates_generation_and_snapshot() {
        let _guard = runtime_test_guard();
        let _reset = ConfigReset::capture();
        let start_generation = runtime_config_generation();
        let config = RuntimeConfig::builder()
            .max_allocation_bytes(4096)
            .track_shared_allocations(true)
            .resource_table_capacity(32)
            .leak_strategy(LeakStrategy::Panic)
            .build()
            .unwrap();
        install_runtime_config(config.clone()).unwrap();
        let snapshot = runtime_config_snapshot();
        assert_eq!(snapshot.max_allocation_bytes, 4096);
        assert!(snapshot.track_shared_allocations);
        assert_eq!(snapshot.resource_table_capacity, 32);
        assert_eq!(snapshot.leak_strategy, LeakStrategy::Panic);
        assert_eq!(runtime_config_generation(), start_generation + 1);
    }
}
