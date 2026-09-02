#!/usr/bin/env python3
"""Warn on test placement that violates the project's test organization rules.

Two rules (both warning-only / non-blocking):
  1. Integration test suites must live in the crate's `tests/` directory, not
     under `src/`.
  2. Tests must go through the crate's public API - in-module
     `#[cfg(test)] mod tests` blocks that sit alongside production code are
     flagged (they are the home of private-function tests and bloat prod
     files).

Usage:
    python3 scripts/check_test_placement.py [--json]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Source roots that must not contain test *suites* (only a crate's top-level
# `tests/` may). Files under a `bin/` debug-only tree are excluded because they
# are standalone harnesses, not library tests.
SRC_ROOTS = ("cinder-core/src", "cinder-srv/src")
SKIP_PATH_PARTS = {"target", "node_modules", ".git", "bin", "main.rs", "lib.rs"}

TEST_SUITE_NAMES = {"tests.rs"}  # a single-file test suite declared as `mod tests;`


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


def has_in_module_test_block(path: Path) -> bool:
    """True if the file contains `#[cfg(test)] mod tests` (or `mod tests {`)."""
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return False
    return "#[cfg(test)]" in text and "mod tests" in text


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    args = parser.parse_args()

    misplaced_suites: list[str] = []          # rule 1
    in_module_private_tests: list[str] = []   # rule 2

    for src_file in iter_src_files():
        name = src_file.name
        if name in TEST_SUITE_NAMES:
            misplaced_suites.append(src_file.relative_to(ROOT).as_posix())
        if has_in_module_test_block(src_file):
            in_module_private_tests.append(src_file.relative_to(ROOT).as_posix())

    misplaced_suites.sort()
    in_module_private_tests.sort()

    if args.json:
        print(json.dumps({
            "rules": {
                "integration_suite_in_src": misplaced_suites,
                "in_module_private_tests": in_module_private_tests,
            }
        }, indent=2))
        return 0

    for rel in misplaced_suites:
        print(f"warning: [{rel}] is an integration test suite under src/; "
              "move it to the crate's tests/ directory")
    for rel in in_module_private_tests:
        print(f"warning: [{rel}] has an in-module #[cfg(test)] mod tests block; "
              "test through the public API instead")

    rule1 = len(misplaced_suites)
    rule2 = len(in_module_private_tests)
    print(f"\nTest-placement warnings: {rule1} misplaced suite(s), "
          f"{rule2} in-module private-test block(s). Warning only, non-blocking.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
