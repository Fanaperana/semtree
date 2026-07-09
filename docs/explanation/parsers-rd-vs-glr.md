# Recursive descent vs GLR

SemTree ships two grammar-driven backends.

## Recursive descent (`--backend rd`)

Default. Interprets Grammar IR with backtracking, checkpoints, and recovery tokens.

**Strengths:** simple mental model, fast on unambiguous grammars, good error recovery for tooling.

**Limits:** left recursion needs care; true ambiguity isn’t represented as a forest.

## GLR (`--backend glr`)

Builds LR-style tables, explores conflicts with a Graph-Structured Stack (GSS), and packs alternatives in an SPPF.

**Strengths:** handles ambiguous grammars; closer to tree-sitter’s algorithmic family.

**Limits:** heavier; table generation and multi-stack exploration cost more; still maturing for complex real-world grammars.

## When to switch

| Situation | Backend |
|-----------|---------|
| Most application languages / DSLs | `rd` |
| Known shift/reduce ambiguity you want to explore | `glr` |
| Benchmarking / research | try both |

```bash
semtree run --backend glr -f tree file.py
```
