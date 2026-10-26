use crate::lint::rule::{LintRule, RuleContext};
use cinder_core::content::types::{ContentPack, DropSpec};

pub struct ActorsRule;

impl LintRule for ActorsRule {
    fn check(
        &self,
        pack: &ContentPack,
        ctx: &RuleContext,
        warnings: &mut Vec<String>,
        _errors: &mut Vec<String>,
        pack_name: &str,
    ) {
        for actor in &pack.actors {
            if !actor.room_id.is_empty() && !ctx.room_ids.contains(actor.room_id.as_str()) {
                warnings.push(format!(
                    "[{pack_name}] actor '{}' starts in unknown room '{}'",
                    actor.id, actor.room_id
                ));
            }

            for specs in [&actor.initial_inventory, &actor.drops] {
                for (item_id, drop_spec) in specs {
                    match drop_spec {
                        DropSpec::Always(_) => {
                            if !ctx.item_ids.contains(item_id.as_str()) {
                                warnings.push(format!(
                                    "[{pack_name}] actor '{}' inventory/drop references unknown item '{}'",
                                    actor.id, item_id
                                ));
                            }
                        }
                        DropSpec::Chance(_) => {
                            if !ctx.item_ids.contains(item_id.as_str()) {
                                warnings.push(format!(
                                    "[{pack_name}] actor '{}' inventory/drop references unknown item '{}'",
                                    actor.id, item_id
                                ));
                            }
                        }
                        DropSpec::Conditional(_) => {
                            if !ctx.item_ids.contains(item_id.as_str()) {
                                warnings.push(format!(
                                    "[{pack_name}] actor '{}' inventory/drop references unknown item '{}'",
                                    actor.id, item_id
                                ));
                            }
                        }
                        DropSpec::Weighted(pool) => {
                            for entry in &pool.entries {
                                if !ctx.item_ids.contains(entry.item_id.as_str()) {
                                    warnings.push(format!(
                                        "[{pack_name}] actor '{}' inventory/drop references unknown item '{}'",
                                        actor.id, entry.item_id
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
