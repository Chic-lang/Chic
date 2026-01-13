//! Global language feature toggles and helpers.

use std::sync::RwLock;

use crate::frontend::conditional::ConditionalDefines;

/// Feature switches that depend on the current language version.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LanguageFeatures {
    pub first_class_spans: bool,
}

static FEATURES: RwLock<LanguageFeatures> = RwLock::new(LanguageFeatures {
    first_class_spans: true,
});

/// Snapshot of the currently active language features.
#[must_use]
pub fn language_features() -> LanguageFeatures {
    FEATURES.read().map(|guard| *guard).unwrap_or_default()
}

/// Record the active language features for the current compilation pipeline.
pub fn set_language_features(features: LanguageFeatures) {
    if let Ok(mut guard) = FEATURES.write() {
        *guard = features;
    }
}

/// Whether first-class span behaviour is enabled for this compilation.
#[must_use]
pub fn first_class_spans_enabled() -> bool {
    language_features().first_class_spans
}

/// Derive language feature switches from the supplied conditional defines.
#[must_use]
pub fn features_from_defines(defines: &ConditionalDefines) -> LanguageFeatures {
    if let Some(override_flag) = defines.get("feature_first_class_spans") {
        if let Some(enabled) = override_flag.as_bool() {
            return LanguageFeatures {
                first_class_spans: enabled,
            };
        }
    }

    let lang_version = defines
        .get("LangVersion")
        .or_else(|| defines.get("LANG_VERSION"))
        .and_then(|value| value.as_str().map(str::to_string));

    let inferred = lang_version
        .as_deref()
        .map(|text| {
            let normalized = text.trim().to_ascii_lowercase();
            if normalized.contains("span") {
                return true;
            }
            normalized
                .split(|ch: char| ch == '-' || ch == '_' || ch == '+')
                .next()
                .and_then(|prefix| prefix.parse::<f32>().ok())
                .map(|parsed| parsed >= 0.10f32)
                .unwrap_or(true)
        })
        .unwrap_or(true);

    LanguageFeatures {
        first_class_spans: inferred,
    }
}
