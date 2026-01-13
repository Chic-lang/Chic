use crate::frontend::diagnostics::{Diagnostic, DiagnosticCode, Span};

pub mod codes {
    pub const LAYOUT_REPR_MISMATCH: &str = "TCK001";
    pub const LAYOUT_PACK_MISMATCH: &str = "TCK002";
    pub const LAYOUT_ALIGN_MISMATCH: &str = "TCK003";
    pub const CONST_EVAL_FAILURE: &str = "TCK004";
    #[allow(dead_code)]
    pub const MULTIPLE_BASE_CLASSES: &str = "TCK110";
    #[allow(dead_code)]
    pub const INVALID_BASE_TYPE: &str = "TCK111";
    #[allow(dead_code)]
    pub const SEALED_BASE_INHERITANCE: &str = "TCK112";
    #[allow(dead_code)]
    pub const STATIC_BASE_INHERITANCE: &str = "TCK113";
    #[allow(dead_code)]
    pub const INACCESSIBLE_BASE: &str = "TCK114";
    #[allow(dead_code)]
    pub const ABSTRACT_SEALED_CLASS: &str = "TCK115";
    #[allow(dead_code)]
    pub const INTERFACE_CYCLE: &str = "TCK116";
    #[allow(dead_code)]
    pub const BASE_TYPE_NOT_FOUND: &str = "TCK117";

    pub const AMBIGUOUS_INTERFACE_BASE: &str = "TCK010";
    pub const AMBIGUOUS_EXTENSION_TARGET: &str = "TCK011";
    pub const INVALID_EXTENSION_TARGET_KIND: &str = "TCK012";
    pub const UNKNOWN_EXTENSION_TARGET: &str = "TCK013";
    pub const MISSING_EXTENSION_RECEIVER: &str = "TCK014";
    pub const INVALID_EXTENSION_RECEIVER: &str = "TCK015";
    pub const AMBIGUOUS_CLASS_BASE: &str = "TCK016";
    pub const INVALID_EXTENSION_CONTEXT: &str = "TCK017";
    pub const INVALID_EXTENSION_POSITION: &str = "TCK018";

    pub const DUPLICATE_GENERIC_PARAMETER: &str = "TCK020";
    pub const GENERIC_CONSTRAINT_VIOLATION: &str = "TCK022";
    pub const NUMERIC_LITERAL_SUFFIX_MISMATCH: &str = "TCK120";
    pub const NUMERIC_LITERAL_SUFFIX_OVERFLOW: &str = "TCK121";

    pub const UNKNOWN_TYPE: &str = "TCK030";
    pub const AMBIGUOUS_TYPE: &str = "TCK031";
    pub const GENERIC_ARGUMENT_MISMATCH: &str = "TCK032";
    pub const TYPE_NOT_GENERIC: &str = "TCK033";
    pub const TYPE_ALIAS_CYCLE: &str = "TCK034";
    pub const TYPE_ALIAS_CONFLICT: &str = "TCK048";
    pub const TYPE_ALIAS_CONST_PARAM: &str = "TCK049";
    pub const SIMD_LANES_CONST: &str = "TYPE0701";
    pub const SIMD_WIDTH_UNSUPPORTED: &str = "TYPE0702";
    #[allow(dead_code)]
    pub const SIMD_SHUFFLE_OOB: &str = "TYPE0703";
    #[allow(dead_code)]
    pub const SIMD_BACKEND_UNAVAILABLE: &str = "TYPE0704";
    pub const SIMD_ELEMENT_UNSUPPORTED: &str = "TYPE0705";
    pub const AUTO_TRAIT_REQUIRED: &str = "TCK035";
    pub const AUTO_TRAIT_UNPROVEN: &str = "TCK037";
    pub const ATOMIC_ORDERING_EXPECTED: &str = "MM0001";
    pub const ATOMIC_COMPARE_EXCHANGE_ORDER: &str = "MM0002";
    pub const ATOMIC_INNER_THREADSAFE: &str = "MM0003";
    pub const THREADS_UNAVAILABLE_ON_TARGET: &str = "MM0101";
    pub const THREADSAFE_REQUIRED: &str = "MM0102";
    pub const CONSTRUCTOR_TARGET_INVALID: &str = "TCK130";
    pub const CONSTRUCTOR_NO_MATCH: &str = "TCK131";
    #[allow(dead_code)]
    pub const CONSTRUCTOR_AMBIGUOUS: &str = "TCK132";
    pub const INITIALIZER_MEMBER_UNKNOWN: &str = "TCK133";
    pub const INITIALIZER_MEMBER_INACCESSIBLE: &str = "TCK134";
    pub const INITIALIZER_MEMBER_IMMUTABLE: &str = "TCK135";
    pub const STRUCT_INITIALIZER_MISSING_REQUIRED: &str = "TCK136";
    pub const INITIALIZER_MEMBER_DUPLICATE: &str = "TCK137";
    pub const INITIALIZER_MEMBER_STATIC: &str = "TCK138";
    pub const PUBLIC_MEMBER_INACCESSIBLE_TYPE: &str = "TCK148";
    pub const ABSTRACT_INSTANTIATION: &str = "TCK150";
    pub const ARRAY_LENGTH_REQUIRED: &str = "TCK139";
    pub const ARRAY_LENGTH_MISMATCH: &str = "TCK140";
    pub const ARRAY_INITIALIZER_UNSUPPORTED: &str = "TCK143";
    pub const ARRAY_RANK_UNSUPPORTED: &str = "TCK144";
    pub const ARRAY_LENGTH_NONCONST: &str = "TCK145";
    pub const ARRAY_IMPLICIT_TYPE_UNSUPPORTED: &str = "TCK147";
    pub const CALL_OVERLOAD_NO_MATCH: &str = "TCK141";
    #[allow(dead_code)]
    pub const CALL_OVERLOAD_AMBIGUOUS: &str = "TCK142";
    #[allow(dead_code)]
    pub const CROSS_FUNCTION_INFERENCE_FORBIDDEN: &str = "TCK146";

    pub const PARAMETER_NAME_DUPLICATE: &str = "TCK040";
    pub const PARAMETER_DEFAULT_ORDER: &str = "TCK044";
    pub const PARAMETER_DEFAULT_REF: &str = "TCK045";
    pub const PARAMETER_DEFAULT_CONFLICT: &str = "TCK046";
    pub const DEFAULT_LITERAL_INFER: &str = "TCK240";
    pub const DEFAULT_LITERAL_NONNULL: &str = "TCK241";
    pub const LENDS_UNKNOWN_TARGET: &str = "TCK180";
    pub const LENDS_RETURN_REQUIRES_VIEW: &str = "TCK181";
    pub const LENDS_TARGET_NOT_BORROWED: &str = "TCK182";
    pub const LENDS_TARGET_NOT_VIEW: &str = "TCK183";
    pub const PROPERTY_ACCESSOR_CONFLICT: &str = "TCK041";
    pub const OVERLOAD_CONFLICT: &str = "TCK043";
    pub const REGISTRY_CONFLICT: &str = "TCK400";
    pub const OPERATOR_SIGNATURE_INVALID: &str = "TCK050";
    pub const CONST_FN_SIGNATURE: &str = "TCK160";
    pub const CONST_FN_BODY: &str = "TCK161";
    pub const UNKNOWN_INTERFACE: &str = "TCK060";
    pub const ERROR_INHERITANCE: &str = "TCK061";
    pub const MISSING_INTERFACE_METHOD: &str = "TCK062";
    pub const MISSING_INTERFACE_PROPERTY: &str = "TCK063";
    pub const PROPERTY_STATIC_MISMATCH: &str = "TCK064";
    pub const PROPERTY_TYPE_MISMATCH: &str = "TCK065";
    pub const INTERFACE_METHOD_SIGNATURE_MISMATCH: &str = "TCK066";
    pub const OVERRIDE_TARGET_NOT_FOUND: &str = "TCK200";
    pub const OVERRIDE_SEALED_MEMBER: &str = "TCK201";
    pub const OVERRIDE_VISIBILITY_REDUCTION: &str = "TCK202";
    pub const ABSTRACT_NOT_IMPLEMENTED: &str = "TCK203";
    pub const OVERRIDE_MISSING: &str = "TCK204";
    pub const ABSTRACT_BODY_FORBIDDEN: &str = "TCK205";
    pub const VIRTUAL_BODY_REQUIRED: &str = "TCK206";
    pub const SEALED_REQUIRES_OVERRIDE: &str = "TCK207";
    pub const OVERRIDE_STATIC_CONFLICT: &str = "TCK208";
    pub const OVERRIDE_TYPE_MISMATCH: &str = "TCK209";
    pub const OVERRIDE_GENERIC_MISMATCH: &str = "TCK210";
    pub const DEFAULT_TARGET_INVALID: &str = "DIM0001";
    pub const DEFAULT_CONDITION_INVALID: &str = "DIM0002";
    pub const DEFAULT_AMBIGUITY: &str = "DIM0003";
    pub const DI_THREADLOCAL_UNSUPPORTED: &str = "TCK070";
    pub const DI_MISSING_REGISTRATION: &str = "TCK071";
    pub const DI_SINGLETON_LIFETIME: &str = "TCK072";
    pub const ASYNC_RETURN_TYPE_INVALID: &str = "TCK080";
    pub const EFFECT_NOT_DECLARED: &str = "TCK100";
    pub const RANDOM_EFFECT_MISSING: &str = "RND100";
    pub const RANDOM_DUPLICATED: &str = "RND101";
    pub const NETWORK_EFFECT_MISSING: &str = "NET100";

    #[allow(dead_code)]
    pub const TRAIT_CYCLE_DETECTED: &str = "TCK090";
    #[allow(dead_code)]
    pub const TRAIT_IMPL_OVERLAP: &str = "TCK091";
    #[allow(dead_code)]
    pub const TRAIT_NOT_IMPLEMENTED: &str = "TCK092";
    #[allow(dead_code)]
    pub const TRAIT_IMPL_AMBIGUOUS: &str = "TCK093";
    #[allow(dead_code)]
    pub const TRAIT_ORPHAN_RULE: &str = "TCK094";
    #[allow(dead_code)]
    pub const TRAIT_IMPL_SPECIALIZATION_FORBIDDEN: &str = "TCK095";
    #[allow(dead_code)]
    pub const TRAIT_ASSOC_CYCLE: &str = "TCK096";
    #[allow(dead_code)]
    pub const TRAIT_OBJECT_UNSAFE: &str = "TCK097";
    #[allow(dead_code)]
    pub const TRAIT_MEMBER_MISMATCH: &str = "TCK098";
    pub const TRAIT_FEATURE_UNAVAILABLE: &str = "TCK099";
    pub const IMPL_TRAIT_BOUND_UNSATISFIED: &str = "TCK310";

    pub const BORROW_ESCAPE: &str = "CL0031";
    pub const LEGACY_BORROW_LINT: &str = "CLL0001";

    pub const PATTERN_GUARD_ORDER: &str = "PAT0001";
    pub const PATTERN_FIELD_DUPLICATE: &str = "PAT0002";
    pub const PATTERN_BINDING_CONFLICT: &str = "PAT0003";
    pub const PATTERN_NON_EXHAUSTIVE_SWITCH: &str = "PAT0004";
}

#[must_use]
pub fn error(code: &'static str, message: impl Into<String>, span: Option<Span>) -> Diagnostic {
    let mut message = message.into();
    if !message.starts_with('[') {
        message = format!("[{code}] {message}");
    }
    Diagnostic::error(message, span)
        .with_code(DiagnosticCode::new(code.to_string(), Some("typeck".into())))
}

#[must_use]
pub fn warning(code: &'static str, message: impl Into<String>, span: Option<Span>) -> Diagnostic {
    let mut message = message.into();
    if !message.starts_with('[') {
        message = format!("[{code}] {message}");
    }
    Diagnostic::warning(message, span)
        .with_code(DiagnosticCode::new(code.to_string(), Some("typeck".into())))
}

#[must_use]
pub fn note(message: impl Into<String>, span: Option<Span>) -> Diagnostic {
    Diagnostic::note(message.into(), span)
}

const SPEC_LINK_TABLE: &[(&str, &[&str])] = include!("spec_link_table.in");

#[must_use]
pub fn spec_links(code: &str) -> Option<&'static [&'static str]> {
    SPEC_LINK_TABLE
        .iter()
        .find(|(entry, _)| *entry == code)
        .map(|(_, docs)| *docs)
}

#[must_use]
pub fn simple_name(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}
