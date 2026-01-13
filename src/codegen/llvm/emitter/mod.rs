mod context;
mod dispatch;
mod drop_table;
mod eq_table;
mod function;
mod hash_table;
pub(crate) mod literals;
mod metadata;
pub(crate) mod metadata_pool;
mod module;
mod type_metadata;

pub(crate) use module::emit_module;
