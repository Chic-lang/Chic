mod catalog;
mod memory;
mod references;
mod shared;
mod tables;

pub(crate) use catalog::{
    FixtureCategory, FixtureDescriptor, fixture_catalog, fixtures_by_category,
};
pub(crate) use memory::{
    array_index_fixture, str_index_fixture, string_index_fixture,
    struct_with_missing_offsets_fixture, struct_without_size_layout,
};
pub(crate) use references::{sample_class_layout, sample_pair_layout, struct_projection_fixture};
pub(crate) use tables::{
    enum_layout_table, enum_projection_fixture, tuple_aggregate_fixture, tuple_copy_fixture,
    tuple_layout_table, tuple_param_fixture, union_layout_table, union_projection_fixture,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_all_categories() {
        let grouped = fixtures_by_category();
        assert_eq!(grouped.len(), 3);
        let all = fixture_catalog();
        assert!(!all.is_empty());
        let first: &FixtureDescriptor = &all[0];
        assert!(!first.description.is_empty());
        assert!(matches!(
            first.category,
            FixtureCategory::Memory | FixtureCategory::Tables | FixtureCategory::References
        ));
    }
}
