# AES Symmetric Encryption

`AesAlgorithm` implements AES-CBC with PKCS7 or no padding. Create instances via `Aes.Create()`.

## Configuration
- Key sizes: 16, 24, or 32 bytes (AES-128/192/256).
- IV: 16 bytes (block size). Required for CBC.
- Mode: `CipherMode.CBC` (only supported mode today).
- Padding: `PaddingMode.PKCS7` (default) or `PaddingMode.None` (input must be block aligned).
- Helpers: `Aes.GenerateKey(int sizeBytes = 32)` and `Aes.GenerateIV()` use the OS CSPRNG.

## Usage
```chic
var aes = Aes.Create();
aes.Key = ReadOnlySpan<byte>.FromArray(ref key);
aes.IV = ReadOnlySpan<byte>.FromArray(ref iv);
aes.Padding = PaddingMode.PKCS7;

var encryptor = aes.CreateEncryptor();
var ciphertext = new byte[64];
let written = encryptor.TransformFinalBlock(plain, Span<byte>.FromArray(ref ciphertext));

var decryptor = aes.CreateDecryptor();
var recovered = new byte[64];
let recoveredLen = decryptor.TransformFinalBlock(
    ReadOnlySpan<byte>.FromArray(ref ciphertext).Slice(0usize, NumericUnchecked.ToUSize(written)),
    Span<byte>.FromArray(ref recovered)
);
```

## ICryptoTransform
Transforms expose:
- `TransformBlock(ReadOnlySpan<byte> input, Span<byte> output)` for aligned chunks.
- `TransformFinalBlock(...)` handles padding/finalisation.
- `Reset()` reuses the transform with the original key/IV.

## Safety notes
- The API never auto-generates IV/key unless you call the helper. Always store/transmit the IV alongside ciphertext.
- PKCS7 padding is strict; `PaddingMode.None` will throw if the final block is misaligned.
- The AES implementation is pure Chic and deterministic across backends.

## AES-GCM (AEAD)
`AesGcm` provides authenticated encryption with associated data. It accepts 16/24/32-byte keys, 12-byte nonces, and emits 16-byte authentication tags.

```chic
let key = Aes.GenerateKey(16);
let nonce = RandomNumberGenerator.GetBytes(12);
let plain = "secret".AsUtf8Span();
let aad = "context".AsUtf8Span();
var aes = new AesGcm(ReadOnlySpan<byte>.FromArray(ref key));

var tag = new byte[16];
var ciphertext = new byte[plain.Length];
aes.Encrypt(ReadOnlySpan<byte>.FromArray(ref nonce), plain, Span<byte>.FromArray(ref ciphertext), Span<byte>.FromArray(ref tag), aad);

var recovered = new byte[plain.Length];
aes.Decrypt(ReadOnlySpan<byte>.FromArray(ref nonce), ReadOnlySpan<byte>.FromArray(ref ciphertext), ReadOnlySpan<byte>.FromArray(ref tag), Span<byte>.FromArray(ref recovered), aad);
```

Only modern, nonce-based AEAD is supported here; CBC/PKCS7 remains available via `AesAlgorithm` for interoperability with existing systems.
