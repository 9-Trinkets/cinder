use crate::content::types::{ActCastMember, ActorDefinition};
use std::collections::HashMap;

pub fn build_index<T, F>(items: &[T], id: F) -> HashMap<String, usize>
where
    F: Fn(&T) -> &str,
{
    items
        .iter()
        .enumerate()
        .map(|(index, item)| (id(item).to_string(), index))
        .collect()
}

pub fn collect_act_cast(actors: &[ActorDefinition]) -> Vec<ActCastMember> {
    actors
        .iter()
        .filter_map(|actor| {
            let act_cast = actor.act_cast.as_ref()?;
            Some(ActCastMember {
                id: actor.id.clone(),
                name: actor.name.clone(),
                actor_id: actor.id.clone(),
                inspect_blurb: act_cast.inspect_blurb.clone(),
                intro_blurb: act_cast.intro_blurb.clone(),
                return_blurb: act_cast.return_blurb.clone(),
                metadata: act_cast.metadata.clone(),
                actor_stats: actor.initial_stats.clone(),
            })
        })
        .collect()
}
