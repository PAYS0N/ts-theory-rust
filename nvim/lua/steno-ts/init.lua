-- steno-ts — expand Plover-emitted inline LSP-snippet tokens.
--
-- A terminal Plover entry types a sentinel-wrapped token «@@<body>@@» whose
-- interior IS the LSP snippet body, with each real newline encoded as the
-- INLINE_NEWLINE marker (literal "\n": backslash + n) so the token survives
-- Plover's typing on one buffer line. This plugin
-- watches insert mode; when a complete token appears it deletes the token,
-- decodes the markers back to real newline/tab, and expands the body via
-- Neovim's built-in `vim.snippet` (requires 0.10+). No snippets.json — the
-- Plover value is self-sufficient (see docs/DECISIONS.md ADR-2).

local M = {}

-- Must match SENTINEL_OPEN/CLOSE in crates/steno/src/snippet/mod.rs. ASCII,
-- non-auto-pairing, and never valid TS so it can't appear in real code.
local OPEN = "@@"
local CLOSE = "@@"

-- Must match INLINE_NEWLINE in crates/steno/src/snippet/mod.rs: the two
-- literal, keyboard-producible characters backslash + "n" (not an actual
-- newline, and not a Unicode symbol Plover's typing can't reproduce).
local INLINE_NEWLINE = "\\n"

local config = {
  -- Only expand in these buffers (empty = any buffer).
  filetypes = { "typescript", "javascript", "typescriptreact", "javascriptreact" },
}

local augroup = vim.api.nvim_create_augroup("StenoTs", { clear = true })

-- Return the encoded LSP body of a complete @@…@@ token ending exactly at the
-- cursor, plus the 0-indexed byte column where the token starts; or nil.
local function token_before_cursor()
  local line = vim.api.nvim_get_current_line()
  local col = vim.api.nvim_win_get_cursor(0)[2] -- 0-indexed byte col of cursor
  local prefix = line:sub(1, col)
  if prefix:sub(-#CLOSE) ~= CLOSE then
    return nil
  end
  -- Strip the trailing close first, so when OPEN == CLOSE the search below
  -- can't match the closing fence as if it were the opening one.
  local inner = prefix:sub(1, #prefix - #CLOSE)
  local start, init = nil, 1
  while true do
    local i = inner:find(OPEN, init, true)
    if not i then
      break
    end
    start = i
    init = i + #OPEN
  end
  if not start then
    return nil
  end
  local body = inner:sub(start + #OPEN)
  return body, start - 1 -- start_col is 0-indexed
end

-- Decode the inline body to a real LSP snippet body.
local function decode_body(body)
  return (body:gsub(INLINE_NEWLINE, "\n"))
end

local function try_expand()
  if not vim.snippet then
    return
  end
  local body, start_col = token_before_cursor()
  if not body then
    return
  end
  local row = vim.api.nvim_win_get_cursor(0)[1] - 1
  local end_col = vim.api.nvim_win_get_cursor(0)[2]
  -- Delete the token, then expand the snippet where it stood.
  vim.api.nvim_buf_set_text(0, row, start_col, row, end_col, { "" })
  vim.api.nvim_win_set_cursor(0, { row + 1, start_col })
  vim.snippet.expand(decode_body(body))
end

local function attach_buffer()
  if #config.filetypes > 0 and not vim.tbl_contains(config.filetypes, vim.bo.filetype) then
    return
  end
  vim.api.nvim_create_autocmd("TextChangedI", {
    group = augroup,
    buffer = 0,
    callback = try_expand,
  })
end

function M.setup(opts)
  config = vim.tbl_extend("force", config, opts or {})
  vim.api.nvim_create_autocmd("FileType", {
    group = augroup,
    pattern = #config.filetypes > 0 and config.filetypes or "*",
    callback = attach_buffer,
  })
  -- Attach to already-open buffers too.
  attach_buffer()
end

-- Exposed for manual triggering / tests.
M._try_expand = try_expand
M._token_before_cursor = token_before_cursor

return M
