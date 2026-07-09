local M = {}

M.config = {
    binary_path = nil,
    grammars_dir = nil,
    highlight = true,
    indent = false,
    lint_on_save = false,
    live_parse = true,
    debounce_ms = 200,
}

function M.setup(opts)
    M.config = vim.tbl_deep_extend("force", M.config, opts or {})

    local inspector = require("semtree.inspector")
    local live = require("semtree.live")

    M.config.binary_path = M.config.binary_path or vim.fn.exepath("semtree")
    if M.config.binary_path == "" then
        vim.notify("SemTree binary not found. Install with: cargo install --path crates/semtree_cli", vim.log.levels.WARN)
        return
    end

    vim.api.nvim_create_user_command("SemTreeParse", function(args)
        M.parse_buffer(args)
    end, {
        nargs = "?",
        complete = function() return { "sexp", "sexp-pretty", "tree", "json" } end,
    })

    vim.api.nvim_create_user_command("SemTreeTree", function()
        M.show_tree()
    end, {})

    vim.api.nvim_create_user_command("SemTreeInspect", function()
        inspector.open(M.config)
    end, {})

    vim.api.nvim_create_user_command("SemTreeSymbols", function()
        M.show_symbols()
    end, {})

    vim.api.nvim_create_user_command("SemTreeLint", function()
        M.lint_buffer()
    end, {})

    vim.api.nvim_create_user_command("SemTreeFormat", function()
        M.format_buffer()
    end, {})

    vim.api.nvim_create_user_command("SemTreeLsp", function()
        if live.check_lsp_supported() then
            live.ensure_lsp(0)
            vim.notify("SemTree LSP started", vim.log.levels.INFO)
        else
            vim.notify(
                "SemTree LSP unavailable. Reinstall: cargo install --path crates/semtree_cli --force",
                vim.log.levels.ERROR
            )
        end
    end, {})

    live.setup(M.config)

    if M.config.lint_on_save then
        vim.api.nvim_create_autocmd("BufWritePost", {
            callback = function() M.lint_buffer() end,
        })
    end
end

function M.parse_buffer(args)
    local file = vim.api.nvim_buf_get_name(0)
    if file == "" then
        vim.notify("Buffer has no file", vim.log.levels.ERROR)
        return
    end
    local format = (args and args.args ~= "") and args.args or "sexp-pretty"
    local cmd = string.format("%s run %s -f %s", M.config.binary_path, vim.fn.shellescape(file), format)
    local output = vim.fn.system(cmd)

    vim.cmd("vnew")
    local buf = vim.api.nvim_get_current_buf()
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, vim.split(output, "\n"))
    vim.bo[buf].buftype = "nofile"
    vim.bo[buf].bufhidden = "wipe"
    vim.bo[buf].swapfile = false
    vim.bo[buf].filetype = "semtree-tree"
    vim.api.nvim_buf_set_name(buf, "SemTree: " .. vim.fn.fnamemodify(file, ":t"))
end

function M.show_tree()
    M.parse_buffer({ args = "tree" })
end

function M.show_symbols()
    local file = vim.api.nvim_buf_get_name(0)
    if file == "" then return end
    local cmd = string.format("%s symbols %s", M.config.binary_path, vim.fn.shellescape(file))
    local output = vim.fn.system(cmd)
    vim.cmd("vnew")
    vim.api.nvim_buf_set_lines(0, 0, -1, false, vim.split(output, "\n"))
    vim.bo.buftype = "nofile"
    vim.bo.filetype = "semtree-symbols"
end

function M.lint_buffer()
    local file = vim.api.nvim_buf_get_name(0)
    if file == "" then return end
    local cmd = string.format("%s lint %s 2>&1", M.config.binary_path, vim.fn.shellescape(file))
    local output = vim.fn.system(cmd)
    local diagnostics = {}
    for _, line in ipairs(vim.split(output, "\n")) do
        local row, msg = line:match("line (%d+): (.+)")
        if row and msg then
            table.insert(diagnostics, {
                lnum = tonumber(row) - 1,
                col = 0,
                severity = vim.diagnostic.severity.WARN,
                message = msg,
                source = "semtree",
            })
        end
    end
    local ns = vim.api.nvim_create_namespace("semtree")
    vim.diagnostic.set(ns, 0, diagnostics)
end

function M.format_buffer()
    local file = vim.api.nvim_buf_get_name(0)
    if file == "" then return end

    -- Save first so the CLI sees the latest buffer content
    if vim.bo.modified then
        vim.cmd("write")
    end

    local cmd = string.format("%s format %s", M.config.binary_path, vim.fn.shellescape(file))
    local output = vim.fn.system(cmd)
    if vim.v.shell_error ~= 0 then
        vim.notify("SemTree format failed: " .. output, vim.log.levels.ERROR)
        return
    end

    -- Drop trailing empty line from split so we don't add an extra blank
    local lines = vim.split(output, "\n", { plain = true })
    if #lines > 0 and lines[#lines] == "" then
        table.remove(lines)
    end

    local view = vim.fn.winsaveview()
    vim.api.nvim_buf_set_lines(0, 0, -1, false, lines)
    vim.fn.winrestview(view)
end

return M
