# XML Security

Std.Text.Xml defaults to safe, deterministic parsing:

- DTDs/external entities are disabled. Encountering `<!DOCTYPE ...>` with `DtdProcessing=false` raises `XmlException`.
- Only UTF-8 is supported in v1. The writer emits UTF-8 by default.
- `MaxDepth` guards runaway nesting (default 64).
- `IgnoreWhitespace`/`IgnoreComments` let you trim non-semantic nodes for configuration parsing.

Guidelines:

- Leave `DtdProcessing` disabled unless you control the input and need DTD support.
- Use `MaxDepth` appropriate for your scenario.
- Treat `WriteRaw` as unsafe: it bypasses escaping and can introduce malformed XML if misused.
