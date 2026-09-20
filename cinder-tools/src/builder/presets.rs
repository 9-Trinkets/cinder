use super::floor::FloorBuilder;
use super::zone::{Direction, Zone};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Floor4RoomEntry {
    x: f64,
    y: f64,
    id: String,
    title: String,
    summary: String,
    inspect_text: String,
    feature_label: String,
    feature_aliases: Vec<String>,
    feature_inspect: String,
}

pub fn build_floor1() -> FloorBuilder {
    let mut builder = FloorBuilder::new("upper-works", "The Cave", 9, 9);
    builder.add_zone(Zone::new("upper-works", 0, 8, 0, 8));

    let archetypes = [
        (
            "A Cell of Fitted Stone",
            "A small room of fitted stone blocks. The floor is laid in a grid, lines crossing at every intersection.",
            "Thin lines cross the floor at right angles, forming a pattern of squares. The intersections are marked with small depressions, worn smooth by something placed and removed many times.",
            "a grid scored into the floor",
            vec!["grid".into(), "floor".into(), "line".into(), "intersection".into()],
            "Thin lines cross the floor at right angles, forming a pattern of squares. The intersections are marked with small depressions, worn smooth by something placed and removed many times.",
        ),
        (
            "A Corridor of Worn Flags",
            "A corridor paved in stone flags, each one cut to the same size. The joints between them form straight lines.",
            "Each flag is cut to fit precisely against its neighbors. The joints form lines that cross at right angles, a grid drawn in stone.",
            "the fitted flagstones",
            vec!["flagstone".into(), "flag".into(), "floor".into(), "joint".into()],
            "Each flag is cut to fit precisely against its neighbors. The joints form lines that cross at right angles, a grid drawn in stone.",
        ),
        (
            "A Passage of Scored Walls",
            "A passage where the walls are scored with straight lines, etched at regular intervals into the stone.",
            "The lines are cut with precision, crossing at exact right angles. They form a small grid on the wall, like a board marked for a game no one is playing.",
            "scored lines in the wall",
            vec!["line".into(), "wall".into(), "score".into(), "mark".into()],
            "The lines are cut with precision, crossing at exact right angles. They form a small grid on the wall, like a board marked for a game no one is playing.",
        ),
        (
            "A Hall of Crossing Paths",
            "A hall where paths cross at right angles, worn into the floor by traffic that has long since stopped.",
            "Two paths cross here, worn into the stone. Where they meet, the floor is polished smooth. The intersections feel deliberate, like points on a grid.",
            "worn crossing paths",
            vec!["path".into(), "cross".into(), "floor".into(), "worn".into()],
            "Two paths cross here, worn into the stone. Where they meet, the floor is polished smooth. The intersections feel deliberate, like points on a grid.",
        ),
    ];

    for r in 1..=9 {
        for c in 1..=9 {
            let x = c - 1;
            let y = r - 1;
            let id = format!("r{r}c{c}");

            let (title, summary, inspect, flabel, faliases, finspect) = match (r, c) {
                (3, 3) => (
                    "A Hall of Dark Stone",
                    "The stone here is darker than the rest. A golem of dark granite stands at the center, still as architecture. Its eyes are closed.",
                    "The stone here is darker than the rest. A golem of dark granite stands at the center, still as architecture. Its eyes are closed.",
                    "a grid of dark stone",
                    vec!["grid".into(), "stone".into(), "dark".into(), "floor".into()],
                    "The floor is laid in dark stone, each block precise as a placed thing. The grid lines here are deeper, as if this room matters more than the others.",
                ),
                (3, 7) => (
                    "A Chamber of Pale Pillars",
                    "The pillars here are pale, almost white. A golem of pale marble stands among them, motionless. It does not look at you.",
                    "The pillars here are pale, almost white. A golem of pale marble stands among them, motionless. It does not look at you.",
                    "pale stone pillars",
                    vec!["pillar".into(), "pale".into(), "stone".into(), "marble".into()],
                    "The pillars are arranged in a pattern that feels deliberate. They mark positions on the floor like pieces on a board.",
                ),
                (5, 5) => (
                    "The Center Hall",
                    "The room is open and the floor is laid in a precise grid. Lines cross at every intersection. At the center stands a golem of pale stone, taller than the rest, perfectly still. It does not move. It does not need to.",
                    "The room is open and the floor is laid in a precise grid. Lines cross at every intersection. At the center stands a golem of pale stone, taller than the rest, perfectly still. It does not move. It does not need to.",
                    "a deep central grid",
                    vec!["grid".into(), "center".into(), "line".into(), "intersection".into()],
                    "Lines cross the floor at precise intervals, forming a grid of squares. The intersections are worn smooth. This is the center of everything.",
                ),
                (7, 3) => (
                    "A Vault of Dark Flags",
                    "The floor is laid in dark stone flags. A golem of dark granite stands in the corner, watching nothing.",
                    "The floor is laid in dark stone flags. A golem of dark granite stands in the corner, watching nothing.",
                    "dark fitted flags",
                    vec!["flag".into(), "stone".into(), "dark".into(), "floor".into()],
                    "Each flag is cut from dark stone, fitted precisely against its neighbors. The joints form a grid.",
                ),
                (7, 7) => (
                    "A Hall of Pale Stone",
                    "The stone here is pale, almost white, and catches what little light there is. A pale golem stands in the center, waiting.",
                    "The stone here is pale, almost white, and catches what little light there is. A pale golem stands in the center, waiting.",
                    "luminous pale stone",
                    vec!["stone".into(), "pale".into(), "light".into(), "glow".into()],
                    "The stone holds light like a memory. The grid lines on the floor glow faintly, as if something is beneath them.",
                ),
                _ => {
                    let idx = (r * 3 + c * 7) % archetypes.len();
                    archetypes[idx].clone()
                }
            };

            builder.add_room(x as f64, y as f64, id, title, summary, inspect, flabel, faliases, finspect);
        }
    }

    builder.wire_internal_zones(&[]);
    builder
}

fn connect_segment(
    builder: &mut FloorBuilder,
    rooms: &[&str],
    fwd_label: &str,
    fwd_aliases: &[&str],
    back_label: &str,
    back_aliases: &[&str],
) {
    for window in rooms.windows(2) {
        let a = window[0];
        let b = window[1];
        builder.add_exit(
            a,
            b,
            fwd_label,
            fwd_aliases.iter().map(|s| s.to_string()).collect(),
            Some(fwd_label.to_string()),
            "",
        );
        builder.add_exit(
            b,
            a,
            back_label,
            back_aliases.iter().map(|s| s.to_string()).collect(),
            Some(back_label.to_string()),
            "",
        );
    }
}

pub fn build_floor4() -> FloorBuilder {
    let mut builder = FloorBuilder::new("the-commoners", "The Worker Village", 25, 23);

    const RAW_ROOMS: &str = include_str!("../../data/floor4_rooms.json");
    let entries: Vec<Floor4RoomEntry> = serde_json::from_str(RAW_ROOMS).expect("valid floor4 room data");

    for entry in entries {
        builder.add_room(
            entry.x,
            entry.y,
            entry.id,
            entry.title,
            entry.summary,
            entry.inspect_text,
            entry.feature_label,
            entry.feature_aliases,
            entry.feature_inspect,
        );
    }

    let ne = &["northeast", "ne"];
    let sw = &["southwest", "sw"];
    let se = &["southeast", "se"];
    let nw = &["northwest", "nw"];
    let w = &["west", "w"];
    let e = &["east", "e"];

    // Village Triangle (4x4x4 perimeter = 9 rooms + 3 interior = 12 rooms)
    let village_left = [
        "village_sw_corner",
        "village_west_1",
        "village_west_2",
        "village_north_gate",
    ];
    let village_right = [
        "village_north_gate",
        "village_east_1",
        "village_east_2",
        "village_se_corner",
    ];
    let village_bottom = [
        "village_se_corner",
        "village_south_1",
        "village_south_2",
        "village_sw_corner",
    ];
    connect_segment(&mut builder, &village_left, "Northeast", ne, "Southwest", sw);
    connect_segment(&mut builder, &village_right, "Southeast", se, "Northwest", nw);
    connect_segment(&mut builder, &village_bottom, "West", w, "East", e);

    // Village Hubs
    builder.connect_bidirectional("village_square", "village_north_gate", Direction::North);
    builder.connect_bidirectional("village_square", "village_south_1", Direction::South);
    builder.connect_bidirectional("village_square", "elder_hut", Direction::West);
    builder.connect_bidirectional("village_square", "baker_hut", Direction::East);
    builder.add_exit(
        "village_square",
        "oh",
        "Up to the Heart-Pit",
        vec!["up".into(), "u".into(), "pit".into(), "stairs".into(), "heart".into()],
        Some("Up".into()),
        "",
    );

    builder
}
