#!/usr/bin/env bash
# Installs the versioned pre-commit hook (scripts/pre-commit) into
# .git/hooks/pre-commit, baking in this checkout's absolute repo root so the
# installed hook works from any working directory. Run after cloning or
# whenever scripts/pre-commit changes.
set -eu

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOK_SOURCE="$REPO_ROOT/scripts/pre-commit"
HOOK_DEST="$REPO_ROOT/.git/hooks/pre-commit"

sed "s|__CINDER_REPO_ROOT__|$REPO_ROOT|" "$HOOK_SOURCE" > "$HOOK_DEST"
chmod 0755 "$HOOK_DEST"
echo "Installed pre-commit hook: $HOOK_DEST"
