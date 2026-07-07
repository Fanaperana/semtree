-- Add this to your ~/.config/nvim/lua/plugins/init.lua
-- inside the return { ... } table:

-- SemTree: Universal language parsing (no tree-sitter install needed)
{
    dir = "~/Desktop/REPO/semtree/editors/neovim",
    name = "semtree",
    event = { "BufReadPre", "BufNewFile" },
    config = function()
        require("semtree").setup({
            -- binary_path = nil,    -- auto-detect from PATH
            -- lint_on_save = false, -- set true for auto-lint on save
        })
    end,
},
