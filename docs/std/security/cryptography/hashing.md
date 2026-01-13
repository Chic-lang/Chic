# Hashing & HMAC

`Std.Security.Cryptography` provides incremental, span-first hashing plus HMAC and PBKDF2 support.

## Hash algorithms
- `SHA256`, `SHA384`, and `SHA512` implement `HashAlgorithm`.
- APIs:
  - `void Append(ReadOnlySpan<byte> data)` to stream input.
  - `int FinalizeHash(Span<byte> destination)` writes the digest and returns bytes written.
  - `byte[] ComputeHash(ReadOnlySpan<byte> data)` convenience wrapper (allocating).
  - `void Reset()` re-initialises the state for reuse.
- Factories: `HashAlgorithmFactory.CreateSha256()` / `CreateSha384()` / `CreateSha512()`.

Example:
```chic
var sha = HashAlgorithmFactory.CreateSha256();
sha.Append("header".AsUtf8Span());
sha.Append(body);
var digest = new byte[32];
sha.FinalizeHash(Span<byte>.FromArray(ref digest));
```

## HMAC
- `HmacSha256`, `HmacSha384`, `HmacSha512` implement keyed hashing.
- Call `SetKey(ReadOnlySpan<byte> key)` once, then stream data via `Append`, and finish with `FinalizeHash`.
- `Reset` re-primes the inner hash with the pads.

Example:
```chic
var hmac = new HmacSha256();
hmac.SetKey(secretKey);
hmac.Append(message);
var tag = new byte[32];
hmac.FinalizeHash(Span<byte>.FromArray(ref tag));
```

## PBKDF2 (Rfc2898)
`Rfc2898DeriveBytes.Pbkdf2(password, salt, iterations, keyBytes, hashAlg)` produces a derived key using HMAC-SHA256/SHA384/SHA512.

```chic
let salt = RandomNumberGenerator.GetBytes(16);
var key = Rfc2898DeriveBytes.Pbkdf2(
    "password".AsUtf8Span(),
    ReadOnlySpan<byte>.FromArray(ref salt),
    120_000,
    32,
    HashAlgorithmName.Sha256()
);
```

## HKDF
`HKDF.Extract`/`Expand`/`DeriveKey` implement RFC 5869 for SHA-256/384/512. `Extract` accepts an input keying material and optional salt; `Expand` produces key material bound to optional context `info`.

```chic
var okm = HKDF.DeriveKey(
    HashAlgorithmName.Sha256(),
    secretIkm,
    salt,
    "tls handshake".AsUtf8Span(),
    32
);
```

## Notes
- Hashing/HMAC are allocation-free except when using `ComputeHash`.
- All operations use `ReadOnlySpan<byte>`/`Span<byte>` to avoid copying.
