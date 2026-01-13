//! Serialization helpers for reflection metadata tables.

use super::ReflectionTables;
use serde_json;

#[derive(Debug, Default)]
pub(crate) struct ReflectionEmitter;

impl ReflectionEmitter {
    pub(crate) fn to_pretty_json(tables: &ReflectionTables) -> Result<String, serde_json::Error> {
        let mut clone = tables.clone();
        sort_tables(&mut clone);
        serde_json::to_string_pretty(&clone)
    }

    pub(crate) fn from_str(input: &str) -> Result<ReflectionTables, serde_json::Error> {
        serde_json::from_str(input)
    }
}

fn sort_tables(tables: &mut ReflectionTables) {
    tables.types.sort_by(|a, b| a.full_name.cmp(&b.full_name));
    tables.aliases.sort_by(|a, b| a.full_name.cmp(&b.full_name));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::metadata::reflection::{
        MemberDescriptor, MemberKind, TypeDescriptor, TypeHandle, TypeKind, VisibilityDescriptor,
    };

    fn make_type(name: &str, members: Vec<MemberDescriptor>) -> TypeDescriptor {
        TypeDescriptor {
            namespace: None,
            name: name.to_string(),
            full_name: name.to_string(),
            type_id: None,
            kind: TypeKind::Struct,
            visibility: VisibilityDescriptor::Public,
            is_generic: false,
            generic_arguments: Vec::new(),
            bases: Vec::new(),
            attributes: Vec::new(),
            underlying_type: None,
            members,
            layout: None,
            layout_hints: None,
            readonly: false,
        }
    }

    fn member(name: &str) -> MemberDescriptor {
        MemberDescriptor {
            name: name.to_string(),
            kind: MemberKind::Method,
            visibility: VisibilityDescriptor::Public,
            declaring_type: TypeHandle {
                name: "Owner".to_string(),
                type_id: None,
            },
            attributes: Vec::new(),
            field: None,
            property: None,
            method: None,
            constructor: None,
            children: Vec::new(),
        }
    }

    #[test]
    fn emitter_sorts_types_and_members() {
        let tables = ReflectionTables {
            version: ReflectionTables::default().version,
            types: vec![
                make_type("Beta", vec![member("Run"), member("Apply"), member("Zoo")]),
                make_type("Alpha", vec![member("Bee"), member("Axe")]),
            ],
            aliases: Vec::new(),
        };

        let json = ReflectionEmitter::to_pretty_json(&tables).expect("serialize ok");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("serialized JSON should parse");
        let types = value["types"]
            .as_array()
            .expect("types array should exist")
            .to_owned();
        assert_eq!(
            types[0]["name"].as_str(),
            Some("Alpha"),
            "types should be alphabetically sorted"
        );
        assert_eq!(
            types[1]["name"].as_str(),
            Some("Beta"),
            "Beta should be second after sorting"
        );
        let member_array = types[1]["members"]
            .as_array()
            .expect("members array")
            .to_owned();
        let member_names: Vec<_> = member_array
            .iter()
            .map(|member| member["name"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(member_names, vec!["Run", "Apply", "Zoo"]);
    }

    #[test]
    fn deserialize_reports_errors() {
        let result = ReflectionEmitter::from_str("not-json");
        assert!(result.is_err(), "invalid json should error");
    }
}
