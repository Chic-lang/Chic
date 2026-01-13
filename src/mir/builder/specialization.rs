use crate::mir::Ty;
use std::fmt::Write;

/// Records a request to specialise a generic function for concrete type arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunctionSpecialization {
    pub base: String,
    pub specialized: String,
    pub type_args: Vec<Ty>,
}

/// Generate a stable, human-readable mangled name for a specialised function.
pub(crate) fn specialised_function_name(base: &str, type_args: &[Ty]) -> String {
    if type_args.is_empty() {
        return base.to_string();
    }
    let mut name = String::with_capacity(base.len() + 8 * type_args.len());
    name.push_str(base);
    name.push('<');
    let mut first = true;
    for arg in type_args {
        if !first {
            name.push(',');
        }
        first = false;
        // Use canonical names to keep mangling deterministic and readable.
        let fragment = sanitise_fragment(&arg.canonical_name());
        write!(&mut name, "{fragment}").ok();
    }
    name.push('>');
    name
}

fn sanitise_fragment(fragment: &str) -> String {
    fragment
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '<' | '>' | ',' | ':' | '.' => ch,
            _ => '_',
        })
        .collect()
}
