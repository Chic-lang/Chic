use super::*;
use crate::frontend::parser::StaticMutability;

#[test]
fn parses_static_const_item_with_initializer() {
    let source = r#"
namespace Demo;

public static const int Answer = 42;
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Static(item) = &parse.module.items[0] else {
        panic!("expected static item, found {:?}", parse.module.items[0]);
    };
    assert_eq!(item.visibility, Visibility::Public);
    assert_eq!(item.declaration.mutability, StaticMutability::Const);
    assert_eq!(item.declaration.ty.name, "int");
    assert_eq!(item.declaration.declarators.len(), 1);
    let declarator = &item.declaration.declarators[0];
    assert_eq!(declarator.name, "Answer");
    let Some(init) = declarator.initializer.as_ref() else {
        panic!("expected initializer on static const");
    };
    assert_eq!(init.text.trim(), "42");
}

#[test]
fn parses_static_mut_without_initializer() {
    let source = r#"
namespace Demo;

internal static mut string CurrentUser;
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Static(item) = &parse.module.items[0] else {
        panic!("expected static item");
    };
    assert_eq!(item.visibility, Visibility::Internal);
    assert_eq!(item.declaration.mutability, StaticMutability::Mutable);
    assert_eq!(item.declaration.declarators.len(), 1);
    let declarator = &item.declaration.declarators[0];
    assert!(
        declarator.initializer.is_none(),
        "initializer should be optional"
    );
}
