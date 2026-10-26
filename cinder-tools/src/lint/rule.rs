use cinder_core::content::types::ContentPack;
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub struct RuleContext<'a> {
    pub room_ids: BTreeSet<&'a str>,
    pub item_ids: BTreeSet<&'a str>,
    pub hook_names: BTreeSet<&'a str>,
}

pub trait LintRule {
    fn name(&self) -> &'static str;
    fn check(
        &self,
        pack: &ContentPack,
        ctx: &RuleContext,
        warnings: &mut Vec<String>,
        errors: &mut Vec<String>,
        pack_name: &str,
    );
}
