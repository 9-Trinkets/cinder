use crate::lint::rule::{LintRule, RuleContext};
use cinder_core::content::types::ContentPack;

pub struct ItemsRule;

impl LintRule for ItemsRule {
    fn check(
        &self,
        pack: &ContentPack,
        ctx: &RuleContext,
        warnings: &mut Vec<String>,
        _errors: &mut Vec<String>,
        pack_name: &str,
    ) {
        for item in &pack.items {
            if !item.use_hook.is_empty() && !ctx.hook_names.contains(item.use_hook.as_str()) {
                warnings.push(format!(
                    "[{pack_name}] item '{}' references unknown use_hook '{}'",
                    item.id, item.use_hook
                ));
            }
            if !item.equip_hook.is_empty() && !ctx.hook_names.contains(item.equip_hook.as_str()) {
                warnings.push(format!(
                    "[{pack_name}] item '{}' references unknown equip_hook '{}'",
                    item.id, item.equip_hook
                ));
            }
            if !item.placement_hook.is_empty()
                && !ctx.hook_names.contains(item.placement_hook.as_str())
            {
                warnings.push(format!(
                    "[{pack_name}] item '{}' references unknown placement_hook '{}'",
                    item.id, item.placement_hook
                ));
            }
        }
    }
}
