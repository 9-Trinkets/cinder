use crate::lint::rule::{LintRule, RuleContext};
use cinder_core::content::types::ContentPack;

pub struct MapsRule;

impl LintRule for MapsRule {
    fn check(
        &self,
        pack: &ContentPack,
        ctx: &RuleContext,
        warnings: &mut Vec<String>,
        _errors: &mut Vec<String>,
        pack_name: &str,
    ) {
        for map in &pack.maps {
            for map_room in &map.rooms {
                if !ctx.room_ids.contains(map_room.room_id.as_str()) {
                    warnings.push(format!(
                        "[{pack_name}] map '{}' references unknown room '{}'",
                        map.id, map_room.room_id
                    ));
                }
            }
        }
    }
}
