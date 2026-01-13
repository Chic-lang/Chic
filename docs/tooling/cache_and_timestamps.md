# Cache and Timestamps

Chic uses incremental caches to speed up builds and codegen. **Cache correctness must never depend on filesystem mtimes or wall-clock time.** Reuse/invalidation decisions must be based on deterministic fingerprints (content hashes + relevant semantic metadata).

## Reproducible build metadata timestamps

The Rust bootstrap binary embeds a build timestamp for display/telemetry. For reproducible builds, it follows this precedence:

1. `CHIC_BUILD_UNIX_OVERRIDE` (seconds since UNIX epoch)
2. `SOURCE_DATE_EPOCH` (standard reproducible-builds env var, seconds since UNIX epoch)
3. Git `HEAD` commit timestamp (when available)
4. Local wall-clock time (last resort)

This timestamp is informational and must not affect cache keys.

## Practical guidance

- If you see “stale” builds being reused incorrectly, treat it as a correctness bug and fix the fingerprinting logic (not the timestamp).
- Prefer explicit output directories (`--artifacts-path`) to keep cache/intermediate files out of source trees.

