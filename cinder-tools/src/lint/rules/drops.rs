use crate::lint::rule::{LintRule, RuleContext};
use cinder_core::content::types::ContentPack;

pub struct DropsRule;

impl LintRule for DropsRule {
    fn check(
        &self,
        _pack: &ContentPack,
        _ctx: &RuleContext,
        _warnings: &mut Vec<String>,
        _errors: &mut Vec<String>,
        _pack_name: &str,
    ) {
        // Drops are modeled as part of actor inventories/maps; skip standalone drops check for now.
    }
}
