# How to write a SemTree grammar

## Minimal template

```
language mylang

keyword if
keyword else

SourceFile :=
    Statement*

Statement :=
    IfStmt | ExprStmt

IfStmt :=
    "if" Expression ":" Body

ExprStmt :=
    Expression

Expression :=
    Identifier | Integer | String

Body :=
    Statement+
```

Rules:

1. First rule is the **entry rule** (root of the tree).
2. `keyword X` makes `X` a reserved word (not an identifier).
3. `"lit"` matches a literal token.
4. `A | B` is ordered choice.
5. `A*`, `A+`, `A?` are quantifiers.
6. `name: Rule` binds a named field.

## Recommended layout

```
language NAME

# keywords (grouped)
keyword ...

# ── Top-Level ──
EntryRule :=
    ...

# ── Statements ──
...

# ── Expressions ──
...

# format hints (optional)
indent Body
space around "+"
```

## Built-in leaf types

Use these instead of writing lexer rules for common tokens:

| Name | Matches |
|------|---------|
| `Identifier` | identifiers |
| `Integer` | integer literals |
| `Float` | float literals |
| `String` | string literals |

See [Built-ins](../reference/builtins.md).

## Tips that avoid parse pain

- Prefer left-factored alternatives (`"if" ... | "for" ...`) over overlapping prefixes.
- Put more specific alternatives before general ones in a choice.
- Keep the entry rule first.
- Use `semtree check grammar.semtree` after each change.
- Use `semtree format grammar.semtree` to keep style consistent.
- Test with a small corpus: `semtree run -g grammar.semtree -f tree samples/*`.

## Full syntax

→ [DSL syntax reference](../reference/dsl-syntax.md)
