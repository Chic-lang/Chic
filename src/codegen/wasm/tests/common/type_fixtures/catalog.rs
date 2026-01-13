use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct FixtureDescriptor {
    pub name: String,
    pub category: FixtureCategory,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FixtureCategory {
    Tables,
    Memory,
    References,
}

static CATALOG: OnceLock<Vec<FixtureDescriptor>> = OnceLock::new();

pub(crate) fn fixture_catalog() -> &'static [FixtureDescriptor] {
    CATALOG.get_or_init(|| {
        let raw = include_str!("catalog.json");
        serde_json::from_str(raw).expect("valid fixture catalog")
    })
}

pub(crate) fn fixtures_by_category() -> BTreeMap<FixtureCategory, Vec<FixtureDescriptor>> {
    let mut grouped: BTreeMap<FixtureCategory, Vec<FixtureDescriptor>> = BTreeMap::new();
    for fixture in fixture_catalog() {
        grouped
            .entry(fixture.category.clone())
            .or_default()
            .push(fixture.clone());
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_all_categories() {
        let grouped = fixtures_by_category();
        assert!(grouped.contains_key(&FixtureCategory::Memory));
        assert!(grouped.contains_key(&FixtureCategory::Tables));
        assert!(grouped.contains_key(&FixtureCategory::References));
    }

    #[test]
    fn catalog_entries_have_descriptions() {
        for entry in fixture_catalog() {
            assert!(
                !entry.description.is_empty(),
                "fixture {} missing description",
                entry.name
            );
        }
    }
}
