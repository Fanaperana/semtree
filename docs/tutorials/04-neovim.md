# Tutorial: Use SemTree in Neovim

**Goal:** open a file in Neovim, inspect its syntax tree, and highlight source as you navigate.

**Time:** ~10 minutes

**You need:** Neovim + [lazy.nvim](https://github.com/folke/lazy.nvim), SemTree installed

---

## 1. Add the plugin

In `~/.config/nvim/lua/plugins/init.lua` (inside `return { ... }`):

```lua
{
  dir = "/ABSOLUTE/PATH/TO/semtree/editors/neovim",
  name = "semtree",
  lazy = false,
  config = function()
    require("semtree").setup({
      binary_path = nil, -- auto-detect from PATH
    })
  end,
},
```

Replace `/ABSOLUTE/PATH/TO/semtree` with your clone path.

## 2. Restart Neovim

```bash
nvim /path/to/semtree/examples/demo.py
```

## 3. Open the interactive inspector

```
:SemTreeInspect
```

- Move with `j` / `k` — the matching source span highlights
- Press `Enter` to jump to that location
- Press `q` to close

## 4. Pretty-print the tree

```
:SemTreeParse
```

Opens a side panel with indented S-expressions and byte ranges.

## 5. Format a `.semtree` grammar

```
:e /path/to/semtree/grammars/json.semtree
:SemTreeFormat
```

Grammar files use the dedicated DSL formatter (not the Rust formatter).

## Optional keymaps

```lua
vim.keymap.set("n", "<leader>si", "<cmd>SemTreeInspect<cr>", { desc = "SemTree Inspect" })
vim.keymap.set("n", "<leader>sp", "<cmd>SemTreeParse<cr>", { desc = "SemTree Parse" })
vim.keymap.set("n", "<leader>sf", "<cmd>SemTreeFormat<cr>", { desc = "SemTree Format" })
```

## Next

Want full LSP features (hover, diagnostics, go-to)? Follow the how-to:

→ [Build a Neovim LSP for your language](../how-to/neovim-lsp.md)
