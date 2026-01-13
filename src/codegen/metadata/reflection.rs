//! Reflection metadata emission helpers.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::frontend::ast::Module;
use crate::frontend::metadata::{collect_reflection_tables, serialize_reflection_tables};

/// Compute the path of the reflection manifest adjacent to the primary output.
pub(crate) fn reflection_manifest_path(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let manifest_name = format!("{file_name}.reflect.json");
    output.with_file_name(manifest_name)
}

/// Write the reflection descriptor manifest for the given module.
pub(crate) fn write_reflection_manifest(module: &Module, output: &Path) -> Result<PathBuf, Error> {
    let path = reflection_manifest_path(output);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let tables = collect_reflection_tables(module);
    let json = serialize_reflection_tables(&tables)
        .map_err(|err| {
            Error::Codegen(format!(
                "failed to serialise reflection manifest for {}: {err}",
                path.display()
            ))
        })?
        .into_bytes();

    fs::write(&path, json)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::items::{
        BindingModifier, FieldDecl, FunctionDecl, Item, MemberDispatch, Parameter, Signature,
        StructDecl, Visibility,
    };
    use crate::frontend::ast::types::TypeExpr;
    use tempfile::tempdir;

    #[test]
    fn writes_manifest_with_public_descriptors() {
        let mut module = Module::new(Some("Reflect".into()));
        module.push_item(Item::Struct(StructDecl {
            visibility: Visibility::Public,
            name: "Pair".into(),
            fields: vec![
                FieldDecl {
                    visibility: Visibility::Public,
                    name: "Left".into(),
                    ty: TypeExpr::simple("int"),
                    initializer: None,
                    mmio: None,
                    doc: None,
                    is_required: false,
                    display_name: None,
                    attributes: Vec::new(),
                    is_readonly: false,
                    is_static: false,
                    view_of: None,
                },
                FieldDecl {
                    visibility: Visibility::Public,
                    name: "Right".into(),
                    ty: TypeExpr::simple("int"),
                    initializer: None,
                    mmio: None,
                    doc: None,
                    is_required: false,
                    display_name: None,
                    attributes: Vec::new(),
                    is_readonly: false,
                    is_static: false,
                    view_of: None,
                },
            ],
            properties: Vec::new(),
            constructors: Vec::new(),
            consts: Vec::new(),
            methods: Vec::new(),
            nested_types: Vec::new(),
            bases: Vec::new(),
            thread_safe_override: None,
            shareable_override: None,
            copy_override: None,
            mmio: None,
            doc: None,
            generics: None,
            attributes: Vec::new(),
            is_readonly: false,
            layout: None,
            is_intrinsic: false,
            inline_attr: None,
            is_record: false,
            record_positional_fields: Vec::new(),
        }));
        module.push_item(Item::Function(FunctionDecl {
            visibility: Visibility::Public,
            name: "Add".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![
                    Parameter {
                        binding: BindingModifier::Value,
                        binding_nullable: false,
                        name: "left".into(),
                        name_span: None,
                        ty: TypeExpr::simple("int"),
                        attributes: Vec::new(),
                        di_inject: None,
                        default: None,
                        default_span: None,
                        lends: None,
                        is_extension_this: false,
                    },
                    Parameter {
                        binding: BindingModifier::Value,
                        binding_nullable: false,
                        name: "right".into(),
                        name_span: None,
                        ty: TypeExpr::simple("int"),
                        attributes: Vec::new(),
                        di_inject: None,
                        default: None,
                        default_span: None,
                        lends: None,
                        is_extension_this: false,
                    },
                ],
                return_type: TypeExpr::simple("int"),
                lends_to_return: None,
                variadic: false,
                throws: None,
            },
            body: None,
            is_async: false,
            is_constexpr: false,
            doc: None,
            modifiers: Vec::new(),
            is_unsafe: false,
            attributes: Vec::new(),
            is_extern: false,
            extern_abi: None,
            extern_options: None,
            link_name: None,
            link_library: None,
            operator: None,
            generics: None,
            vectorize_hint: None,
            dispatch: MemberDispatch::default(),
        }));

        let tmp = tempdir().expect("temp dir");
        let output = tmp.path().join("libreflect.a");
        let manifest_path =
            write_reflection_manifest(&module, &output).expect("write reflection manifest");
        assert!(
            manifest_path.exists(),
            "reflection manifest missing at {}",
            manifest_path.display()
        );

        let json = fs::read(&manifest_path).expect("read reflection manifest");
        let value: serde_json::Value =
            serde_json::from_slice(&json).expect("parse reflection manifest json");
        assert_eq!(
            value["version"].as_u64(),
            Some(2),
            "reflection manifest should include schema version"
        );
        let types = value["types"]
            .as_array()
            .expect("types array missing in reflection manifest");
        assert!(
            types.iter().any(|ty| ty["name"] == "Reflect::Pair"),
            "missing Pair descriptor: {types:?}"
        );
        assert!(
            types.iter().any(|ty| ty["name"] == "Reflect::Add"),
            "missing Add descriptor: {types:?}"
        );
    }
}
