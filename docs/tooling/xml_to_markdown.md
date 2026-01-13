# XML Documentation → Markdown

Chic stores inline documentation as XML doc comments (`///`) and turns them into Markdown during doc generation. This guide defines the canonical mapping, Chic-specific extensions, and how to configure/extend the pipeline.

For how to write doc comments and run the generator, see `docs/tooling/documentation.md`.

## Tag mapping (XML → Markdown)
- `summary` → Leading paragraph(s) at the top of the page.
- `remarks` → “Remarks” section rendered after the summary.
- `param` / `typeparam` → Parameter and type-parameter tables (name + description).
- `returns` → “Returns” section.
- `value` → “Value” section for properties/indexers.
- `example` → Fenced code block with optional caption; defaults to ` ```chic ` fences.
- `see` / `seealso` → Markdown links using the configured link resolver. If `cref` cannot be resolved, the raw text is emitted with a warning.
- `code` → Fenced code block; honours `lang` and `title` attributes when present.
- `c` → Inline code span.
- `para` → Paragraph break.
- `list` → Bullet or numbered list (`type="bullet"`/`number"`). `type="table"` becomes a Markdown table; headers are taken from `<listheader><term>/<description>` when present.
- `item` → Entry inside `list`; `<term>` maps to a table/list term, `<description>` to the body.

## Chic extensions
- `chic:sample` → Code sample with optional `lang`/`title`; defaults to `chic` fences and is grouped under “Examples”.
- `chic:note` → Callout rendered as a blockquote prefixed with **Note:**.
- Unknown/unsupported tags are ignored by default but surfaced as diagnostics; custom handlers can override this behaviour.

## Link resolution
- `<see>`/`<seealso>` with `cref="Namespace.Type.Member"` are resolved via the link resolver hook.
- Default resolver:
  - Internal symbols → `#namespace-type-member` anchors.
  - External/absolute `cref` (e.g., `https://...`) → direct link.
- Custom resolvers can map symbols to portals, API browsers, or repository-relative paths.

## Code fences and formatting
- Block code uses ` ```chic ` fences unless a `lang` attribute overrides it (`lang="csharp"` → ` ```csharp `).
- Inline code (`<c>`) uses backticks.
- Whitespace inside `<code>` is preserved verbatim.

## Sections and ordering
1. Banner (optional, for generated files).
2. Summary.
3. Signature/heading (template-controlled).
4. Parameters / type parameters.
5. Returns / value.
6. Examples.
7. Remarks.
8. See also.
9. Custom sections emitted by tag handlers or templates.

## Templates and front matter
- Templates drive headings, section order, TOC, and surrounding chrome. If no template is provided, the default Chic layout above is used.
- `docs.markdown.front_matter_template` may point to YAML/JSON inserted at the top of every generated Markdown file (useful for SSGs). Template variables can reference symbol metadata (name, namespace, kind, etc.).

## Extensibility hooks
- Tag handlers: register types that consume custom/unknown XML elements and return Markdown blocks.
- Link resolvers: map `cref` values to Markdown targets.
- Front matter: inject structured metadata per file before the rendered body.
- Configuration lives under `docs.markdown` in `manifest.yaml`; see the manifest section of the CLI docs for examples.

## Diagnostics
- Malformed XML, unsupported tags, and unresolved `cref` values produce diagnostics. Severities are configurable (error/warning/ignore) via `docs` settings in `manifest.yaml`.
- For library builds, missing XML documentation for public surfaces is enforced by default; see the documentation enforcement section in the manifest guide.

## Updating goldens
- Golden Markdown fixtures live under `tests/docs/golden/`. When intentional mapping changes occur, re-run the doc generator in test mode and refresh the snapshots to keep CI green.
