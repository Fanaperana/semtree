# SemTree for Neovim

A Neovim plugin for [SemTree](../../README.md) — universal language infrastructure.

## Installation

### With lazy.nvim

```lua
{
    "your-username/semtree",
    config = function()
        require("semtree").setup({
            lint_on_save = true,
        })
    end,
}
```

### With packer.nvim

```lua
use {
    "your-username/semtree",
    config = function()
        require("semtree").setup({
            lint_on_save = true,
        })
    end,
}
```

### Manual

Clone the repository and add to your `runtimepath`:

```vim
set rtp+=~/path/to/semtree/editors/neovim
```

Then in your `init.lua`:

```lua
require("semtree").setup()
```

## Configuration

```lua
require("semtree").setup({
    binary_path = nil,    -- auto-detect from PATH
    grammars_dir = nil,   -- auto-detect from binary location
    highlight = true,     -- enable syntax highlighting
    indent = false,       -- enable indentation
    lint_on_save = false, -- run linter on save
})
```

## Commands

| Command | Description |
|---------|-------------|
| `:SemTreeParse [format]` | Parse current buffer (formats: `tree`, `sexp`, `json`) |
| `:SemTreeTree` | Show syntax tree in a vertical split |
| `:SemTreeSymbols` | Show extracted symbols in a vertical split |
| `:SemTreeLint` | Lint current buffer and populate diagnostics |
| `:SemTreeFormat` | Format current buffer in place |

## Health Check

Run `:checkhealth semtree` to verify your installation.

## Requirements

- Neovim >= 0.9.0
- `semtree` binary in your PATH

Install the binary:

```bash
cargo install --path crates/semtree_cli
```
