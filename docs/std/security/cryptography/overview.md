# Std.Security.Cryptography Overview

Chic ships a span-first cryptography stack under `Std.Security.Cryptography`. The APIs mirror the C# ecosystem while keeping deterministic behaviour across LLVM/WASM and avoiding hidden allocations. All algorithms are implemented in Chic, with native hooks only for entropy (`RandomNumberGenerator.Fill`).

## Building blocks
- **Hashing:** `SHA256`/`SHA384`/`SHA512` plus HMAC wrappers (`HmacSha256`, `HmacSha384`, `HmacSha512`). `HashAlgorithmFactory` creates ready-to-use instances.
- **Key derivation:** `Rfc2898DeriveBytes.Pbkdf2` (PBKDF2) with SHA-256/384/512 and `HKDF` (RFC 5869) for extract/expand with span-first APIs.
- **Symmetric encryption:** `AesAlgorithm` via `Aes.Create()`, supporting CBC with PKCS7 or no padding. `Aes.GenerateKey/GenerateIV` produce validated material using the OS CSPRNG. `AesGcm` adds authenticated encryption with 96-bit nonces and 128-bit tags.
- **Transforms:** `ICryptoTransform` drives block encryption/decryption and plugs into streams.
- **Streaming:** `CryptoStream` wraps any `Std.IO.Stream` to encrypt/decrypt on the fly in read or write mode.
- **Random:** `RandomNumberGenerator.Fill`/`GetBytes` expose a cryptographically strong RNG; unsupported platforms surface a deterministic exception.

## Safety defaults
- AES defaults to CBC + PKCS7; callers must set a 16/24/32-byte key and a 16-byte IV explicitly (helpers are provided).
- Padding is strict: `PaddingMode.None` requires input length to align with the block size, otherwise an exception is thrown.
- `CryptoStream` finalises padding on dispose/`FlushFinalBlock` so ciphertext is always well-formed.

## Quick start
```chic
// Hash a payload
var sha = HashAlgorithmFactory.CreateSha256();
var digest = sha.ComputeHash("hello".AsUtf8Span());

// Derive a key
var salt = RandomNumberGenerator.GetBytes(16);
var key = Rfc2898DeriveBytes.Pbkdf2("password".AsUtf8Span(), ReadOnlySpan<byte>.FromArray(ref salt), 100_000, 32, HashAlgorithmName.Sha256());

// Encrypt into a stream
var aes = Aes.Create();
aes.Key = ReadOnlySpan<byte>.FromArray(ref key);
var iv = Aes.GenerateIV();
aes.IV = ReadOnlySpan<byte>.FromArray(ref iv);
using var output = new Std.IO.MemoryStream();
using var crypto = new CryptoStream(output, aes.CreateEncryptor(), CryptoStreamMode.Write);
crypto.Write("secret".AsUtf8Span());
crypto.Dispose(); // flush final block
let ciphertext = output.ToArray();
```
