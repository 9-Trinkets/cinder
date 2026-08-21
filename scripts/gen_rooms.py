#!/usr/bin/env python3
"""Generate 9x9 grid rooms.json for Layla's first floor.
Uses 4 room archetypes distributed across the grid. Star point rooms are special."""
import json

ROWS = 9
COLS = 9
STAR_POINTS = {(3, 3), (3, 7), (5, 5), (7, 3), (7, 7)}

# 4 room archetypes
ARCHETYPES = [
    {
        "title": "A Cell of Fitted Stone",
        "summary": "A small room of fitted stone blocks. The floor is laid in a grid, lines crossing at every intersection.",
        "feature_label": "a grid scored into the floor",
        "feature_aliases": ["grid", "floor", "line", "intersection"],
        "feature_inspect": "Thin lines cross the floor at right angles, forming a pattern of squares. The intersections are marked with small depressions, worn smooth by something placed and removed many times."
    },
    {
        "title": "A Corridor of Worn Flags",
        "summary": "A corridor paved in stone flags, each one cut to the same size. The joints between them form straight lines.",
        "feature_label": "the fitted flagstones",
        "feature_aliases": ["flagstone", "flag", "floor", "joint"],
        "feature_inspect": "Each flag is cut to fit precisely against its neighbors. The joints form lines that cross at right angles, a grid drawn in stone."
    },
    {
        "title": "A Passage of Scored Walls",
        "summary": "A passage where the walls are scored with straight lines, etched at regular intervals into the stone.",
        "feature_label": "scored lines in the wall",
        "feature_aliases": ["line", "wall", "score", "mark"],
        "feature_inspect": "The lines are cut with precision, crossing at exact right angles. They form a small grid on the wall, like a board marked for a game no one is playing."
    },
    {
        "title": "A Hall of Crossing Paths",
        "summary": "A hall where paths cross at right angles, worn into the floor by traffic that has long since stopped.",
        "feature_label": "worn crossing paths",
        "feature_aliases": ["path", "cross", "floor", "worn"],
        "feature_inspect": "Two paths cross here, worn into the stone. Where they meet, the floor is polished smooth. The intersections feel deliberate, like points on a grid."
    },
]

# Special rooms at star points
STAR_ROOMS = {
    (3, 3): {
        "title": "A Hall of Dark Stone",
        "summary": "The stone here is darker than the rest. A golem of dark granite stands at the center, still as architecture. Its eyes are closed.",
        "feature_label": "a grid of dark stone",
        "feature_aliases": ["grid", "stone", "dark", "floor"],
        "feature_inspect": "The floor is laid in dark stone, each block precise as a placed thing. The grid lines here are deeper, as if this room matters more than the others."
    },
    (3, 7): {
        "title": "A Chamber of Pale Pillars",
        "summary": "The pillars here are pale, almost white. A golem of pale marble stands among them, motionless. It does not look at you.",
        "feature_label": "pale stone pillars",
        "feature_aliases": ["pillar", "pale", "stone", "marble"],
        "feature_inspect": "The pillars are arranged in a pattern that feels deliberate. They mark positions on the floor like pieces on a board."
    },
    (5, 5): {
        "title": "The Center Hall",
        "summary": "The room is open and the floor is laid in a precise grid. Lines cross at every intersection. At the center stands a golem of pale stone, taller than the rest, perfectly still. It does not move. It does not need to.",
        "feature_label": "a deep central grid",
        "feature_aliases": ["grid", "center", "line", "intersection"],
        "feature_inspect": "Lines cross the floor at precise intervals, forming a grid of squares. The intersections are worn smooth. This is the center of everything."
    },
    (7, 3): {
        "title": "A Vault of Dark Flags",
        "summary": "The floor is laid in dark stone flags. A golem of dark granite stands in the corner, watching nothing.",
        "feature_label": "dark fitted flags",
        "feature_aliases": ["flag", "stone", "dark", "floor"],
        "feature_inspect": "Each flag is cut from dark stone, fitted precisely against its neighbors. The joints form a grid."
    },
    (7, 7): {
        "title": "A Hall of Pale Stone",
        "summary": "The stone here is pale, almost white, and catches what little light there is. A pale golem stands in the center, waiting.",
        "feature_label": "luminous pale stone",
        "feature_aliases": ["stone", "pale", "light", "glow"],
        "feature_inspect": "The stone holds light like a memory. The grid lines on the floor glow faintly, as if something is beneath them."
    },
}


def make_room(row, col):
    room_id = f"r{row}c{col}"
    is_star = (row, col) in STAR_POINTS
    is_center = (row, col) == (5, 5)

    if is_star:
        spec = STAR_ROOMS[(row, col)]
    else:
        # Pick archetype based on position for variety
        idx = (row * 3 + col * 7) % len(ARCHETYPES)
        spec = ARCHETYPES[idx]

    exits = []
    if row > 1:
        exits.append({"room_id": f"r{row-1}c{col}", "label": "North", "aliases": ["n"], "menu_label": "North"})
    if row < ROWS:
        exits.append({"room_id": f"r{row+1}c{col}", "label": "South", "aliases": ["s"], "menu_label": "South"})
    if col > 1:
        exits.append({"room_id": f"r{row}c{col-1}", "label": "West", "aliases": ["w"], "menu_label": "West"})
    if col < COLS:
        exits.append({"room_id": f"r{row}c{col+1}", "label": "East", "aliases": ["e"], "menu_label": "East"})

    return {
        "id": room_id,
        "title": spec["title"],
        "summary": spec["summary"],
        "inspect_text": spec["summary"],
        "features": [{
            "id": f"{room_id}-feature",
            "label": spec["feature_label"],
            "aliases": spec["feature_aliases"],
            "inspect_text": spec["feature_inspect"]
        }],
        "exits": exits
    }


def main():
    rooms = [make_room(r, c) for r in range(1, ROWS + 1) for c in range(1, COLS + 1)]

    with open("content/layla/locales/en/rooms.json", "w") as f:
        json.dump(rooms, f, indent=2)
        f.write("\n")

    print(f"Generated {len(rooms)} rooms")
    # Verify adjacency
    ids = {f"r{r}c{c}" for r in range(1, ROWS+1) for c in range(1, COLS+1)}
    opp = {"North": "South", "South": "North", "East": "West", "West": "East"}
    for room in rooms:
        for ex in room["exits"]:
            assert ex["room_id"] in ids, f"Bad exit: {room['id']} -> {ex['room_id']}"
            target = next(r for r in rooms if r["id"] == ex["room_id"])
            back = opp[ex["label"]]
            assert any(e["room_id"] == room["id"] and e["label"] == back for e in target["exits"]), \
                f"Non-bidirectional: {room['id']} -> {ex['room_id']} ({ex['label']})"
    print("Adjacency verified")


if __name__ == "__main__":
    main()
