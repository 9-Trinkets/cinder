use super::*;
use crate::content::text_defs::{SystemTextDefinition, UiTextDefinition};

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
    pub stats: StatsDefinition,
    pub commands: CommandsDefinition,
    pub affordances: AffordancesDefinition,
    pub hooks: BTreeMap<String, Value>,
    pub speech_intents: SpeechIntentsConfig,
    pub room_index: HashMap<String, usize>,
    pub actor_index: HashMap<String, usize>,
    pub command_index: HashMap<String, usize>,
    pub affordance_index: HashMap<String, usize>,
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

    pub fn command(&self, command_id: &str) -> Option<&CommandDefinition> {
        self.command_index
            .get(command_id)
            .map(|&i| &self.commands.actions[i])
    }

    pub fn content_event(&self, event_id: &str) -> Option<&ContentEventDefinition> {
        self.commands
            .actions
            .iter()
            .filter_map(|command| command.content_event.as_ref())
            .find(|event| event.id == event_id)
    }

    pub fn player_commands(&self) -> Vec<&CommandDefinition> {
        self.commands
            .actions
            .iter()
            .filter(|command| command.player_enabled)
            .collect()
    }

    pub fn affordance(&self, affordance_id: &str) -> Option<&AffordanceDefinition> {
        self.affordance_index
            .get(affordance_id)
            .map(|&i| &self.affordances.actions[i])
    }

    pub fn menu(&self, menu_id: &str) -> Option<&OpeningMenuDefinition> {
        self.menus.iter().find(|menu| menu.id == menu_id)
    }

    pub fn movement_rules(&self, actor_id: &str) -> Option<&ActorMovementRulesDefinition> {
        self.actor(actor_id)?.movement_rules.as_ref()
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

    pub fn render_template(&self, template: &str, replacements: &[(&str, &str)]) -> String {
        let mut rendered = template.to_string();
        for (key, value) in replacements {
            rendered = rendered.replace(&format!("{{{key}}}"), value);
        }
        rendered
    }
}
