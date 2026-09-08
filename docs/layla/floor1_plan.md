# Floor 1 — The Goblin Cave (Design Plan)

Living design plan for **level 1** of Layla: the goblin cave. The plan is the
design-side companion for `content/layla/` — as content is built, this doc
holds the *intent* (premise, cast, fiction bible, feature-surface order, engine
knobs) so the level's tutorial-by-discovery arc is never lost.

> Status: **redesigned as a goblin cave, fiction settled and built.** The sensory
> descriptions below are canon and were ported nearly verbatim into `rooms.json` /
> `actors.json`. Supersedes the earlier "stone board + stone warden" and the first
> goblin pass. Implementation decisions are recorded under
> [Open Threads](#open-threads).

---

## Premise

Layla wakes on cold stone in one of the golems' guard rooms, next to a **golem**
that is watching her silently. She should be afraid — but feels oddly calm.
Despite having no memory, she feels like she's been here before.

The floor is a **goblin cave**, not a prison. A **goblin tribe** lives here, and
its **shaman made the golems**. The cave is theirs: dens, bone piles, hot
fires, hunting grounds — and at the very center, the shaman's work-ring where
the golems were shaped.

She is not a prisoner here. She is a stranger in someone's home, and she has to
learn the rules of the floor to earn her way down.

---

## The Cave (fiction bible — sensory)

The map still reads as a grid for play, but the fiction is **organic stone**:
rust-colored rock, low ceilings, pockets of space worn smooth by generations of
small feet. Two soft constants carry the floor:

- **Smell.** Everywhere: damp earth and old smoke, animal musk, the sweet-sour
  rot of meat hanging too close to fire. Near the dens it thickens — wet fur,
  piss, crushed herbs. Near the center it sharpens to ash and hot clay.
- **Sound.** Drips. Wind moaning up a chimney. A far-off, rhythmic *tock… tock*
  of something striking stone — the shaman's work. Now and then goblin voices:
  not speech, but a wet chitter and a shrill yip, bouncing off walls. Somewhere
  a small drum never quite stops.

**Light.** Goblins like fire. This floor is never fully dark — gutting fire-pits,
  ember trays, the shaman's coals. Light comes low, orange, and it throws long
  shadows that look like they're moving on their own.

**The grid, if any.** The old scored "grid" concept is gone. What the map reads
  as rows and columns are, in fiction, the **worn patrolling tracks** goblins
  walk between dens and hunting grounds — faint shallow channels in the cave
  floor, older than the tribe. Layla, who counts and measures, notices the
  tracks cross at right angles. (Go hint, kept quiet.)

---

## The Tribe (fiction bible — the creatures)

### Goblin hunters (×4 — the weakest enemies on the floor)

Small, quick, pack-minded. Not a horde — a few, and they hunt like dogs.

- **Body / size.** About the height of a child. Thin and stooped, arms long and
  knuckly, bellies round over skinny legs. Big heads, oversized round ears that
  swivel toward every sound. Flat faces, wide nostrils, mouths full of small
  sharp teeth that never close all the way.
- **Color / skin.** Dull grey-green, mottled with darker patches like old mud.
  Coarse and dry; the palms of their hands are pale from always touching stone.
  Eyes the color of river-silt, with slit pupils. In firelight they glint like
  wet pebbles.
- **Smell.** Damp fur, stale smoke, and a sour musk — the smell of living too
  close to your own kind. When they're angry it sharpens.
- **Noises.** Chittering and clicking as a kind of language. When they hunt,
  they're almost silent — then a shrill yip as one spots prey, and the others
  answer it. A hiss when they bare those teeth. Their feet patter like rain on
  stone.
- **Gear / behavior.** Tangled scraps of hide, bone shards strung on gut,
  leather pouches. They carry crude chisel-axes and skinning knives. They chase
  small game and each other; a lone goblin runs, a pack stands. One of them is
  almost always cooking something.
- **On the page.** Enemies in material terms: rough, damp, dog-like, cheap.
  Not evil — hungry and suspicious of anything that walks on two legs that
  isn't one of them.

### The goblin shaman (×1 — the boss)

Older, larger, quieter. The floor's maker.

- **Body / size.** Twice the hunters' mass and hunched with age, but broad —
  a stone block covered in hide. Long grey fingers. A face that has fewer teeth
  and more scars, and one milky eye that moves where the clear one doesn't.
- **Color / skin.** Ashy grey, cracking at the knuckles and neck, stained with
  char and pale clay where it shapes stone. Its hands are lime-white from
  working chalk.
- **Smell.** Ash, burned clay, dried herbs, old blood. The strongest single
  smell on the floor.
- **Noises.** It almost never chitters. It drums — a staff, struck on the floor,
  *tock… tock… tock* — and it mutters in a low rumble that is less a voice than
  a second drum under the first. When it stands, its joints pop and it hisses
  through its teeth.
- **Role.** Made the golems. Its stamp is on every one of them — the same
  chalk-ring marks, the same hand in the shaping. It cannot be converted; only
  defeated. It drops the **shaman's ring**.
- **The thread to Layla.** The shaman makes rings and shapes servants from
  living stone. Layla traces rings and draws servants into step. Same mark,
  same craft, same boarded life — the sigil she learns is the shaman's trade.
  Keep that echo in prose, never say it.

### Golems (×4 — the shaman's constructs)

Crude, patient, made. Still, silent, and no longer elegant "architecture."

- **Body / material.** Rough-cut stone — dark volcanic rock and pale river
  stone — lumped together like a child's snowman twice grown. Broad slabs for
  shoulders, a head shaped only just enough to have a face. No mouth. Eyes of
  flat polished pebble set in sockets.
- **Color / skin.** Two pairs, as in the old content: **dark** (near-black
  granite) and **pale** (bone-white river stone), but now visibly *built* —
  seams where slabs meet, thumb-deep pocks where the shaman's hands pressed.
  Chalk-ring marks on chest and brow: the maker's stamp.
- **Smell.** Cold stone and chalk dust. Clean next to the goblins.
- **Noises.** None. They do not breathe, do not shift, do not creak. The
  scariest thing about them is that they make no sound at all, ever.
- **Behavior.** Still pieces. They hold rooms the way furniture holds rooms.
  They do not know the game. They can be **charmed** by enclosing rings — drawn
  into step with Layla (first charm → **party**).
- **On the page.** Material, not moral. The golem Layla wakes beside is just
  a made thing that is looking at her.

---

## Cast & Power Economy

Nine actors on the floor:

| Actor | Count | Role | Strength |
|---|---|---|---|
| **goblin shaman** | 1 | Center of the cave; made the golems. The L1 boss. Cannot be converted, only defeated. | **> golems** |
| **golems** | 4 | The shaman's constructs, holding the four off-center guard rooms. Silent, neutral, made things that don't know the game. Charmable. | mid |
| **goblins** | 4 | Hunt the cave. Conscious and hostile. The weakest enemies on the floor. | **< golems** |

**Strength ladder:** shaman > golems > goblins.

- **Goblins** are the natural "first mob" — the ones Layla can realistically
  defeat, and the defeat that surfaces her **stats**.
- **Golems** are charmable servants: enclosing rings bring them into step with
  her (first charm → **party**).
- **Shaman** anchors the top (above the old `golem-boss` anchor of HP 20 /
  strength 4).

## Loot & the justified chisel/salve

The goblins' **hunting** is why a weapon and healing exist on the floor. Killing
hunt-goblins (or looting their camps) yields their gear:

- **Chisel-axe** — a goblin skinning tool: a wedge of chipped stone half-wrapped
  in hide. The **weapon** Layla picks up before facing the shaman. (Rename of
  the old `iron-chisel`; drops from a goblin or lies in a camp.)
- **Moss-poultice** — a wad of crushed herbs and moss, wet, tied in a scrap of
  hide. Goblin medicine; heals when used. (Rename of `herb-salve`.)

Starting inventory is **empty** in intent: Layla wakes with nothing but what she
finds. *Decision:* the **magic-chalk is a deliberate starting item** —
`starting_items = { magic-chalk: 1 }` — and stays on her for the whole floor.
No loose-on-the-floor pickup, no `requires_equipped_item` gate on `trace`: the
chalk is simply on her and the handler teaches the sigil procedure at game
start through the scripted opening comms exchange rather than a disembodied
system message.
Exact drop placement was set during implementation (see
[Open Threads](#open-threads)).

---

## Room Fiction (archetypes for the 81-room grid)

Content rooms are written from repeating archetypes, each given cheap,
livable variation by goblin life. Reference these for `rooms.json` prose.

| Archetype | Prose chestnuts (smell / sound / sight) |
|---|---|
| **Guard room (start)** | Layla wakes in one of the four guard rooms (r3c3 / r3c7 / r7c3 / r7c7), beside the golem that holds it. Cold stone, the smell of a living cave, and a golem watching her the way a door watches. The rooms read as **"A Guarded Crossing"**. |
| **Den** | Hide and old fur underfoot, a dead fire, gnawed bones stacked in a corner, the musky-fur smell. Small — ceilings low enough that you duck. |
| **Hunting ground** | Fire-scorch on the walls, a gutting stone worn black, half-dried meat on a frame, the sour rot of kill. Buzzing flies you can hear before you see. |
| **Fire-pit chamber** | A shallow pit of embers in the middle, ringed with scored rock, smoke layered along the ceiling like a roof. Warm enough to sweat in. |
| **Larder / cache** | Deep cool room, meat hung from hooks, clay pots of pounded herbs, the fattest smell on the floor. Anything worth taking hides here. |
| **Shaman's work-ring (center, r5c5)** | A wide chalk circle on the cave floor, drawn in the same pale light as Layla's sigils. Half-formed golems and stone slabs stand inside it. Ash and hot clay. The shaman works here. Coals. Tock. |
| **Pattern rooms** | The worn goblin tracks cross the floor in a faint grid. Layla measures them. The old content's "scored grid" prose is replaced by these *worn tracks*, and the ritual geometry now lives only in the work-ring. |

Keyed rooms: start = one of the four guard rooms (r3c3 / r3c7 / r7c3 / r7c7) ·
shaman `r5c5` · the four golems hold the guard rooms · the four goblins den
and hunt in the rooms along the patrolling tracks.

---

## Feature Surface Order (the learn-to-play subplot)

Every system feature unlocks through a diegetic act. Each lock sets a story var
(see [Engine Knobs](#engine-knobs) / [Content Story Vars](#content-story-vars)).

| # | Diegetic beat | Feature unlocks | Mechanic |
|---|---|---|---|
| 1 | Start, in a guard room with a golem, chalk already in her pocket | **Look**, **Attack** | attack is target-gated only (golem is a legit neutral target in the room) — no gate |
| 2 | The handler's opening comms exchange teaches the chalk procedure: trace a closed ring around an enemy until it closes and draws it into step | **Trace** is available from the start | Scripted `handler-comms` sequence — chalk is on Layla, so there is no pickup/equip gate to climb |
| 3 | **Defeat the first mob** (a goblin — or the golem) | **Stats / vitals sidebar** | `actor.defeated` → `vitals_sidebar_story_var` (new knob) |
| 4 | **First charm** (ring a golem) | **Party / follower bar** | already derived from follower existence — no gate |
| 5 | **Defeat the shaman** (first boss) | **Map / minimap** widget | `shaman_defeated` → `minimap_requires_story_var` (new knob) |
| 6 | **Descend to floor 2 (the chess level)** | **Level / XP section** appears on the sidebar | XP accrues silently on floor 1; the levels display is itself an unlocked feature, gated on descending (`level_reveal_room_prefix: "d"`) |

### On XP and levels (explicit design choice)

- Layla **earns XP on floor 1** but the sidebar shows **no level section**.
- The levels display **is itself an unlock**: it appears only when she goes down
  a level to the chess floor. **No levels until chess.**

---

## System Messages (new kind + color)

- New `NarrativeLineKind::System`, added alongside Narration / Heading / Player /
  Error.
- Rendered **cold and creepy**, in a **distinct color**: pale blue-white, in the
  `crt_glow` family — visually distinct from error-red (`text-love`) and
  player-echo (`text-foam`).
- Voice: clipped, literal, unemotional — the corporation's voice. Keeps the
  warm narration / cold system contrast sharp.
- First use: the sigil-teaching line at session start, carried by the opening's
  `system_lines` (layered under the intro). Content-authored, not hardcoded.

---

## Engine Knobs

Additions to the engine (cinder-core / cinder-srv + web UI) required to support
the surface order:

1. `vitals_sidebar_story_var` — replaces the static `show_vitals_sidebar` bool
   with "vitals shown when this story var is truthy."
2. `minimap_requires_story_var` — hide the minimap until a story var is truthy
   (currently the minimap always renders).
3. Levels-section visibility — already implemented as
   `level_reveal_room_prefix` (rooms on the "d" board reveal party levels), so
   XP can accrue without a visible level bar on floor 1.
4. A scripted opening sequence on `handler-comms` so the chalk procedure comes
   from Layla's assigned handler rather than a disembodied system voice.
5. *(Removed during implementation)* `requires_equipped_item` — dropped with the
   chalk-on-floor pickup. The chalk stays in Layla's starting inventory and
   `trace` is available from the start.

---

## Content Story Vars

| Story var | Set by | Gates |
|---|---|---|
| `first_mob_defeated` | `actor.defeated` on first kill | Stats/vitals (`vitals_sidebar_story_var`) |
| `shaman_defeated` | `actor.defeated` on the shaman | Map/minimap widget (`minimap_requires_story_var`) |
| `knows_drain` / `knows_spawn` | `item.scroll_read` / `item.spawn_scroll_read` (already implemented) | drain / spawn sigils |

Levels are gated by `level_reveal_room_prefix: "d"` (room-based, not a story
var). No `chalk_equipped` / `looked_room` gates: the chalk is on Layla at start
and `trace`/`move` are available immediately.

---

## Rename & Motif Sweep (prison → goblin cave)

- `warden_defeated` → `shaman_defeated`.
- `warden-ring` / `warden ring` → **shaman's ring** (a ring of dark carved bone
  and clay, still warm; on equip, surviving golems bow to Layla's will).
- `iron-chisel` → **chisel-axe** (goblin skinning tool / the weapon find).
- `herb-salve` → **moss-poultice** (goblin medicine).
- `warden.reveal` / `warden.memory` → shaman reveal/memory.
- Map label **"Upper Works"** → **"The Cave"**.
- Pack `description` / opening `intro_text` → rewritten for the cave premise.
- Floor-1 prose sweep: remove prison/sealed/cell/scored-grid language from
  rooms.json; replace with the goblin cave palette (dens, bone piles, fire,
  hunting grounds, worn tracks, the work-ring).

---

## Open Threads (floor-1 level, settle during implementation)

Settled during the content build (Decisions recorded):

- **First mob → stats.** A goblin defeat sets `first_mob_defeated`
  (`actor.defeated` rules for `goblin-1..4`). The waking golem is a charm
  target, not the stats-unlock; attacking it instead of a goblin keeps stats
  hidden until a goblin is downed.
- **Golem ids / start room.** Four golems total, no extra watcher. `start_room_id`
  falls back to `r3c3`, and `start_room_ids` randomizes the spawn across the four
  guard rooms (r3c3 / r3c7 / r7c3 / r7c7) so Layla always wakes beside a golem.
  The guard golems keep their ids and coords (`golem-dark-nw` r3c3,
  `golem-pale-ne` r3c7, `golem-dark-sw` r7c3, `golem-pale-se` r7c7) and their
  rooms read as **"A Guarded Crossing"**.
- **Goblin placements + drops.** `goblin-1` r2c2 (drops chisel-axe),
  `goblin-2` r2c8 (drops moss-poultice), `goblin-3` r8c2 (drops chisel-axe),
  `goblin-4` r8c8 (drops moss-poultice ×2). The weapon and medicine are each
  reachable from two corners. Goblins wander the patrol tracks (movement.json,
  cadence 2); golems and the shaman stay still.
- **Shaman's work-ring.** Traversable — it is the center room r5c5 itself, and
  the ring is read as a feature of that room. Post-shaman, r5c5's prose swaps
  to the broken-ring aftermath via the room's `descriptions` override, and the
  Down stair (and d1c1's Up return) gate on `shaman_defeated`.
- **Charm-proof shaman.** `actor.surrounded` converts any actor whose id is not
  `goblin-shaman` (a `not_equal` condition), so the shaman can only be defeated.