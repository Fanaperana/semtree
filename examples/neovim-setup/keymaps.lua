-- SemTree keymaps — add these to your mappings.lua or keymaps.lua
-- All prefixed with <leader>s for "SemTree"

local map = vim.keymap.set

map("n", "<leader>sp", "<cmd>SemTreeParse<cr>", { desc = "SemTree: Parse (S-expression)" })
map("n", "<leader>st", "<cmd>SemTreeTree<cr>", { desc = "SemTree: Show tree" })
map("n", "<leader>ss", "<cmd>SemTreeSymbols<cr>", { desc = "SemTree: Show symbols" })
map("n", "<leader>sl", "<cmd>SemTreeLint<cr>", { desc = "SemTree: Lint buffer" })
map("n", "<leader>sf", "<cmd>SemTreeFormat<cr>", { desc = "SemTree: Format buffer" })
map("n", "<leader>sj", "<cmd>SemTreeParse json<cr>", { desc = "SemTree: Parse (JSON)" })
