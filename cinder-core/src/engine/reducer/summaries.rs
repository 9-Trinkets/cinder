use std::collections::BTreeMap;

pub(super) fn summarize_actor_names(actor_names: &[String]) -> Option<String> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for name in actor_names {
        *counts.entry(name.as_str()).or_default() += 1;
    }
    let mut labels = counts
        .into_iter()
        .map(|(name, count)| {
            if count == 1 {
                name.to_string()
            } else {
                format!("{name} (x{count})")
            }
        })
        .collect::<Vec<_>>();
    match labels.len() {
        0 => None,
        1 => labels.pop(),
        2 => Some(format!("{} and {}", labels[0], labels[1])),
        _ => {
            let last = labels.pop().expect("non-empty actor summary");
            Some(format!("{}, and {last}", labels.join(", ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_summary_collapses_duplicate_names() {
        assert_eq!(
            summarize_actor_names(&[
                "dark golem".to_string(),
                "pale golem".to_string(),
                "dark golem".to_string(),
            ]),
            Some("dark golem (x2) and pale golem".to_string())
        );
    }
}
