use crate::lint::rule::{LintRule, RuleContext};
use cinder_core::content::types::ContentPack;

pub struct HooksRule;

impl LintRule for HooksRule {
    fn check(
        &self,
        pack: &ContentPack,
        _ctx: &RuleContext,
        warnings: &mut Vec<String>,
        _errors: &mut Vec<String>,
        pack_name: &str,
    ) {
        for (hook_name, hook) in &pack.hooks {
            if let Some(obj) = hook.as_object() {
                if let Some(conditions) = obj.get("conditions").and_then(|v| v.as_array()) {
                    for condition in conditions {
                        if let Some(cond_obj) = condition.as_object() {
                            if let Some(room_id) = cond_obj.get("room_id").and_then(|v| v.as_str())
                                && !pack.rooms.iter().any(|r| r.id == room_id)
                            {
                                warnings.push(format!(
                                    "[{pack_name}] hook '{}' condition references unknown room '{}'",
                                    hook_name, room_id
                                ));
                            }
                            if let Some(item_id) = cond_obj.get("item_id").and_then(|v| v.as_str())
                                && !pack.items.iter().any(|i| i.id == item_id)
                            {
                                warnings.push(format!(
                                    "[{pack_name}] hook '{}' condition references unknown item '{}'",
                                    hook_name, item_id
                                ));
                            }
                        }
                    }
                }
                if let Some(next_hook) = obj.get("next_hook").and_then(|v| v.as_str())
                    && !pack.hooks.contains_key(next_hook)
                {
                    warnings.push(format!(
                        "[{pack_name}] hook '{}' references unknown next_hook '{}'",
                        hook_name, next_hook
                    ));
                }
            }
        }
    }
}
