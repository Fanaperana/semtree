# Built-in token types

These names are recognized by the runtime lexer / parser without defining them as rules.

| Name | Aliases (where accepted) | Matches |
|------|--------------------------|---------|
| `Identifier` | `identifier`, `_identifier` | Identifier tokens |
| `Integer` | `integer`, `number` | Integer literals |
| `Float` | `float` | Floating-point literals |
| `String` | `string` | String literals |

## Usage

```
Expression :=
    Identifier | Integer | Float | String
```

## Notes

- Keywords declared with `keyword X` are **not** matched as `Identifier`.
- For language-specific number/string rules, wrap built-ins:

```
Number :=
    Integer | Float

StringLit :=
    String
```
