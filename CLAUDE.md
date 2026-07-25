- Source: native Read/Edit/Grep — never `cat` or `sed` for file reads. 
  to read only a specific set of lines, use the read tool's offset 
  and limit optional paramaters, not sed. The
  context chain is hook-injected on read; read a directory's chain
  before changing its contents. Go only as deep as needed. Never
  hand-edit `.context/`.
- Do not run cargo yourself. If cargo is unavoidable:
  `-q --message-format=short`, never paste build dumps.

- Full contract for any tool: `<tool> --contract`.
<!-- BEGIN GENERATED tool-contracts (scripts/gen_tool_contracts.sh --write) -->
- **ctx-context** — `ctx-context <path>` — read a directory's chain before editing anything under it; the Read hook serves it automatically, this is the on-demand path.
- **ctx-verify** — `ctx-verify [crate]` — the only way to build, lint, or test; the task is not done until it prints `pass`.
- **ctx-status** — `ctx-status list` — see the backlog; `add-task`/`delete-task <id>` to change it, never hand-edit `docs/STATUS.md`.
<!-- END GENERATED tool-contracts -->

- Lints: `#[allow]` is banned. unwrap/expect compile only inside
  `#[test]`/`#[cfg(test)]` bodies — test helpers outside them must
  return `Result`. To reduce 30-line fn / 250-line file: refactor first; a
  single-line `// rationale:` directly above (fn) or after `//!`
  (file) is the last resort, and multi-line is not recognized.
- `.env` holds the summarizer key: never feed it to a model, never
  commit it. 
- Retiring a tool/identifier: add it to `scripts/retired_terms_check.sh`'s
  `BANNED` array.
- when debugging, always ensure that the thing you are changing 
  is the true source of the issue. Do not reach a purely logical 
  conclusion and change code; verify the issue.
- The task is not done until ctx-verify passes.
