use super::{ActiveMenuInfo, CinderRuntime, LookOptionItem, MenuChoiceOption};
use crate::content::types::{OpeningMenuOptionDefinition, RoomDefinition};
use crate::engine::conversation_memory::refresh_conversation_summaries;
use crate::engine::dialogue::DynamicMenuRequest;
use crate::engine::dialogue_grounding::current_objective_beat_notes;
use crate::engine::dialogue_grounding::viewer_participant_id;
use crate::engine::events::{ObservationMode, TimestampedWorldEvent, WorldEvent};
use crate::engine::menus::render_menu_prompt;
use crate::engine::reducer::apply_events;
use crate::engine::state::{
    TurnOutcome, display_actor_name, remap_story_actor_id, render_dynamic_story_text,
    resolved_actor_prompt_context,
};
use std::error::Error;

impl CinderRuntime {
    pub fn current_room_look_options(&self) -> Result<Vec<LookOptionItem>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for look options")?;
        let current_room_id = &state.current_room_id;
        let Some(room) = self.content.room(current_room_id) else {
            return Ok(Vec::new());
        };
        let mut options = Vec::new();
        options.push(LookOptionItem {
            id: "__room__".to_string(),
            label: room.title.clone(),
            command: "look".to_string(),
        });
        for feature in &room.features {
            let alias = feature
                .aliases
                .first()
                .map(|a| a.as_str())
                .unwrap_or(&feature.label);
            options.push(LookOptionItem {
                id: format!("feature:{}", feature.id),
                label: feature.label.clone(),
                command: format!("x {}", alias),
            });
        }
        for actor in &self.content.actors {
            let actor_room = state.actor_room_id(&actor.id, &actor.room_id);
            if actor_room == current_room_id {
                let actor_name = display_actor_name(&state, actor);
                options.push(LookOptionItem {
                    id: format!("actor:{}", actor.id),
                    label: actor_name,
                    command: format!("look at {}", actor.id),
                });
            }
        }
        for item in &self.content.items {
            if state.has_item(&item.id) {
                options.push(LookOptionItem {
                    id: format!("item:{}", item.id),
                    label: item.label.clone(),
                    command: format!("look at {}", item.label),
                });
            }
        }
        Ok(options)
    }

    pub fn current_room_talk_options(&self) -> Result<Vec<LookOptionItem>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for talk options")?;
        let current_room_id = &state.current_room_id;
        let mut options = Vec::new();
        for actor in &self.content.actors {
            let actor_room = state.actor_room_id(&actor.id, &actor.room_id);
            if actor_room == current_room_id {
                let actor_name = display_actor_name(&state, actor);
                options.push(LookOptionItem {
                    id: format!("actor:{}", actor.id),
                    label: actor_name,
                    command: format!("talk to {}", actor.id),
                });
            }
        }
        Ok(options)
    }

    pub fn current_active_menu_info(&self) -> Result<Option<ActiveMenuInfo>, Box<dyn Error>> {
        let menu_id = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for active menu")?;
            state.active_menu_id.clone()
        };
        let Some(ref menu_id) = menu_id else {
            return Ok(None);
        };
        let Some(menu) = self.content.menu(menu_id) else {
            return Ok(None);
        };
        let prompt = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for active menu prompt")?;
            render_menu_prompt(self.content.as_ref(), menu, &state)
        };
        if menu.dynamic {
            let needs_generation = {
                let state = self
                    .state
                    .lock()
                    .map_err(|_| "failed to lock runtime state for dynamic menu")?;
                !state.generated_menu_options.contains_key(menu_id.as_str())
            };
            if needs_generation {
                self.menu_choice_options()?;
            }
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state after dynamic menu gen")?;
            let options = state
                .generated_menu_options
                .get(menu_id.as_str())
                .cloned()
                .unwrap_or_default();
            return Ok(Some(ActiveMenuInfo { prompt, options }));
        }
        Ok(Some(ActiveMenuInfo {
            prompt,
            options: menu.options.clone(),
        }))
    }

    pub fn relocalize_story_vars(&self) -> Result<(), Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for story-var relocalization")?;
        for menu in &self.content.menus {
            if menu.selection_var_key.is_empty() || menu.selection_id_var_key.is_empty() {
                continue;
            }
            let Some(selected_id) = state.story_vars.get(&menu.selection_id_var_key).cloned()
            else {
                continue;
            };
            let generated_options = state.generated_menu_options.get(&menu.id);
            let Some(option_title) = generated_options
                .and_then(|options| options.iter().find(|option| option.id == selected_id))
                .or_else(|| menu.options.iter().find(|option| option.id == selected_id))
                .map(|option| option.title.clone())
            else {
                continue;
            };
            state
                .story_vars
                .insert(menu.selection_var_key.clone(), option_title.clone());
            state
                .story_vars
                .insert("selection_title".to_string(), option_title);
        }
        Ok(())
    }

    pub fn menu_choice_options(&self) -> Result<Option<Vec<MenuChoiceOption>>, Box<dyn Error>> {
        let menu_id = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for menu")?;
            state.active_menu_id.clone()
        };
        let Some(ref menu_id) = menu_id else {
            return Ok(None);
        };
        let Some(menu) = self.content.menu(menu_id) else {
            return Ok(None);
        };
        if menu.dynamic {
            {
                let state = self
                    .state
                    .lock()
                    .map_err(|_| "failed to lock runtime state for dynamic menu")?;
                if let Some(options) = state.generated_menu_options.get(menu_id) {
                    return Ok(Some(render_menu_choice_options(
                        self.content.as_ref(),
                        menu,
                        &state,
                        options,
                    )));
                }
            }
            let needs_generation = {
                let state = self
                    .state
                    .lock()
                    .map_err(|_| "failed to lock runtime state for dynamic menu")?;
                !state.generated_menu_options.contains_key(menu_id)
            };
            if needs_generation {
                let (menu_prompt, actor_name, character_bio, current_beat_notes, recent_memory) = {
                    let state = self
                        .state
                        .lock()
                        .map_err(|_| "failed to lock runtime state for dynamic menu")?;
                    let actor = self
                        .content
                        .actor(remap_story_actor_id(&state, &menu.actor_id))
                        .ok_or_else(|| format!("missing actor '{}'", menu.actor_id))?;
                    let prompt_context =
                        resolved_actor_prompt_context(self.content.as_ref(), &state, actor);
                    let mut beat_notes =
                        current_objective_beat_notes(self.content.as_ref(), &state);
                    beat_notes.extend(
                        menu.narrative_lines
                            .iter()
                            .map(|line| render_dynamic_story_text(line, &state)),
                    );
                    (
                        render_menu_prompt(self.content.as_ref(), menu, &state),
                        display_actor_name(&state, actor),
                        prompt_context
                            .character_notes
                            .iter()
                            .chain(prompt_context.subtext_notes.iter())
                            .chain(prompt_context.response_notes.iter())
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("\n"),
                        beat_notes,
                        state
                            .conversation_history(
                                remap_story_actor_id(&state, &menu.actor_id),
                                &viewer_participant_id(self.content.as_ref()),
                            )
                            .iter()
                            .rev()
                            .take(10)
                            .cloned()
                            .collect::<Vec<_>>(),
                    )
                };
                let role_name = if menu.generation_role.is_empty() {
                    "dynamic_menu"
                } else {
                    &menu.generation_role
                };
                let result = self
                    .dialogue
                    .generate_dynamic_menu_options(&DynamicMenuRequest {
                        locale: self.content.locale.clone(),
                        system_text: self.content.system_text.clone(),
                        role_name: role_name.to_string(),
                        menu_id: menu.id.clone(),
                        menu_prompt,
                        intent_guidance: menu.intent_guidance.clone(),
                        actor_name,
                        character_bio,
                        current_beat_notes,
                        recent_memory,
                    });
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| "failed to lock runtime state after dynamic menu generation")?;
                if let Ok(options) = result {
                    let options: Vec<OpeningMenuOptionDefinition> = options
                        .into_iter()
                        .map(|opt| OpeningMenuOptionDefinition {
                            id: opt.id,
                            title: opt.title,
                            menu_text: opt.menu_text,
                            narrative_lines: vec![],
                        })
                        .collect();
                    state
                        .generated_menu_options
                        .insert(menu_id.clone(), options.clone());
                    return Ok(Some(render_menu_choice_options(
                        self.content.as_ref(),
                        menu,
                        &state,
                        &options,
                    )));
                }
            }
        }
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for menu rendering")?;
        Ok(Some(render_menu_choice_options(
            self.content.as_ref(),
            menu,
            &state,
            &menu.options,
        )))
    }

    pub fn room_switch_options(&self) -> Result<Vec<MenuChoiceOption>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for room switching")?;
        let Some(current_room) = self.content.room(&state.current_room_id) else {
            return Ok(Vec::new());
        };
        let prompt = self
            .content
            .ui_text
            .room_switch_prompt
            .replace("{}", &current_room.title);

        let exit_ids: Vec<String> = current_room
            .exits
            .iter()
            .map(|e| e.room_id.clone())
            .collect();
        let rooms_iter: Box<dyn Iterator<Item = &RoomDefinition>> =
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

        Ok(rooms_iter
            .map(|room| MenuChoiceOption {
                prompt: prompt.clone(),
                title: room.title.clone(),
                menu_text: room.title.clone(),
                command: room.id.clone(),
                transcript_label: Some(room.title.clone()),
            })
            .collect())
    }

    pub fn follow_actor_options(&self) -> Result<Vec<MenuChoiceOption>, Box<dyn Error>> {
        let state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for follow options")?;
        let follow_prompt = self.content.ui_text.follow_actor_prompt.clone();
        let nobody_label = self.content.ui_text.follow_nobody_option.clone();
        let mut options = vec![MenuChoiceOption {
            prompt: follow_prompt.clone(),
            title: nobody_label.clone(),
            menu_text: nobody_label,
            command: "none".to_string(),
            transcript_label: None,
        }];
        options.extend(self.content.actors.iter().map(|actor| {
            let room_id = state.actor_room_id(&actor.id, &actor.room_id);
            let room_title = self
                .content
                .room(room_id)
                .map(|room| room.title.clone())
                .unwrap_or_else(|| room_id.to_string());
            let actor_name = display_actor_name(&state, actor);
            MenuChoiceOption {
                prompt: follow_prompt.clone(),
                title: actor_name.clone(),
                menu_text: format!("{} ({room_title})", actor_name),
                command: actor.id.clone(),
                transcript_label: Some(actor_name),
            }
        }));
        Ok(options)
    }

    pub fn switch_room_view(&self, room_id: &str) -> Result<TurnOutcome, Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for room switching")?;
        let Some(room) = self.content.room(room_id) else {
            return Err(format!("missing room '{room_id}'").into());
        };
        let turn_number = state.turn_number + 1;
        let mut events = vec![TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number,
            raw_input: format!("switch-room:{room_id}"),
            advances_time: false,
        })];
        if state.current_room_id != room.id {
            events.push(TimestampedWorldEvent::now(WorldEvent::PlayerMoved {
                from_room_id: state.current_room_id.clone(),
                to_room_id: room.id.clone(),
            }));
        }
        events.push(TimestampedWorldEvent::now(
            WorldEvent::CurrentRoomObserved {
                room_id: room.id.clone(),
                mode: ObservationMode::Summary,
            },
        ));
        let reduced = apply_events(&mut state, self.content.as_ref(), &events);
        refresh_conversation_summaries(self.content.as_ref(), self.dialogue.as_ref(), &mut state)
            .map_err(std::io::Error::other)?;
        Ok(TurnOutcome {
            text: reduced.lines.join("\n\n"),
            game_over: reduced.game_over,
        })
    }

    pub fn follow_actor(&self, actor_id: Option<&str>) -> Result<TurnOutcome, Box<dyn Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "failed to lock runtime state for following actor")?;
        let turn_number = state.turn_number + 1;
        let feedback_line = format!(
            "> {}",
            match actor_id {
                Some(actor_id) => {
                    let actor = self
                        .content
                        .actor(actor_id)
                        .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
                    let actor_name = display_actor_name(&state, actor);
                    self.content
                        .ui_text
                        .follow_actor_transcript
                        .replace("{title}", &actor_name)
                }
                None => self.content.ui_text.follow_actor_stop_transcript.clone(),
            }
        );
        let mut events = vec![TimestampedWorldEvent::now(WorldEvent::TurnStarted {
            turn_number,
            raw_input: format!("follow:{}", actor_id.unwrap_or("none")),
            advances_time: false,
        })];
        match actor_id {
            Some(actor_id) => {
                let actor = self
                    .content
                    .actor(actor_id)
                    .ok_or_else(|| format!("missing actor '{actor_id}'"))?;
                state.followed_actor_id = Some(actor_id.to_string());
                let room_id = state.actor_room_id(actor_id, &actor.room_id).to_string();
                if state.current_room_id != room_id {
                    events.push(TimestampedWorldEvent::now(WorldEvent::PlayerMoved {
                        from_room_id: state.current_room_id.clone(),
                        to_room_id: room_id.clone(),
                    }));
                    events.push(TimestampedWorldEvent::now(
                        WorldEvent::CurrentRoomObserved {
                            room_id,
                            mode: ObservationMode::Summary,
                        },
                    ));
                }
            }
            None => {
                state.followed_actor_id = None;
            }
        }
        let reduced = apply_events(&mut state, self.content.as_ref(), &events);
        refresh_conversation_summaries(self.content.as_ref(), self.dialogue.as_ref(), &mut state)
            .map_err(std::io::Error::other)?;
        let mut lines = vec![feedback_line];
        lines.extend(reduced.lines);
        Ok(TurnOutcome {
            text: lines.join("\n\n"),
            game_over: reduced.game_over,
        })
    }
}

fn render_menu_choice_options(
    content: &crate::content::types::ContentPack,
    menu: &crate::content::types::OpeningMenuDefinition,
    state: &crate::engine::state::WorldState,
    options: &[OpeningMenuOptionDefinition],
) -> Vec<MenuChoiceOption> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| MenuChoiceOption {
            prompt: render_menu_prompt(content, menu, state),
            title: option.title.clone(),
            menu_text: option.menu_text.clone(),
            command: (index + 1).to_string(),
            transcript_label: None,
        })
        .collect()
}
