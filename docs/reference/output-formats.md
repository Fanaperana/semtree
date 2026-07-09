# Output formats (`semtree run -f`)

## `tree`

Indented tree with byte ranges. Human-readable for debugging.

```
source_file@0..42
  Module@0..42
    Statement@0..42
      ...
```

## `sexp`

Compact single-line S-expression.

```
(source_file (Module (Statement ...)))
```

## `sexp-pretty`

Indented S-expression; skips pure whitespace/newline leaves; includes `[start..end]` ranges.

```
(ClassDef [0..100]
  (identifier "class") [0..5]
  (identifier "Foo") [6..9]
  ...
)
```

Default for Neovim `:SemTreeParse`.

## `inspect`

Machine-readable lines for editor integrations:

```
DEPTH|START|END|KIND|TEXT
```

Example:

```
0|0|42|source_file|
1|0|42|Module|
2|0|5|identifier|class
```

Used by `:SemTreeInspect` and the `todo-lsp` example.

## `json`

Pretty JSON tree with `kind`, `range`, and `children`.

```json
{
  "kind": "Module",
  "range": [0, 42],
  "children": [ ... ]
}
```

Best for scripts and other languages consuming SemTree output.
