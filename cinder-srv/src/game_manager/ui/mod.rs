mod minimap;
mod panels;
mod room;
mod sidebar;
pub mod types;

pub use types::*;

use cinder_core::content::loader;
use cinder_core::content::types::{ContentPack, PanelDataSource};
use cinder_core::engine::runtime::{CinderRuntime, PanelOption};
use cinder_core::engine::state::{GamePhase, WorldState};

use super::response;

use self::minimap::build_minimap;
pub(crate) use self::panels::shared::msg;
use self::panels::{
    build_action_bar_items, build_active_menu, build_drop_panel_options,
    build_equipment_panel_options, build_interactable_labels, build_look_options,
    build_overflow_actions, build_panel_options, build_talk_options, build_use_panel_options,
    menu_action,
};
use self::room::{build_room_consumables, crafted_consumable_labels};
use self::sidebar::{
    build_current_room_items, build_equipped_items, build_inventory, build_party_members,
    build_player_skills, build_player_status,
};

/// The borrowed runtime, content pack, and exported state shared by the panel
/// builders, so each builder does not re-thread the same three references.
pub(crate) struct UiContext<'a> {
    pub runtime: &'a CinderRuntime,
    pub content: &'a ContentPack,
    pub state: &'a WorldState,
}

pub(super) fn build_ui_snapshot(
    runtime: &CinderRuntime,
    pack_id: &str,
    transcript_lines: &[String],
) -> Result<UiSnapshot, String> {
    let time_label = runtime
        .current_time_label()
        .map_err(|error| error.to_string())?;
    let day_number = runtime
        .current_day_number()
        .map_err(|error| error.to_string())?;
    let objectives: Vec<ObjectiveItem> = runtime
        .current_objective_summaries()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|item| ObjectiveItem {
            stage_id: item.stage_id,
            summary: item.summary,
            message: item.message,
            quest_id: item.quest_id,
            quest_title: item.quest_title,
            quest_kind: item.quest_kind,
        })
        .collect();
    let quests = runtime
        .quest_summaries()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|quest| QuestItem {
            quest_id: quest.quest_id,
            title: quest.title,
            kind: quest.kind,
            status: quest.status,
            summary: quest.summary,
            message: quest.message,
        })
        .collect();
    let (secrets_found, secrets_total) = runtime
        .current_secret_progress()
        .map_err(|error| error.to_string())?;
    let objective_message = objectives
        .iter()
        .find(|o| o.quest_kind.as_deref() == Some("main"))
        .or_else(|| objectives.first())
        .map(|objective| {
            if !objective.message.is_empty() {
                objective.message.clone()
            } else {
                objective.summary.clone()
            }
        })
        .unwrap_or_default();
    let locales = loader::available_locales(&loader::pack_dir(pack_id))
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|locale| LocaleItem {
            code: locale.code,
            label: locale.label,
        })
        .collect();
    let content = runtime.content();

    let current_room_id = runtime
        .current_room_id()
        .map_err(|error| error.to_string())?;
    let current_room_name = content
        .room(&current_room_id)
        .map(|room| room.title.clone())
        .unwrap_or(current_room_id.clone());
    let followed_actor_name = runtime
        .followed_actor_id()
        .map_err(|error| error.to_string())?
        .and_then(|id| runtime.actor_display_name(&id).ok().flatten());

    let state = runtime.export_state().map_err(|e| e.to_string())?;
    let party = build_party_members(runtime, &state, content);

    let (mut action_bar_actions, take_panel_options, give_panel_options) =
        build_action_bar_items(content, &state, &party);
    let panels = PanelOptionGroups {
        take: take_panel_options,
        use_item: build_use_panel_options(content, &state),
        give: give_panel_options,
        drop: build_drop_panel_options(content, &state),
        equipment: build_equipment_panel_options(content, &state),
    };
    let look_options = build_look_options(runtime)?;
    let talk_options = build_talk_options(runtime)?;
    let active_menu = build_active_menu(runtime)?;
    let exit_labels = runtime
        .current_room_exit_labels()
        .map_err(|e| e.to_string())?;
    let inventory_labels = state
        .inventory
        .player_inventory
        .keys()
        .filter_map(|item_id| content.item(item_id).map(|item| item.label.clone()))
        .collect::<Vec<_>>();
    let interactable_labels =
        build_interactable_labels(&look_options, &exit_labels, &inventory_labels);

    // An open menu is answered from the action bar like any other action.
    let menu_panel = active_menu.as_ref().map(|menu| menu_action(content, menu));
    if let Some((action, _)) = &menu_panel {
        action_bar_actions.push(action.clone());
    }

    let bar_ids: Vec<&str> = action_bar_actions
        .iter()
        .map(|action| action.id.as_str())
        .collect();
    let ctx = UiContext {
        runtime,
        content,
        state: &state,
    };
    let overflow_actions = build_overflow_actions(&ctx, &bar_ids, &panels)?;
    let mut panel_options = build_panel_options(&ctx, panels)?;
    panel_options.extend(sidebar::build_party_order_panels(content, &state, &party));
    if let Some((_, options)) = menu_panel {
        panel_options.insert("menu".to_string(), options);
    }

    let sidebar_sections = content.visible_sidebar_sections(&state);
    let show_map = sidebar_sections.iter().any(|id| id == "map");
    let show_quests = sidebar_sections.iter().any(|id| id == "quests");
    let show_relationships = sidebar_sections.iter().any(|id| id == "relationships");

    let (objectives, quests, objective_message) = if show_quests {
        (objectives, quests, objective_message)
    } else {
        (Vec::new(), Vec::new(), String::new())
    };

    Ok(UiSnapshot {
        pack_id: pack_id.to_string(),
        title: content.opening.title.clone(),
        time_label,
        npc_tick_interval_ms: content.settings.npc_tick_interval_ms,
        day_number,
        current_room_name,
        followed_actor_name,
        help_text: runtime.help_text(),
        about_body: content.ui_text.about_body.clone(),
        current_locale: content.locale.clone(),
        locale_options: locales,
        objectives,
        quests,
        objective_message,
        secrets_found,
        secrets_total,
        rooms: menu_option_data(
            runtime
                .panel_options(&PanelDataSource::Exits)
                .map_err(|error| error.to_string())?,
        ),
        follow_options: menu_option_data(
            runtime
                .follow_actor_options()
                .map_err(|error| error.to_string())?,
        ),
        autonomous_actor_dialogue: content.settings.autonomous_actor_dialogue,
        show_player_input: content.settings.show_player_input,
        show_clock: content.settings.show_clock,
        action_bar_actions,
        overflow_actions,
        look_options,
        interactable_labels,
        talk_options,
        panel_options,
        active_menu,
        ui_text: content.ui_text.clone(),
        act_closure: if content.settings.show_act_closure {
            response::act_closure_data(runtime, transcript_lines)
        } else {
            None
        },
        game_closure: response::game_closure_data(runtime, transcript_lines),
        game_over: state.phase != GamePhase::Active,
        party,
        player: build_player_status(&state, content),
        skills: build_player_skills(&state, content),
        minimap: if show_map {
            build_minimap(&state, content, &current_room_id)
        } else {
            None
        },
        sidebar_sections,
        current_room_items: build_current_room_items(content, &state, &current_room_id),
        equipped_items: build_equipped_items(&state, content),
        inventory: build_inventory(runtime, content),
        room_consumables: build_room_consumables(runtime, content, &current_room_id),
        crafted_consumable_labels: crafted_consumable_labels(content, &current_room_id),
        relationship_pairs: if show_relationships {
            runtime.relationship_pairs().unwrap_or_default()
        } else {
            Vec::new()
        },
        theme: content.settings.theme.clone(),
    })
}

fn menu_option_data(options: Vec<PanelOption>) -> Vec<MenuOptionData> {
    options
        .into_iter()
        .map(|option| MenuOptionData {
            id: option.id,
            title: option.title,
            menu_text: option.menu_text,
        })
        .collect()
}

/// Ids of inventory items the player can currently drop (present in inventory).
/// Equipped items are stored in `state.inventory.equipment` and removed from `player_inventory`.
/// Items remaining in `player_inventory` with count > 0 are held and can be dropped or given.
fn droppable_inventory_items(state: &WorldState) -> Vec<String> {
    let mut ids: Vec<String> = state
        .inventory
        .player_inventory
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(item_id, _)| item_id.clone())
        .collect();
    ids.sort();
    ids
}

/// Ids of inventory items the player can currently use:
/// - Held items with a `use_hook` (consumables, potions, scrolls)
/// - Held keys that have an active, available lock action in the current room
fn usable_inventory_items(content: &ContentPack, state: &WorldState) -> Vec<String> {
    let current_room_id = &state.current_room_id;
    let mut ids: Vec<String> = state
        .inventory
        .player_inventory
        .iter()
        .filter(|(item_id, count)| {
            if **count == 0 {
                return false;
            }
            let Some(item) = content.item(item_id) else {
                return false;
            };
            if !item.use_hook.is_empty() {
                return true;
            }
            if item.kind == cinder_core::content::types::ItemKind::Key {
                return content.actions.iter().any(|action| {
                    (action.available.requires_item.as_deref() == Some(item_id.as_str())
                        || action.available.consumes_item.as_deref() == Some(item_id.as_str()))
                        && cinder_core::engine::turn_policies::action_is_available(
                            content,
                            state,
                            action,
                            current_room_id,
                        )
                });
            }
            false
        })
        .map(|(item_id, _)| item_id.clone())
        .collect();
    ids.sort();
    ids
}

fn append_stage_menu_overflow_actions(
    overflow_actions: &mut Vec<OverflowAction>,
    content: &cinder_core::content::types::ContentPack,
    active_stages: &[String],
    active_menu_id: Option<&str>,
    state: &cinder_core::engine::state::WorldState,
) {
    for stage_id in active_stages {
        let Some(stage) = content
            .beats
            .stages
            .iter()
            .find(|stage| &stage.id == stage_id)
        else {
            continue;
        };
        let Some(menu) = content
            .menus
            .iter()
            .find(|menu| &menu.stage_id == stage_id && !menu.dynamic && !menu.options.is_empty())
        else {
            continue;
        };
        // A menu whose actor is somewhere else waits until the player is back.
        if !cinder_core::engine::menus::menu_is_available(content, menu, state) {
            continue;
        }
        // An open menu is already shown as the docked choices; listing its
        // options here as well would show every choice twice.
        if active_menu_id == Some(menu.id.as_str()) {
            continue;
        }
        let waits_for_menu_selection = stage
            .advance_signals
            .iter()
            .any(|signal| signal.signal() == format!("menu_selected:{}", menu.id));
        if !waits_for_menu_selection {
            continue;
        }
        for option in &menu.options {
            if overflow_actions.iter().any(|action| action.id == option.id) {
                continue;
            }
            overflow_actions.push(OverflowAction {
                id: option.id.clone(),
                label: option.title.clone(),
                group: "support".to_string(),
                usage: String::new(),
                panel: String::new(),
                panel_config: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_open_stage_menu_is_a_bar_action_and_not_repeated_in_the_overflow() {
        use cinder_core::content::types::{
            AdvanceSignal, BeatDefinition, OpeningMenuDefinition, OpeningMenuOptionDefinition,
        };
        let option = |id: &str, title: &str| OpeningMenuOptionDefinition {
            id: id.to_string(),
            title: title.to_string(),
            menu_text: String::new(),
            ..Default::default()
        };
        let mut pack =
            cinder_core::loader::load_named_pack("layla", Some("en")).expect("layla loads");
        pack.beats.initial_stage_ids = vec!["test-stage".to_string()];
        pack.beats.stages.push(BeatDefinition {
            id: "test-stage".to_string(),
            open_menu: "test-menu".to_string(),
            advance_signals: vec![AdvanceSignal::Simple("menu_selected:test-menu".to_string())],
            ..Default::default()
        });
        pack.menus.push(OpeningMenuDefinition {
            id: "test-menu".to_string(),
            stage_id: "test-stage".to_string(),
            options: vec![option("a", "First"), option("b", "Second")],
            ..Default::default()
        });
        let runtime = CinderRuntime::new(pack, false).expect("runtime creates");
        let snapshot = build_ui_snapshot(&runtime, "layla", &[]).expect("snapshot builds");

        let menu = snapshot
            .active_menu
            .expect("the initial stage's menu is open");
        assert_eq!(menu.options.len(), 2);
        for option in &menu.options {
            assert!(
                !snapshot
                    .overflow_actions
                    .iter()
                    .any(|action| action.id == option.id || action.label == option.title),
                "choice '{}' must appear in the menu panel only",
                option.title
            );
        }
        let choices = snapshot
            .action_bar_actions
            .iter()
            .find(|action| action.id == "menu")
            .expect("an open menu is an action-bar action");
        assert_eq!(choices.panel.as_deref(), Some("menu"));
        let panel = &snapshot.panel_options["menu"];
        assert_eq!(panel.len(), 2);
        assert_eq!(panel[0].command.as_deref(), Some("1"));
    }

    #[test]
    fn isla_waiting_room_offers_talk_topics_and_the_tote() {
        let pack = cinder_core::loader::load_named_pack("isla", Some("en")).expect("isla loads");
        let runtime = CinderRuntime::new(pack, false).expect("runtime creates");
        runtime.run_turn("go inside").expect("go inside");
        let snapshot = build_ui_snapshot(&runtime, "isla", &[]).expect("snapshot builds");

        let bar: Vec<_> = snapshot
            .action_bar_actions
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        assert!(bar.contains(&"talk"), "{bar:?}");
        let talk = &snapshot.panel_options["talk"];
        let titles: Vec<_> = talk.iter().map(|o| o.title.as_str()).collect();
        assert_eq!(titles, vec!["Greet Wren", "Ask Gloria about Wren"]);
        assert_eq!(talk[0].command.as_deref(), Some("say wren:greet"));
        assert_eq!(snapshot.ui_text.look_group_items, "Tote bag");
        assert!(snapshot.sidebar_sections.iter().any(|id| id == "inventory"));
        assert!(snapshot.sidebar_sections.iter().any(|id| id == "quests"));
    }

    #[test]
    fn isla_choices_are_hidden_while_she_is_away_from_wren() {
        let pack = cinder_core::loader::load_named_pack("isla", Some("en")).expect("isla loads");
        let runtime = CinderRuntime::new(pack, false).expect("runtime creates");
        for command in ["go inside", "say wren:greet"] {
            runtime.run_turn(command).expect(command);
        }
        let here = build_ui_snapshot(&runtime, "isla", &[]).expect("snapshot builds");
        assert!(here.action_bar_actions.iter().any(|a| a.id == "menu"));

        runtime.run_turn("go hall").expect("go hall");
        let away = build_ui_snapshot(&runtime, "isla", &[]).expect("snapshot builds");
        assert!(
            !away.action_bar_actions.iter().any(|a| a.id == "menu"),
            "no Choices action out in the hall"
        );
        assert!(
            !away
                .overflow_actions
                .iter()
                .any(|a| a.label == "Usually Mom brings you on Thursdays."),
            "the answers are not offered as overflow buttons either"
        );
    }

    #[test]
    fn quests_and_levels_stay_hidden_until_player_visits_qualifying_room() {
        let pack = cinder_core::loader::load_named_pack("layla", Some("en")).expect("layla loads");
        let runtime = CinderRuntime::new(pack.clone(), false).expect("runtime creates");

        // Turn 0: Layla starts on floor 1 (r1c1). Even though Floor 4 NPCs (like Sakhra)
        // are initialized in village_square, the unlock story vars are unset.
        let snapshot = build_ui_snapshot(&runtime, "layla", &[]).expect("snapshot builds");

        assert!(
            !snapshot.sidebar_sections.iter().any(|id| id == "quests"),
            "quests section must not be visible on floor 1"
        );
        assert!(
            !snapshot.sidebar_sections.iter().any(|id| id == "level"),
            "level section must not be visible on floor 1"
        );
        assert!(
            snapshot.quests.is_empty(),
            "quests list must be empty when not revealed"
        );
        assert!(
            snapshot.objectives.is_empty(),
            "objectives list must be empty when not revealed"
        );
        assert!(
            snapshot.objective_message.is_empty(),
            "objective message must be empty when not revealed"
        );

        // Descending to Floor 4 fires the board.entered hook, which sets the
        // sticky unlock vars. Simulate that here along with the room change.
        let mut state = runtime.export_state().expect("state exports");
        state.story_vars.set_unchecked("levels_revealed", "true");
        state.story_vars.set_unchecked("quests_revealed", "true");
        state.current_room_id = "village_square".to_string();
        state.mark_actor_room_visited("player", "village_square");
        let runtime_f4 = CinderRuntime::from_state(pack, state, false).expect("runtime from state");

        let snapshot_f4 = build_ui_snapshot(&runtime_f4, "layla", &[]).expect("f4 snapshot builds");
        assert!(
            snapshot_f4.sidebar_sections.iter().any(|id| id == "quests"),
            "quests section must be visible in village_square"
        );
        assert!(
            snapshot_f4.sidebar_sections.iter().any(|id| id == "level"),
            "level section must be visible in village_square"
        );
        assert!(
            !snapshot_f4.objectives.is_empty(),
            "objectives must be visible on floor 4"
        );
    }
}
