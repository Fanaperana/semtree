local M = {}

function M.check()
    vim.health.start("SemTree")

    local binary = vim.fn.exepath("semtree")
    if binary ~= "" then
        vim.health.ok("semtree binary found: " .. binary)
    else
        vim.health.error("semtree binary not found", {
            "Install with: cargo install --path crates/semtree_cli",
        })
    end

    local grammars = { "json", "javascript", "rust", "python", "css", "toml" }
    for _, g in ipairs(grammars) do
        local path = "grammars/" .. g .. ".semtree"
        if vim.fn.filereadable(path) == 1 then
            vim.health.ok("Grammar found: " .. g)
        else
            vim.health.info("Grammar not found: " .. g)
        end
    end
end

return M
