# Raw Sockets

`SocketType.Raw` and `ProtocolType.Raw/Icmp/IcmpV6` expose low-level packet access. These operations are **unsafe** and often require elevated host permissions.

Bootstrap behavior:
- Creation routes through libc `socket(AF_INET, SOCK_RAW, protocol)`; failure maps to `SocketError.PermissionDenied`/`SocketError.Unsupported`.
- Span-first `SendTo`/`ReceiveFrom` are the supported data paths; `Connect` is not required for raw sockets.
- IPv4 only; IPv6/raw dual-mode is gated.
- WASM and hosts without raw socket support throw `NotSupportedException` deterministically.

Usage reminder:
- Keep buffers small and fixed; avoid extra allocations in hot paths.
- Validate privileges before attempting to create raw sockets; handle deterministic failures instead of silently downgrading.
