use super::helpers::*;

#[test]
fn captures_union_and_enum_descriptors() {
    fn build_union() -> UnionDecl {
        UnionDecl {
            visibility: Visibility::Public,
            name: "Register".into(),
            members: vec![
                UnionMember::Field(UnionField {
                    visibility: Visibility::Public,
                    name: "Raw".into(),
                    ty: TypeExpr::simple("uint"),
                    is_readonly: false,
                    doc: doc("raw register value"),
                    attributes: vec![attr("Data")],
                }),
                UnionMember::View(UnionViewDecl {
                    visibility: Visibility::Public,
                    name: "Bits".into(),
                    fields: vec![FieldDecl {
                        visibility: Visibility::Public,
                        name: "Low".into(),
                        ty: TypeExpr::simple("ushort"),
                        initializer: None,
                        mmio: Some(MmioFieldAttr {
                            offset: 0,
                            width_bits: 16,
                            access: MmioAccess::ReadWrite,
                        }),
                        doc: doc("lower bits"),
                        is_required: false,
                        display_name: Some("LowField".into()),
                        attributes: vec![attr("Bits")],
                        is_readonly: false,
                        is_static: false,
                        view_of: None,
                    }],
                    is_readonly: true,
                    doc: doc("bit view"),
                    attributes: vec![attr("View")],
                }),
            ],
            thread_safe_override: None,
            shareable_override: None,
            copy_override: None,
            doc: doc("mmio register"),
            generics: None,
            attributes: vec![attr("Mmio")],
        }
    }

    fn build_enum() -> EnumDecl {
        EnumDecl {
            visibility: Visibility::Public,
            name: "State".into(),
            underlying_type: None,
            variants: vec![
                EnumVariant {
                    name: "Ready".into(),
                    fields: Vec::new(),
                    discriminant: None,
                    doc: doc("ready"),
                },
                EnumVariant {
                    name: "Busy".into(),
                    fields: vec![FieldDecl {
                        visibility: Visibility::Public,
                        name: "Code".into(),
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
                    }],
                    discriminant: None,
                    doc: None,
                },
            ],
            thread_safe_override: None,
            shareable_override: None,
            copy_override: None,
            is_flags: false,
            doc: None,
            generics: None,
            attributes: vec![attr("StateMachine")],
        }
    }

    let mut module = Module::new(Some("Root".into()));
    module.push_item(Item::Union(build_union()));
    module.push_item(Item::Enum(build_enum()));

    let tables = DescriptorQuery::collect(&module);
    let register = tables
        .types
        .iter()
        .find(|ty| ty.name == "Root::Register")
        .expect("union descriptor missing");
    let register_attrs: Vec<_> = register
        .attributes
        .iter()
        .map(|attr| attr.name.as_str())
        .collect();
    assert_eq!(register_attrs, vec!["Mmio"]);
    assert!(
        register
            .members
            .iter()
            .any(|member| member.kind == MemberKind::UnionField),
        "union should emit field members"
    );
    let view_member = register
        .members
        .iter()
        .find(|member| member.kind == MemberKind::UnionView)
        .expect("view member missing");
    assert_eq!(view_member.children.len(), 1, "view should expose fields");

    let state_enum = tables
        .types
        .iter()
        .find(|ty| ty.name == "Root::State")
        .expect("enum descriptor missing");
    let enum_attrs: Vec<_> = state_enum
        .attributes
        .iter()
        .map(|attr| attr.name.as_str())
        .collect();
    assert_eq!(enum_attrs, vec!["StateMachine"]);
    assert_eq!(
        state_enum
            .members
            .iter()
            .filter(|member| member.kind == MemberKind::EnumVariant)
            .count(),
        2
    );
    let busy = state_enum
        .members
        .iter()
        .find(|member| member.name == "Busy")
        .expect("busy variant missing");
    assert_eq!(busy.children.len(), 1, "variant should carry field members");
}
