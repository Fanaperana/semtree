# SemTree DSL syntax reference

File extension: `.semtree`  
Encoding: UTF-8  
Comments: `#` to end of line

---

## File structure

A grammar file is an ordered list of:

1. Optional comments / blank lines
2. One `language` directive
3. Zero or more `keyword` / `extra` directives
4. One or more **rules** (`Name := …`)
5. Optional **format hints** (`indent`, `linebreak`, `space …`)

```
language NAME

keyword ...
extra ...

RuleName :=
    expression

indent SomeRule
linebreak SomeRule
space around "+"
```

**Entry rule:** the **first** rule defined in the file is the parse root.

---

## Directives

### `language <name>`

```
language python
```

Sets the grammar name. Required for clarity; used in tooling metadata.

### `keyword <word>`

```
keyword def
keyword class
```

Registers a reserved word. The lexer emits it as a keyword token instead of an identifier.

### `extra <name>`

```
extra comment
```

Marks a rule/token as “extra” (trivia-like). Used by importers / advanced tooling.

---

## Rules

### Definition

```
RuleName :=
    body
```

- `RuleName` is typically PascalCase.
- Body is indented (spaces or tabs).
- Blank line ends the body.

Same-line body is allowed:

```
Null := "null"
```

### Sequence

Whitespace-separated expressions must all match in order:

```
LetStmt :=
    "let" Identifier "=" Expression ";"
```

### Choice `|`

Ordered choice — first successful alternative wins:

```
Statement :=
    LetStmt | ReturnStmt | ExprStmt
```

Multi-line style (recommended):

```
Statement :=
    LetStmt
    | ReturnStmt
    | ExprStmt
```

`|` inside string literals is **not** a choice separator.

### Quantifiers

| Suffix | Meaning |
|--------|---------|
| `A?` | optional (0 or 1) |
| `A*` | zero or more |
| `A+` | one or more |

```
Params :=
    "(" ParamList? ")"

ParamList :=
    Param ParamTail*

Statements :=
    Statement+
```

Quantifiers attach to the preceding **token** (rule name or literal form as tokenized). Prefer naming helper rules for complex quantified groups.

### Literals

Double-quoted strings:

```
IfStmt :=
    "if" Expression ":" Body
```

### Rule references

Bare identifiers refer to other rules or built-ins:

```
Expression :=
    Identifier | Integer | String
```

### Fields

```
name: RuleRef
```

Binds a named field for AST / tooling:

```
FunctionDef :=
    "fn" name: Identifier Parameters Block
```

---

## Format hints

These do not change parsing; they guide formatters / codegen.

```
indent Block
linebreak Function
space around "+"
space before ","
space after ":"
```

| Hint | Meaning |
|------|---------|
| `indent <Rule>` | Indent contents of rule |
| `linebreak <Rule>` | Prefer line break around rule |
| `space around <lit>` | Spaces on both sides of literal |
| `space before <lit>` | Space before literal |
| `space after <lit>` | Space after literal |

---

## Built-in leaves

See [Built-ins](builtins.md). Common ones:

- `Identifier`
- `Integer`
- `Float`
- `String`
- `Number` (when defined as a rule aliasing Integer/Float)

---

## Comments

```
# Full-line comment
```

Section banners are conventional:

```
# ── Expressions ──────────────────────────────────────────────
```

---

## Grammar checklist

| Rule | Detail |
|------|--------|
| First rule | Becomes entry / root |
| Keywords | Declared before use as literals if reserved |
| Choices | More specific alternatives first |
| Names | Unique rule names |
| Validation | `semtree check file.semtree` |

---

## Formal expression summary

```
expr      := seq | choice
choice    := expr ("|" expr)+
seq       := piece+
piece     := atom quant?
quant     := "?" | "*" | "+"
atom      := literal | field | rule_ref
literal   := '"' chars '"'
field     := name ":" rule_ref
rule_ref  := Identifier
```

(As implemented by the current DSL parser — see `crates/semtree_grammar/src/dsl.rs`.)

---

## Complete example

```
language calc

keyword let

Program :=
    Stmt*

Stmt :=
    LetStmt | ExprStmt

LetStmt :=
    "let" name: Identifier "=" Expr ";"

ExprStmt :=
    Expr ";"

Expr :=
    Term ExprTail*

ExprTail :=
    AddOp Term

AddOp :=
    "+" | "-"

Term :=
    Identifier | Integer

space around "+"
space around "-"
```
