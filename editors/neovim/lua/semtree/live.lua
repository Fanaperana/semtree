--- Live incremental parsing via LSP and debounced CLI fallback.

local util = require("semtree.util")

local M = {}

local ns_diag = vim.api.nvim_create_namespace("semtree_live")

--- @type table<number, {generation: number}>
M.buffers = {}

--- @type boolean|nil
M.lsp_supported = nil

function M.setup(config)
    M.config = config

    if not config.live_parse then
        return
    end

    if not M.check_lsp_supported() then
        vim.notify(
            "SemTree: binary lacks 'lsp' subcommand — using CLI fallback. Reinstall: cargo install --path crates/semtree_cli --force",
            vim.log.levels.WARN,
            { once = true }
        )
    end

    local augroup = vim.api.nvim_create_augroup("SemTreeLive", { clear = true })

    vim.api.nvim_create_autocmd({ "BufEnter", "BufWritePost" }, {
        group = augroup,
        callback = function(args)
            M.ensure_lsp(args.buf)
            M.schedule_parse(args.buf)
        end,
    })

    vim.api.nvim_create_autocmd("TextChanged", {
        group = augroup,
        callback = function(args)
            M.schedule_parse(args.buf)
        end,
    })

    vim.api.nvim_create_autocmd("BufDelete", {
        group = augroup,
        callback = function(args)
            M.buffers[args.buf] = nil
        end,
    })
end

--- Check once whether the installed binary supports `semtree lsp`.
function M.check_lsp_supported()
    if M.lsp_supported ~= nil then
        return M.lsp_supported
    end

    local binary = M.config and M.config.binary_path or vim.fn.exepath("semtree")
    if binary == "" then
        M.lsp_supported = false
        return false
    end

    M.lsp_supported = util.has_lsp_command(binary)
    return M.lsp_supported
end

function M.ensure_lsp(buf)
    if not M.check_lsp_supported() then
        return
    end

    if util.get_lsp_clients({ bufnr = buf, name = "semtree" })[1] then
        return
    end

    local file = vim.api.nvim_buf_get_name(buf)
    if file == "" then
        return
    end

    local root_dir = util.find_root(file) or util.file_dir(file)
    if not root_dir or root_dir == "" then
        return
    end

    pcall(vim.lsp.start, {
        name = "semtree",
        cmd = { M.config.binary_path, "lsp" },
        root_dir = root_dir,
    }, { bufnr = buf })
end

function M.schedule_parse(buf)
    if not vim.api.nvim_buf_is_valid(buf) then
        return
    end

    local state = M.buffers[buf] or { generation = 0 }
    M.buffers[buf] = state

    state.generation = state.generation + 1
    local gen = state.generation

    vim.defer_fn(function()
        if not vim.api.nvim_buf_is_valid(buf) then
            return
        end
        local current = M.buffers[buf]
        if not current or current.generation ~= gen then
            return
        end
        M.parse_buffer_cli(buf)
    end, M.config.debounce_ms or 200)
end

--- Fallback: incremental CLI parse for diagnostics when LSP isn't attached.
function M.parse_buffer_cli(buf)
    local file = vim.api.nvim_buf_get_name(buf)
    if file == "" or not vim.api.nvim_buf_is_valid(buf) then
        return
    end

    if util.get_lsp_clients({ bufnr = buf, name = "semtree" })[1] then
        return
    end

    if vim.bo[buf].modified then
        vim.api.nvim_buf_call(buf, function()
            vim.cmd("write")
        end)
    end

    local cmd = string.format(
        "%s run %s --incremental -f json 2>/dev/null",
        M.config.binary_path,
        vim.fn.shellescape(file)
    )
    local output = vim.fn.system(cmd)
    if vim.v.shell_error ~= 0 then
        return
    end

    local ok, tree = pcall(vim.json.decode, output)
    if not ok or not tree then
        return
    end

    local errors = {}
    if tree.errors then
        for _, err in ipairs(tree.errors) do
            table.insert(errors, {
                lnum = 0,
                col = 0,
                severity = vim.diagnostic.severity.ERROR,
                message = tostring(err),
                source = "semtree",
            })
        end
    end

    vim.diagnostic.set(ns_diag, buf, errors)
end

return M
