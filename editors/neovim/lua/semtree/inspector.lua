--- SemTree Interactive Tree Inspector
--- Like :InspectTree in tree-sitter — navigate the syntax tree and see
--- corresponding source highlighted in real time.

local M = {}

local ns = vim.api.nvim_create_namespace("semtree_inspector")
local hl_ns = vim.api.nvim_create_namespace("semtree_highlight")

--- Parse the inspect format: "DEPTH|START|END|KIND|TEXT"
local function parse_inspect_line(line)
    local depth, start_byte, end_byte, kind, text = line:match("^(%d+)|(%d+)|(%d+)|([^|]*)|(.*)$")
    if depth then
        return {
            depth = tonumber(depth),
            start_byte = tonumber(start_byte),
            end_byte = tonumber(end_byte),
            kind = kind,
            text = text,
            is_leaf = text ~= "",
        }
    end
    return nil
end

--- Build tree display lines from parsed nodes, with foldable indentation.
local function build_display(nodes)
    local lines = {}
    local node_map = {} -- maps display line index → node data

    for _, node in ipairs(nodes) do
        local indent = string.rep("  ", node.depth)
        local display

        if node.is_leaf then
            -- Leaf: show kind and text
            local text = node.text
            if #text > 40 then
                text = text:sub(1, 37) .. "..."
            end
            -- Escape displayed text
            text = text:gsub("\\n", "⏎"):gsub("\\r", "")
            if text:match("^%s+$") then
                display = string.format("%s%s %q", indent, node.kind, text)
            else
                display = string.format("%s%s %q", indent, node.kind, text)
            end
        else
            -- Branch node: show kind with range
            display = string.format("%s%s [%d..%d]", indent, node.kind, node.start_byte, node.end_byte)
        end

        table.insert(lines, display)
        node_map[#lines] = node
    end

    return lines, node_map
end

--- Convert byte offset to (0-indexed line, 0-indexed col) in a buffer.
local function byte_to_pos(buf, byte_offset)
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
local function highlight_range(source_buf, source_win, start_byte, end_byte)
    vim.api.nvim_buf_clear_namespace(source_buf, hl_ns, 0, -1)

    if start_byte >= end_byte then return end

    local start_line, start_col = byte_to_pos(source_buf, start_byte)
    local end_line, end_col = byte_to_pos(source_buf, end_byte)

    -- Clamp values
    local line_count = vim.api.nvim_buf_line_count(source_buf)
    start_line = math.min(start_line, line_count - 1)
    end_line = math.min(end_line, line_count - 1)

    -- Use extmarks for highlighting
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
        -- Multi-line highlight
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

    -- Scroll source window to show highlighted region
    if vim.api.nvim_win_is_valid(source_win) then
        pcall(vim.api.nvim_win_set_cursor, source_win, { start_line + 1, start_col })
    end
end

--- Build the inspector UI once we have parsed nodes.
--- @param source_buf number
--- @param source_win number
--- @param file string
--- @param nodes table[]
local function show_inspector(source_buf, source_win, file, nodes)
    if #nodes == 0 then
        vim.notify("No tree nodes found", vim.log.levels.WARN)
        return
    end

    local display_lines, node_map = build_display(nodes)

    -- Create the highlight group
    vim.api.nvim_set_hl(0, "SemTreeHighlight", { bg = "#3a3a5c", bold = true })
    vim.api.nvim_set_hl(0, "SemTreeNodeKind", { fg = "#7aa2f7", bold = true })
    vim.api.nvim_set_hl(0, "SemTreeLeafKind", { fg = "#9ece6a" })
    vim.api.nvim_set_hl(0, "SemTreeRange", { fg = "#565f89" })
    vim.api.nvim_set_hl(0, "SemTreeText", { fg = "#e0af68" })
    vim.api.nvim_set_hl(0, "SemTreeCursorNode", { bg = "#2a2a3c" })

    -- Open tree panel on the right
    vim.cmd("vsplit")
    local tree_win = vim.api.nvim_get_current_win()
    local tree_buf = vim.api.nvim_create_buf(false, true)
    vim.api.nvim_win_set_buf(tree_win, tree_buf)

    vim.api.nvim_buf_set_lines(tree_buf, 0, -1, false, display_lines)
    vim.bo[tree_buf].buftype = "nofile"
    vim.bo[tree_buf].bufhidden = "wipe"
    vim.bo[tree_buf].swapfile = false
    vim.bo[tree_buf].modifiable = false
    vim.bo[tree_buf].filetype = "semtree-inspect"
    vim.api.nvim_buf_set_name(tree_buf, "SemTree Inspector: " .. vim.fn.fnamemodify(file, ":t"))

    -- Apply syntax highlighting to the tree buffer (batch in chunks to stay responsive)
    local CHUNK = 500
    local total = #display_lines
    local function highlight_chunk(start_i)
        local end_i = math.min(start_i + CHUNK - 1, total)
        for i = start_i, end_i do
            local node = node_map[i]
            if node then
                local line_text = display_lines[i]
                local indent_len = node.depth * 2
                local kind_start = indent_len
                local kind_end = kind_start + #node.kind

                if node.is_leaf then
                    pcall(vim.api.nvim_buf_add_highlight, tree_buf, ns, "SemTreeLeafKind", i - 1, kind_start, kind_end)
                    local text_start = line_text:find('"')
                    if text_start then
                        pcall(vim.api.nvim_buf_add_highlight, tree_buf, ns, "SemTreeText", i - 1, text_start - 1, #line_text)
                    end
                else
                    pcall(vim.api.nvim_buf_add_highlight, tree_buf, ns, "SemTreeNodeKind", i - 1, kind_start, kind_end)
                    local range_start = line_text:find("%[")
                    if range_start then
                        pcall(vim.api.nvim_buf_add_highlight, tree_buf, ns, "SemTreeRange", i - 1, range_start - 1, #line_text)
                    end
                end
            end
        end
        if end_i < total then
            vim.schedule(function()
                if vim.api.nvim_buf_is_valid(tree_buf) then
                    highlight_chunk(end_i + 1)
                end
            end)
        end
    end
    highlight_chunk(1)

    -- Set up cursor movement handler for interactive highlighting
    local augroup = vim.api.nvim_create_augroup("SemTreeInspector", { clear = true })

    vim.api.nvim_create_autocmd("CursorMoved", {
        group = augroup,
        buffer = tree_buf,
        callback = function()
            local cursor = vim.api.nvim_win_get_cursor(tree_win)
            local line_nr = cursor[1]
            local node = node_map[line_nr]

            if node and vim.api.nvim_buf_is_valid(source_buf) and vim.api.nvim_win_is_valid(source_win) then
                highlight_range(source_buf, source_win, node.start_byte, node.end_byte)

                -- Highlight current line in tree buffer
                vim.api.nvim_buf_clear_namespace(tree_buf, hl_ns, 0, -1)
                pcall(vim.api.nvim_buf_set_extmark, tree_buf, hl_ns, line_nr - 1, 0, {
                    end_row = line_nr - 1,
                    end_col = #(display_lines[line_nr] or ""),
                    hl_group = "SemTreeCursorNode",
                })
            end
        end,
    })

    -- Clean up when the tree buffer is closed
    vim.api.nvim_create_autocmd("BufWipeout", {
        group = augroup,
        buffer = tree_buf,
        callback = function()
            if vim.api.nvim_buf_is_valid(source_buf) then
                vim.api.nvim_buf_clear_namespace(source_buf, hl_ns, 0, -1)
            end
            vim.api.nvim_del_augroup_by_id(augroup)
        end,
    })

    -- Keymaps for the tree buffer
    local keymap_opts = { buffer = tree_buf, noremap = true, silent = true }

    -- Press Enter to jump to the source location
    vim.keymap.set("n", "<CR>", function()
        local cursor = vim.api.nvim_win_get_cursor(tree_win)
        local node = node_map[cursor[1]]
        if node and vim.api.nvim_win_is_valid(source_win) then
            vim.api.nvim_set_current_win(source_win)
            local line, col = byte_to_pos(source_buf, node.start_byte)
            vim.api.nvim_win_set_cursor(source_win, { line + 1, col })
        end
    end, keymap_opts)

    -- Press q to close the inspector
    vim.keymap.set("n", "q", function()
        if vim.api.nvim_buf_is_valid(source_buf) then
            vim.api.nvim_buf_clear_namespace(source_buf, hl_ns, 0, -1)
        end
        vim.api.nvim_win_close(tree_win, true)
    end, keymap_opts)

    -- Press ? for help
    vim.keymap.set("n", "?", function()
        vim.notify(table.concat({
            "SemTree Inspector Keybindings:",
            "  ↑/↓  Navigate tree nodes",
            "  ⏎    Jump to source location",
            "  q    Close inspector",
            "  ?    Show this help",
        }, "\n"), vim.log.levels.INFO)
    end, keymap_opts)

    -- Focus the tree window
    vim.api.nvim_set_current_win(tree_win)
    vim.api.nvim_win_set_cursor(tree_win, { 1, 0 })

    vim.notify(
        "SemTree Inspector: navigate with ↑/↓, press ⏎ to jump, q to close",
        vim.log.levels.INFO
    )
end

--- Parse raw output lines into node table.
local function parse_output(raw_output)
    local raw_lines = vim.split(raw_output, "\n")
    local nodes = {}
    for _, line in ipairs(raw_lines) do
        local node = parse_inspect_line(line)
        if node then
            if not (node.is_leaf and (node.kind == "whitespace" or node.kind == "newline")) then
                table.insert(nodes, node)
            end
        end
    end
    return nodes
end

function M.open(config)
    local source_buf = vim.api.nvim_get_current_buf()
    local source_win = vim.api.nvim_get_current_win()
    local file = vim.api.nvim_buf_get_name(source_buf)

    if file == "" then
        vim.notify("Buffer has no file", vim.log.levels.ERROR)
        return
    end

    -- Save the file first so the CLI sees the latest content
    if vim.bo[source_buf].modified then
        vim.cmd("write")
    end

    -- Force --backend rd: the GLR backend can be orders of magnitude slower
    -- on grammars with conflicts and provides no benefit for inspection.
    local cmd = { config.binary_path, "run", file, "-f", "inspect", "--backend", "rd" }

    vim.notify("SemTree: parsing…", vim.log.levels.INFO)

    -- Prefer vim.system (Neovim ≥ 0.10) for true async; fall back to jobstart.
    if vim.system then
        vim.system(cmd, { text = true, stderr = false }, function(obj)
            vim.schedule(function()
                if obj.code ~= 0 then
                    vim.notify("SemTree parse failed (exit " .. tostring(obj.code) .. ")", vim.log.levels.ERROR)
                    return
                end
                local nodes = parse_output(obj.stdout or "")
                show_inspector(source_buf, source_win, file, nodes)
            end)
        end)
    else
        -- Fallback for Neovim < 0.10
        local stdout_chunks = {}
        vim.fn.jobstart(cmd, {
            stdout_buffered = true,
            on_stdout = function(_, data)
                if data then
                    for _, chunk in ipairs(data) do
                        table.insert(stdout_chunks, chunk)
                    end
                end
            end,
            on_exit = function(_, exit_code)
                vim.schedule(function()
                    if exit_code ~= 0 then
                        vim.notify("SemTree parse failed (exit " .. tostring(exit_code) .. ")", vim.log.levels.ERROR)
                        return
                    end
                    local raw_output = table.concat(stdout_chunks, "\n")
                    local nodes = parse_output(raw_output)
                    show_inspector(source_buf, source_win, file, nodes)
                end)
            end,
        })
    end
end

return M
