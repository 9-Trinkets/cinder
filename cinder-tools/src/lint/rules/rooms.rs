use crate::lint::rule::{LintRule, RuleContext};
use cinder_core::content::types::{ContentPack, RoomDefinition, RoomExitDefinition};

pub struct RoomsRule;

impl LintRule for RoomsRule {
    fn check(
        &self,
        pack: &ContentPack,
        ctx: &RuleContext,
        warnings: &mut Vec<String>,
        _errors: &mut Vec<String>,
        pack_name: &str,
    ) {
        for room in &pack.rooms {
            check_exits(pack, ctx, room, warnings, pack_name);
            check_features(room, warnings, pack_name);
        }
    }
}

fn check_exits(
    pack: &ContentPack,
    ctx: &RuleContext,
    room: &RoomDefinition,
    warnings: &mut Vec<String>,
    pack_name: &str,
) {
    for exit in &room.exits {
        let target_known = ctx.room_ids.contains(exit.room_id.as_str());
        if !target_known {
            warnings.push(format!(
                "[{pack_name}] room '{}' has exit to unknown room '{}'",
                room.id, exit.room_id
            ));
            continue;
        }
        if let Some(back_label) = opposite_label(&exit.label) {
            check_reciprocal(pack, room, exit, back_label, warnings, pack_name);
        }
    }
}

fn check_reciprocal(
    pack: &ContentPack,
    room: &RoomDefinition,
    exit: &RoomExitDefinition,
    back_label: &str,
    warnings: &mut Vec<String>,
    pack_name: &str,
) {
    if !exit.when.is_empty() {
        return;
    }
    let Some(target_room) = pack.room(&exit.room_id) else {
        return;
    };
    let has_back = target_room
        .exits
        .iter()
        .any(|e| e.room_id == room.id && e.label == back_label);
    if !has_back {
        warnings.push(format!(
            "[{pack_name}] one-way cardinal exit: '{}' -> '{}' ({}) lacks reciprocal '{}' in target",
            room.id, exit.room_id, exit.label, back_label
        ));
    }
}

fn check_features(room: &RoomDefinition, warnings: &mut Vec<String>, pack_name: &str) {
    for feature in &room.features {
        if feature.label.trim().is_empty() {
            warnings.push(format!(
                "[{pack_name}] room '{}' feature '{}' has empty label",
                room.id, feature.id
            ));
        }
        if feature.inspect_text.trim().is_empty() {
            warnings.push(format!(
                "[{pack_name}] room '{}' feature '{}' has empty inspect text",
                room.id, feature.id
            ));
        }
    }
}

fn opposite_label(label: &str) -> Option<&'static str> {
    match label {
        "North" => Some("South"),
        "South" => Some("North"),
        "East" => Some("West"),
        "West" => Some("East"),
        _ => None,
    }
}
