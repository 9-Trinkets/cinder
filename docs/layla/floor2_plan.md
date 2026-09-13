# Floor 2 — The Underground Forest (Design Plan)

Living design plan for **level 2** of Layla: the underground forest and the elf chess army. The plan is the design-side companion for `content/layla/` — holding the *intent* (premise, chess metaphor, fiction bible, cast and power economy, feature-surface order, and mechanical engine knobs) so the level's thematic arc and tactical depth remain cohesive.

> Status: **design audited and fleshed out; chess army and loot built in content.** The sensory descriptions and feature architectures below document canonical intent and govern current and future content passes across `rooms.json`, `actors.json`, and `movement.json`.

---

## Premise

Following the defeat of the goblin shaman at `r5c5`, a heavy stone trapdoor grinds open, revealing a vertical ladder descending into a cool, blue-green glow.

Layla steps down from the dry, ash-choked stone of Floor 1 into an **underground forest** (`d1c1` through `d8c8`). Here, the subterranean cavern expands into an immense vaulted biome spanned by a canopy of living roots and bioluminescent flora — a "false sky."

The board language shifts sharply from **Go** to **Chess**:
- **Go (Floor 1)** was about space, territory, encircling silent stones, and making breathing room.
- **Chess (Floor 2)** is about **ranked armies**, **geometric movement**, and **commanding combat**. The objective is no longer enclosing territory, but navigating an advancing military hierarchy to face and defeat their **King**.

The forest is held by an **Elf Army** of 16 conscious, disciplined soldiers. They are not mindless constructs like the shaman's golems, nor scavengers like the goblins. They are organized, watchful, and free-willed.

---

## The Underground Forest (fiction bible — sensory)

The map forms an **8×8 Chess grid** (ranks `d1`–`d8`, files `c1`–`c8`). In the fiction, it reads not as a synthetic chessboard, but as a vast, ancient subterranean grove bounded by sheer limestone escarpments and channeled root-paths.

- **Smell.** Cool, vegetal, and damp. Rich leaf-mold, crushed sweet fern, green sap, and cold spring water seeping through limestone. Near the fungal clearings it carries a sweet, fermented aroma like overripe wild fruit; near the chalk trails, the faint dry alkaline dust of older travelers.
- **Sound.** A profound, low-frequency resonance — the living canopy breathing like a slow heartbeat. Constant gentle water drips echoing into limestone pools. The unprompted whispering of tall ferns rustling without wind. Underneath it all, the sharp, rhythmic footfalls of elven patrols: spear butts tapping rock and leather-stitched leaf armor flexing in unison.
- **Light (The "False Sky").** The vaulted ceiling is thick with bioluminescent mosses, luminous vines, and drifting spores that pulse on an even rhythm. Columns of ancient pale light filter through vertical fissures high above ("A Shaft of Sky"). The light is cool, steady, and slightly surreal — bright enough to read by, but noticeably unearthly.
- **The Chess Metaphor in Fiction.**
  - **Ranks (Rows `d1` to `d8`):** Represent zones of depth progressing from the entrance verge to the royal court:
    - *Rank 1 (`d1`):* **The Verge** (`d1c1` ladder landing in the Mushroom Grove; `d1c2`–`d1c8` Glowing Clearings). The edge of the forest.
    - *Rank 2 (`d2`):* **The Root Tangle.** Heavy gnarled root-walls channeling travel.
    - *Rank 3 (`d3`):* **A Still Pool Glade.** Luminous water mirrors reflecting the false sky.
    - *Rank 4 (`d4`):* **The Fungus Hollow.** Knee-high sentinel mushrooms arranged in eerie, orderly files.
    - *Rank 5 (`d5`):* **A Chalked Trail.** Ghostly paths marking historical movement channels.
    - *Rank 6 (`d6`):* **The Whispering Ferns.** Dense fronds rustling with half-remembered whispers; the perimeter buffer of the army.
    - *Rank 7 (`d7`):* **The Pawn Line (A Shaft of Sky).** The forward military screen: eight elf pawns stationed under shafts of pale light.
    - *Rank 8 (`d8`):* **The Royal Back Rank (The Deep Glade & King's Grove).** Heavy curtains of glowing vine protecting the specialized officers (Rooks, Knights, Bishops, Queen) and centering on the Elf King enthroned at `d8c5`.
  - **Files (Columns `c1` to `c8` / Files a–h):** Root channels and watercourses running North–South:
    - *Files 1–3 (a–c, Queenside):* Flanked by deep root-tangles and dark limestone pools; home to the western Rook, Knight, and Bishop.
    - *Files 4–5 (d–e, Royal Center):* The central artery leading directly to the Queen (`d8c4`) and the King's Grove (`d8c5`).
    - *Files 6–8 (f–h, Kingside):* Denser whispering ferns, brighter spore drifts, and the eastern officer line.

---

## The Elf Army (fiction bible — the creatures)

Elves in the underground forest are slender, tall, and composed. Their skin has the texture of smooth, pale birch bark; their eyes are wide, reflective pools of silver-green that see clearly in twilight. They do not shout; they give orders with sharp, fluid hand-signs and low vocal clicks.

They wear armor fashioned from hardened forest materials: layered broad leaves cured in resin (`leaf-plate`), helm crests of curved bark (`leaf-helm`), and boots of compressed root-fiber (`leaf-boots`) that leave no sound on stone.

### Pawns (×8 — `elf-pawn-1` to `elf-pawn-8`)
- **Station:** Row `d7` (`d7c1` through `d7c8`).
- **Stats:** Level 3 | HP 6 | Str 2 | Def 1 | Int 3 | XP 10.
- **Weapon & Gear:** `leaf-spear`, `leaf-paste`.
- **Behavior:** The advance screen. Quick and alert. In `movement.json`, they move on a fast **1-tick cadence**, probing the mid-forest and intercepting intruders.
- **Inspect text:** *"A lithe elf sentry clad in stiff leaf armor, balancing a spear with effortless precision. Their silver-green eyes track your every twitch with clinical discipline."*

### Knights (×2 — `elf-knight-2`, `elf-knight-7`)
- **Station:** `d8c2` and `d8c7` (back rank flanks).
- **Stats:** Level 4 | HP 10 | Str 4 | Def 2 | Int 4 | XP 25.
- **Weapon & Gear:** `leaf-helm`, `leaf-gloves`.
- **Behavior:** Outriders and skirmishers. Patrol on a **2-tick cadence**, capable of aggressive pursuit.
- **Inspect text:** *"An agile elf warrior bearing a crest of dark curved bark. They balance on the balls of their feet, ready to spring sideways or leap past obstacles in an instant."*

### Bishops (×2 — `elf-bishop-3`, `elf-bishop-6`)
- **Station:** `d8c3` and `d8c6` (royal guard flank).
- **Stats:** Level 4 | HP 10 | Str 4 | Def 2 | Int 4 | XP 25.
- **Weapon & Gear:** `leaf-crook`, `leaf-boots`.
- **Behavior:** Channelers and battlefield tacticians. Patrol on a **3-tick cadence**, maintaining sightlines.
- **Inspect text:** *"An austere elf holding a tall crooked bough that pulses with faint green light. Their gaze cuts diagonally across the grove, measuring distances and angles."*

### Rooks (×2 — `elf-rook-1`, `elf-rook-8`)
- **Station:** `d8c1` and `d8c8` (corner bastions).
- **Stats:** Level 5 | HP 14 | Str 5 | Def 3 | Int 4 | XP 40.
- **Weapon & Gear:** `leaf-plate`, `leaf-paste`.
- **Behavior:** Heavy shock infantry clad in dense, multi-layered leaf breastplates. Move on a deliberate **3-tick cadence**.
- **Inspect text:** *"A broad-shouldered elf sentinel armored in heavy plates of lacquered bark and compressed leaf. They stand like a living redoubt, unyielding and direct."*

### The Elf Queen (×1 — `elf-queen-4`)
- **Station:** `d8c4` (beside the King).
- **Stats:** Level 6 | HP 16 | Str 6 | Def 3 | Int 6 | XP 60.
- **Weapon & Gear:** `leaf-ring` (50% drop chance).
- **Behavior:** The army's supreme champion. Lethal combatant with high strength and intellect. Patrols on a **4-tick cadence**, sweeping across adjacent glades.
- **Inspect text:** *"A formidable elf commander whose braided leaf ring hums with quiet power. Her movements are fluid, swift, and completely without hesitation."*

### The Elf King (×1 — `elf-king-5` — Boss of Floor 2)
- **Station:** `d8c5` ("The King's Grove").
- **Stats:** Level 8 | HP 20 | Str 6 | Def 4 | Int 7 | XP 0 (Defeat is a story victory).
- **Weapon & Gear:** Drops the **`drain-scroll`** (100%) and `leaf-cloak` (35%).
- **Behavior:** **Stationary.** Sits upon the woven root-throne of `d8c5`, calmly awaiting Layla.
- **Inspect text:** *"The sovereign of the underground forest, draped in a mantle of dark root-stitched leaves. He sits upon a throne grown from living wood, watching you approach with neither malice nor fear."*

---

## Cast & Power Economy

16 total actors on the floor:

| Actor | Count | Level | HP | Str | Def | Int | Move Cadence | Key Drop |
|---|---|---|---|---|---|---|---|---|
| **Elf King** | 1 | 8 | 20 | 6 | 4 | 7 | Stationary | `drain-scroll`, `leaf-cloak` |
| **Elf Queen** | 1 | 6 | 16 | 6 | 3 | 6 | 4 ticks | `leaf-ring` |
| **Elf Rook** | 2 | 5 | 14 | 5 | 3 | 4 | 3 ticks | `leaf-plate`, `leaf-paste` |
| **Elf Knight** | 2 | 4 | 10 | 4 | 2 | 4 | 2 ticks | `leaf-helm`, `leaf-gloves` |
| **Elf Bishop** | 2 | 4 | 10 | 4 | 2 | 4 | 3 ticks | `leaf-crook`, `leaf-boots` |
| **Elf Pawn** | 8 | 3 | 6 | 2 | 1 | 3 | 1 tick | `leaf-spear`, `leaf-paste` |

### Progression Dynamics:
1. **Party Synergy Requirement:** Arriving on Floor 2, Layla and her Floor 1 golems (Level 1) cannot simply steamroll the elven ranks. A Level 3 pawn can threaten Layla alone; fighting a Rook or the Queen requires smart use of the **Party System** (having dark golems on `guard` intercept incoming blows while pale golems counterattack).
2. **Mental Resistance against Chalk Rings:**
   - The engine's surround rule (`settings.surround_rule`) tests `intelligence`:
     $$\text{player\_int} + \text{player\_level} \ge \text{target\_int} + 2 \times \text{target\_level}$$
   - An Elf Pawn requires $3 + 2(3) = 9$. A Knight/Bishop requires $4 + 2(4) = 12$. The King requires $7 + 2(8) = 23$.
   - This mathematical reality prevents early-level trivialization via chalk encirclement, reinforcing the fiction: *elves are free minds who resist magical binding*.

---

## Loot & The Leaf Set

The elves forge their equipment from the living materials of their subterranean habitat. Defeating patrols allows Layla to equip a full greenwarden ensemble:

- **Weapons & Off-Hand:**
  - `leaf-spear` (Weapon): Str +1. Standard pawn armament.
  - `leaf-crook` (Weapon / Off-hand): Int +2. Bishop's channeling focus.
  - `leaf-buckler` (Off-hand): Def +1. Compact leaf shield.
- **Armor Set:**
  - `leaf-helm` (Helm): Def +1.
  - `leaf-plate` (Chest): Def +2. Heavy armor from rooks.
  - `leaf-gloves` (Gloves): Def +1.
  - `leaf-boots` (Boots): Def +1.
  - `leaf-cloak` (Cloak): Int +2. Dropped by the King.
- **Trinket & Healing:**
  - `leaf-ring` (Ring): Int +2. Dropped by the Queen.
  - `leaf-paste` (Potion): Heals wounds when used (`item.salve_used`).
- **The Keystone Drop:**
  - **`drain-scroll`** (Dropped by `elf-king-5`): An ancient parchment inscribed with spiral sigils. Reading it teaches the **Drain Sigil** (`drain-sigil`).

---

## Feature Surface Order (Progression Beats)

| # | Trigger / Action | Feature / Story Beat Unlocked | Engine / Script Mechanism |
|---|---|---|---|
| 1 | **Descend to Floor 2** (`d1c1`) | **Levels & XP Sidebar Display** becomes visible | Gated by `level_reveal_room_prefix: "d"`. Party levels are now revealed. |
| 2 | **Engage wandering patrols** | Dynamic combat against coordinated multi-cadence foes | Driven by `movement.json` cadences and `settings.combat.ally_attack`. |
| 3 | **Defeat Elf King** (`elf-king-5` at `d8c5`) | **King's Defeat Narrative** & **Army Ceasefire** | Hook `actor.defeated` on `elf-king-5`: narrates `king.defeated`, sets `elf_king_defeated = true`, sets all `elf` tags to `stance: neutral`. |
| 4 | **Loot & Read Drain Scroll** | **Drain Sigil (`drain-sigil`)** unlocked | Triggers `item.scroll_read`, sets `knows_drain = true`, enabling `trace drain-sigil`. |
| 5 | **Descend to Floor 3** (from `d8c5` to `oan`) | Opens path to **The Fire-Rooms (Mancala)** | Exit `Down` at `d8c5` gated by `requires_story_var: "elf_king_defeated"`. |

---

## Freedom vs. Binding (The Core Theme)

The climax of Floor 2 provides the first major philosophical turning point in Layla's journey:

1. **The Contrast:** On Floor 1, the goblin shaman used chalk rings to bind inanimate stone into mindless golems. Layla used the same magic to claim the golems as her own thralls. The shaman's ring literally commands obedience.
2. **The Elves' Choice:** When the Elf King is struck down, **no ring binds the survivors**. The engine hook changes their stance to `neutral`, but `follows_player` remains `false`. In prose (`king.defeated`):
   > *"The elf king falls. The forest holds its breath. Then, one by one, the other elves lower their weapons and step back. No ring binds them — they simply choose to let you pass. You feel their eyes on you a long moment, and you lower your own blade."*
3. **The Warning:** Layla begins to glimpse that power in this place is fundamentally about choice: whether she will become an instrument of binding (the corporate Dungeon Master), or someone capable of recognizing and granting freedom.

---

## Room Feature Architecture (Inspection Surfaces)

Every room in the 8×8 grid must have inspectable features to ground the player's spatial awareness and interaction.

| Archetype | Rooms | Inspectable Features | Details & Aliases |
|---|---|---|---|
| **The Mushroom Grove** | `d1c1` (Entry) | `the pale mushrooms`, `the glowing canopy` | Caps crowding the floor; false sky ceiling overhead. |
| **A Glowing Clearing** | `d1c2`–`d1c8` | `the drifting spores`, `the false sky` | Swirling luminous spores; the shimmering bioluminescent vault. |
| **The Root Tangle** | `d2c1`–`d2c8` | `the root tangle`, `the living walls` | Arm-thick roots breathing in rhythm; moss-laced stone crevices. |
| **A Still Pool Glade** | `d3c1`–`d3c8` | `the still pool`, `the mirror surface` | Luminous subterranean pool; unprompted ripples reflecting the canopy. |
| **The Fungus Hollow** | `d4c1`–`d4c8` | `the sentinel mushrooms`, `the spore light` | Knee-high fungi arranged in strict, planted ranks. |
| **A Chalked Trail** | `d5c1`–`d5c8` | `the chalk trail`, `the worn track` | Pale ancient lines scored down the center of the stone. |
| **The Whispering Ferns** | `d6c1`–`d6c8` | `the whispering ferns`, `the fronds` | Shoulder-height fern fronds murmuring in a phantom tongue. |
| **A Shaft of Sky** | `d7c1`–`d7c8` (Pawns) | `the shaft of sky`, `the moss bed` | Overhead chasm letting in starlike light; thick cushion of green moss. |
| **The Deep Glade** | `d8c1`–`d8c4`, `d8c6`–`d8c8` | `the glowing vines`, `the curtain boughs` | Curtains of heavy luminous vines pulsing like a green heart. |
| **The King's Grove** | `d8c5` (Throne) | `the root throne`, `the hollowed seat` | Throne woven of grey bark and ancient root; changes state post-defeat. |

---

## Open Threads & Next Steps

1. **Features in `rooms.json`:** Currently, Floor 2 rooms have `features: []`. Populate the canonical features defined above across `d1c1`–`d8c8`.
2. **Rich Inspect Text for Elf Actors:** Update `actors.json` elf definitions with evocative inspect text and behavioral subtext reflecting their ranks.
3. **Column / File Variations:** As with Floor 1's distance gradients, subtle flavor variations distinguishing Queenside files (`c1`–`c3`), Central Royal files (`c4`–`c5`), and Kingside files (`c6`–`c8`) will make the 64 rooms feel like a living geography rather than 8 cloned rows.
