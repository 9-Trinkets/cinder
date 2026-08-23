use super::*;
use crate::content::text_defs::{SystemTextDefinition, UiTextDefinition};
use crate::engine::state::{VariableDeclaration, WorldState};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct ContentPack {
    pub locale: String,
    pub settings: ContentSettingsDefinition,
    pub ui_text: UiTextDefinition,
    pub system_text: SystemTextDefinition,
    pub opening: OpeningDefinition,
    pub beats: BeatsDefinition,
    pub menus: Vec<OpeningMenuDefinition>,
    pub movies: Vec<OpeningMovieDefinition>,
    pub presentation: PresentationDefinition,
    pub rooms: Vec<RoomDefinition>,
    pub actors: Vec<ActorDefinition>,
    pub act_cast: Vec<ActCastMember>,
    pub stats: StatsDefinition,
    pub actions: Vec<ActionDefinition>,
    pub movement: MovementConfigDefinition,
    pub speech: SpeechConfigDefinition,
    pub rule_bundles: RuleBundlesDefinition,
    pub hooks: BTreeMap<String, Value>,
    pub speech_intents: SpeechIntentsConfig,
    pub items: Vec<ItemDefinition>,
    pub variables: BTreeMap<String, VariableDeclaration>,
    pub room_index: HashMap<String, usize>,
    pub actor_index: HashMap<String, usize>,
    pub action_index: HashMap<String, usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct RoomConsumableRef<'a> {
    pub feature: &'a RoomFeatureDefinition,
    pub consumable: &'a ConsumableDefinition,
}

impl ContentPack {
    pub fn room(&self, room_id: &str) -> Option<&RoomDefinition> {
        self.room_index.get(room_id).map(|&i| &self.rooms[i])
    }

    pub fn resolve_exit<'a>(
        &'a self,
        room_id: &str,
        raw_target: &str,
    ) -> Option<&'a RoomExitDefinition> {
        let target = raw_target.trim().to_ascii_lowercase();
        self.room(room_id)?.exits.iter().find(|exit| {
            exit.label.eq_ignore_ascii_case(&target)
                || exit.room_id.eq_ignore_ascii_case(&target)
                || exit
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(&target))
        })
    }

    pub fn actor(&self, actor_id: &str) -> Option<&ActorDefinition> {
        self.actor_index.get(actor_id).map(|&i| &self.actors[i])
    }

    pub fn hook(&self, hook_id: &str) -> Option<&Value> {
        self.hooks.get(hook_id)
    }

    pub fn command(&self, command_id: &str) -> Option<&ActionDefinition> {
        self.action(command_id)
    }

    pub fn content_event(&self, event_id: &str) -> Option<&ActionContentEvent> {
        self.actions
            .iter()
            .filter_map(|action| action.content_event.as_ref())
            .find(|event| event.id == event_id)
    }

    pub fn player_commands(&self) -> Vec<&ActionDefinition> {
        self.actions
            .iter()
            .filter(|action| action.player_enabled)
            .collect()
    }

    pub fn action(&self, action_id: &str) -> Option<&ActionDefinition> {
        self.action_index.get(action_id).map(|&i| &self.actions[i])
    }

    pub fn menu(&self, menu_id: &str) -> Option<&OpeningMenuDefinition> {
        self.menus.iter().find(|menu| menu.id == menu_id)
    }

    pub fn item(&self, item_id: &str) -> Option<&ItemDefinition> {
        self.items.iter().find(|item| item.id == item_id)
    }

    pub fn resolve_item_in_scope<'a>(
        &'a self,
        state: &WorldState,
        current_room_id: &str,
        raw_target: &str,
    ) -> Option<&'a ItemDefinition> {
        let target = raw_target.trim().to_ascii_lowercase();
        self.items.iter().find(|item| {
            (state.has_item(&item.id)
                || state.has_item_in_storage(
                    &item.id,
                    super::ItemStorageTarget::CurrentRoom,
                    current_room_id,
                ))
                && (item.id.eq_ignore_ascii_case(&target)
                    || item.label.eq_ignore_ascii_case(&target))
        })
    }

    /// An actor with no entry in movement.json simply has no target rules -- it still
    /// gets NPC turns like any other actor. Never returns None.
    pub fn movement_rules(&self, actor_id: &str) -> ActorMovementRulesDefinition {
        self.movement
            .actors
            .get(actor_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn room_is_reachable(&self, room_id: &str) -> bool {
        !self
            .movement
            .unreachable_rooms
            .iter()
            .any(|id| id == room_id)
    }

    /// Rooms directly connected to `room_id` by an exit in either direction.
    pub fn adjacent_room_ids(&self, room_id: &str) -> Vec<String> {
        let mut neighbors: Vec<String> = Vec::new();
        if let Some(room) = self.room(room_id) {
            for exit in &room.exits {
                if exit.room_id != room_id && !neighbors.contains(&exit.room_id) {
                    neighbors.push(exit.room_id.clone());
                }
            }
        }
        for other in &self.rooms {
            if other.id == room_id {
                continue;
            }
            if other.exits.iter().any(|exit| exit.room_id == room_id)
                && !neighbors.contains(&other.id)
            {
                neighbors.push(other.id.clone());
            }
        }
        neighbors
    }

    pub fn resolve_actor(&self, raw_target: &str) -> Option<&ActorDefinition> {
        let target = raw_target.trim().to_ascii_lowercase();
        self.actors.iter().find(|actor| {
            actor.name.eq_ignore_ascii_case(&target)
                || actor.id.eq_ignore_ascii_case(&target)
                || actor
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(&target))
        })
    }

    pub fn resolve_feature_in_room<'a>(
        &'a self,
        room_id: &str,
        raw_target: &str,
    ) -> Option<&'a RoomFeatureDefinition> {
        let target = raw_target.trim().to_ascii_lowercase();
        self.room(room_id)?.features.iter().find(|feature| {
            feature.label.eq_ignore_ascii_case(&target)
                || feature.id.eq_ignore_ascii_case(&target)
                || feature
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(&target))
        })
    }

    pub fn room_consumables<'a>(&'a self, room_id: &str) -> Vec<RoomConsumableRef<'a>> {
        self.room(room_id)
            .map(|room| {
                room.features
                    .iter()
                    .flat_map(|feature| {
                        feature
                            .consumables
                            .iter()
                            .map(|consumable| RoomConsumableRef {
                                feature,
                                consumable,
                            })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn resolve_consumable_in_room<'a>(
        &'a self,
        room_id: &str,
        raw_target: &str,
    ) -> Option<RoomConsumableRef<'a>> {
        let target = raw_target.trim().to_ascii_lowercase();
        self.room_consumables(room_id)
            .into_iter()
            .find(|candidate| {
                candidate.consumable.label.eq_ignore_ascii_case(&target)
                    || candidate.consumable.id.eq_ignore_ascii_case(&target)
                    || candidate
                        .consumable
                        .aliases
                        .iter()
                        .any(|alias| alias.eq_ignore_ascii_case(&target))
            })
    }

    pub fn room_consumable<'a>(
        &'a self,
        room_id: &str,
        feature_id: &str,
        consumable_id: &str,
    ) -> Option<RoomConsumableRef<'a>> {
        self.room(room_id)?.features.iter().find_map(|feature| {
            if feature.id != feature_id {
                return None;
            }
            feature
                .consumables
                .iter()
                .find(|consumable| consumable.id == consumable_id)
                .map(|consumable| RoomConsumableRef {
                    feature,
                    consumable,
                })
        })
    }

    pub fn crafted_current_room_item_ids(&self) -> BTreeSet<String> {
        self.actions
            .iter()
            .filter(|action| {
                action
                    .item_creation
                    .as_ref()
                    .is_some_and(|ic| ic.storage == ActionItemStorageTarget::CurrentRoom)
            })
            .filter_map(|action| {
                action
                    .item_creation
                    .as_ref()
                    .map(|ic| ic.creates_item.clone())
            })
            .collect()
    }

    pub fn render_template(&self, template: &str, replacements: &[(&str, &str)]) -> String {
        let mut rendered = template.to_string();
        for (key, value) in replacements {
            rendered = rendered.replace(&format!("{{{key}}}"), value);
        }
        rendered
    }
}
