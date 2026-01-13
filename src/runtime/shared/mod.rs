const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; shared ownership runtime lives in the native runtime."
);

pub mod config;
mod native;
pub mod resources;
mod shim_types;

pub use config::{
    LeakStrategy, RuntimeConfig, RuntimeConfigBuilder, RuntimeConfigError, install_runtime_config,
    runtime_config_generation, runtime_config_snapshot, with_runtime_config,
};
pub use resources::{
    ResourceKind, ResourceRecord, ResourceTableStats, register_shared_allocation,
    release_shared_allocation, resource_table_stats, teardown_tracked_allocations,
};
pub use shim_types::*;

pub use native::{
    chic_rt_arc_clone, chic_rt_arc_downgrade, chic_rt_arc_drop, chic_rt_arc_get,
    chic_rt_arc_get_data, chic_rt_arc_get_mut, chic_rt_arc_new, chic_rt_arc_strong_count,
    chic_rt_arc_weak_count, chic_rt_object_new, chic_rt_rc_clone, chic_rt_rc_downgrade,
    chic_rt_rc_drop, chic_rt_rc_get, chic_rt_rc_get_mut, chic_rt_rc_new, chic_rt_rc_strong_count,
    chic_rt_rc_weak_count, chic_rt_weak_clone, chic_rt_weak_drop, chic_rt_weak_rc_clone,
    chic_rt_weak_rc_drop, chic_rt_weak_rc_upgrade, chic_rt_weak_upgrade,
};
