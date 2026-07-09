--- Live incremental parsing via LSP and debounced CLI fallback.

local M = {}

local ns_diag = vim.api.nvim_create_namespace("semtree_live")

--- @type table<number, {job: number|nil, timer: uv_timer_t|nil}>
M.buffers = {}

function M.setup(config)
    M.config = config

    if not config.live_parse then
        return
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

function M.ensure_lsp(buf)
    if vim.lsp.get_clients({ bufnr = buf, name = "semtree" })[1] then
        return
    end

    local file = vim.api.nvim_buf_get_name(buf)
    if file == "" then
        return
    end

    vim.lsp.start({
        name = "semtree",
        cmd = { M.config.binary_path, "lsp" },
        root_dir = vim.fs.dirname(file),
        attach = false,
    })
end

function M.schedule_parse(buf)
    if not vim.api.nvim_buf_is_valid(buf) then
        return
    end

    local state = M.buffers[buf] or {}
    M.buffers[buf] = state

    if state.timer then
        state.timer:stop()
        state.timer:close()
    end

    state.timer = vim.defer_fn(function()
        M.parse_buffer_cli(buf)
    end, M.config.debounce_ms or 200)
end

--- Fallback: incremental CLI parse for diagnostics when LSP isn't attached.
function M.parse_buffer_cli(buf)
    local file = vim.api.nvim_buf_get_name(buf)
    if file == "" or not vim.api.nvim_buf_is_valid(buf) then
        return
    end

    if vim.lsp.get_clients({ bufnr = buf, name = "semtree" })[1] then
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
