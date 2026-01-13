use crate::defines::DefineFlag;
use crate::frontend::conditional::{ConditionalDefines, DefineValue};
use crate::target::Target;

pub(crate) fn resolve_conditional_defines(
    target: &Target,
    overrides: &[DefineFlag],
) -> ConditionalDefines {
    let mut defines = ConditionalDefines::new();

    let debug_profile = cfg!(debug_assertions);
    insert_define(&mut defines, "DEBUG", DefineValue::Bool(debug_profile));
    insert_define(&mut defines, "RELEASE", DefineValue::Bool(!debug_profile));
    insert_define(
        &mut defines,
        "PROFILE",
        DefineValue::String(if debug_profile {
            "debug".into()
        } else {
            "release".into()
        }),
    );
    insert_define(
        &mut defines,
        "TARGET_ARCH",
        DefineValue::String(target.arch().as_str().to_string()),
    );

    let triple = target.triple().to_string();
    let (target_os, target_env) = infer_platform(target.triple());
    insert_define(&mut defines, "TARGET", DefineValue::String(triple.clone()));
    insert_define(
        &mut defines,
        "TARGET_TRIPLE",
        DefineValue::String(triple.clone()),
    );
    insert_define(
        &mut defines,
        "TARGET_OS",
        DefineValue::String(target_os.clone()),
    );
    insert_define(
        &mut defines,
        "TARGET_ENV",
        DefineValue::String(target_env.clone()),
    );
    insert_define(&mut defines, "target", DefineValue::String(triple));
    insert_define(&mut defines, "TRACE", DefineValue::Bool(true));

    let mut debug_overridden = false;
    let mut release_overridden = false;

    for flag in overrides {
        let key = flag.name.trim();
        if key.is_empty() {
            continue;
        }
        let lowered = key.to_ascii_uppercase();
        let store_key = if lowered == "DEBUG" || lowered == "RELEASE" {
            lowered.clone()
        } else {
            key.to_string()
        };
        if lowered == "DEBUG" {
            debug_overridden = true;
        }
        if lowered == "RELEASE" {
            release_overridden = true;
        }
        if let Some(value) = flag.value.as_deref() {
            if let Some(boolean) = parse_bool_literal(value) {
                insert_define(&mut defines, &store_key, DefineValue::Bool(boolean));
            } else {
                let value = value.to_string();
                if store_key.eq_ignore_ascii_case("feature") {
                    for feature in value.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                        let normalized = feature
                            .chars()
                            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                            .collect::<String>();
                        insert_define(
                            &mut defines,
                            &format!("feature_{normalized}"),
                            DefineValue::Bool(true),
                        );
                    }
                }
                insert_define(&mut defines, &store_key, DefineValue::String(value));
            }
        } else {
            insert_define(&mut defines, &store_key, DefineValue::Bool(true));
        }
    }

    if !release_overridden {
        let debug_state = defines.is_true("DEBUG");
        insert_define(&mut defines, "RELEASE", DefineValue::Bool(!debug_state));
    } else if !debug_overridden {
        let release_state = defines.is_true("RELEASE");
        insert_define(&mut defines, "DEBUG", DefineValue::Bool(!release_state));
    }

    defines
}

fn infer_platform(triple: &str) -> (String, String) {
    let mut parts = triple.split('-');
    let _arch = parts.next();
    let vendor = parts.next();
    let os = parts.next();
    let env = parts.next();

    let os_name = canonical_os(os.or(vendor).unwrap_or("unknown"));
    let env_name = canonical_env(env.unwrap_or("none"), &os_name);

    (os_name, env_name)
}

fn canonical_os(raw: &str) -> String {
    match raw {
        "darwin" | "macos" | "ios" => "macos".into(),
        "windows" | "pc" => "windows".into(),
        "linux" => "linux".into(),
        "none" | "unknown" => "unknown".into(),
        other => other.to_string(),
    }
}

fn canonical_env(raw: &str, os: &str) -> String {
    match raw {
        "gnu" if os == "linux" => "glibc".into(),
        "musl" => "musl".into(),
        "none" | "unknown" => "none".into(),
        other => other.to_string(),
    }
}

fn parse_bool_literal(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn insert_define(defines: &mut ConditionalDefines, key: &str, value: DefineValue) {
    let aliases = [
        key.to_string(),
        key.to_ascii_lowercase(),
        key.to_ascii_uppercase(),
    ];
    for alias in aliases {
        defines.insert(alias, value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::Target;

    #[test]
    fn defaults_include_target_os_and_env() {
        let target = Target::parse("x86_64-apple-darwin").expect("parse target");
        let defines = resolve_conditional_defines(&target, &[]);
        assert_eq!(
            defines
                .iter()
                .find(|(k, _)| k.as_str() == "TARGET_OS")
                .map(|(_, v)| v),
            Some(&crate::frontend::conditional::DefineValue::String(
                "macos".into()
            ))
        );
    }

    #[test]
    fn overrides_replace_default_values() {
        let target = Target::parse("x86_64-unknown-linux-gnu").expect("parse target");
        let override_flag = DefineFlag::new("DEBUG", Some("false".into()));
        let defines = resolve_conditional_defines(&target, &[override_flag]);
        assert!(defines.is_true("RELEASE"));
        assert!(
            !defines.is_true("DEBUG"),
            "debug should flip when DEBUG=false override applied"
        );
    }
}
