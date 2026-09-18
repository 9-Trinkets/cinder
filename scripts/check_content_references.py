#!/usr/bin/env python3
"""Warn on cross-file references in content packs that do not resolve.

Content packs reference each other by id (an actor's room, a drop's item, an
action's permitted room, a movement rule's target room, a user-config item,
scripted-sequence speakers, map tiles, and so on). The engine resolves a
missing reference gracefully at runtime, so this check is advisory — it exists
to catch authoring typos early, in the same non-blocking style as
check_file_lengths.py. There is no registry of the legacy Rust load-time
validators to satisfy; this mirrors the handful of checks they performed.

Checks are generic and reusable: one ``require()`` primitive drives every
existence rule, and each rule is a small visitor over one JSON document.

Usage:
    python3 scripts/check_content_references.py [--json]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKS_DIR = ROOT / "content"

# The implicit local speech channel authored as "local" in scripted steps.
LOCAL_CHANNEL_ID = "local"

# Namespaces a reference can point at, with the file they come from.
NAMESPACES = {
    "actors": ("actors.json", "id"),
    "rooms": ("rooms.json", "id"),
    "stages": ("beats.json", "id"),
    "items": ("items.json", "id"),
    "channels": ("settings.json", "id"),
    "actions": ("actions.json", "id"),
    "sequences": ("sequences.json", "id"),
    "objectives": ("beat_objectives.json", "id"),
    "hooks": ("hooks.json", "id"),
    "statactor": ("stats.json", "actor"),
    "statpair": ("stats.json", "pair"),
}

# Root files apply to every locale; locale files are per-locale.
ROOT_FILES = {
    "settings.json", "items.json", "actions.json", "beat_objectives.json",
    "movement.json", "levels.json", "stats.json", "hooks.json", "beats.json",
}
LOCALE_FILES = {
    "actors.json", "rooms.json", "sequences.json", "opening.json", "maps.json",
}


class PackDoc:
    """One pack+locale bundle: all root + locale JSON, plus known-id sets."""

    def __init__(self, pack: Path, locale: Path | None) -> None:
        self.pack_name = pack.name
        self.locale_name = locale.name if locale else "root"
        self.docs: dict[str, dict | list] = {}
        for root_file in ROOT_FILES:
            self._load(pack / root_file)
        if locale is not None:
            for locale_file in LOCALE_FILES:
                self._load(locale / locale_file)
        self.known: dict[str, set[str]] = {}
        self.objective_keys: dict[str, set[str]] = {}
        self._index()

    def _load(self, path: Path) -> None:
        if path.exists():
            try:
                self.docs[path.name] = json.loads(path.read_text())
            except json.JSONDecodeError as exc:
                print(f"warning: [{self.pack_name}] {path.name} is not valid JSON: {exc}")

    def _index(self) -> None:
        for namespace, (filename, key) in NAMESPACES.items():
            doc = self.docs.get(filename)
            if doc is None:
                if filename in ROOT_FILES and namespace in ("hooks", "statactor", "statpair"):
                    # Optional root files default to empty namespaces.
                    self.known[namespace] = set()
                continue
            ids: set[str] = set()
            if isinstance(doc, list):
                for entry in doc:
                    if isinstance(entry, dict) and entry.get("id"):
                        ids.add(entry["id"])
            elif isinstance(doc, dict):
                if namespace == "hooks" and isinstance(doc.get(key), dict) is False:
                    ids = set(doc.keys())
                elif key == "id":
                    for entry in doc.get("sequences") or []:
                        if entry.get("id"):
                            ids.add(entry["id"])
                    for entry in doc.get("objectives") or []:
                        if entry.get("id"):
                            ids.add(entry["id"])
                    for entry in doc.get("actions") or []:
                        if entry.get("id"):
                            ids.add(entry["id"])
                    for entry in doc.get("stages") or []:
                        if entry.get("id"):
                            ids.add(entry["id"])
                    for entry in doc.get("channels") or []:
                        if entry.get("id"):
                            ids.add(entry["id"])
                elif isinstance(doc.get(key), dict):
                    ids = set(doc[key].keys())
            self.known[namespace] = ids

        objectives = self.docs.get("beat_objectives.json")
        if objectives:
            for objective in objectives.get("objectives") or []:
                if not objective.get("id"):
                    continue
                keys = {
                    progress.get("key")
                    for progress in objective.get("progress", {}).get("keys") or []
                }
                self.objective_keys[objective["id"]] = {k for k in keys if k}

    def relative(self, filename: str) -> str:
        if self.locale_name != "root":
            return f"[{self.pack_name}:{self.locale_name}] {filename}".replace(
                f"content/{self.pack_name}/", ""
            )
        return f"[{self.pack_name}] {filename}"


class Linter:
    """Collects advisory warnings; every rule funnels through ``require``."""

    def __init__(self, doc: PackDoc) -> None:
        self.doc = doc
        self.warnings: list[str] = []

    def warn(self, message: str) -> None:
        self.warnings.append(message)

    def require(self, namespace: str, value: str, where: str) -> None:
        """The one generic existence check."""
        if not value or not value.strip():
            return
        value = value.strip()
        known = self.doc.known.get(namespace)
        if known is None:
            return  # namespace not declared for this pack; nothing to resolve
        if value not in known:
            self.warn(
                f"{self.doc.relative('content')}: {where}: "
                f"id '{value}' not found in {namespace}",
            )

    def require_key(self, objective_id: str, key: str, where: str) -> None:
        keys = self.doc.objective_keys.get(objective_id)
        if keys is None:
            self.warn(
                f"{self.doc.relative('content')}: {where}: "
                f"objective_id '{objective_id}' not found in objectives",
            )
        elif key and key not in keys:
            self.warn(
                f"{self.doc.relative('content')}: {where}: "
                f"key '{key}' not found in rule objective '{objective_id}'",
            )


def lint_rooms(lint: Linter, doc: PackDoc) -> None:
    doc_rooms = doc.docs.get("rooms.json")
    if not doc_rooms:
        return
    seen: set[str] = set()
    for room in doc_rooms:
        room_id = room.get("id") or ""
        if not room_id:
            continue
        if room_id in seen:
            lint.warn(f"{doc.relative('rooms.json')}: duplicate room id '{room_id}'")
        seen.add(room_id)
        for exit_ in room.get("exits") or []:
            lint.require("rooms", exit_.get("room_id") or "", f"room '{room_id}' exits[].room_id")


def lint_maps(lint: Linter, doc: PackDoc) -> None:
    doc_maps = doc.docs.get("maps.json")
    if not doc_maps:
        return
    for map_ in doc_maps:
        map_id = map_.get("id") or ""
        for tile in map_.get("rooms") or []:
            lint.require("rooms", tile.get("room_id") or "", f"map '{map_id}' rooms[].room_id")


def lint_actors(lint: Linter, doc: PackDoc) -> None:
    doc_actors = doc.docs.get("actors.json")
    if not doc_actors:
        return
    seen: set[str] = set()
    for actor in doc_actors:
        actor_id = actor.get("id") or ""
        if not actor_id:
            continue
        if actor_id in seen:
            lint.warn(f"{doc.relative('actors.json')}: duplicate actor id '{actor_id}'")
        seen.add(actor_id)
        room_id = actor.get("room_id") or ""
        if room_id:
            lint.require("rooms", room_id, f"actor '{actor_id}' room_id")
        for drop_id, spec in (actor.get("drops") or {}).items():
            if not isinstance(spec, dict):
                continue
            if spec.get("entries") is not None:
                for entry in spec["entries"]:
                    lint.require("items", entry.get("item_id") or "", f"actor '{actor_id}' drops '{drop_id}' entries[].item_id")
            else:
                lint.require("items", drop_id, f"actor '{actor_id}' drops")
                chance = spec.get("chance_percent")
                if isinstance(chance, (int, float)) and chance > 100:
                    lint.warn(f"actor '{actor_id}' drop '{drop_id}' chance_percent {chance} exceeds 100")
        for member in actor.get("act_cast") or []:
            if isinstance(member, dict) and member.get("actor_id"):
                lint.require("actors", member["actor_id"], f"actor '{actor_id}' act_cast actor_id")
        for item_id in (actor.get("initial_inventory") or {}).keys():
            lint.require("items", item_id, f"actor '{actor_id}' initial_inventory")
        for slot, item_id in (actor.get("initial_equipment") or {}).items():
            lint.require("items", item_id, f"actor '{actor_id}' initial_equipment")


def lint_settings(lint: Linter, doc: PackDoc) -> None:
    settings = doc.docs.get("settings.json")
    if not settings:
        return
    seen_channels: set[str] = set()
    for channel in settings.get("channels") or []:
        channel_id = channel.get("id") or ""
        if not channel_id:
            continue
        if channel_id == "local":
            lint.warn(f"{doc.relative('settings.json')}: channel id '{channel_id}' collides with the implicit local speech channel")
        if channel_id in seen_channels:
            lint.warn(f"{doc.relative('settings.json')}: duplicate channel id '{channel_id}'")
        seen_channels.add(channel_id)
        for participant in channel.get("participants") or []:
            lint.require("actors", participant, f"channel '{channel_id}' participants")
        if channel.get("privacy") == "private" and not channel.get("participants"):
            lint.warn(f"channel '{channel_id}' is private but declares no participants")

    feedback = settings.get("feedback_channel_id") or ""
    if feedback:
        lint.require("channels", feedback, "settings.feedback_channel_id")
        channel = next((c for c in settings.get("channels") or [] if c.get("id") == feedback), None)
        if channel is not None:
            if channel.get("kind") != "direct":
                lint.warn(f"feedback_channel_id '{feedback}' must be a direct channel (kind 'direct')")
            if channel.get("privacy") != "private":
                lint.warn(f"feedback_channel_id '{feedback}' must be a private channel")
            player = settings.get("combat", {}).get("player_actor_id") or ""
            if not any(p != player for p in channel.get("participants") or []):
                lint.warn(f"feedback_channel_id '{feedback}' needs a non-player speaker in participants")

    for actor_id in (settings.get("party", {}).get("initial_orders") or {}).keys():
        lint.require("actors", actor_id, "settings.party.initial_orders")
    player = settings.get("combat", {}).get("player_actor_id") or ""
    if player:
        lint.require("actors", player, "settings.combat.player_actor_id")


def lint_movement(lint: Linter, doc: PackDoc) -> None:
    movement = doc.docs.get("movement.json")
    if not movement:
        return
    for actor_id in movement.get("actors") or {}:
        lint.require("actors", actor_id, "movement.actors")
        rules = movement["actors"][actor_id]
        if not isinstance(rules, dict):
            continue
        for index, rule in enumerate(rules.get("target_rules") or []):
            target_room = rule.get("target_room_id") or ""
            if target_room:
                lint.require("rooms", target_room, f"movement actor '{actor_id}' target_rules[{index}].target_room_id")
            if not target_room and not (rule.get("target_from_story_var") or ""):
                lint.warn(f"movement actor '{actor_id}' target_rules[{index}] must set target_room_id or target_from_story_var")
            for stage_id in rule.get("any_active_stage_ids") or []:
                lint.require("stages", stage_id, f"movement actor '{actor_id}' target_rules[{index}].any_active_stage_ids")
    for stage_id in movement.get("stage_locks") or []:
        lint.require("stages", stage_id, "movement.stage_locks")
    for room_id in movement.get("unreachable_rooms") or []:
        lint.require("rooms", room_id, "movement.unreachable_rooms")


def lint_actions(lint: Linter, doc: PackDoc) -> None:
    actions = doc.docs.get("actions.json") or {}
    for action in actions.get("actions") or []:
        action_id = action.get("id") or ""
        available = action.get("available") or {}
        for room_id in available.get("allowed_rooms") or []:
            lint.require("rooms", room_id, f"action '{action_id}' available.allowed_rooms")
        for stage_id in available.get("available_during") or []:
            lint.require("stages", stage_id, f"action '{action_id}' available.available_during")
        for field in ("required_objective_progress", "blocked_by_objective_progress",
                      "sets_objective_progress", "clears_objective_progress"):
            for ref in available.get(field) or []:
                objective_id = ref.get("objective_id") or ""
                key = ref.get("key") or ""
                lint.require_key(objective_id, key, f"action '{action_id}' available.{field}")


def lint_beat_objectives(lint: Linter, doc: PackDoc) -> None:
    objectives = doc.docs.get("beat_objectives.json") or {}
    seen: set[str] = set()
    for objective in objectives.get("objectives") or []:
        objective_id = objective.get("id") or ""
        if not objective_id:
            continue
        if objective_id in seen:
            lint.warn(f"{doc.relative('beat_objectives.json')}: duplicate objective id '{objective_id}'")
        seen.add(objective_id)
        for stage_id in objective.get("stage_ids") or []:
            lint.require("stages", stage_id, f"objective '{objective_id}' stage_ids")
        for priority in objective.get("guidance", {}).get("prioritize") or []:
            lint.require("actions", priority.get("command_id") or "", f"objective '{objective_id}' guidance.prioritize[].command_id")
        for index, conditional in enumerate(objective.get("guidance", {}).get("conditional") or []):
            for priority in conditional.get("prioritize") or []:
                lint.require("actions", priority.get("command_id") or "", f"objective '{objective_id}' guidance.conditional[{index}].prioritize[].command_id")


def lint_levels(lint: Linter, doc: PackDoc) -> None:
    levels = doc.docs.get("levels.json")
    if not levels:
        return
    for actor_id in (levels.get("actors") or {}).keys():
        lint.require("actors", actor_id, "levels.actors")


def lint_items(lint: Linter, doc: PackDoc) -> None:
    for item in doc.docs.get("items.json") or []:
        item_id = item.get("id") or ""
        for hook in ("use_hook", "equip_hook"):
            if item.get(hook):
                lint.require("hooks", item[hook], f"item '{item_id}' {hook}")


def lint_stages(lint: Linter, doc: PackDoc) -> None:
    beats = doc.docs.get("beats.json")
    if not beats:
        return
    seen: set[str] = set()
    for stage in beats.get("stages") or []:
        stage_id = stage.get("id") or ""
        if not stage_id:
            continue
        if stage_id in seen:
            lint.warn(f"{doc.relative('beats.json')}: duplicate stage id '{stage_id}'")
        seen.add(stage_id)
        assignment = stage.get("stage_assignment")
        if isinstance(assignment, dict):
            lint.require("actors", assignment.get("initiator_actor_id") or "", f"beat stage '{stage_id}' stage_assignment.initiator_actor_id")
            lint.require("rooms", assignment.get("selected_room_id") or "", f"beat stage '{stage_id}' stage_assignment.selected_room_id")
            lint.require("rooms", assignment.get("remaining_room_id") or "", f"beat stage '{stage_id}' stage_assignment.remaining_room_id")
        for next_id in stage.get("next_stage_ids") or []:
            lint.require("stages", next_id, f"beat stage '{stage_id}' next_stage_ids")
    for stage_id in beats.get("initial_stage_ids") or []:
        lint.require("stages", stage_id, "beats.initial_stage_ids")


def lint_sequences(lint: Linter, doc: PackDoc) -> None:
    sequences = doc.docs.get("sequences.json")
    if not sequences:
        return
    seen: set[str] = set()
    for sequence in sequences.get("sequences") or []:
        sequence_id = sequence.get("id") or ""
        if sequence_id and sequence_id in seen:
            lint.warn(f"{doc.relative('sequences.json')}: duplicate sequence id '{sequence_id}'")
        if sequence_id:
            seen.add(sequence_id)
        for index, step in enumerate(sequence.get("steps") or []):
            if not isinstance(step, dict):
                continue
            speaker = step.get("speaker_id") or ""
            if speaker and speaker != "player":
                lint.require("actors", speaker, f"sequence '{sequence_id}' steps[{index}].speaker_id")
            recipient = step.get("recipient_id") or ""
            if recipient:
                lint.require("actors", recipient, f"sequence '{sequence_id}' steps[{index}].recipient_id")
            channel_id = step.get("channel_id") or ""
            if channel_id and channel_id != LOCAL_CHANNEL_ID:
                lint.require("channels", channel_id, f"sequence '{sequence_id}' steps[{index}].channel_id")
                channel = next((c for c in doc.docs.get("settings.json", {}).get("channels") or [] if c.get("id") == channel_id), None)
                if channel is not None:
                    for actor_id in (p for p in [speaker, recipient] if p):
                        if actor_id not in channel.get("participants") or []:
                            lint.warn(f"sequence '{sequence_id}' steps[{index}] actor '{actor_id}' is not a participant of channel '{channel_id}'")
        for index, effect in enumerate(sequence.get("completion_effects") or []):
            kind = effect.get("kind") or ""
            if kind == "adjust_actor_stat":
                lint.require("actors", effect.get("actor_id") or "", f"sequence '{sequence_id}' completion_effects[{index}].actor_id")
                lint.require("statactor", effect.get("stat") or "", f"sequence '{sequence_id}' completion_effects[{index}].stat")
            elif kind == "adjust_pair_stat":
                lint.require("actors", effect.get("participant_a_id") or "", f"sequence '{sequence_id}' completion_effects[{index}].participant_a_id")
                lint.require("actors", effect.get("participant_b_id") or "", f"sequence '{sequence_id}' completion_effects[{index}].participant_b_id")
                lint.require("statpair", effect.get("stat") or "", f"sequence '{sequence_id}' completion_effects[{index}].stat")


def lint_opening(lint: Linter, doc: PackDoc) -> None:
    opening = doc.docs.get("opening.json")
    if not opening:
        return
    lint.require("rooms", opening.get("start_room_id") or "", "opening.start_room_id")
    for room_id in opening.get("start_room_ids") or []:
        lint.require("rooms", room_id, "opening.start_room_ids")
    lint.require("sequences", opening.get("opening_sequence_id") or "", "opening.opening_sequence_id")


def lint_pack(pack: Path) -> list[str]:
    locales = sorted(pack.glob("locales/*/")) if (pack / "locales").exists() else []
    if not locales:
        return [lint_bundle(pack, None)]

    all_warnings: list[str] = []
    for locale in locales:
        all_warnings.extend(lint_bundle(pack, locale))
    return all_warnings


def lint_bundle(pack: Path, locale: Path | None) -> list[str]:
    doc = PackDoc(pack, locale)
    linter = Linter(doc)
    lint_rooms(linter, doc)
    lint_maps(linter, doc)
    lint_actors(linter, doc)
    lint_settings(linter, doc)
    lint_movement(linter, doc)
    lint_actions(linter, doc)
    lint_beat_objectives(linter, doc)
    lint_levels(linter, doc)
    lint_items(linter, doc)
    lint_stages(linter, doc)
    lint_sequences(linter, doc)
    lint_opening(linter, doc)
    return linter.warnings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--json", action="store_true", help="emit machine-readable JSON",
    )
    args = parser.parse_args()

    packs = sorted(p for p in PACKS_DIR.iterdir() if p.is_dir())
    warnings: list[dict] = []
    for pack in packs:
        for message in lint_pack(pack):
            warnings.append({"pack": pack.name, "message": message})

    if args.json:
        print(json.dumps({"content_reference_warnings": warnings}, indent=2))
        return 0

    for warning in warnings:
        print(f"warning: {warning['message']}")
    print(
        f"\nContent reference warnings: {len(warnings)}. "
        "Warning only, non-blocking.",
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())