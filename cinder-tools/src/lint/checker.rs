use crate::lint::rule::{LintRule, RuleContext};
use crate::lint::rules::{ActorsRule, DropsRule, HooksRule, ItemsRule, MapsRule, RoomsRule};
use cinder_core::content::loader::load_pack_from_dir_with_locale;
use std::path::Path;

#[derive(Debug, Default)]
pub struct LintReport {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub fn lint_pack(pack_dir: &Path, locale: &str) -> LintReport {
    let mut report = LintReport::default();
    let pack_name = pack_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let pack = match load_pack_from_dir_with_locale(pack_dir, Some(locale)) {
        Ok(p) => p,
        Err(err) => {
            report
                .errors
                .push(format!("[{pack_name}] pack failed to load: {err}"));
            return report;
        }
    };

    let ctx = RuleContext {
        room_ids: pack.rooms.iter().map(|r| r.id.as_str()).collect(),
        item_ids: pack.items.iter().map(|i| i.id.as_str()).collect(),
        hook_names: pack.hooks.keys().map(|k| k.as_str()).collect(),
    };

    let rules: Vec<Box<dyn LintRule>> = vec![
        Box::new(RoomsRule),
        Box::new(MapsRule),
        Box::new(ActorsRule),
        Box::new(ItemsRule),
        Box::new(DropsRule),
        Box::new(HooksRule),
    ];

    for rule in &rules {
        rule.check(
            &pack,
            &ctx,
            &mut report.warnings,
            &mut report.errors,
            pack_name,
        );
    }

    report
}
