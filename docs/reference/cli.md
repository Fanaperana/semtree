# CLI reference — `semtree`

```
semtree <COMMAND> [OPTIONS]
```

---

## `semtree run`

Parse a file with a grammar (auto-detected or explicit).

```
semtree run [OPTIONS] <FILE>
```

| Flag | Default | Description |
|------|---------|-------------|
| `-g, --grammar <PATH>` | auto | `.semtree` or tree-sitter `.json` |
| `-f, --format <FMT>` | `tree` | `tree`, `sexp`, `sexp-pretty`, `inspect`, `json` |
| `--backend <NAME>` | `rd` | `rd` or `glr` |

Auto-detect maps extensions → bundled grammars (`py`→python, `js`→javascript, `rs`→rust, `css`, `json`, `toml`).

---

## `semtree parse`

Parse with the built-in event parser (not grammar-driven). Useful for SemTree’s own Rust-like demo syntax.

```
semtree parse [OPTIONS] <FILE>
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --format <FMT>` | `tree` | `tree`, `json`, `sexp` |

Prefer `run` for real languages.

---

## `semtree check`

Validate a `.semtree` grammar (undefined rules, cycles, etc.).

```
semtree check <FILE>
```

---

## `semtree format`

Format a file.

```
semtree format <FILE>
```

- `*.semtree` → dedicated DSL pretty-printer
- other files → tree-based source formatter

---

## `semtree query`

Query a syntax tree.

```
semtree query <FILE> <PATTERN>
```

`PATTERN` may be a kind name or S-expression pattern.

---

## `semtree lint`

Lint a source file.

```
semtree lint <FILE>
```

---

## `semtree symbols`

List symbols (functions, variables, types).

```
semtree symbols <FILE>
```

---

## `semtree init`

Scaffold a language project.

```
semtree init [-n NAME] [-o DIR]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-n, --name` | `my_language` | Project / language name |
| `-o, --output` | `.` | Parent directory |

Creates `<name>/grammar.semtree` and `<name>/semtree.json`.

---

## `semtree import`

Import a Tree-sitter `grammar.json`.

```
semtree import <FILE> [-o OUTPUT]
```

---

## `semtree migrate`

Import + validate a Tree-sitter grammar.

```
semtree migrate <FILE> [-o OUTPUT]
```

---

## `semtree generate`

Generate typed AST Rust code from a grammar.

```
semtree generate <FILE> [-o OUTPUT]
```

---

## `semtree test`

Run grammar tests in a directory (lossless roundtrip checks).

```
semtree test <DIR>
```

---

## `semtree benchmark`

Benchmark parsing.

```
semtree benchmark <FILE> [-i ITERATIONS]
```

| Flag | Default |
|------|---------|
| `-i, --iterations` | `100` |

---

## `semtree doctor`

Print installation / environment diagnostics.

```
semtree doctor
```

---

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Error (message on stderr) |
