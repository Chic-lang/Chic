# Global import directives

This note captures the behaviour and implementation strategy for Chic’s `global import`
directives so the parser, resolver, and backends stay consistent with the language spec.

## Motivation

Large codebases often repeat the same import block across many files just to pull common namespaces,
aliases, or static helpers into scope. `global import` lets authors centralise those imports while
keeping lookup predictable.

## Syntax

* `global import Namespace.Path;`
* `global import Alias = Namespace.Path;`
* `global import static Type.Name;`

Rules:

1. The `global` keyword may only prefix an import directive. Anything else triggers a targeted
   diagnostic.
2. Global directives must appear at the top of the file before any namespace or type declarations
   and cannot be nested inside namespaces or types. Misplaced directives are rejected and ignored
   for resolution.
3. All global directives across a compilation are collected once and applied to every source file,
   making it easy to centralise imports in a single `global_usings.ch`.
4. `@cimport` does not support `global`.

## Resolution Ordering

Name resolution now merges scopes in the following order:

1. Global import directives (namespace imports, aliases, and `import static`).
2. File-scoped directives (root namespace).
3. Each enclosing namespace from outermost to innermost.

Alias bindings follow the same order, which means a local alias overrides a global binding with the
same name. Namespace imports and static imports simply append to the search list; ambiguities are
still reported when multiple candidates match. Conflicting alias targets between global directives
or between a global alias and a local alias are rejected with a diagnostic.

## Diagnostics

* `global` without a following `import` reports “`global` keyword may only prefix an import directive”.
* Global directives must appear before other declarations at file scope and are rejected inside
  namespaces or types.
* Conflicting alias targets note both declarations.
* Ambiguity diagnostics list both local and global owners so authors know where each import came
  from.

## Backend/Metadata Considerations

* `ImportDirective` now carries an `is_global` flag so MIR lowering, type checking, and metadata
  emitters can see the distinction.
* The import resolver stores a dedicated global scope that is merged before namespace-specific
  scopes. Static import collection reuses this data so `global import static` works everywhere.
* Textual emitters (`codegen/text`, metadata dumps, and header generation) include the `global`
  keyword when re-serialising directives so downstream tooling faithfully represents the source.
