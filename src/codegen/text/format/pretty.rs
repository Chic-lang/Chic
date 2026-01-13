use crate::frontend::ast::{BindingModifier, ConstDeclaration, Parameter, Visibility};

pub(crate) fn format_const_declarators(declaration: &ConstDeclaration) -> String {
    if declaration.declarators.is_empty() {
        return String::from("/* <no declarators> */");
    }
    declaration
        .declarators
        .iter()
        .map(|declarator| {
            let value = declarator.initializer.text.trim();
            if value.is_empty() {
                format!("{} = <invalid>", declarator.name)
            } else {
                format!("{} = {}", declarator.name, value)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn format_parameter(param: &Parameter) -> String {
    let modifier = match (param.binding, param.binding_nullable) {
        (BindingModifier::In, _) => "in ",
        (BindingModifier::Ref, true) => "ref? ",
        (BindingModifier::Ref, false) => "ref ",
        (BindingModifier::Out, true) => "out? ",
        (BindingModifier::Out, false) => "out ",
        (BindingModifier::Value, _) => "",
    };
    format!("{}{} {}", modifier, param.ty.name, param.name)
}

pub(crate) fn format_union_modifiers(is_readonly: bool) -> String {
    if is_readonly {
        "readonly ".to_string()
    } else {
        String::new()
    }
}

pub(crate) fn format_visibility(vis: Visibility) -> &'static str {
    match vis {
        Visibility::Public => "public ",
        Visibility::Internal => "internal ",
        Visibility::Protected => "protected ",
        Visibility::Private => "private ",
        Visibility::ProtectedInternal => "protected internal ",
        Visibility::PrivateProtected => "private protected ",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::{ConstDeclarator, Expression, TypeExpr};

    #[test]
    fn formats_const_declarators_and_invalids() {
        let decl = ConstDeclaration {
            ty: TypeExpr::simple("int"),
            declarators: vec![
                ConstDeclarator {
                    name: "A".into(),
                    initializer: Expression::new("1", None),
                    span: None,
                },
                ConstDeclarator {
                    name: "B".into(),
                    initializer: Expression::new("", None),
                    span: None,
                },
            ],
            doc: None,
            span: None,
        };
        assert_eq!(format_const_declarators(&decl), "A = 1, B = <invalid>");
    }

    #[test]
    fn formats_parameters_with_bindings() {
        let param_in = Parameter {
            binding: BindingModifier::In,
            binding_nullable: false,
            name: "value".into(),
            name_span: None,
            ty: TypeExpr::simple("int"),
            attributes: Vec::new(),
            di_inject: None,
            default: None,
            default_span: None,
            lends: None,
            is_extension_this: false,
        };
        let param_ref = Parameter {
            binding: BindingModifier::Ref,
            binding_nullable: true,
            name: "other".into(),
            name_span: None,
            ty: TypeExpr::simple("string"),
            attributes: Vec::new(),
            di_inject: None,
            default: None,
            default_span: None,
            lends: None,
            is_extension_this: false,
        };
        assert_eq!(format_parameter(&param_in), "in int value");
        assert_eq!(format_parameter(&param_ref), "ref? string other");
    }

    #[test]
    fn formats_visibility_and_union_modifiers() {
        assert_eq!(format_visibility(Visibility::Private), "private ");
        assert_eq!(format_union_modifiers(true), "readonly ");
        assert_eq!(format_union_modifiers(false), "");
    }
}
