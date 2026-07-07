# SemTree Neovim Integration — Step by Step

This guide shows how to set up SemTree in Neovim for languages
you **don't have tree-sitter parsers** for (e.g. Python, Rust, TOML).

## Prerequisites

```bash
# Build the semtree binary
cd /path/to/semtree
cargo install --path crates/semtree_cli

# Verify it works
semtree --help
```

## Step 1: Add to Neovim (lazy.nvim)

Add this to your `~/.config/nvim/lua/plugins/init.lua`:

```lua
{
    dir = "/path/to/semtree/editors/neovim",
    name = "semtree",
    config = function()
        require("semtree").setup({
            binary_path = nil,       -- auto-detect from PATH
            lint_on_save = false,    -- set true to auto-lint
        })
    end,
},
```

Or if you want to use the repo directly:

```lua
{
    dir = "~/Desktop/REPO/semtree/editors/neovim",
    name = "semtree",
    config = function()
        require("semtree").setup()
    end,
},
```

## Step 2: Available Commands

Once installed, you get these commands:

| Command | What it does |
|---------|-------------|
| `:SemTreeInspect` | **Interactive tree inspector** — navigate nodes, highlights source in real time |
| `:SemTreeParse` | Parse current file, show prettified S-expression tree in split |
| `:SemTreeParse tree` | Parse with indented tree format |
| `:SemTreeParse json` | Parse with JSON output |
| `:SemTreeTree` | Shortcut for `:SemTreeParse tree` |
| `:SemTreeSymbols` | Show all symbols (functions, variables, types) |
| `:SemTreeLint` | Lint current file, show diagnostics |
| `:SemTreeFormat` | Format current file |

### SemTreeInspect (Interactive Tree Inspector)

This is the flagship feature, similar to tree-sitter's `:InspectTree`. It opens
the syntax tree in a side panel, and as you navigate with `j`/`k`, the
corresponding source code is highlighted in real time.

**Keybindings in the inspector:**

| Key | Action |
|-----|--------|
| `j`/`k` | Navigate tree nodes (highlights source automatically) |
| `Enter` | Jump cursor to the source location of the node |
| `q` | Close the inspector |
| `?` | Show help |

## Step 3: Try it on a Python file

Open a Python file (no tree-sitter-python needed!):

```bash
nvim examples/demo.py
```

Then run:
```
:SemTreeParse
```

You'll see the syntax tree in a split window.

Try `:SemTreeSymbols` to see all symbols in the file.

## Step 4: Try it on Rust and TOML

```bash
nvim examples/demo.rs     # then :SemTreeParse
nvim examples/demo.toml   # then :SemTreeParse
```

## Step 5: Add Keymaps (Optional)

Add these to your `mappings.lua`:

```lua
vim.keymap.set("n", "<leader>si", "<cmd>SemTreeInspect<cr>", { desc = "SemTree Inspector" })
vim.keymap.set("n", "<leader>sp", "<cmd>SemTreeParse<cr>", { desc = "SemTree Parse" })
vim.keymap.set("n", "<leader>st", "<cmd>SemTreeTree<cr>", { desc = "SemTree Tree" })
vim.keymap.set("n", "<leader>ss", "<cmd>SemTreeSymbols<cr>", { desc = "SemTree Symbols" })
vim.keymap.set("n", "<leader>sl", "<cmd>SemTreeLint<cr>", { desc = "SemTree Lint" })
vim.keymap.set("n", "<leader>sf", "<cmd>SemTreeFormat<cr>", { desc = "SemTree Format" })
```

## Step 6: Health Check

Run `:checkhealth semtree` to verify everything is set up correctly.

## How it compares to Tree-sitter

| Feature | Tree-sitter | SemTree |
|---------|------------|---------|
| Install parser | `:TSInstall python` | Grammar file included |
| Parse speed | Fast (C) | 1.5-3.5x faster (Rust) |
| Highlighting | Built-in | Via highlight queries |
| Symbols | External (LSP) | Built-in `:SemTreeSymbols` |
| Linting | External | Built-in `:SemTreeLint` |
| Formatting | External | Built-in `:SemTreeFormat` |
| Setup | Per-language install | One binary, all languages |

## Supported Languages

| Extension | Grammar | Auto-detected |
|-----------|---------|---------------|
| `.js`, `.jsx`, `.mjs` | `grammars/javascript.semtree` | Yes |
| `.py` | `grammars/python.semtree` | Yes |
| `.rs` | `grammars/rust.semtree` | Yes |
| `.css` | `grammars/css.semtree` | Yes |
| `.json` | `grammars/json.semtree` | Yes |
| `.toml` | `grammars/toml.semtree` | Yes |
