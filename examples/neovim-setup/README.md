# SemTree Neovim Integration — Complete Setup Guide

> Parse **any** language in Neovim without installing tree-sitter parsers.
> Includes an interactive tree inspector with real-time source highlighting.

---

## Quick Start (3 steps)

### 1. Install the binary

```bash
cd /path/to/semtree
cargo install --path crates/semtree_cli

# Verify
semtree --help
```

This installs the `semtree` binary to `~/.cargo/bin/semtree`.

### 2. Add the plugin (lazy.nvim)

Add this block to `~/.config/nvim/lua/plugins/init.lua` inside `return { ... }`:

```lua
{
    dir = "/path/to/semtree/editors/neovim",  -- absolute path to the plugin
    name = "semtree",
    lazy = false,
    config = function()
        require("semtree").setup({
            binary_path = nil,  -- nil = auto-detect from PATH (recommended)
        })
    end,
},
```

**Example** (actual working config):

```lua
return {
  {
    dir = "/Users/yourname/Desktop/REPO/semtree/editors/neovim",
    name = "semtree",
    lazy = false,
    config = function()
      require("semtree").setup({
        binary_path = "/Users/yourname/.cargo/bin/semtree",
      })
    end,
  },

  -- your other plugins...
}
```

### 3. Restart Neovim and try it

```
nvim any_file.py
:SemTreeInspect
```

---

## All Commands

| Command | What it does |
|---------|-------------|
| `:SemTreeInspect` | Interactive tree inspector with real-time source highlighting |
| `:SemTreeParse` | Prettified S-expression tree in a split panel |
| `:SemTreeParse tree` | Indented tree format with byte ranges |
| `:SemTreeParse sexp` | Compact single-line S-expression |
| `:SemTreeParse json` | Full JSON output with ranges |
| `:SemTreeTree` | Shortcut for `:SemTreeParse tree` |
| `:SemTreeSymbols` | List all symbols (functions, variables, classes) |
| `:SemTreeLint` | Lint current file, show diagnostics inline |
| `:SemTreeFormat` | Format current file |

---

## SemTreeInspect — Interactive Tree Inspector

This is the main feature. It works like tree-sitter's `:InspectTree`:

1. Open any supported file
2. Run `:SemTreeInspect`
3. A side panel opens with the full syntax tree
4. Navigate with `j`/`k` — the corresponding source code highlights in real time
5. Press `Enter` to jump your cursor to that source location
6. Press `q` to close

### Inspector Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` | Move up/down through tree nodes |
| `Enter` | Jump to the source location of the current node |
| `q` | Close the inspector and clear highlights |
| `?` | Show help |

### What it looks like

The tree panel shows a color-coded syntax tree:
- **Blue** — non-terminal nodes (e.g. `ClassDef`, `FunctionDef`, `Expression`)
- **Green** — leaf token kinds (e.g. `identifier`, `integer`, `string`)
- **Orange** — token text values
- **Gray** — byte ranges `[start..end]`

As you move through the tree, the source file highlights the exact span
of whichever node your cursor is on.

---

## SemTreeParse — Pretty-Printed Tree

`:SemTreeParse` now shows a clean, indented S-expression tree:

```
(source_file [0..543]
  (Module [0..542]
    (Statement [0..542]
      (ClassDef [0..542]
        (identifier "class") [0..5]
        (identifier "Calculator") [6..16]
        (identifier ":") [16..17]
        (Body [17..542]
          (Statement [17..542]
            (FunctionDef [17..542]
              (identifier "def") [22..25]
              (identifier "__init__") [26..34]
              ...
```

Whitespace and newline tokens are hidden for readability.
Each node shows its byte range `[start..end]` for precise source mapping.

---

## Recommended Keymaps

Add to your Neovim config (e.g. `lua/mappings.lua` or after the plugin block):

```lua
vim.keymap.set("n", "<leader>si", "<cmd>SemTreeInspect<cr>", { desc = "SemTree Inspector" })
vim.keymap.set("n", "<leader>sp", "<cmd>SemTreeParse<cr>",   { desc = "SemTree Parse" })
vim.keymap.set("n", "<leader>st", "<cmd>SemTreeTree<cr>",    { desc = "SemTree Tree" })
vim.keymap.set("n", "<leader>ss", "<cmd>SemTreeSymbols<cr>", { desc = "SemTree Symbols" })
vim.keymap.set("n", "<leader>sl", "<cmd>SemTreeLint<cr>",    { desc = "SemTree Lint" })
vim.keymap.set("n", "<leader>sf", "<cmd>SemTreeFormat<cr>",  { desc = "SemTree Format" })
```

---

## Setup Options

```lua
require("semtree").setup({
    binary_path = nil,       -- path to semtree binary (nil = auto-detect from PATH)
    lint_on_save = false,    -- run :SemTreeLint automatically on save
})
```

---

## Health Check

Run `:checkhealth semtree` to verify the binary is found and working.

---

## CLI Usage (Outside Neovim)

The same binary works from the terminal:

```bash
# Pretty S-expression (same as :SemTreeParse)
semtree run -f sexp-pretty myfile.py

# Indented tree with byte ranges (same as :SemTreeTree)
semtree run -f tree myfile.py

# JSON output
semtree run -f json myfile.py

# Use GLR parser backend for ambiguous grammars
semtree run --backend glr -f tree myfile.py

# Specify a grammar explicitly
semtree run -g grammars/python.semtree -f tree myfile.py
```

---

## Supported Languages (Auto-Detected)

| Extension | Grammar File |
|-----------|-------------|
| `.py`, `.pyw` | `grammars/python.semtree` |
| `.js`, `.jsx`, `.mjs`, `.cjs` | `grammars/javascript.semtree` |
| `.ts`, `.tsx` | `grammars/javascript.semtree` |
| `.rs` | `grammars/rust.semtree` |
| `.css`, `.scss`, `.less` | `grammars/css.semtree` |
| `.json` | `grammars/json.semtree` |
| `.toml` | `grammars/toml.semtree` |

The grammar is auto-detected from the file extension. No `:TSInstall` needed.

---

## How It Compares to Tree-sitter

| Feature | Tree-sitter | SemTree |
|---------|------------|---------|
| Setup | `:TSInstall` per language | One binary, all languages |
| Inspect tree | `:InspectTree` | `:SemTreeInspect` |
| Parse speed | Fast (C) | Comparable (Rust) |
| Source highlighting | Built-in | Real-time in inspector |
| Symbols | External (LSP) | Built-in `:SemTreeSymbols` |
| Linting | External | Built-in `:SemTreeLint` |
| Formatting | External | Built-in `:SemTreeFormat` |
| Parser backend | LR/GLR | Recursive Descent + GLR |

---

## Troubleshooting

**"Not an editor command: SemTreeParse"**
- Make sure `lazy = false` is set in the plugin config
- Run `:Lazy` and check that `semtree` shows as loaded
- Restart Neovim completely after adding the plugin

**"SemTree binary not found"**
- Run `which semtree` in your terminal — if empty, the binary isn't in PATH
- Either add `~/.cargo/bin` to your PATH, or set `binary_path` explicitly

**"No grammar found for .xyz files"**
- The file extension isn't auto-detected. Use `-g` to specify a grammar:
  ```
  semtree run -g /path/to/grammar.semtree myfile.xyz
  ```

**Parse errors in the tree**
- Some grammars are partial. The parser will recover and produce a tree,
  but error nodes will appear. Check the bottom of the output for error details.
