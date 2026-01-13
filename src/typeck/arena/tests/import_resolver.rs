#![cfg(test)]

use super::fixtures::{make_type_expr, module_with_imports};
use crate::mir::TypeLayoutTable;
use crate::typeck::TypeckQueries;
use crate::typeck::arena::ImportResolution;

#[test]
fn typechecker_resolves_namespace_import() {
    let module = module_with_imports(false);
    let layouts = TypeLayoutTable::default();
    let queries = TypeckQueries::new(&module, &layouts);
    let expr = make_type_expr("Widget");
    match queries.resolve_type_expr(&expr, None, None) {
        ImportResolution::Found(name) => assert_eq!(name, "Alpha::Widget"),
        other => panic!("unexpected resolution {other:?}"),
    }
}

#[test]
fn typechecker_resolves_alias_import() {
    let module = module_with_imports(true);
    let layouts = TypeLayoutTable::default();
    let queries = TypeckQueries::new(&module, &layouts);
    let expr = make_type_expr("Alias.Widget");
    match queries.resolve_type_expr(&expr, None, None) {
        ImportResolution::Found(name) => assert_eq!(name, "Alpha::Widget"),
        other => panic!("unexpected resolution {other:?}"),
    }
}
