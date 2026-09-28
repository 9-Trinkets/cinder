#!/usr/bin/env bash
# Removes the cargo target directory when it exceeds a size threshold.
# Safe to run at any time: no-op when target/ is healthy, when a build is
# already running, or when target/ does not exist. Called from the pre-commit
# hook so bloat is cleaned exactly when a build is about to happen.
set -eu

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="$REPO_ROOT/target"
THRESHOLD_MB="${CINDER_CLEAN_THRESHOLD_MB:-20000}"

if [ ! -d "$TARGET" ]; then
    exit 0
fi

if pgrep -x cargo >/dev/null 2>&1 || pgrep -x rustc >/dev/null 2>&1; then
    exit 0
fi

size_mb="$(du -sm "$TARGET" 2>/dev/null | cut -f1)"

if [ "$size_mb" -gt "$THRESHOLD_MB" ]; then
    echo "pre-commit: target/ is ${size_mb}MB, cleaning (threshold ${THRESHOLD_MB}MB)"
    cargo clean --manifest-path "$REPO_ROOT/Cargo.toml"
fi