#!/usr/bin/env python3

import argparse
import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Render a readable transcript/play from Cinder NDJSON trace logs. "
            "The target may be a run id, a single .ndjson file, or a directory of .ndjson files. "
            "If omitted, the latest run is used."
        )
    )
    parser.add_argument(
        "target",
        nargs="?",
        help="Run id (e.g. syn-1779751810600), .ndjson file, or directory of .ndjson files.",
    )
    parser.add_argument(
        "--runs-dir",
        default=".cinder-state/runs",
        help="Directory containing run NDJSON files. Default: .cinder-state/runs",
    )
    parser.add_argument(
        "--raw-complete",
        action="store_true",
        help="Also print the raw workflow.complete text block under each rendered run.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    runs_dir = Path(args.runs_dir)
    try:
        run_files = resolve_targets(args.target, runs_dir)
    except ValueError as error:
        print(f"Error: {error}", file=sys.stderr)
        return 1

    output_blocks = []
    for run_file in run_files:
        output_blocks.append(render_run(run_file, include_raw_complete=args.raw_complete))

    print("\n\n".join(output_blocks))
    return 0


def resolve_targets(target: str | None, runs_dir: Path) -> list[Path]:
    if target is None:
        latest = latest_run_file(runs_dir)
        if latest is None:
            raise ValueError(f"no .ndjson files found in {runs_dir}")
        return [latest]

    candidate = Path(target)
    if candidate.exists():
        if candidate.is_dir():
            files = sorted(
                candidate.glob("*.ndjson"),
                key=lambda path: path.stat().st_mtime,
            )
            if not files:
                raise ValueError(f"no .ndjson files found in directory {candidate}")
            return files
        if candidate.suffix != ".ndjson":
            raise ValueError(f"expected a .ndjson file, got {candidate}")
        return [candidate]

    run_file = runs_dir / f"{target}.ndjson"
    if run_file.exists():
        return [run_file]

    raise ValueError(
        f"could not resolve '{target}' as a run id, file, or directory under {runs_dir}"
    )


def latest_run_file(runs_dir: Path) -> Path | None:
    files = sorted(
        runs_dir.glob("*.ndjson"),
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    return files[0] if files else None


def render_run(path: Path, include_raw_complete: bool) -> str:
    events = load_events(path)
    if not events:
        return f"=== {path.stem} ===\n(empty trace)"

    first = events[0]
    run_id = str(first.get("run_id", path.stem))
    started_at = format_timestamp(first.get("timestamp_ms"))
    lines: list[str] = []
    seen: set[str] = set()
    turn_requests: dict[str, dict[str, Any]] = {}
    raw_complete_blocks: list[str] = []

    for event in events:
        topic = str(event.get("topic", ""))
        role = str(event.get("sender_role", ""))
        payload = event.get("payload", {})
        if not isinstance(payload, dict):
            continue

        if role == "actor_turn_decider" and topic == "model.request":
            request = payload.get("dialogue_request", {})
            actor_id = str(payload.get("actor_id") or request.get("actor_id") or "")
            if actor_id:
                turn_requests[actor_id] = request if isinstance(request, dict) else {}
            continue

        if role == "actor_turn_decider" and topic == "model.response":
            actor_id = str(payload.get("actor_id", ""))
            actor_name = str(payload.get("actor_name", actor_id))
            decision = str(payload.get("decision", "")).strip()
            request = turn_requests.get(actor_id, {})
            rendered = render_decision(actor_name, decision, request)
            if rendered is not None:
                add_line(lines, seen, rendered)
            continue

        if role == "actor_dialogue" and topic == "model.response":
            actor_name = str(payload.get("actor_name", ""))
            response_text = str(payload.get("response_text", "")).strip()
            if actor_name and response_text:
                add_line(lines, seen, f"{actor_name}: {response_text}")
            continue

        if topic == "workflow.complete":
            raw_text = str(payload.get("text", "")).strip()
            if raw_text:
                raw_complete_blocks.append(raw_text)
                for line in raw_text.splitlines():
                    stripped = line.strip()
                    if stripped:
                        add_line(lines, seen, stripped)

    body = [f"=== {run_id} ({started_at}) ===", f"Source: {path}"]
    if lines:
        body.append("")
        body.extend(lines)
    else:
        body.append("")
        body.append("(no dialogue, movement, or action lines reconstructed)")

    if include_raw_complete and raw_complete_blocks:
        body.append("")
        body.append("--- raw workflow.complete ---")
        for block in raw_complete_blocks:
            body.append(block)

    return "\n".join(body)


def render_decision(actor_name: str, decision: str, request: dict[str, Any]) -> str | None:
    if not decision:
        return None

    if decision == "MOVE" or decision.startswith("MOVE "):
        room_title = request.get("move_target_room_title")
        if isinstance(room_title, str) and room_title.strip():
            return f"{actor_name} heads to the {room_title}."
        return None

    if decision.startswith("ACT"):
        action = decision[3:].lstrip(" :-—").strip()
        if action:
            return render_actor_action_text(actor_name, action)

    return None


def render_actor_action_text(actor_name: str, action: str) -> str:
    trimmed = action.strip()
    if trimmed.startswith(actor_name):
        trimmed = trimmed[len(actor_name) :].lstrip()
    normalized = trimmed.rstrip(" .!?")
    return f"{actor_name} {normalized}."


def add_line(lines: list[str], seen: set[str], line: str) -> None:
    if line not in seen:
        seen.add(line)
        lines.append(line)


def load_events(path: Path) -> list[dict[str, Any]]:
    events = []
    with path.open() as handle:
        for raw_line in handle:
            stripped = raw_line.strip()
            if not stripped:
                continue
            events.append(json.loads(stripped))
    return events


def format_timestamp(timestamp_ms: Any) -> str:
    if not isinstance(timestamp_ms, int):
        return "unknown time"
    return datetime.fromtimestamp(timestamp_ms / 1000).isoformat(sep=" ", timespec="seconds")


if __name__ == "__main__":
    raise SystemExit(main())
