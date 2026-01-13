# Documentation comments and `chic doc`

Chic uses XML documentation comments (`///`) to describe public APIs. The `chic doc` command turns those comments into Markdown.

## Write XML doc comments

Place `///` comments immediately above a declaration:

```chic
/// <summary>Add two integers.</summary>
public static int Add(int a, int b) { return a + b; }
```

Use `summary`, `remarks`, `param`, `returns`, and `example` to document an API.

## Generate Markdown

From a package directory:

```sh
chic doc --output docs/api
```

## Configuration

Doc generation is configured in `manifest.yaml` under the `docs` section. The exact XML-to-Markdown mapping and supported tags are defined in `docs/tooling/xml_to_markdown.md`.

## Next steps

- Document the manifest fields that control docs: `docs/manifest_manifest.md`
- Learn the XML tag mapping: `docs/tooling/xml_to_markdown.md`

