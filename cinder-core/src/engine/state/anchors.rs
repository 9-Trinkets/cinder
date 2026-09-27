use super::WorldState;

impl WorldState {
    /// Adds a temporary chalk anchor in the specified room. If the maximum capacity of 3
    /// is exceeded, the oldest anchor room is evicted and returned.
    pub fn add_chalk_anchor(&mut self, room_id: &str) -> Option<String> {
        if let Some(pos) = self.chalk_anchors.iter().position(|r| r == room_id) {
            // Re-tracing in the same room refreshes its place to newest
            self.chalk_anchors.remove(pos);
            self.chalk_anchors.push(room_id.to_string());
            None
        } else {
            let evicted = if self.chalk_anchors.len() >= 3 {
                Some(self.chalk_anchors.remove(0))
            } else {
                None
            };
            self.chalk_anchors.push(room_id.to_string());
            evicted
        }
    }

    /// Removes a temporary chalk anchor from the specified room if present.
    pub fn remove_chalk_anchor(&mut self, room_id: &str) -> bool {
        if let Some(pos) = self.chalk_anchors.iter().position(|r| r == room_id) {
            self.chalk_anchors.remove(pos);
            true
        } else {
            false
        }
    }

    /// Whether the specified room has an active temporary chalk anchor.
    pub fn has_chalk_anchor(&self, room_id: &str) -> bool {
        self.chalk_anchors.iter().any(|r| r == room_id)
    }

    /// Whether any teleport destination (permanent or chalk anchor) is currently accessible.
    pub fn has_any_teleport_anchor(&self) -> bool {
        !self.chalk_anchors.is_empty()
            || self.story_vars.get("anchor_floor4_platform") == Some("true")
            || self.story_vars.get("anchor_floor5_gate") == Some("true")
    }
}
