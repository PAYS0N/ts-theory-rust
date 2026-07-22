#!/usr/bin/env bash
# Reject any `#[allow(...)]` attribute outside of permitted locations.
# Permitted: nowhere in MVP. Tests get unwrap/expect via clippy config, not
# via #[allow]. Covers both #[allow(...)] and #![allow(...)].
#
# This is intentionally strict. The appeal mechanism for legitimate
# suppressions is deferred past MVP.
#
# The inspected set is enumerated via `git ls-files` (tracked +
# untracked-but-not-ignored), NOT a filesystem walk. A walk descends into
# `target/` before excluding it and can race a concurrent writer (cargo,
# rust-analyzer), aborting mid-walk with no diagnostic — a phantom,
# non-deterministic failure. git prunes ignored paths itself, so this
# check is a pure function of repo state, independent of build state.

set -euo pipefail

ROOT="${1:-.}"
cd "$ROOT"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "FAIL: no_allow_check.sh: '$ROOT' is not a git repository" >&2
    exit 1
fi

FAIL=0

mapfile -d '' -t files < <(
    git ls-files -z --cached --others --exclude-standard -- '*.rs'
)

if [[ ${#files[@]} -eq 0 ]]; then
    exit 0
fi

# `/dev/null` guarantees >1 path arg so grep always prefixes `file:line:`
# (the `path:line` form ctx-verify's parser splits) even for a lone file.
# Do not prefix `$line` with a label before emitting it — `path:line:content`
# must stay the first thing after `FAIL:` or ctx-verify's `split_loc` misreads
# the label as the file (see retired_terms_check.sh, which follows the same
# rule). The suffix appended after `$line` is free-form message text: this is
# where the "why banned / what to do" guidance belongs, since the matched
# line alone doesn't say either.
while IFS= read -r line; do
    echo "FAIL: $line -- #[allow] is banned with no exceptions (MVP has no suppression mechanism); fix the underlying lint instead of suppressing it" >&2
    FAIL=1
done < <(grep -nE '^[[:space:]]*#!?\[allow\(' -- "${files[@]}" /dev/null || true)

exit $FAIL
