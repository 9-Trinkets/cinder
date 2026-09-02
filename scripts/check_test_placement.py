#!/usr/bin/env python3
"""Warn on integration test suites placed under a crate's src/ tree.

Project rule: integration test suites must live in the crate's top-level
`tests/` directory, not under `src/`. Suites living in `src/` cannot import
the crate as a separate test crate and are the root cause of oversized,
private-internal test files.

A test suite is any `tests.rs`/`tests/**` file declared as a standalone
test module (e.g. `#[cfg(test)] mod tests;`). In-module unit tests
(`#[cfg(test)] mod tests {}` embedded alongside production code that test
private functions) are idiomatic Rust and are intentionally NOT flagged.

This check is warning-only / non-blocking. Files under a `bin/` debug-only
tree are excluded because they are standalone harnesses, not library suites.

Usage:
    python3 scripts/check_test_placement.py [--json]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# A crate's library tests may only live in its `tests/` dir; flag any suite
# file under a `src/` tree.
SRC_ROOTS = ("cinder-core/src", "cinder-srv/src")
SKIP_PATH_PARTS = {"target", "node_modules", ".git", "bin", "main.rs", "lib.rs"}

# Names/patterns identifying a standalone test-suite file under src/.
SUITE_FILENAME = "tests.rs"
SUITE_DIR = "tests"


def iter_src_files() -> list[Path]:
    files: list[Path] = []
    for root in SRC_ROOTS:
        base = ROOT / root
        if not base.exists():
            continue
        for path in base.rglob("*.rs"):
            if any(part in SKIP_PATH_PARTS for part in path.relative_to(base).parts):
                continue
            files.append(path)
    return files


def is_misplaced_suite(path: Path) -> bool:
    if path.name == SUITE_FILENAME:
        return True
    return SUITE_DIR in path.parts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    args = parser.parse_args()

    misplaced: list[str] = []
    for src_file in iter_src_files():
        if is_misplaced_suite(src_file):
            misplaced.append(src_file.relative_to(ROOT).as_posix())
    misplaced.sort()

    if args.json:
        print(json.dumps({"misplaced_integration_suites": misplaced}, indent=2))
        return 0

    for rel in misplaced:
        print(f"warning: [{rel}] is an integration test suite under src/; "
              "move it to the crate's tests/ directory")
    print(f"\nIntegration-suite placement warnings: {len(misplaced)}. "
          "Warning only, non-blocking.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
