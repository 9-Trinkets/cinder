use super::*;

impl WorldState {
    pub fn actor_room_id<'a>(&'a self, actor_id: &str, default_room_id: &'a str) -> &'a str {
        self.actor_room_overrides
            .get(actor_id)
            .map(String::as_str)
            .unwrap_or(default_room_id)
    }

    pub fn actor_has_visited_room(&self, actor_id: &str, room_id: &str) -> bool {
        self.actor_known_room_ids
            .get(actor_id)
            .is_some_and(|rooms| rooms.contains(room_id))
    }

    pub fn mark_actor_room_visited(&mut self, actor_id: &str, room_id: &str) {
        self.actor_known_room_ids
            .entry(actor_id.to_string())
            .or_default()
            .insert(room_id.to_string());
    }

    pub fn actor_has_seen_feature(&self, actor_id: &str, room_id: &str, feature_id: &str) -> bool {
        self.actor_known_feature_ids
            .get(actor_id)
            .is_some_and(|features| features.contains(&feature_key(room_id, feature_id)))
    }

    pub fn mark_actor_feature_seen(&mut self, actor_id: &str, room_id: &str, feature_id: &str) {
        self.actor_known_feature_ids
            .entry(actor_id.to_string())
            .or_default()
            .insert(feature_key(room_id, feature_id));
    }

    pub fn actor_has_studied_actor(&self, actor_id: &str, target_actor_id: &str) -> bool {
        self.actor_known_actor_ids
            .get(actor_id)
            .is_some_and(|actors| actors.contains(target_actor_id))
    }

    pub fn mark_actor_studied_actor(&mut self, actor_id: &str, target_actor_id: &str) {
        self.actor_known_actor_ids
            .entry(actor_id.to_string())
            .or_default()
            .insert(target_actor_id.to_string());
    }

    pub fn push_actor_observation_note(&mut self, actor_id: &str, note: String) {
        let notes = self
            .actor_recent_observation_notes
            .entry(actor_id.to_string())
            .or_default();
        notes.push(note);
        if notes.len() > 6 {
            notes.drain(..notes.len() - 6);
        }
    }

    pub fn actor_recent_observation_notes(&self, actor_id: &str) -> &[String] {
        self.actor_recent_observation_notes
            .get(actor_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn actor_has_observed_room(&self, actor_id: &str, room_id: &str) -> bool {
        self.actor_observed_room_ids
            .get(actor_id)
            .is_some_and(|rooms| rooms.contains(room_id))
    }

    pub fn mark_actor_observed_room(&mut self, actor_id: &str, room_id: &str) {
        self.actor_observed_room_ids
            .entry(actor_id.to_string())
            .or_default()
            .insert(room_id.to_string());
    }
}
