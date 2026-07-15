//! Headless-nvim integration tests for the real `nvim/lua/steno-ts/init.lua`
//! plugin.
//!
//! These catch runtime bugs that are invisible to the Rust-side pipeline
//! tests, since they only manifest once real Neovim buffer edits, cursor
//! arithmetic, and `vim.snippet.expand` get involved. Each test loads the
//! actual plugin file (not a copy) under a throwaway `nvim --headless`
//! process and drives it through `M._try_expand`/`M._token_before_cursor`.
//!
//! `nvim` (>=0.10, for `vim.snippet`) must be on `PATH` for these to run.
//! A missing binary is a loud failure, not a silent skip — same posture as
//! `scripts/cycle_check.sh`/`scripts/machete_check.sh` for their own
//! required tools, since silently mapping "tool absent" to "test passed"
//! is exactly what let a wrong assertion in this file go unnoticed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// Fail loudly if `nvim` isn't on `PATH`. These tests exist specifically to
/// exercise real `vim.snippet.expand` behavior the Rust pipeline can't
/// reach; silently skipping when the binary is absent would let a wrong
/// assertion pass unnoticed, as happened before this check existed.
fn require_nvim() {
    assert!(
        Command::new("nvim").arg("--version").output().is_ok(),
        "nvim not found on PATH: install nvim >=0.10 (needed for vim.snippet) \
         to run this test"
    );
}

/// Render `path` as a double-quoted Lua string literal.
fn lua_quote(path: &Path) -> String {
    let raw = path.display().to_string();
    format!("\"{}\"", raw.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Run `lua` headlessly against the real plugin (`nvim/` is prepended to
/// `runtimepath` so `require("steno-ts")` loads the checked-in file).
fn run_lua(lua: &str) -> Result<Output, String> {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("steno_ts_nvim_test_{}_{n}.lua", std::process::id()));
    let prelude = format!(
        "vim.opt.rtp:prepend({})\nvim.opt.virtualedit = 'onemore'\n",
        lua_quote(&repo_root().join("nvim"))
    );
    fs::write(&path, format!("{prelude}{lua}")).map_err(|e| e.to_string())?;
    let result = Command::new("nvim")
        .args(["--headless", "-u", "NONE", "-l"])
        .arg(&path)
        .output()
        .map_err(|e| e.to_string());
    let _ = fs::remove_file(&path);
    result
}

/// Run a headless script, turning a nonzero exit (an `assert()` failure
/// inside the Lua surfaces on stderr) into a descriptive `Err`.
fn assert_lua_ok(lua: &str) -> Result<(), String> {
    let out = run_lua(lua)?;
    if out.status.success() {
        return Ok(());
    }
    Err(format!(
        "nvim script failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

/// A terminal snippet with a single body-break before the exit tabstop:
/// `if(${1}) {\n${0}\}`.
const IF_SNIPPET_LUA: &str = r#"
local M = require("steno-ts")
local fixture = vim.fn.tempname() .. ".json"
local fd = assert(io.open(fixture, "w"))
fd:write(vim.json.encode({ ["STKWR-F"] = "if(${1}) {\n${0}\\}" }))
fd:close()

M.setup({ snippets_path = fixture, filetypes = {} })
vim.bo.filetype = "typescript"

vim.api.nvim_buf_set_lines(0, 0, -1, false, { "@@STKWR-F@@" })
vim.api.nvim_win_set_cursor(0, { 1, #"@@STKWR-F@@" })
M._try_expand()

local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
local expected = { "if() {", "}" }
assert(
  vim.deep_equal(lines, expected),
  ("expected %s, got %s"):format(vim.inspect(expected), vim.inspect(lines))
)
"#;

/// A terminal snippet with a single body-break (`%b`) before the exit
/// tabstop has exactly one `\n` in its rendered body — matching
/// `render::tokenize`'s and `snippet::render_terminal`'s shared treatment of
/// `Chunk::BodyBreak` as a single `Enter`/`\n` — so it must expand to two
/// lines with `${0}` sitting directly before the escaped closing brace, not
/// pushed onto a line of its own.
#[test]
fn terminal_snippet_places_exit_tabstop_before_closing_brace() {
    require_nvim();
    assert_lua_ok(IF_SNIPPET_LUA).unwrap();
}

/// Mirrors a real corpus shape (e.g. a nested generic like `Promise<%t>`
/// appended onto another type slot): the shorter chord's non-terminal
/// partial ("PARTIAL") has no dictionary entry here at all — per the fixed
/// `build_snippets` contract, only terminals are sentinel-wrapped, so
/// Plover types it as plain literal text and the plugin never touches it.
/// When the chord extends to the terminal, Plover corrects by backspacing
/// exactly what it itself last typed (`"PARTIAL"`, 7 chars) — accurate only
/// because nothing rewrote the buffer in between.
const CHORD_LUA: &str = r#"
local M = require("steno-ts")
local fixture = vim.fn.tempname() .. ".json"
local fd = assert(io.open(fixture, "w"))
fd:write(vim.json.encode({
  ["STROKE-A/STROKE-B"] = "function ${1}(${2}): number {\n${0}\\}",
}))
fd:close()

M.setup({ snippets_path = fixture, filetypes = {} })
vim.bo.filetype = "typescript"

-- Mirrors Plover's own correction model: delete the `backspace`
-- characters it remembers typing, then type the replacement.
local function plover_type(backspace, text)
  local cur = vim.api.nvim_win_get_cursor(0)
  local row, col = cur[1] - 1, cur[2]
  vim.api.nvim_buf_set_text(0, row, col - backspace, row, col, { text })
  vim.api.nvim_win_set_cursor(0, { row + 1, col - backspace + #text })
end

vim.api.nvim_buf_set_lines(0, 0, -1, false, { "PREFIX" })
vim.api.nvim_win_set_cursor(0, { 1, #"PREFIX" })

plover_type(0, "PARTIAL")
M._try_expand()

plover_type(#"PARTIAL", "@@STROKE-A/STROKE-B@@")
M._try_expand()

local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
local text = table.concat(lines, "\n")
assert(
  text:find("PREFIX", 1, true),
  ("expected PREFIX to survive the chord's correction, got %s"):format(vim.inspect(lines))
)
"#;

/// Text already in the buffer before the chord even started must survive
/// Plover's correction once the chord is extended.
#[test]
fn extending_a_chord_does_not_eat_preceding_text() {
    require_nvim();
    assert_lua_ok(CHORD_LUA).unwrap();
}
