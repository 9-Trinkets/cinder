use super::super::{
    OverflowAction, PanelConfigData, PanelOptionData, PanelOptionGroups, UiContext,
    append_stage_menu_overflow_actions,
};
use super::panel_config_data;
use cinder_core::content::types::{
    ActionDefinition, ContentPack, PanelDataSource, PanelSelectAction, UiTextDefinition,
};
use cinder_core::engine::state::WorldState;
use cinder_core::engine::turn_policies::action_is_available;

/// Display title for an overflow action button: the authored `label` (e.g.
/// "Read Worn Scroll").
pub(crate) fn overflow_action_title(action: &ActionDefinition) -> String {
    action.label.clone()
}

/// Actions owned by dedicated modal panels and never surfaced as overflow buttons.
const MODAL_COVERED: &[&str] = &["inspect_feature", "inspect_actor"];

/// The item-panel actions built generically rather than authored (see
/// [`GENERIC_ITEM_ACTIONS`]).
fn is_items_group(id: &str) -> bool {
    matches!(id, "take" | "use" | "drop" | "give")
}

/// Where a generic item action is spliced into the authored overflow list.
#[derive(Clone, Copy)]
enum InsertAnchor {
    /// Append after every authored action.
    End,
    /// Insert immediately before the first action whose id is listed; append if
    /// none is present.
    Before(&'static [&'static str]),
}

/// A display string for a generic item action: a localized `UiTextDefinition`
/// field, or a fixed literal.
#[derive(Clone, Copy)]
enum TextSource {
    TakeLabel,
    UseLabel,
    DropLabel,
    Literal(&'static str),
}

impl TextSource {
    fn resolve(self, ui_text: &UiTextDefinition) -> String {
        match self {
            TextSource::TakeLabel => ui_text.take_label.clone(),
            TextSource::UseLabel => ui_text.use_label.clone(),
            TextSource::DropLabel => ui_text.drop_label.clone(),
            TextSource::Literal(text) => text.to_string(),
        }
    }
}

/// Declarative spec for one generic item action. They all produce the same
/// `OverflowAction` shape; only the labels, panel wiring, and where they land in
/// the overflow list differ, so a table replaces the hand-written blocks.
struct GenericItemAction {
    id: &'static str,
    label: TextSource,
    usage: &'static str,
    panel: &'static str,
    panel_title: TextSource,
    prompt: TextSource,
    group: &'static str,
    data_source: PanelDataSource,
    anchor: InsertAnchor,
}

/// Ordered table of the generic item actions. Order is meaningful: it fixes the
/// relative position of the plain `End` entries (`drop`, then `equipment`).
const GENERIC_ITEM_ACTIONS: &[GenericItemAction] = &[
    GenericItemAction {
        id: "take",
        label: TextSource::TakeLabel,
        usage: "take <item>",
        panel: "take",
        panel_title: TextSource::TakeLabel,
        prompt: TextSource::Literal(""),
        group: "items",
        data_source: PanelDataSource::LooseRoomItems,
        anchor: InsertAnchor::Before(&["give", "drop"]),
    },
    GenericItemAction {
        id: "use",
        label: TextSource::UseLabel,
        usage: "use <item>",
        panel: "use",
        panel_title: TextSource::UseLabel,
        prompt: TextSource::Literal(""),
        group: "items",
        data_source: PanelDataSource::InventoryItems,
        anchor: InsertAnchor::Before(&["give", "drop"]),
    },
    GenericItemAction {
        id: "give",
        label: TextSource::Literal("Give"),
        usage: "give <item> to <companion>",
        panel: "give",
        panel_title: TextSource::Literal("Give to Companion"),
        prompt: TextSource::Literal("Choose an item to give"),
        group: "items",
        data_source: PanelDataSource::InventoryItems,
        anchor: InsertAnchor::Before(&["drop"]),
    },
    GenericItemAction {
        id: "drop",
        label: TextSource::DropLabel,
        usage: "drop <item>",
        panel: "drop",
        panel_title: TextSource::DropLabel,
        prompt: TextSource::Literal(""),
        group: "items",
        data_source: PanelDataSource::InventoryItems,
        anchor: InsertAnchor::End,
    },
    GenericItemAction {
        id: "equipment",
        label: TextSource::Literal("Equipment"),
        usage: "equip <item> / unequip <item>",
        panel: "equipment",
        panel_title: TextSource::Literal("Equipment"),
        prompt: TextSource::Literal("Choose an item to equip or stow."),
        group: "",
        data_source: PanelDataSource::InventoryItems,
        anchor: InsertAnchor::End,
    },
];

/// The panel options that gate whether a generic item action is worth surfacing.
fn generic_panel_options<'a>(id: &str, panels: &'a PanelOptionGroups) -> &'a [PanelOptionData] {
    match id {
        "take" => &panels.take,
        "use" => &panels.use_item,
        "give" => &panels.give,
        "drop" => &panels.drop,
        "equipment" => &panels.equipment,
        _ => &[],
    }
}

/// Whether an authored action belongs as an overflow button.
fn surfaces_in_overflow(
    action: &ActionDefinition,
    bar_ids: &[&str],
    has_talk: bool,
    content: &ContentPack,
    state: &WorldState,
    current_room_id: &str,
    panels: &PanelOptionGroups,
) -> bool {
    if !action.player_enabled || bar_ids.contains(&action.id.as_str()) {
        return false;
    }
    if MODAL_COVERED.contains(&action.id.as_str()) {
        return false;
    }
    if (action.id == "speak" || action.id == "talk") && has_talk {
        return false;
    }
    // Sigil erasing is consolidated into the Trace panel and never surfaces in
    // overflow.
    if action.id == "erase" {
        return false;
    }
    if is_items_group(&action.id) && generic_panel_options(&action.id, panels).is_empty() {
        return false;
    }
    action_is_available(content, state, action, current_room_id)
}

/// Surfaces the generic item actions (take/use/give/drop/equipment) into the
/// overflow list, each only when its panel has options and it is not already
/// present as an authored action.
fn insert_generic_item_actions(
    overflow_actions: &mut Vec<OverflowAction>,
    content: &ContentPack,
    panels: &PanelOptionGroups,
) {
    for spec in GENERIC_ITEM_ACTIONS {
        if generic_panel_options(spec.id, panels).is_empty() {
            continue;
        }
        if overflow_actions.iter().any(|action| action.id == spec.id) {
            continue;
        }
        let action = OverflowAction {
            id: spec.id.to_string(),
            label: spec.label.resolve(&content.ui_text),
            group: spec.group.to_string(),
            usage: spec.usage.to_string(),
            panel: spec.panel.to_string(),
            panel_config: Some(PanelConfigData {
                title: spec.panel_title.resolve(&content.ui_text),
                prompt: spec.prompt.resolve(&content.ui_text),
                data_source: spec.data_source.clone(),
                on_select: PanelSelectAction::ExecuteCommand,
            }),
        };
        match spec.anchor {
            InsertAnchor::End => overflow_actions.push(action),
            InsertAnchor::Before(ids) => {
                match overflow_actions
                    .iter()
                    .position(|existing| ids.contains(&existing.id.as_str()))
                {
                    Some(position) => overflow_actions.insert(position, action),
                    None => overflow_actions.push(action),
                }
            }
        }
    }
}

pub(crate) fn build_overflow_actions(
    ctx: &UiContext,
    bar_ids: &[&str],
    panels: &PanelOptionGroups,
) -> Result<Vec<OverflowAction>, String> {
    let (runtime, content, state) = (ctx.runtime, ctx.content, ctx.state);
    let has_talk = bar_ids.contains(&"speak") || bar_ids.contains(&"talk");
    let current_room_id = if !state.current_room_id.is_empty() {
        state.current_room_id.clone()
    } else {
        runtime.current_room_id().unwrap_or_default()
    };
    let mut overflow_actions: Vec<OverflowAction> = content
        .actions
        .iter()
        .filter(|action| {
            surfaces_in_overflow(
                action,
                bar_ids,
                has_talk,
                content,
                state,
                &current_room_id,
                panels,
            )
        })
        .map(|action| {
            let group = if is_items_group(&action.id) && action.ui.group.is_empty() {
                "items".to_string()
            } else {
                action.ui.group.clone()
            };
            OverflowAction {
                id: action.id.clone(),
                label: overflow_action_title(action),
                group,
                usage: action
                    .player_command
                    .as_ref()
                    .map(|player_command| player_command.usage.clone())
                    .unwrap_or_default(),
                panel: action.ui.panel.clone().unwrap_or_default(),
                panel_config: action.ui.panel_config.as_ref().map(panel_config_data),
            }
        })
        .collect();

    if let Ok(active_stages) = runtime.active_stage_ids() {
        append_stage_menu_overflow_actions(&mut overflow_actions, content, &active_stages);
    }

    insert_generic_item_actions(&mut overflow_actions, content, panels);

    Ok(overflow_actions)
}
