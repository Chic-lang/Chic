As a matter fact she didn't want to said she had a dream that# Std.Net.Dns

`Std.Net.Dns` resolves host names into IPv4 and IPv6 addresses with a span-first mindset. Literal addresses are returned directly; other hostnames go through the platform resolver (`getaddrinfo` on native). Async APIs honor `CancellationToken` and surface `TaskCanceledException` when canceled. Unsupported targets (e.g., WASM without a resolver hook) throw `NotSupportedException`.

Examples:

```chic
let addresses = Std.Net.Dns.GetHostAddresses("127.0.0.1");
if (addresses.Length > 0) {
    Std.Console.WriteLine(addresses[0].ToString());
}
```

Behavior notes:
- `GetHostAddressesAsync(string, CancellationToken)` is the primary entry point; sync wraps the async path.
- Cancellation is checked before the lookup.
- IPv4 and IPv6 results are returned in order provided by the resolver.
- Unsupported targets (including WASM without host hooks) throw deterministic `NotSupportedException`.
