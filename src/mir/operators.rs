use std::collections::HashMap;

use crate::mir::data::{BinOp, UnOp};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConversionKind {
    Implicit,
    Explicit,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OperatorKind {
    Unary(UnOp),
    Binary(BinOp),
    Conversion(ConversionKind),
}

#[derive(Clone, Debug)]
pub struct OperatorOverload {
    pub kind: OperatorKind,
    pub params: Vec<String>,
    pub result: String,
    pub function: String,
}

#[derive(Clone, Debug, Default)]
pub struct OperatorRegistry {
    overloads: HashMap<String, Vec<OperatorOverload>>,
}

#[derive(Clone, Debug)]
pub enum OperatorMatch<'a> {
    None,
    Found(&'a OperatorOverload),
    Ambiguous(Vec<&'a OperatorOverload>),
}

#[derive(Clone, Debug)]
pub enum ConversionResolution<'a> {
    None {
        explicit_candidates: Vec<&'a OperatorOverload>,
    },
    Found(&'a OperatorOverload),
    Ambiguous(Vec<&'a OperatorOverload>),
}

impl OperatorRegistry {
    pub fn register(&mut self, owner: &str, overload: OperatorOverload) {
        self.overloads
            .entry(owner.to_string())
            .or_default()
            .push(overload);
    }

    pub fn resolve_unary<'a>(&'a self, operand_ty: &str, op: UnOp) -> OperatorMatch<'a> {
        let owners = self.owner_candidates(operand_ty);
        let mut matches = Vec::new();
        for owner in owners {
            if let Some(overloads) = self.overloads.get(owner) {
                for overload in overloads {
                    if let OperatorKind::Unary(kind) = overload.kind {
                        if kind == op
                            && overload.params.len() == 1
                            && type_matches(&overload.params[0], operand_ty)
                        {
                            matches.push(overload);
                        }
                    }
                }
            }
        }
        match matches.len() {
            0 => OperatorMatch::None,
            1 => OperatorMatch::Found(matches[0]),
            _ => OperatorMatch::Ambiguous(matches),
        }
    }

    pub fn resolve_binary<'a>(
        &'a self,
        lhs_ty: &str,
        rhs_ty: &str,
        op: BinOp,
    ) -> OperatorMatch<'a> {
        let mut owners = self.owner_candidates(lhs_ty);
        owners.extend(self.owner_candidates(rhs_ty));
        owners.sort_unstable();
        owners.dedup();

        let mut matches = Vec::new();
        for owner in owners {
            if let Some(overloads) = self.overloads.get(owner) {
                for overload in overloads {
                    if let OperatorKind::Binary(kind) = overload.kind {
                        if kind == op
                            && overload.params.len() == 2
                            && type_matches(&overload.params[0], lhs_ty)
                            && type_matches(&overload.params[1], rhs_ty)
                        {
                            matches.push(overload);
                        }
                    }
                }
            }
        }

        match matches.len() {
            0 => OperatorMatch::None,
            1 => OperatorMatch::Found(matches[0]),
            _ => OperatorMatch::Ambiguous(matches),
        }
    }

    pub fn resolve_conversion<'a>(
        &'a self,
        source_ty: &str,
        target_ty: &str,
        allow_explicit: bool,
    ) -> ConversionResolution<'a> {
        let matches = self.matching_conversion_candidates(source_ty, target_ty);
        if matches.is_empty() {
            return ConversionResolution::None {
                explicit_candidates: Vec::new(),
            };
        }

        let mut implicit = Vec::new();
        let mut explicit = Vec::new();
        for candidate in matches {
            match candidate.kind {
                OperatorKind::Conversion(ConversionKind::Implicit) => implicit.push(candidate),
                OperatorKind::Conversion(ConversionKind::Explicit) => explicit.push(candidate),
                _ => {}
            }
        }

        if !implicit.is_empty() {
            if implicit.len() == 1 {
                return ConversionResolution::Found(implicit[0]);
            }
            return ConversionResolution::Ambiguous(implicit);
        }

        if allow_explicit && !explicit.is_empty() {
            if explicit.len() == 1 {
                return ConversionResolution::Found(explicit[0]);
            }
            return ConversionResolution::Ambiguous(explicit);
        }

        ConversionResolution::None {
            explicit_candidates: explicit,
        }
    }

    fn matching_conversion_candidates<'a>(
        &'a self,
        source_ty: &str,
        target_ty: &str,
    ) -> Vec<&'a OperatorOverload> {
        let mut owners = self.owner_candidates(source_ty);
        owners.extend(self.owner_candidates(target_ty));
        owners.sort_unstable();
        owners.dedup();

        let mut matches = Vec::new();
        for owner in owners {
            if let Some(overloads) = self.overloads.get(owner) {
                for overload in overloads {
                    if let OperatorKind::Conversion(_) = overload.kind {
                        if overload.params.len() == 1
                            && type_matches(&overload.params[0], source_ty)
                            && type_matches(&overload.result, target_ty)
                        {
                            matches.push(overload);
                        }
                    }
                }
            }
        }
        matches
    }

    fn owner_candidates<'a>(&'a self, ty: &'a str) -> Vec<&'a str> {
        let mut owners = Vec::new();
        if self.overloads.contains_key(ty) {
            owners.push(ty);
            return owners;
        }
        let short = short_name(ty);
        for key in self.overloads.keys() {
            if short_name(key) == short {
                owners.push(key.as_str());
            }
        }
        owners
    }
}

fn type_matches(expected: &str, actual: &str) -> bool {
    let expected = strip_nullable(expected);
    let actual = strip_nullable(actual);
    let expected_base = strip_generics(expected);
    let actual_base = strip_generics(actual);
    expected_base == actual_base || short_name(expected_base) == short_name(actual_base)
}

fn short_name(name: &str) -> &str {
    strip_generics(name)
        .rsplit("::")
        .next()
        .unwrap_or(name)
        .trim()
}

fn strip_generics(name: &str) -> &str {
    name.split('<').next().unwrap_or(name).trim()
}

fn strip_nullable(name: &str) -> &str {
    name.strip_suffix('?').unwrap_or(name)
}

#[cfg(test)]
mod tests;
