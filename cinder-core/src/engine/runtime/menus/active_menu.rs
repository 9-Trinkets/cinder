use super::super::{ActiveMenuInfo, CinderRuntime, PanelOption};
use crate::content::types::OpeningMenuOptionDefinition;
use crate::engine::dialogue::DynamicMenuRequest;
use crate::engine::dialogue_grounding::{current_objective_beat_notes, viewer_participant_id};
use crate::engine::menus::render_menu_prompt;
use crate::engine::state::{
    display_actor_name, remap_story_actor_id, render_dynamic_story_text,
    resolved_actor_prompt_context,
};
use crate::engine::turn_policies::actor_objective_guidance_notes;
use std::error::Error;

impl CinderRuntime {
    pub fn current_active_menu_info(&self) -> Result<Option<ActiveMenuInfo>, Box<dyn Error>> {
        let (menu_id, max_selections, min_selections, selected_ids) = {
            let state = self
                .state
                .lock()
                .map_err(|_| "failed to lock runtime state for active menu")?;
            (
                state.active_menu_id.clone(),
                state
                    .active_menu_id
                    .as_ref()
                    .and_then(|id| self.content.menu(id))
                    .map(|m| m.max_selections)
                    .unwrap_or(0),
                state
                    .active_menu_id
                    .as_ref()
                    .and_then(|id| self.content.menu(id))
                    .map(|m| m.min_selections)
                    .unwrap_or(0),
                state.pending_menu_selections.clone(),
            )
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
            return Ok(Some(ActiveMenuInfo {
                prompt,
                options,
                max_selections,
                min_selections,
                selected_ids,
            }));
        }
        Ok(Some(ActiveMenuInfo {
            prompt,
            options: menu.options.clone(),
            max_selections,
            min_selections,
            selected_ids,
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
            let Some(selected_id) = state
                .story_vars
                .get(&menu.selection_id_var_key)
                .map(|s| s.to_string())
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
                .set_unchecked(&menu.selection_var_key, &option_title);
            state
                .story_vars
                .set_unchecked("selection_title", &option_title);
        }
        Ok(())
    }

    pub fn menu_choice_options(&self) -> Result<Option<Vec<PanelOption>>, Box<dyn Error>> {
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
                    let mut beat_notes = current_objective_beat_notes(
                        self.content.as_ref(),
                        &state,
                        Some(actor.id.as_str()),
                    );
                    beat_notes.extend(actor_objective_guidance_notes(
                        self.content.as_ref(),
                        &state,
                        actor.id.as_str(),
                    ));
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
                        ui_text: self.content.ui_text.clone(),
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
                            room_id: String::new(),
                            host_actor_id: String::new(),
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
}

fn render_menu_choice_options(
    _content: &crate::content::types::ContentPack,
    _menu: &crate::content::types::OpeningMenuDefinition,
    _state: &crate::engine::state::WorldState,
    options: &[OpeningMenuOptionDefinition],
) -> Vec<PanelOption> {
    let mut result: Vec<PanelOption> = options
        .iter()
        .enumerate()
        .map(|(index, option)| PanelOption {
            id: option.id.clone(),
            title: option.title.clone(),
            command: (index + 1).to_string(),
            menu_text: option.menu_text.clone(),
        })
        .collect();
    if _menu.max_selections > 0 {
        result.push(PanelOption {
            id: "done".to_string(),
            title: "Done".to_string(),
            command: "done".to_string(),
            menu_text: "Confirm selection".to_string(),
        });
    }
    result
}
