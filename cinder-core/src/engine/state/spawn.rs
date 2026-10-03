use super::{ActorStance, WorldState};
use crate::content::types::ContentPack;

#[derive(Debug, Clone)]
pub struct SpawnActorConfig<'a> {
    pub template_id: &'a str,
    pub room_id: Option<&'a str>,
    pub stance: Option<ActorStance>,
    pub follows_player: bool,
    pub scale_with_actor_id: Option<&'a str>,
    pub scale_stat: Option<&'a str>,
    pub max_active_instances: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnedActorInfo {
    pub instance_id: String,
    pub template_id: String,
    pub actor_name: String,
    pub target_room_id: String,
    pub scaled_hp: i32,
    pub scaled_str: i32,
    pub scaled_intel: i32,
    pub scaler_val: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnActorOutcome {
    Success(SpawnedActorInfo),
    CapacityExceeded { max: usize, template_name: String },
    TemplateNotFound,
}

impl WorldState {
    pub fn spawn_actor(
        &mut self,
        content: &ContentPack,
        config: SpawnActorConfig<'_>,
    ) -> SpawnActorOutcome {
        let Some(template) = self.actor(content, config.template_id).cloned() else {
            eprintln!(
                "[cinder] spawn actor: template '{}' not found",
                config.template_id
            );
            return SpawnActorOutcome::TemplateNotFound;
        };

        if let Some(max) = config.max_active_instances {
            let active_count = self.active_spawned_actor_count(content, config.template_id);
            if active_count >= max {
                return SpawnActorOutcome::CapacityExceeded {
                    max,
                    template_name: template.name.clone(),
                };
            }
        }

        let template_initial_hostile = template.initial_hostile;
        let target_room_id = config
            .room_id
            .filter(|r| !r.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.current_room_id.clone());

        self.spawn_counter += 1;
        let instance_id = format!("{}-{}", config.template_id, self.spawn_counter);
        let mut instance = template;
        instance.id = instance_id.clone();
        instance.room_id = target_room_id.clone();

        let health_stat_id = if content.settings.combat.health_stat_id.is_empty() {
            "hp"
        } else {
            &content.settings.combat.health_stat_id
        };

        let (scaled_hp, scaled_str, scaled_intel, scaler_val) =
            if let Some(scaler_id) = config.scale_with_actor_id {
                let stat_name = config.scale_stat.unwrap_or("intelligence");
                let scaler = self.effective_actor_stat(content, scaler_id, stat_name);
                let base_hp = instance
                    .initial_stats
                    .get(health_stat_id)
                    .copied()
                    .unwrap_or(4);
                let base_str = instance.initial_stats.get("strength").copied().unwrap_or(2);
                let base_intel = instance
                    .initial_stats
                    .get("intelligence")
                    .copied()
                    .unwrap_or(2);
                let hp = (base_hp + scaler).max(1);
                let str_val = (base_str + scaler / 3).max(1);
                let intel_val = (base_intel + scaler / 2).max(1);
                instance
                    .initial_stats
                    .insert(health_stat_id.to_string(), hp);
                instance
                    .initial_stats
                    .insert("strength".to_string(), str_val);
                instance
                    .initial_stats
                    .insert("intelligence".to_string(), intel_val);
                (hp, str_val, intel_val, scaler)
            } else {
                let hp = instance
                    .initial_stats
                    .get(health_stat_id)
                    .copied()
                    .unwrap_or(4);
                let str_val = instance.initial_stats.get("strength").copied().unwrap_or(2);
                let intel_val = instance
                    .initial_stats
                    .get("intelligence")
                    .copied()
                    .unwrap_or(2);
                (hp, str_val, intel_val, 0)
            };

        let actor_name = instance.name.clone();
        self.actor_stats
            .insert(instance_id.clone(), instance.initial_stats.clone());
        self.initial_actor_stats
            .insert(instance_id.clone(), instance.initial_stats.clone());
        self.actor_room_overrides
            .insert(instance_id.clone(), target_room_id.clone());
        self.actor_skills.insert(
            instance_id.clone(),
            instance
                .skills
                .iter()
                .map(|skill| skill.id().to_string())
                .collect(),
        );
        self.mark_actor_room_visited(&instance_id, &target_room_id);
        self.spawned_actors.insert(instance_id.clone(), instance);

        let stance = config.stance.unwrap_or(if template_initial_hostile {
            ActorStance::Hostile
        } else {
            ActorStance::Allied
        });
        self.set_actor_stance(content, &instance_id, stance, config.follows_player);

        SpawnActorOutcome::Success(SpawnedActorInfo {
            instance_id,
            template_id: config.template_id.to_string(),
            actor_name,
            target_room_id,
            scaled_hp,
            scaled_str,
            scaled_intel,
            scaler_val,
        })
    }
}
