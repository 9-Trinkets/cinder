use super::super::{CinderRuntime, PanelOption};
use crate::content::types::PanelDataSource;
use crate::engine::state::{WorldState, display_actor_name};
use std::error::Error;

impl CinderRuntime {
    /// Builds the option list for a session action's panel, dispatching on the
    /// panel's `data_source`. All session menus (look, talk, move, follow) and
    /// the authored action panels are driven through this single generic entry
    /// point so option construction stays data-driven rather than bespoke.
    pub fn panel_options(
        &self,
        data_source: &PanelDataSource,
    ) -> Result<Vec<PanelOption>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for panel options")?;
        let current_room_id = &state.current_room_id;
        let options = match data_source {
            PanelDataSource::ActorsInRoom => self.room_actors_panel_options(&state),
            PanelDataSource::Exits => self.room_exit_panel_options(&state, current_room_id),
            PanelDataSource::Features => {
                self.current_room_feature_panel_options(&state, current_room_id)
            }
            PanelDataSource::LooseRoomItems => state
                .loose_room_items(current_room_id)
                .into_iter()
                .filter_map(|(item_id, _count)| {
                    let item = self.content.item(&item_id)?;
                    if !item.is_takeable() {
                        return None;
                    }
                    Some((item_id.clone(), item.label.clone()))
                })
                .map(|(item_id, title)| PanelOption {
                    id: item_id.clone(),
                    title,
                    command: format!("take {item_id}"),
                    menu_text: String::new(),
                })
                .collect(),
            PanelDataSource::InventoryItems => self.inventory_panel_options(&state),
            PanelDataSource::CraftableItems => Vec::new(),
        };
        Ok(options)
    }

    /// Non-player, non-defeated actors currently present in the given room.
    fn room_actors_panel_options(&self, state: &WorldState) -> Vec<PanelOption> {
        self.in_room_actor_ids(state)
            .map(|(actor, room_id)| {
                let _ = room_id;
                let actor_name = display_actor_name(state, actor);
                PanelOption {
                    id: format!("actor:{}", actor.id),
                    title: actor_name.clone(),
                    command: format!("talk to {}", actor.id),
                    menu_text: actor_name,
                }
            })
            .collect()
    }

    /// Ids of in-room, alive, non-player actors. Shared between look, talk,
    /// and follow so the actor-iteration predicate lives in one place.
    fn in_room_actor_ids<'a>(
        &'a self,
        state: &'a WorldState,
    ) -> impl Iterator<Item = (&'a crate::content::types::ActorDefinition, &'a str)> {
        let current_room_id = &state.current_room_id;
        self.content.actors.iter().filter_map(move |actor| {
            if self.content.is_player_actor(&actor.id) {
                return None;
            }
            let actor_room = state.actor_room_id(&actor.id, &actor.room_id);
            if actor_room == current_room_id
                && !state.actor_is_defeated(&actor.id, &self.content.settings.combat.health_stat_id)
            {
                Some((actor, actor_room))
            } else {
                None
            }
        })
    }

    fn room_exit_panel_options(&self, state: &WorldState, current_room_id: &str) -> Vec<PanelOption> {
        let Some(current_room) = self.content.room(current_room_id) else {
            return Vec::new();
        };
        let exit_ids: Vec<String> = current_room
            .exits
            .iter()
            .filter(|e| {
                e.requires_story_var.is_empty()
                    || crate::engine::turn_policies::story_var_is_truthy(
                        state,
                        &e.requires_story_var,
                    )
            })
            .map(|e| e.room_id.clone())
            .collect();
        let rooms_iter: Box<dyn Iterator<Item = &crate::content::types::RoomDefinition>> =
            if self.content.settings.channel_surfing_only {
                Box::new(self.content.rooms.iter())
            } else {
                Box::new(
                    self.content
                        .rooms
                        .iter()
                        .filter(move |room| exit_ids.contains(&room.id)),
                )
            };
        rooms_iter
            .map(|room| {
                let exit_label = current_room.exits.iter().find(|e| e.room_id == room.id);
                let title = exit_label
                    .and_then(|e| e.menu_label.clone())
                    .unwrap_or_else(|| room.title.clone());
                PanelOption {
                    id: room.id.clone(),
                    title: title.clone(),
                    command: room.id.clone(),
                    menu_text: title,
                }
            })
            .collect()
    }

    fn current_room_feature_panel_options(
        &self,
        state: &WorldState,
        current_room_id: &str,
    ) -> Vec<PanelOption> {
        let _ = state;
        let Some(room) = self.content.room(current_room_id) else {
            return Vec::new();
        };
        room.features
            .iter()
            .map(|feature| {
                let alias = feature
                    .aliases
                    .first()
                    .map(|a| a.as_str())
                    .unwrap_or(&feature.label);
                PanelOption {
                    id: format!("feature:{}", feature.id),
                    title: feature.label.clone(),
                    command: format!("x {alias}"),
                    menu_text: String::new(),
                }
            })
            .collect()
    }

    fn inventory_panel_options(&self, state: &WorldState) -> Vec<PanelOption> {
        let mut ids: Vec<String> = state
            .player_inventory
            .iter()
            .filter(|(item_id, count)| {
                **count > 0
                    && !state
                        .equipment
                        .values()
                        .any(|equipped| equipped.as_str() == item_id.as_str())
            })
            .map(|(item_id, _)| item_id.clone())
            .collect();
        ids.sort();
        ids.into_iter()
            .map(|item_id| {
                let title = self
                    .content
                    .item(&item_id)
                    .map(|item| item.label.clone())
                    .unwrap_or_else(|| item_id.clone());
                PanelOption {
                    id: item_id.clone(),
                    title,
                    command: format!("drop {item_id}"),
                    menu_text: String::new(),
                }
            })
            .collect()
    }

    /// The full list of interactable objects in the current room: the room
    /// itself, its features, present actors, and loose/owned items. Used by
    /// the "look" composite panel shown in `UiSnapshot.look_options`.
    pub fn room_interactable_options(&self) -> Result<Vec<PanelOption>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for interactable options")?;
        let current_room_id = &state.current_room_id;
        let Some(room) = self.content.room(current_room_id) else {
            return Ok(Vec::new());
        };
        let mut options = Vec::new();
        options.push(PanelOption {
            id: "__room__".to_string(),
            title: room.title.clone(),
            command: "look".to_string(),
            menu_text: String::new(),
        });
        for feature in &room.features {
            let alias = feature
                .aliases
                .first()
                .map(|a| a.as_str())
                .unwrap_or(&feature.label);
            options.push(PanelOption {
                id: format!("feature:{}", feature.id),
                title: feature.label.clone(),
                command: format!("x {alias}"),
                menu_text: String::new(),
            });
        }
        for (actor, _) in self.in_room_actor_ids(&state) {
            let actor_name = display_actor_name(&state, actor);
            options.push(PanelOption {
                id: format!("actor:{}", actor.id),
                title: actor_name,
                command: format!("look at {}", actor.id),
                menu_text: String::new(),
            });
        }
        for item in &self.content.items {
            if self
                .content
                .room_consumables(current_room_id)
                .into_iter()
                .any(|candidate| {
                    candidate.consumable.id == item.id
                        && state.remaining_consumable_stock(
                            current_room_id,
                            &candidate.feature.id,
                            &candidate.consumable.id,
                        ) > 0
                })
            {
                continue;
            }
            if state.has_item(&item.id)
                || state.has_item_in_storage(
                    &item.id,
                    crate::content::types::ItemStorageTarget::CurrentRoom,
                    current_room_id,
                )
            {
                options.push(PanelOption {
                    id: format!("item:{}", item.id),
                    title: item.label.clone(),
                    command: format!("look at {}", item.label),
                    menu_text: String::new(),
                });
            }
        }
        Ok(options)
    }

    /// Builds the follow-actor option list: a "nobody" sentinel plus every
    /// non-player actor (annotated with its current room).
    pub fn follow_actor_options(&self) -> Result<Vec<PanelOption>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for follow options")?;
        let nobody_label = self.content.ui_text.follow_nobody_option.clone();
        let mut options = vec![PanelOption {
            id: "none".to_string(),
            title: nobody_label.clone(),
            command: "none".to_string(),
            menu_text: nobody_label,
        }];
        options.extend(
            self.content
                .actors
                .iter()
                .filter(|actor| !self.content.is_player_actor(&actor.id))
                .map(|actor| {
                    let room_id = state.actor_room_id(&actor.id, &actor.room_id);
                    let room_title = self
                        .content
                        .room(room_id)
                        .map(|room| room.title.clone())
                        .unwrap_or_else(|| room_id.to_string());
                    let actor_name = display_actor_name(&state, actor);
                    PanelOption {
                        id: actor.id.clone(),
                        title: actor_name.clone(),
                        command: actor.id.clone(),
                        menu_text: format!("{} ({room_title})", actor_name),
                    }
                }),
        );
        Ok(options)
    }
}
