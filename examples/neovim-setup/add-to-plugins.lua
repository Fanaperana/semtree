-- SemTree Neovim Plugin Configuration
-- =====================================
--
-- Add this block inside your return { ... } table in:
--   ~/.config/nvim/lua/plugins/init.lua
--
-- IMPORTANT:
--   1. Change `dir` to the absolute path of semtree/editors/neovim on YOUR machine
--   2. Change `binary_path` to where `cargo install` put the binary,
--      or set to nil to auto-detect from PATH
--   3. Set `lazy = false` so the plugin loads immediately (commands are available right away)

{
    dir = "/path/to/semtree/editors/neovim",  -- CHANGE THIS to your clone location
    name = "semtree",
    lazy = false,
    config = function()
        require("semtree").setup({
            binary_path = nil,           -- nil = auto-detect from PATH
            -- binary_path = "/Users/you/.cargo/bin/semtree",  -- or set explicitly
            lint_on_save = false,        -- set true to auto-lint on save
        })
    end,
},

-- Optional keymaps (add to your keymaps config or after the plugin block):
--
-- vim.keymap.set("n", "<leader>si", "<cmd>SemTreeInspect<cr>", { desc = "SemTree Inspector" })
-- vim.keymap.set("n", "<leader>sp", "<cmd>SemTreeParse<cr>",   { desc = "SemTree Parse" })
-- vim.keymap.set("n", "<leader>st", "<cmd>SemTreeTree<cr>",    { desc = "SemTree Tree" })
-- vim.keymap.set("n", "<leader>ss", "<cmd>SemTreeSymbols<cr>", { desc = "SemTree Symbols" })
-- vim.keymap.set("n", "<leader>sl", "<cmd>SemTreeLint<cr>",    { desc = "SemTree Lint" })
-- vim.keymap.set("n", "<leader>sf", "<cmd>SemTreeFormat<cr>",  { desc = "SemTree Format" })
