use super::PlanningContext;
use crate::content::types::{ActionDefinition, ContentPack, ItemStorageTarget};
use crate::engine::events::WorldEvent;
use crate::engine::turn_runner::PlannedTurn;

pub(super) fn plan_erase_command(
    content: &ContentPack,
    _action: &ActionDefinition,
    input: Option<&str>,
    context: &PlanningContext<'_>,
    planned: &mut PlannedTurn,
) -> bool {
    let state = context.planner_state;
    let room_items = state.loose_room_items(context.current_room_id);
    let trace_items: Vec<String> = room_items
        .into_iter()
        .filter(|(item_id, count)| {
            *count > 0 && content.item(item_id).is_some_and(|item| item.trace_mark)
        })
        .map(|(item_id, _)| item_id)
        .collect();

    if trace_items.is_empty() {
        planned.events.push(WorldEvent::ActionRejected {
            message: content
                .render_message("error.erase_nothing", &[])
                .unwrap_or_else(|| "There are no chalk sigils to erase here.".to_string()),
        });
        return false;
    }

    let raw_target = input.map(str::trim).filter(|s| !s.is_empty());
    let to_erase: Vec<String> = if let Some(target) = raw_target {
        let target_lower = target.to_ascii_lowercase();
        let matched: Vec<String> = trace_items
            .into_iter()
            .filter(|id| {
                let label = content.item_label(id).to_ascii_lowercase();
                id.to_ascii_lowercase().contains(&target_lower)
                    || label.contains(&target_lower)
                    || target_lower.contains(&label)
            })
            .collect();
        if matched.is_empty() {
            planned.events.push(WorldEvent::ActionRejected {
                message: content
                    .render_message("error.erase_no_match", &[("target", target)])
                    .unwrap_or_else(|| {
                        format!("There is no sigil matching '{target}' to erase here.")
                    }),
            });
            return false;
        }
        matched
    } else {
        trace_items
    };

    for item_id in &to_erase {
        if item_id == "teleport-sigil" {
            planned.events.push(WorldEvent::ChalkAnchorConsumed {
                room_id: context.current_room_id.to_string(),
            });
        }
        planned.events.push(WorldEvent::ItemConsumed {
            item_id: item_id.clone(),
            storage: ItemStorageTarget::CurrentRoom,
            consumer_id: None,
            consumer_name: None,
        });
    }

    let narrative = if to_erase.len() == 1 {
        let label = content.item_label(&to_erase[0]);
        content
            .render_message("item.erase_one", &[("item", label)])
            .unwrap_or_else(|| format!("You rub out the {label} from the stone floor."))
    } else {
        let labels = content.join_list(
            &to_erase
                .iter()
                .map(|id| content.item_label(id))
                .collect::<Vec<_>>(),
        );
        content
            .render_message("item.erase_many", &[("labels", labels.as_str())])
            .unwrap_or_else(|| {
                format!("You rub out the chalk sigils from the stone floor: {labels}.")
            })
    };

    planned
        .events
        .push(WorldEvent::NarrativeLine { text: narrative });

    true
}
