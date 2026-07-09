# How to import a Tree-sitter grammar

## Import compiled `grammar.json`

```bash
semtree import path/to/src/grammar.json -o mylang.semtree.json
```

## Import + validate

```bash
semtree migrate path/to/src/grammar.json -o mylang.semtree.json
```

## After import

1. Inspect the generated IR / grammar.
2. Prefer rewriting hot paths into idiomatic `.semtree` DSL for readability.
3. Validate:

```bash
semtree check grammars/mylang.semtree
semtree run -g grammars/mylang.semtree -f tree samples/hello.ext
```

## Notes

- Tree-sitter's JS DSL is not executed; SemTree imports the compiled JSON.
- Some tree-sitter features (external scanners, complex precedence) may need hand fixes.
- Keep SemTree's first rule as the entry point after migration.
