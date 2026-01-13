//! Build-time metadata helpers used by the CLI.

/// Short git hash determined at compile time when available.
#[must_use]
pub fn commit_hash() -> &'static str {
    option_env!("CHIC_GIT_HASH").unwrap_or("unknown")
}

/// Full git hash determined at compile time when available.
#[must_use]
pub fn commit_hash_full() -> &'static str {
    option_env!("CHIC_GIT_HASH_FULL").unwrap_or("unknown")
}

/// Whether the repository had uncommitted changes at build time.
#[must_use]
pub fn git_dirty() -> &'static str {
    option_env!("CHIC_GIT_DIRTY").unwrap_or("unknown")
}

/// Unix timestamp (seconds since epoch) for the current commit when available.
#[must_use]
pub fn commit_timestamp() -> &'static str {
    option_env!("CHIC_GIT_COMMIT_UNIX").unwrap_or("unknown")
}

/// Unix timestamp (seconds since epoch) recorded at build time.
#[must_use]
pub fn build_timestamp() -> &'static str {
    option_env!("CHIC_BUILD_UNIX").unwrap_or("unknown")
}

/// Cargo build profile associated with the binary.
#[must_use]
pub fn build_profile() -> &'static str {
    option_env!("CHIC_BUILD_PROFILE").unwrap_or("unknown")
}

/// Render a scripting-friendly version string.
#[must_use]
pub fn formatted() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let commit = commit_hash();
    let commit_full = commit_hash_full();
    let dirty = git_dirty();
    let commit_unix = commit_timestamp();
    let built = build_timestamp();
    let profile = build_profile();
    let target = option_env!("CHIC_BUILD_TARGET").unwrap_or("unknown");
    let commit_line = if commit_full != "unknown" && commit_full != commit {
        format!("{commit} ({commit_full})")
    } else {
        commit.to_string()
    };
    format!(
        "chic {version}\ncommit: {commit_line}\ncommit_unix: {commit_unix}\ndirty: {dirty}\nbuilt: {built}\nprofile: {profile}\ntarget: {target}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatted_includes_required_fields() {
        let version = formatted();
        assert!(version.starts_with("chic ")); // version prefix
        assert!(version.contains("commit:"));
        assert!(version.contains("dirty:"));
        assert!(version.contains("built:"));
        assert!(version.contains("profile:"));
        assert!(version.contains("target:"));
    }
}
