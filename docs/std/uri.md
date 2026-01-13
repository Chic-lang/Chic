# Std.Uri

## Overview
`Std.Uri` and `Std.UriBuilder` provide Chic-native URI parsing, escaping, comparison, and resolution aligned with RFC 3986.

This implementation is deterministic across LLVM and WASM and avoids host-dependent parsing behavior.

## Parsing and Normalization
- The parser splits URIs into scheme, authority (userinfo, host, port), path, query, and fragment.
- Relative references are resolved against a base URI using RFC 3986 section 5.2 rules (merge + dot-segment removal).
- Schemes and hosts are canonicalized to lowercase ASCII.
- Default ports are suppressed in `AbsoluteUri`/`ToString` (e.g. `http://example.com:80` -> `http://example.com/`).
- Dot-segments are removed for absolute URIs (and for resolved results).

### Absolute vs Relative
- `IsAbsoluteUri` is `true` only when a valid scheme is present.
- `AbsoluteUri` throws `InvalidOperationException` for relative URIs.
- `UriKind.Absolute` rejects relative inputs; `UriKind.Relative` rejects absolute inputs.

## Escaping Rules
- `EscapeDataString` escapes everything except RFC 3986 unreserved characters.
- `EscapeUriString` leaves reserved characters intact and normalizes existing percent-escapes to uppercase.
- `UnescapeDataString` decodes percent-escapes; malformed sequences throw `UriFormatException`.
- `UriFormat.SafeUnescaped` only decodes escapes that map to unreserved bytes.

## Component Extraction
- `GetComponents` supports `UriComponents.PathAndQuery`, `UriComponents.AbsoluteUri`, and individual components.
- `Query` and `Fragment` include leading `?` / `#` when non-empty.
- `Segments` is computed from the escaped `AbsolutePath`.

## File and UNC
- `file:///path` yields `IsFile = true`, `IsUnc = false`, `LocalPath = /path`.
- `file://server/share/path` yields `IsUnc = true`, `LocalPath = //server/share/path`.
- `LocalPath` uses forward slashes on all targets for deterministic behavior.

## IDN and IRI
- Internationalized hostnames (IDN/IRI) are currently not enabled; inputs must use ASCII host forms.

## Backend Parity
- LLVM and WASM share identical parsing, escaping, and resolution logic.
- Behavior is deterministic and tested through the same fixture program on both backends.

## Behavioral notes
- Userinfo is restricted to RFC 3986 unreserved/sub-delims/`:` plus percent-escapes.
- Host validation is strict: IPv4/IPv6 literals, DNS labels, or reg-name (unreserved/sub-delims/percent) only.
- `UriBuilder` always emits hierarchical URIs (`scheme://host/...`).
- `LocalPath` does not apply OS-specific path conversions.
