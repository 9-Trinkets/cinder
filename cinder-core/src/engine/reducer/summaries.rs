use crate::content::types::ContentPack;
use std::collections::BTreeMap;

pub(super) fn summarize_actor_names(
    content: &ContentPack,
    actor_names: &[String],
) -> Option<String> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for name in actor_names {
        *counts.entry(name.as_str()).or_default() += 1;
    }
    let labels = counts
        .into_iter()
        .map(|(name, count)| {
            if count == 1 {
                name.to_string()
            } else {
                format!("{name} (x{count})")
            }
        })
        .collect::<Vec<_>>();
    content.join_list_natural(labels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_fixtures::minimal_test_pack;

    #[test]
    fn actor_summary_collapses_duplicate_names() {
        assert_eq!(
            summarize_actor_names(
                &minimal_test_pack(),
                &[
                    "dark golem".to_string(),
                    "pale golem".to_string(),
                    "dark golem".to_string(),
                ],
            ),
            Some("dark golem (x2) and pale golem".to_string())
        );
    }
}
