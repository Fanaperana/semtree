# Tutorial: Write your first grammar

**Goal:** create a tiny language, parse a sample file, and iterate on the grammar.

**Time:** ~15 minutes

---

## 1. Scaffold a project

```bash
semtree init --name todo --output .
cd todo
```

You get:

```
todo/
├── grammar.semtree
└── semtree.json
```

## 2. Replace the grammar

Open `grammar.semtree` and replace it with:

```
language todo

keyword todo
keyword done
keyword priority

# First rule = entry rule (root of the tree)
# Items end with ';' so the parser can separate them cleanly.
Document :=
    Item*

Item :=
    TodoItem | DoneItem

TodoItem :=
    "todo" Priority? Word+ ";"

DoneItem :=
    "done" Word+ ";"

Priority :=
    "priority" ":" Identifier

Word :=
    Identifier
```

Save the file.

## 3. Create a sample source file

```bash
cat > sample.todo << 'EOF'
todo priority:high buy milk;
todo write docs;
done clean desk;
EOF
```

## 4. Parse it

```bash
semtree run -g grammar.semtree -f sexp-pretty sample.todo
```

You should see a tree with `Document`, `TodoItem`, `DoneItem`, etc.

## 5. Check the grammar

```bash
semtree check grammar.semtree
```

Fix any reported issues (undefined rules, cycles, etc.).

## 6. Format the grammar file

```bash
semtree format grammar.semtree
```

This pretty-prints `.semtree` files (keywords, `:=`, `|` alternatives).

## 7. Iterate

Change the grammar — for example add a `due` field — then re-run:

```bash
semtree run -g grammar.semtree -f tree sample.todo
```

## Done

You authored a language and parsed real input. Next:

- [Use SemTree in Neovim](04-neovim.md)
- Or jump to [Apply SemTree to any project](../how-to/apply-to-any-project.md)
