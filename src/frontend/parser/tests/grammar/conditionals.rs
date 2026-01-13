use crate::frontend::ast::Item;
use crate::frontend::conditional::ConditionalDefines;
use crate::frontend::parser::parse_module_with_defines;

fn struct_names(items: &[Item]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(def) => Some(def.name.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn selects_debug_branch_when_flag_enabled() {
    let mut defines = ConditionalDefines::default();
    defines.set_bool("DEBUG", true);
    defines.set_bool("RELEASE", false);
    let source = r#"
        #if DEBUG
        public struct Active {}
        #else
        public struct Inactive {}
        #endif
    "#;

    let parsed = parse_module_with_defines(source, &defines).expect("parse module");
    let names = struct_names(&parsed.module.items);
    assert!(names.contains(&"Active"));
    assert!(!names.contains(&"Inactive"));
}

#[test]
fn evaluates_string_comparisons_in_elif() {
    let mut defines = ConditionalDefines::default();
    defines.set_bool("DEBUG", false);
    defines.set_bool("RELEASE", true);
    defines.set_string("TARGET_OS", "linux");
    let source = r#"
        #if DEBUG
        public struct DebugOnly {}
        #elif TARGET_OS == "linux"
        public struct LinuxPath {}
        #else
        public struct Other {}
        #endif
    "#;

    let parsed = parse_module_with_defines(source, &defines).expect("parse module");
    let names = struct_names(&parsed.module.items);
    assert!(names.contains(&"LinuxPath"));
    assert!(!names.contains(&"DebugOnly"));
    assert!(!names.contains(&"Other"));
}
