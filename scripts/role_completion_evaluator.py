#!/usr/bin/env python3

import json
import sys


def approve():
    return {"decision": {"kind": "approve"}}


def reject(message: str):
    return {
        "decision": {
            "kind": "reject",
            "error": {
                "code": "invalid_completion",
                "message": message,
            },
        }
    }


def strip_annotation(label: str) -> str:
    for separator in (" — ", " - "):
        if separator in label:
            return label.split(separator, 1)[0].rstrip()
    return label


def decision_commands_from_prompt(instruction: str) -> list[str]:
    marker = "\nDecision\n"
    if marker in instruction:
        decision_block = instruction.split(marker, 1)[1]
    elif instruction.startswith("Decision\n"):
        decision_block = instruction[len("Decision\n") :]
    else:
        return []
    commands: list[str] = []
    for raw_line in decision_block.splitlines():
        line = raw_line.strip()
        if line.startswith("- "):
            commands.append(line[2:].strip())
    return commands


def is_open_text_act_command(label: str) -> bool:
    normalized = label.strip()
    if not normalized:
        return False
    verb, _, remainder = normalized.partition(" ")
    return verb.upper() == "ACT" and bool(remainder.strip())


def is_valid_decision_command(
    text: str,
    allowed_commands: list[str],
    allow_annotated: bool,
) -> bool:
    candidate = text.strip()
    if not candidate:
        return False

    normalized = strip_annotation(candidate) if allow_annotated else candidate
    normalized_allowed_commands = [
        strip_annotation(command) if allow_annotated else command
        for command in allowed_commands
    ]
    if normalized in normalized_allowed_commands:
        return True

    normalized_upper = normalized.upper()
    if any(is_open_text_act_command(command) for command in normalized_allowed_commands):
        if normalized_upper.startswith("ACT"):
            remainder = normalized[3:].lstrip(" :-—").strip()
            return bool(remainder)

    return False


def decision_command_rejection_message(
    text: str,
    allowed_commands: list[str],
    allow_annotated: bool,
) -> str:
    candidate = text.strip()
    normalized = strip_annotation(candidate) if allow_annotated else candidate
    normalized_allowed_commands = [
        strip_annotation(command) if allow_annotated else command
        for command in allowed_commands
    ]
    for allowed_command in normalized_allowed_commands:
        if is_open_text_act_command(allowed_command):
            continue
        if normalized.startswith(allowed_command):
            remainder = normalized[len(allowed_command) :].lstrip(" :-—").strip()
            if remainder:
                return (
                    f"You returned '{candidate}'. Return only '{allowed_command}' with no extra "
                    "words, target suffix, or explanation."
                )

    candidate_verb = normalized.split(" ", 1)[0].upper() if normalized else ""
    matching_verbs = [
        command
        for command in normalized_allowed_commands
        if command.split(" ", 1)[0].upper() == candidate_verb
        and not is_open_text_act_command(command)
    ]
    if matching_verbs:
        return (
            f"You returned '{candidate}'. Use one exact command from the Decision section, "
            "including any required target or room id. "
            f"Allowed commands: {', '.join(allowed_commands)}."
        )

    return (
        f"You returned '{candidate}'. Return exactly one command from the Decision section with "
        f"no explanation. Allowed commands: {', '.join(allowed_commands)}."
    )


def validate_enum(text: str, config: dict) -> dict:
    allowed_values = [str(value).strip().upper() for value in config.get("values", [])]
    if text.strip().upper() in allowed_values:
        return approve()
    return reject(f"Return exactly one of: {', '.join(allowed_values)}.")


def validate_decision_commands(text: str, config: dict, instruction: str) -> dict:
    allowed_commands = decision_commands_from_prompt(instruction)
    if not allowed_commands:
        return reject("Decision section did not contain any command list to validate against.")
    allow_annotated = bool(config.get("allow_annotated", False))
    if is_valid_decision_command(
        text,
        allowed_commands,
        allow_annotated,
    ):
        return approve()
    return reject(decision_command_rejection_message(text, allowed_commands, allow_annotated))


def main() -> int:
    payload = json.load(sys.stdin)
    step = payload.get("step", {})
    if step.get("kind") != "complete":
        print(json.dumps(approve()))
        return 0

    config = payload.get("evaluator_rule_config") or {}
    mode = config.get("mode")
    text = str(step.get("text", "") or step.get("completion", {}).get("comment", ""))
    instruction = str(payload.get("request", {}).get("instruction", ""))

    if mode == "enum":
        result = validate_enum(text, config)
    elif mode == "decision_commands":
        result = validate_decision_commands(text, config, instruction)
    else:
        result = reject(f"Unsupported evaluator mode '{mode}'.")

    print(json.dumps(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
