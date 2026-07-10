--- Shared side-panel with interactive source highlighting.
--- Used by SemTreeParse, SemTreeTree, and SemTreeSymbols to provide
--- the same cursor-follows-source experience as SemTreeInspect.

local M = {}

local hl_ns = vim.api.nvim_create_namespace("semtree_panel_hl")

--- Convert byte offset to (0-indexed line, 0-indexed col) in a buffer.
function M.byte_to_pos(buf, byte_offset)
    local line_count = vim.api.nvim_buf_line_count(buf)
    local current_byte = 0

    for lnum = 0, line_count - 1 do
        local line_text = vim.api.nvim_buf_get_lines(buf, lnum, lnum + 1, false)[1] or ""
        local line_bytes = #line_text + 1 -- +1 for newline

        if current_byte + line_bytes > byte_offset then
            return lnum, byte_offset - current_byte
        end

        current_byte = current_byte + line_bytes
    end

    return line_count - 1, 0
end

--- Highlight a byte range in the source buffer.
function M.highlight_range(source_buf, source_win, start_byte, end_byte)
    vim.api.nvim_buf_clear_namespace(source_buf, hl_ns, 0, -1)

    if start_byte >= end_byte then return end

    local start_line, start_col = M.byte_to_pos(source_buf, start_byte)
    local end_line, end_col = M.byte_to_pos(source_buf, end_byte)

    local line_count = vim.api.nvim_buf_line_count(source_buf)
    start_line = math.min(start_line, line_count - 1)
    end_line = math.min(end_line, line_count - 1)

    if start_line == end_line then
        local line_text = vim.api.nvim_buf_get_lines(source_buf, start_line, start_line + 1, false)[1] or ""
        end_col = math.min(end_col, #line_text)
        start_col = math.min(start_col, #line_text)
        pcall(vim.api.nvim_buf_set_extmark, source_buf, hl_ns, start_line, start_col, {
            end_row = end_line,
            end_col = end_col,
            hl_group = "SemTreeHighlight",
        })
    else
        for lnum = start_line, end_line do
            local line_text = vim.api.nvim_buf_get_lines(source_buf, lnum, lnum + 1, false)[1] or ""
            local sc = (lnum == start_line) and math.min(start_col, #line_text) or 0
            local ec = (lnum == end_line) and math.min(end_col, #line_text) or #line_text
            pcall(vim.api.nvim_buf_set_extmark, source_buf, hl_ns, lnum, sc, {
                end_row = lnum,
                end_col = ec,
                hl_group = "SemTreeHighlight",
            })
        end
    end

    if vim.api.nvim_win_is_valid(source_win) then
        pcall(vim.api.nvim_win_set_cursor, source_win, { start_line + 1, start_col })
    end
end

--- Parse byte ranges from tree-format lines: `NodeKind@start..end`
local function parse_tree_ranges(lines)
    local range_map = {}
    for i, line in ipairs(lines) do
        local s, e = line:match("@(%d+)%.%.(%d+)")
        if s and e then
            range_map[i] = { start_byte = tonumber(s), end_byte = tonumber(e) }
        end
    end
    return range_map
end

--- Parse byte ranges from sexp-pretty lines: `[start..end]`
local function parse_sexp_ranges(lines)
    local range_map = {}
    for i, line in ipairs(lines) do
        local s, e = line:match("%[(%d+)%.%.(%d+)%]")
        if s and e then
            range_map[i] = { start_byte = tonumber(s), end_byte = tonumber(e) }
        end
    end
    return range_map
end

--- Parse byte ranges from symbols output: `(start..end)`
--- Since the built-in parser's byte offsets don't map correctly to the source,
--- we search for the symbol name in the source buffer instead.
local function parse_symbol_ranges(lines)
    local range_map = {}
    local name_counts = {} -- track occurrence index for duplicate names
    for i, line in ipairs(lines) do
        -- Extract kind and name: "  [pub ][mut ]kind name (start..end)"
        local name = line:match("^%s*%S+%s+%S+%s+(%S+)%s+%(") -- "pub mut kind name"
            or line:match("^%s*%S+%s+(%S+)%s+%(")              -- "kind name"
        if name then
            name_counts[name] = (name_counts[name] or 0) + 1
            range_map[i] = { symbol_name = name, occurrence = name_counts[name] }
        end
    end
    return range_map
end

--- Detect which format the output is in and parse ranges accordingly.
function M.parse_ranges(lines)
    -- Check first few non-empty content lines to detect format
    for _, line in ipairs(lines) do
        if line:match("@%d+%.%.%d+") then
            return parse_tree_ranges(lines)
        elseif line:match("%[%d+%.%.%d+%]") then
            return parse_sexp_ranges(lines)
        elseif line:match("%((%d+)%.%.(%d+)%)") then
            return parse_symbol_ranges(lines)
        end
    end
    return {}
end

--- Open a side panel with interactive source highlighting.
--- @param source_buf number  The buffer containing the source code
--- @param source_win number  The window showing the source code
--- @param lines string[]     The display lines for the panel
--- @param range_map table    Map of line number → {start_byte, end_byte} or {symbol_name}
--- @param opts table         {filetype: string, title: string}
function M.open_panel(source_buf, source_win, lines, range_map, opts)
    opts = opts or {}
    local filetype = opts.filetype or "semtree-tree"
    local title = opts.title or "SemTree"

    -- Ensure highlight group exists
    vim.api.nvim_set_hl(0, "SemTreeHighlight", { bg = "#3a3a5c", bold = true })
    vim.api.nvim_set_hl(0, "SemTreeCursorNode", { bg = "#2a2a3c" })

    vim.cmd("vsplit")
    local panel_win = vim.api.nvim_get_current_win()
    local panel_buf = vim.api.nvim_create_buf(false, true)
    vim.api.nvim_win_set_buf(panel_win, panel_buf)

    vim.api.nvim_buf_set_lines(panel_buf, 0, -1, false, lines)
    vim.bo[panel_buf].buftype = "nofile"
    vim.bo[panel_buf].bufhidden = "wipe"
    vim.bo[panel_buf].swapfile = false
    vim.bo[panel_buf].modifiable = false
    vim.bo[panel_buf].filetype = filetype
    pcall(vim.api.nvim_buf_set_name, panel_buf, title)

    local cursor_ns = vim.api.nvim_create_namespace("semtree_panel_cursor")
    local augroup = vim.api.nvim_create_augroup("SemTreePanel_" .. panel_buf, { clear = true })

    --- Find a symbol by name in the source buffer and highlight it.
    --- Uses occurrence to disambiguate duplicate names.
    --- Returns (line_0indexed, col) or nil.
    local function find_and_highlight_symbol(name, occurrence)
        local source_lines = vim.api.nvim_buf_get_lines(source_buf, 0, -1, false)
        local count = 0
        for lnum, sline in ipairs(source_lines) do
            local start = 1
            while true do
                local col = sline:find(name, start, true)
                if not col then break end
                -- Ensure it's a word boundary (not part of a larger identifier)
                local before = col > 1 and sline:sub(col - 1, col - 1) or " "
                local after_pos = col + #name
                local after = after_pos <= #sline and sline:sub(after_pos, after_pos) or " "
                if not before:match("[%w_]") and not after:match("[%w_]") then
                    count = count + 1
                    if count == (occurrence or 1) then
                        vim.api.nvim_buf_clear_namespace(source_buf, hl_ns, 0, -1)
                        local start_col = col - 1
                        local end_col = start_col + #name
                        pcall(vim.api.nvim_buf_set_extmark, source_buf, hl_ns, lnum - 1, start_col, {
                            end_row = lnum - 1,
                            end_col = end_col,
                            hl_group = "SemTreeHighlight",
                        })
                        if vim.api.nvim_win_is_valid(source_win) then
                            pcall(vim.api.nvim_win_set_cursor, source_win, { lnum, start_col })
                        end
                        return lnum - 1, start_col
                    end
                end
                start = col + 1
            end
        end
        return nil
    end

    --- Highlight source for a range_map entry.
    local function highlight_entry(entry)
        if not entry then return end
        if entry.start_byte and entry.end_byte then
            M.highlight_range(source_buf, source_win, entry.start_byte, entry.end_byte)
        elseif entry.symbol_name then
            find_and_highlight_symbol(entry.symbol_name, entry.occurrence)
        end
    end

    -- Highlight source on cursor movement
    vim.api.nvim_create_autocmd("CursorMoved", {
        group = augroup,
        buffer = panel_buf,
        callback = function()
            local cursor = vim.api.nvim_win_get_cursor(panel_win)
            local line_nr = cursor[1]
            local entry = range_map[line_nr]

            if entry and vim.api.nvim_buf_is_valid(source_buf) and vim.api.nvim_win_is_valid(source_win) then
                highlight_entry(entry)

                -- Highlight current line in panel
                vim.api.nvim_buf_clear_namespace(panel_buf, cursor_ns, 0, -1)
                local line_text = vim.api.nvim_buf_get_lines(panel_buf, line_nr - 1, line_nr, false)[1] or ""
                pcall(vim.api.nvim_buf_set_extmark, panel_buf, cursor_ns, line_nr - 1, 0, {
                    end_row = line_nr - 1,
                    end_col = #line_text,
                    hl_group = "SemTreeCursorNode",
                })
            end
        end,
    })

    -- Clean up when panel is closed
    vim.api.nvim_create_autocmd("BufWipeout", {
        group = augroup,
        buffer = panel_buf,
        callback = function()
            if vim.api.nvim_buf_is_valid(source_buf) then
                vim.api.nvim_buf_clear_namespace(source_buf, hl_ns, 0, -1)
            end
            pcall(vim.api.nvim_del_augroup_by_id, augroup)
        end,
    })

    local keymap_opts = { buffer = panel_buf, noremap = true, silent = true }

    -- Enter: jump to source location
    vim.keymap.set("n", "<CR>", function()
        local cursor = vim.api.nvim_win_get_cursor(panel_win)
        local entry = range_map[cursor[1]]
        if entry and vim.api.nvim_win_is_valid(source_win) then
            vim.api.nvim_set_current_win(source_win)
            if entry.start_byte then
                local line, col = M.byte_to_pos(source_buf, entry.start_byte)
                vim.api.nvim_win_set_cursor(source_win, { line + 1, col })
            elseif entry.symbol_name then
                find_and_highlight_symbol(entry.symbol_name, entry.occurrence)
            end
        end
    end, keymap_opts)

    -- q: close panel
    vim.keymap.set("n", "q", function()
        if vim.api.nvim_buf_is_valid(source_buf) then
            vim.api.nvim_buf_clear_namespace(source_buf, hl_ns, 0, -1)
        end
        vim.api.nvim_win_close(panel_win, true)
    end, keymap_opts)

    -- ?: help
    vim.keymap.set("n", "?", function()
        vim.notify(table.concat({
            "SemTree Panel Keybindings:",
            "  ↑/↓  Navigate nodes",
            "  ⏎    Jump to source location",
            "  q    Close panel",
            "  ?    Show this help",
        }, "\n"), vim.log.levels.INFO)
    end, keymap_opts)

    vim.api.nvim_set_current_win(panel_win)
    vim.api.nvim_win_set_cursor(panel_win, { 1, 0 })
end

return M
