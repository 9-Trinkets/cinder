use crate::lint::rule::{LintRule, RuleContext};
use cinder_core::content::types::ContentPack;

pub struct RoomsRule;

impl LintRule for RoomsRule {
    fn name(&self) -> &'static str {
        "rooms"
    }

    fn check(
        &self,
        pack: &ContentPack,
        ctx: &RuleContext,
        warnings: &mut Vec<String>,
        _errors: &mut Vec<String>,
        pack_name: &str,
    ) {
        for room in &pack.rooms {
            for exit in &room.exits {
                if !ctx.room_ids.contains(exit.room_id.as_str()) {
                    warnings.push(format!(
                        "[{pack_name}] room '{}' has exit to unknown room '{}'",
                        room.id, exit.room_id
                    ));
                }

                let opp_label = match exit.label.as_str() {
                    "North" => Some("South"),
                    "South" => Some("North"),
                    "East" => Some("West"),
                    "West" => Some("East"),
                    _ => None,
                };
                if let Some(back_label) = opp_label
                    && let Some(target_room) = pack.room(&exit.room_id)
                {
                    let has_back = target_room
                        .exits
                        .iter()
                        .any(|e| e.room_id == room.id && e.label == back_label);
                    if !has_back && exit.when.is_empty() {
                        warnings.push(format!(
                            "[{pack_name}] one-way cardinal exit: '{}' -> '{}' ({}) lacks reciprocal '{}' in target",
                            room.id, exit.room_id, exit.label, back_label
                        ));
                    }
                }
            }

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
    }
}
