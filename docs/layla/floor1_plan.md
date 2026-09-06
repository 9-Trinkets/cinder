# Floor 1 — The Goblin Cave (Design Plan)

Living design plan for **level 1** of Layla: the goblin cave. The plan is the
design-side companion for `content/layla/` — as content is built, this doc
holds the *intent* (premise, cast, feature-surface order, engine knobs) so the
level's tutorial-by-discovery arc is never lost.

> Status: design agreed, implementation pending. Supersedes the earlier
> "stone board + stone warden" concept as canon for floor 1.

---

## Premise

Layla wakes on cold stone in the mouth of a cave, next to a **golem** that is
watching her silently. She should be afraid — but feels oddly calm. Despite
having no memory, she feels like she's been here before.

The board (upper level) is a **goblin cave**, not a prison. A **goblin tribe**
lives on this floor, and its **shaman made the golems**. The floor is theirs:
crude dens, bone piles, hunting camps, and at the center the shaman's work-ring
where the golems were shaped.

She is not a prisoner here. She is a stranger in someone's home, and she has to
learn the rules of the floor to earn her way down.

---

## Cast & Power Economy

Nine actors on the floor:

| Actor | Count | Role | Strength |
|---|---|---|---|
| **goblin shaman** | 1 | Center of the cave; made the golems. The L1 boss. Cannot be converted, only defeated. | **> golems** |
| **golems** | 4 | The shaman's constructs, stationed across the floor. Silent, neutral, still pieces — "made things that don't know the game." Charmable. | mid |
| **goblins** | 4 | Roam the cave hunting for food or enemies. Conscious and hostile. The weakest enemies on the floor. | **< golems** |

**Strength ladder:** shaman > golems > goblins.

- **Goblins** are the natural "first mob" — the ones Layla can realistically
  defeat at the start, and the defeat that surfaces her **stats**.
- **Golems** are charmable servants: tracing rings around them brings them into
  step with her (first charm → **party**).
- **Shaman** anchors the power economy at the top (above the old `golem-boss`
  anchor of HP 20 / strength 4).

## Loot & the deferred chisel/salve

The goblins' **hunting** justifies the earlier question of *why a weapon and
healing exist on floor 1*. Killing hunt-goblins (or looting their camps) yields
their hunting gear — a crude chisel-axe and salve-poultices — so there is a
concrete way to *pick up a weapon and healing before facing the shaman*.

- Starting inventory is **empty**: Layla wakes with nothing but what she finds.
- Specific item forms (chisel-axe, poultice nomenclature, drop rates) are **not
  yet decided** — deliberately deferred. The *justification* (goblin hunting) is
  settled; the item content is open.

---

## Feature Surface Order (the learn-to-play subplot)

Every system feature unlocks through a diegetic act. Each lock sets a story var
(see [Engine Knobs](#engine-knobs) / [Content Story Vars](#content-story-vars)).

| # | Diegetic beat | Feature unlocks | Mechanic |
|---|---|---|---|
| 1 | Start, in r1c1 with a golem | **Look**, **Pick up**, **Attack** | attack is target-gated only (golem is a legit neutral target in the room) — no gate |
| 2 | **Look** at the room | room desc + exits; **Move** appears | story var on first look → `requires_story_var` |
| 3 | **Pick up** the chalk (loose on the floor) | **Equip** appears | possession gate (existing) |
| 4 | **Equip** the chalk | **Trace** appears | `requires_equipped_item` (new knob) |
| 5 | Chalk pickup emits cold **system message** teaching sigil → charm (trace a closed ring around an enemy until it closes and draws it into step) | — | `NarrativeLineKind::System` |
| 6 | **Defeat the first mob** (a goblin — or the golem) | **Stats / vitals sidebar** | `actor.defeated` → `vitals_sidebar_story_var` (new knob) |
| 7 | **First charm** (ring a golem) | **Party / follower bar** | already derived from follower existence — no gate |
| 8 | **Defeat the shaman** (first boss) | **Map / minimap** | `shaman_defeated` → `minimap_requires_story_var` (new knob) |
| 9 | **Descend to floor 2 (the chess level)** | **Level / XP section** appears on the sidebar | XP accrues silently on floor 1; the levels display is itself an unlocked feature, gated on descending (story var) |

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
- First use: the sigil-teaching line after chalk pickup.

---

## Engine Knobs

Additions to the engine (cinder-core / cinder-srv + web UI) required to support
the surface order:

1. `requires_equipped_item` on `ActionAvailability` — gates an action on a
   specific equipped item (trace → chalk equipped). Equip actions already gate
   on possession; this extends the same idea to equipped state.
2. `vitals_sidebar_story_var` — replaces the static `show_vitals_sidebar` bool
   with "vitals shown when this story var is truthy."
3. `minimap_requires_story_var` — hide the minimap until a story var is truthy
   (currently the minimap always renders).
4. Levels-section visibility — only rendered when a story var is set
   (e.g. `descended`), so XP can accrue without a visible level bar.
5. `NarrativeLineKind::System` + UI color mapping (pale blue-white).

---

## Content Story Vars

| Story var | Set by | Gates |
|---|---|---|
| `looked_room` | first room look | Move |
| `chalk_equipped` | equip hook / equipped-item knob | Trace |
| `first_mob_defeated` | `actor.defeated` on first kill | Stats/vitals |
| `shaman_defeated` | `actor.defeated` on the shaman | Map/minimap |
| `descended` | leaving floor 1 | Level/XP section |
| `knows_drain` / `knows_spawn` | `item.scroll_read` / `item.spawn_scroll_read` (already implemented) | drain / spawn sigils |

---

## Rename & Motif Sweep (prison → goblin cave)

- `warden_defeated` → `shaman_defeated`.
- `warden-ring` → **shaman's ring** (fetish / talisman) — on equip, converts
  surviving golems to Layla's will (same hook mechanics, new lore).
- `warden.reveal` / `warden.memory` narrations → shaman reveal/memory.
- Map label **"Upper Works"** → a goblin cave name.
- Pack `description` / opening `intro_text` → rewritten for the cave premise.
- Floor-1 prose sweep: remove prison/sealed/cell/scored-grid language from
  rooms.json; replace with goblin cave texture (dens, bones, hunting, shaman's
  work-ring).

---

## Open Threads (floor-1 level, to settle during implementation)

- Exact goblin room placements and the shaman's room.
- Whether the starting golem in r1c1 is itself the "first mob," or whether a
  goblin must be encountered for the stats unlock (both currently plausible).
- Whether golems are placed in the four former corner-guard rooms or re-seated.
- Item forms / drop placement for the chisel-axe and salves (justification settled
  via goblin hunting; content open).