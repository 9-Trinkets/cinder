#!/usr/bin/env python3
"""Warn on non-JSON source files that exceed a line threshold.

Surfaces long files in a clippy-style warning (always exits 0, non-blocking)
so line counts stay visible without blocking commits. JSON files are excluded
because generated/declarative content (rooms, beats, actions) is typically
machine-written and large by nature.

Usage:
    python3 scripts/check_file_lengths.py [--limit 500]
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Extensions considered "source" (non-JSON). JSON is excluded.
SOURCE_EXTS = {".rs", ".ts", ".tsx", ".js", ".jsx", ".go", ".py"}

# Directories never scanned (build artifacts and vendored deps).
SKIP_DIRS = {"target", "node_modules", ".git", "dist", "build", ".vercel"}


def iter_source_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(root):
        # Prune SKIP_DIRS in-place to avoid descending into ignored directories
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for filename in filenames:
            path = Path(dirpath) / filename
            if path.suffix in SOURCE_EXTS:
                files.append(path)
    return files


def count_lines(path: Path) -> int:
    with path.open("r", encoding="utf-8", errors="replace") as handle:
        return sum(1 for _ in handle)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--limit", type=int, default=500, help="line threshold (default: 500)")
    parser.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    args = parser.parse_args()

    offenders: list[tuple[int, str]] = []
    for source_file in iter_source_files(ROOT):
        count = count_lines(source_file)
        if count > args.limit:
            offenders.append((count, source_file.relative_to(ROOT).as_posix()))

    offenders.sort(reverse=True)

    if args.json:
        payload = [{"lines": count, "file": rel} for count, rel in offenders]
        print(json.dumps({"limit": args.limit, "offenders": payload}, indent=2))
    else:
        for count, rel in offenders:
            print(f"warning: {rel} has {count} lines, exceeding the {args.limit}-line soft limit")
        if offenders:
            print(f"\n{len(offenders)} source file(s) exceed {args.limit} lines "
                  "(warning only, non-blocking). Consider splitting the largest by responsibility.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
