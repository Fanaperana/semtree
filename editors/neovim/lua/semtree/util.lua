local M = {}

--- Whether `semtree --help` lists the `lsp` subcommand.
--- Clap indents commands (`  lsp  ...`), so we match per line.
function M.has_lsp_command(binary)
    if not binary or binary == "" then
        return false
    end

    local help = vim.fn.system({ binary, "--help" })
    if vim.v.shell_error ~= 0 then
        return false
    end

    for line in help:gmatch("[^\r\n]+") do
        if line:match("^%s*lsp%s") then
            return true
        end
    end

    return false
end

--- Absolute path to `path`, with fallbacks for older Neovim builds.
function M.abspath(path)
    if path == "" then
        return nil
    end

    if vim.fs and type(vim.fs.realpath) == "function" then
        local ok, resolved = pcall(vim.fs.realpath, path)
        if ok and resolved then
            return resolved
        end
    end

    local uv = vim.uv or vim.loop
    if uv and type(uv.fs_realpath) == "function" then
        local ok, resolved = pcall(uv.fs_realpath, path)
        if ok and resolved then
            return resolved
        end
    end

    return vim.fn.fnamemodify(path, ":p")
end

--- Parent directory of a file path (absolute).
function M.file_dir(path)
    local abs = M.abspath(path)
    if not abs or abs == "" then
        return nil
    end
    return (vim.fs and vim.fs.dirname(abs)) or vim.fn.fnamemodify(abs, ":h")
end

--- Compatibility wrapper for vim.lsp.get_clients (Neovim 0.10+) / vim.lsp.get_active_clients (older).
function M.get_lsp_clients(filter)
    if vim.lsp.get_clients then
        return vim.lsp.get_clients(filter)
    end
    ---@diagnostic disable-next-line: deprecated
    return vim.lsp.get_active_clients(filter)
end

--- Find project root by searching upward for a grammars/ directory or common root markers.
function M.find_root(path)
    local markers = { "grammars", ".git", "Cargo.toml", "package.json" }
    local dir = M.file_dir(path)
    if not dir then
        return nil
    end

    if vim.fs and vim.fs.find then
        local found = vim.fs.find(markers, { path = dir, upward = true, type = "directory" })
        if found and found[1] then
            return vim.fs.dirname(found[1])
        end
        -- Also check file markers
        local file_found = vim.fs.find({ ".git", "Cargo.toml", "package.json" }, { path = dir, upward = true, type = "file" })
        if file_found and file_found[1] then
            return vim.fs.dirname(file_found[1])
        end
    end

    return nil
end

return M
