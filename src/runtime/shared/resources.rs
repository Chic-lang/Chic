use crate::runtime::shared::config::{self, LeakStrategy, RuntimeConfig};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Rc,
    Arc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceRecord {
    pub kind: ResourceKind,
    pub size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceTableStats {
    pub total_registered: usize,
    pub pending: usize,
    pub leaks: usize,
}

impl Default for ResourceTableStats {
    fn default() -> Self {
        Self {
            total_registered: 0,
            pending: 0,
            leaks: 0,
        }
    }
}

#[derive(Debug)]
enum ResourceError {
    CapacityExceeded,
}

struct ResourceTable {
    state: Mutex<ResourceTableState>,
}

struct ResourceTableState {
    entries: HashMap<usize, ResourceRecord>,
    total_registered: usize,
    generation: u64,
}

impl Default for ResourceTableState {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            total_registered: 0,
            generation: crate::runtime::shared::config::runtime_config_generation(),
        }
    }
}

impl ResourceTable {
    fn new() -> Self {
        Self {
            state: Mutex::new(ResourceTableState::default()),
        }
    }

    fn refresh_generation(state: &mut ResourceTableState, current: u64) {
        if state.generation != current {
            state.entries.clear();
            state.generation = current;
            // We intentionally do not reset total_registered so capacity stats remain monotonic.
        }
    }

    fn register(
        &self,
        ptr: *mut u8,
        record: ResourceRecord,
        capacity: usize,
    ) -> Result<(), ResourceError> {
        let mut guard = self.state.lock().expect("resource table poisoned");
        Self::refresh_generation(
            &mut guard,
            crate::runtime::shared::config::runtime_config_generation(),
        );
        if guard.entries.len() >= capacity {
            return Err(ResourceError::CapacityExceeded);
        }
        guard.entries.insert(ptr as usize, record);
        guard.total_registered += 1;
        Ok(())
    }

    fn release(&self, ptr: *mut u8) -> Result<(), ResourceError> {
        let mut guard = self.state.lock().expect("resource table poisoned");
        Self::refresh_generation(
            &mut guard,
            crate::runtime::shared::config::runtime_config_generation(),
        );
        if guard.entries.remove(&(ptr as usize)).is_some() {
            return Ok(());
        }
        // When the runtime config toggles tracking on/off mid-execution, pre-existing
        // allocations may not have been registered. Treat missing entries as a no-op
        // instead of panicking so unrelated tests and users are unaffected.
        Ok(())
    }

    fn teardown(&self) -> Vec<ResourceRecord> {
        let mut guard = self.state.lock().expect("resource table poisoned");
        let leaks: Vec<_> = guard.entries.values().copied().collect();
        guard.entries.clear();
        leaks
    }

    fn stats(&self) -> ResourceTableStats {
        let guard = self.state.lock().expect("resource table poisoned");
        ResourceTableStats {
            total_registered: guard.total_registered,
            pending: guard.entries.len(),
            leaks: 0,
        }
    }
}

static RESOURCE_TABLE: OnceLock<ResourceTable> = OnceLock::new();

fn table() -> &'static ResourceTable {
    RESOURCE_TABLE.get_or_init(ResourceTable::new)
}

pub fn register_shared_allocation(kind: ResourceKind, ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }
    let cfg = config::runtime_config_snapshot();
    if !cfg.track_shared_allocations {
        return;
    }
    if let Err(err) = table().register(
        ptr,
        ResourceRecord { kind, size },
        cfg.resource_table_capacity,
    ) {
        handle_resource_error(err, &cfg, Some(ptr));
    }
}

pub fn release_shared_allocation(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let cfg = config::runtime_config_snapshot();
    if !cfg.track_shared_allocations {
        return;
    }
    if let Err(err) = table().release(ptr) {
        handle_resource_error(err, &cfg, Some(ptr));
    }
}

pub fn teardown_tracked_allocations() -> ResourceTableStats {
    let cfg = config::runtime_config_snapshot();
    if !cfg.track_shared_allocations {
        return ResourceTableStats::default();
    }
    let leaks = table().teardown();
    if leaks.is_empty() {
        return ResourceTableStats {
            leaks: 0,
            ..table().stats()
        };
    }
    let stats = table().stats();
    let leak_summary = format!(
        "{} leaked shared allocation(s); sample={:?}",
        leaks.len(),
        leaks.first()
    );
    match cfg.leak_strategy {
        LeakStrategy::Ignore => {}
        LeakStrategy::Warn => warn!(target: "runtime", "{}", leak_summary),
        LeakStrategy::Panic => panic!("{}", leak_summary),
    }
    ResourceTableStats {
        leaks: leaks.len(),
        ..stats
    }
}

pub fn resource_table_stats() -> ResourceTableStats {
    table().stats()
}

fn handle_resource_error(err: ResourceError, cfg: &RuntimeConfig, ptr: Option<*mut u8>) {
    match cfg.leak_strategy {
        LeakStrategy::Ignore => {}
        LeakStrategy::Warn => warn!(
            target: "runtime",
            error = ?err,
            ptr = ?ptr,
            "shared resource tracking error"
        ),
        LeakStrategy::Panic => panic!("shared resource tracking error {:?} for {:?}", err, ptr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::shared::config::{
        RuntimeConfig, install_runtime_config, runtime_config_snapshot,
    };
    use crate::runtime::test_lock::runtime_test_guard;

    struct ConfigReset(RuntimeConfig);

    impl ConfigReset {
        fn install(next: RuntimeConfig) -> Self {
            let prev = runtime_config_snapshot();
            install_runtime_config(next).unwrap();
            ConfigReset(prev)
        }
    }

    impl Drop for ConfigReset {
        fn drop(&mut self) {
            let _ = install_runtime_config(self.0.clone());
        }
    }

    #[test]
    fn register_and_release_track_allocations() {
        let _lock = runtime_test_guard();
        let config = RuntimeConfig {
            max_allocation_bytes: 1024,
            track_shared_allocations: true,
            leak_strategy: LeakStrategy::Panic,
            resource_table_capacity: 4,
        };
        let _reset = ConfigReset::install(config);
        teardown_tracked_allocations();
        let ptr = 0x1234 as *mut u8;
        register_shared_allocation(ResourceKind::Rc, ptr, 4);
        assert_eq!(resource_table_stats().pending, 1);
        release_shared_allocation(ptr);
        assert_eq!(resource_table_stats().pending, 0);
    }

    #[test]
    fn teardown_reports_leaks_and_resets_table() {
        let _lock = runtime_test_guard();
        let config = RuntimeConfig {
            max_allocation_bytes: 1024,
            track_shared_allocations: true,
            leak_strategy: LeakStrategy::Warn,
            resource_table_capacity: 4,
        };
        let _reset = ConfigReset::install(config);
        teardown_tracked_allocations();
        register_shared_allocation(ResourceKind::Arc, 0x22 as *mut u8, 8);
        register_shared_allocation(ResourceKind::Arc, 0x33 as *mut u8, 8);
        let stats = teardown_tracked_allocations();
        assert_eq!(stats.leaks, 2);
        assert_eq!(resource_table_stats().pending, 0);
    }

    #[test]
    fn capacity_exceeded_triggers_strategy() {
        let _lock = runtime_test_guard();
        let config = RuntimeConfig {
            max_allocation_bytes: 1024,
            track_shared_allocations: true,
            leak_strategy: LeakStrategy::Ignore,
            resource_table_capacity: 1,
        };
        let _reset = ConfigReset::install(config);
        teardown_tracked_allocations();
        register_shared_allocation(ResourceKind::Arc, 0x1 as *mut u8, 4);
        register_shared_allocation(ResourceKind::Arc, 0x2 as *mut u8, 4);
        assert!(resource_table_stats().pending <= 1);
        teardown_tracked_allocations();
    }
}
