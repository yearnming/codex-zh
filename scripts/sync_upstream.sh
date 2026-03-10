#!/usr/bin/env bash
set -euo pipefail

UPSTREAM_URL="${UPSTREAM_URL:-https://github.com/openai/codex.git}"
UPSTREAM_REMOTE="${UPSTREAM_REMOTE:-upstream}"
DEFAULT_BRANCH="${DEFAULT_BRANCH:-main}"
L10N_BRANCH="${L10N_BRANCH:-l10n/zh}"

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "Not a git repository." >&2
  exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Working tree is dirty. Please commit or stash changes first." >&2
  exit 1
fi

if ! git remote get-url "$UPSTREAM_REMOTE" >/dev/null 2>&1; then
  git remote add "$UPSTREAM_REMOTE" "$UPSTREAM_URL"
fi

git fetch "$UPSTREAM_REMOTE" --tags

if git show-ref --verify --quiet "refs/heads/$L10N_BRANCH"; then
  git checkout "$L10N_BRANCH"
  git rebase "$UPSTREAM_REMOTE/$DEFAULT_BRANCH"
else
  echo "Branch $L10N_BRANCH not found. Create it after initial l10n work." >&2
  exit 2
fi

