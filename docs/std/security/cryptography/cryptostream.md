# CryptoStream

`Std.Security.Cryptography.CryptoStream` wraps any `Std.IO.Stream` to encrypt or decrypt data on the fly using an `ICryptoTransform`. It is span-first and buffers only as needed to respect block boundaries.

## Modes
- `CryptoStreamMode.Write`: plaintext is written, ciphertext is emitted to the inner stream.
- `CryptoStreamMode.Read`: ciphertext is read from the inner stream, plaintext is returned.

## Key behaviours
- Uses `Read(Span<byte>)`/`Write(ReadOnlySpan<byte>)` plus async counterparts.
- Buffers partial blocks and calls `TransformFinalBlock` automatically on `Dispose()` or when `FlushFinalBlock()` is invoked in write mode.
- `Flush()` delegates to the inner stream without finalising padding.
- Honors `leaveOpen` to keep the underlying stream alive after disposal.

## Usage examples
### Encrypting to a file
```chic
var aes = Aes.Create();
aes.Key = keySpan;
aes.IV = ivSpan;

using var file = Std.IO.File.OpenWrite("secret.bin");
using var crypto = new CryptoStream(file, aes.CreateEncryptor(), CryptoStreamMode.Write);
crypto.Write(plainData);
crypto.Dispose(); // finalizes padding and flushes
```

### Decrypting from a network stream
```chic
using var network = await client.GetStreamAsync();
var aes = Aes.Create();
aes.Key = keySpan;
aes.IV = ivSpan;

using var crypto = new CryptoStream(network, aes.CreateDecryptor(), CryptoStreamMode.Read);
var buffer = new byte[4096];
let read = crypto.Read(Span<byte>.FromArray(ref buffer));
// buffer[..read] now holds decrypted bytes
```

### Async with cancellation
```chic
let token = cancellationSource.Token();
await crypto.WriteAsync(ReadOnlyMemory<byte>.FromArray(ref payload), token);
crypto.Dispose(); // finalizes and flushes
```

## Gotchas
- Writing after `FlushFinalBlock` or disposal throws.
- Read mode finalizes once the underlying stream returns 0 bytes; ensure the ciphertext is complete before closing the writer.
- Padding is handled by the transform; supply a transform configured with PKCS7 when encrypting data that is not block aligned.
