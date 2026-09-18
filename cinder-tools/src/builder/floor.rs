use cinder_core::content::types::{
    MapDefinition, MapRoomDefinition, RoomDefinition, RoomExitDefinition, RoomFeatureDefinition,
};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use super::zone::{Direction, Zone};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RoomDraft {
    pub x: f64,
    pub y: f64,
    pub id: String,
    pub title: String,
    pub summary: String,
    pub inspect_text: String,
    pub features: Vec<RoomFeatureDefinition>,
}

pub struct FloorBuilder {
    pub map_id: String,
    pub map_label: String,
    pub width: usize,
    pub height: usize,
    pub zones: Vec<Zone>,
    pub grid: BTreeMap<(usize, usize), String>,
    pub rooms: Vec<RoomDraft>,
    pub exits: HashMap<String, Vec<RoomExitDefinition>>,
}

impl FloorBuilder {
    pub fn new(map_id: impl Into<String>, map_label: impl Into<String>, width: usize, height: usize) -> Self {
        Self {
            map_id: map_id.into(),
            map_label: map_label.into(),
            width,
            height,
            zones: Vec::new(),
            grid: BTreeMap::new(),
            rooms: Vec::new(),
            exits: HashMap::new(),
        }
    }

    pub fn add_zone(&mut self, zone: Zone) {
        self.zones.push(zone);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_room(
        &mut self,
        x: f64,
        y: f64,
        id: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        inspect_text: impl Into<String>,
        feature_label: impl Into<String>,
        feature_aliases: Vec<String>,
        feature_inspect: impl Into<String>,
    ) {
        let id_str = id.into();
        let feature = RoomFeatureDefinition {
            id: format!("{id_str}-feature"),
            label: feature_label.into(),
            aliases: feature_aliases,
            allow_rest: false,
            consumables: Vec::new(),
            inspect_text: feature_inspect.into(),
        };

        if x >= 0.0 && y >= 0.0 && x.fract() == 0.0 && y.fract() == 0.0 {
            self.grid.insert((x as usize, y as usize), id_str.clone());
        }

        self.rooms.push(RoomDraft {
            x,
            y,
            id: id_str.clone(),
            title: title.into(),
            summary: summary.into(),
            inspect_text: inspect_text.into(),
            features: vec![feature],
        });
        self.exits.entry(id_str).or_default();
    }

    pub fn add_exit(
        &mut self,
        from_id: &str,
        to_id: &str,
        label: impl Into<String>,
        aliases: Vec<String>,
        menu_label: Option<String>,
        requires_story_var: impl Into<String>,
    ) {
        let exit = RoomExitDefinition {
            room_id: to_id.to_string(),
            label: label.into(),
            aliases,
            menu_label,
            requires_story_var: requires_story_var.into(),
        };
        self.exits.entry(from_id.to_string()).or_default().push(exit);
    }

    pub fn connect_bidirectional(&mut self, id_a: &str, id_b: &str, dir: Direction) {
        let opp = dir.opposite();
        self.add_exit(id_a, id_b, dir.label(), dir.default_aliases(), Some(dir.label().to_string()), "");
        self.add_exit(id_b, id_a, opp.label(), opp.default_aliases(), Some(opp.label().to_string()), "");
    }

    #[allow(dead_code)]
    pub fn connect_gated(
        &mut self,
        id_a: &str,
        id_b: &str,
        dir: Direction,
        label_a: impl Into<String>,
        aliases_a: Vec<String>,
        requires_story_var: impl Into<String>,
    ) {
        let opp = dir.opposite();
        self.add_exit(id_a, id_b, label_a, aliases_a, Some(dir.label().to_string()), requires_story_var);
        self.add_exit(id_b, id_a, opp.label(), opp.default_aliases(), Some(opp.label().to_string()), "");
    }

    pub fn zone_for(&self, x: usize, y: usize) -> Option<&Zone> {
        self.zones.iter().find(|z| z.contains(x, y))
    }

    pub fn wire_internal_zones(&mut self, exclude_pairs: &[(usize, usize, usize, usize)]) {
        let coords: Vec<(usize, usize)> = self.grid.keys().copied().collect();
        for &(x, y) in &coords {
            let Some(zone) = self.zone_for(x, y) else { continue };
            let zone_name = zone.name.clone();
            let from_id = self.grid[&(x, y)].clone();

            // East neighbor
            if x + 1 < self.width
                && let Some(to_id) = self.grid.get(&(x + 1, y))
                    && let Some(target_zone) = self.zone_for(x + 1, y)
                        && target_zone.name == zone_name && !exclude_pairs.contains(&(x, y, x + 1, y)) {
                            let to_id_clone = to_id.clone();
                            self.connect_bidirectional(&from_id, &to_id_clone, Direction::East);
                        }

            // South neighbor
            if y + 1 < self.height
                && let Some(to_id) = self.grid.get(&(x, y + 1))
                    && let Some(target_zone) = self.zone_for(x, y + 1)
                        && target_zone.name == zone_name && !exclude_pairs.contains(&(x, y, x, y + 1)) {
                            let to_id_clone = to_id.clone();
                            self.connect_bidirectional(&from_id, &to_id_clone, Direction::South);
                        }
        }
    }

    #[allow(dead_code)]
    pub fn connect_adjacent_zones(&mut self, zone_a_name: &str, zone_b_name: &str, dir: Direction) {
        let coords: Vec<(usize, usize)> = self.grid.keys().copied().collect();
        for &(x, y) in &coords {
            let Some(za) = self.zone_for(x, y) else { continue };
            if za.name != zone_a_name { continue; }

            let (nx, ny) = match dir {
                Direction::North => if y > 0 { (x, y - 1) } else { continue },
                Direction::South => (x, y + 1),
                Direction::East => (x + 1, y),
                Direction::West => if x > 0 { (x - 1, y) } else { continue },
                _ => continue,
            };

            if let Some(target_id) = self.grid.get(&(nx, ny))
                && let Some(zb) = self.zone_for(nx, ny)
                    && zb.name == zone_b_name {
                        let from_id = self.grid[&(x, y)].clone();
                        let to_id = target_id.clone();
                        self.connect_bidirectional(&from_id, &to_id, dir);
                    }
        }
    }

    pub fn build(mut self) -> (Vec<RoomDefinition>, MapDefinition) {
        let mut built_rooms = Vec::with_capacity(self.rooms.len());
        let mut map_rooms = Vec::with_capacity(self.rooms.len());

        for draft in self.rooms {
            let exits = self.exits.remove(&draft.id).unwrap_or_default();
            map_rooms.push(MapRoomDefinition {
                room_id: draft.id.clone(),
                x: draft.x,
                y: draft.y,
            });
            built_rooms.push(RoomDefinition {
                id: draft.id,
                title: draft.title,
                summary: draft.summary,
                inspect_text: draft.inspect_text,
                allow_rest: false,
                features: draft.features,
                exits,
                descriptions: Vec::new(),
            });
        }

        let map = MapDefinition {
            id: self.map_id,
            label: self.map_label,
            rooms: map_rooms,
            reveal_conditions: Vec::new(),
        };

        (built_rooms, map)
    }

    pub fn export_to_pack(
        self,
        pack_dir: &Path,
        locale: &str,
        is_previous_floor_fn: impl Fn(&str) -> bool,
    ) -> Result<(usize, usize), Box<dyn std::error::Error>> {
        let (floor_rooms, floor_map) = self.build();
        let floor_count = floor_rooms.len();

        let locale_dir = pack_dir.join("locales").join(locale);
        let rooms_path = locale_dir.join("rooms.json");
        let maps_path = locale_dir.join("maps.json");

        // Read and merge rooms
        let existing_rooms: Vec<RoomDefinition> = serde_json::from_str(&fs::read_to_string(&rooms_path)?)?;
        let mut merged_rooms: Vec<RoomDefinition> = existing_rooms
            .into_iter()
            .filter(|r| is_previous_floor_fn(&r.id))
            .collect();
        merged_rooms.extend(floor_rooms);

        fs::write(&rooms_path, serde_json::to_string_pretty(&merged_rooms)? + "\n")?;

        // Read and merge maps
        let mut existing_maps: Vec<MapDefinition> = serde_json::from_str(&fs::read_to_string(&maps_path)?)?;
        if let Some(pos) = existing_maps.iter().position(|m| m.id == floor_map.id) {
            existing_maps[pos] = floor_map;
        } else {
            existing_maps.push(floor_map);
        }

        fs::write(&maps_path, serde_json::to_string_pretty(&existing_maps)? + "\n")?;

        Ok((floor_count, merged_rooms.len()))
    }
}
