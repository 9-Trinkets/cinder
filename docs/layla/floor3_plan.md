# Floor 3 — The Fire-Rooms (Mancala & The Old Fire)

Living design plan for **level 3** of Layla: the fire-rooms and the fire elemental. The plan is the design-side companion for `content/layla/` — holding the *intent* (premise, Mancala metaphor, sensory bible, boss movement, resource and trap economy, cast and party roles, and engine knobs) so the level's puzzle-combat tension and narrative resolution remain cohesive.

> Status: **redesign finalized around Mancala mechanics (movement & resources); kinetic boss loop, trap economy, and sprite seed dynamics defined.** Supersedes the earlier stationary boss placement in `oh`. Implementation decisions are recorded under [Engine & Content Requirements](#engine--content-requirements).

---

## Premise

Following the peaceful surrender of the elf army at `d8c5`, Layla descends through the root-floor into an abrupt and searing shift in environment: a vast, volcanic hollow paved in dark, heat-tempered black glass (`oan` through `o12`, and `oh`).

The board language shifts to its third remembered game: **Mancala**:
- **Go (Floor 1)** was about **space and territory** (encircling vacant intersections).
- **Chess (Floor 2)** was about **hierarchy and lines** (advancing through a ranked military order).
- **Mancala (Floor 3)** is about **resource counting, circular flow, and sowing seeds**.

The floor is a circular loop of pits scored into obsidian glass. The **Fire Elemental** is not a static guardian waiting in an inner sanctum; it is the **Sower** — an ancient creature of memory and heat that circles the ring clockwise, dropping volatile **Fire Sprites** ("seeds") into the basins as it moves.

Layla cannot defeat the elemental with conventional blades: its body is made of old fire, possessing near-total physical resistance. To overcome it, she must treat the floor as a kinetic game of Mancala: predicting the elemental's path, clearing out the fire sprites before they clog the lanes, and laying **Drain Sigils** ahead of the elemental's arrival to sap its ancient heat.

---

## The Land Ravaged by Fire (fiction bible — sensory)

The setting is an ancient caldron of fused volcanic glass, scorched rock, and banked embers. The air is bone-dry and searing hot, warping sightlines with thermal mirages.

- **Smell.** Sharp sulfur, silica dust, hot iron, banked charcoal, and dry metallic heat. Unlike Floor 1's greasy goblin smoke or Floor 2's damp leaf-sap, this floor is scorch-clean and caustic — a dry furnace that parches the tongue and stings the back of the throat.
- **Sound.** A deep, low-frequency volcanic pulse thrumming up through the obsidian floor like a massive bellows. The sharp, erratic *clack-crackle* of fire sprites skittering over vitrified stone. The resonant, wind-tunnel hiss of the elemental shifting between basins. In empty rooms, the eerie, breathless silence of cooling stone.
- **Light.** Intense chiaroscuro. High-contrast cherry red, ember-gold, and furnace-amber glowing against pitch-black glass. Thermal hazes shimmer over the basins. There are no torches or false skies here; the board is lit entirely by the fire that lives upon it.
- **The Mancala Metaphor in Fiction.**
  - **The Pits (Basins `o1` to `o12`):** Twelve circular dishes scooped into the black glass floor. Each holds a bed of embers where heat pools.
  - **The Stores (Offerings `oan` and `oas`):** Two monumental black-glass cauldrons anchoring the north and south ends of the circuit. In game terms, these are the Mancala kalahs/stores where the loop bends.
  - **The Seeds (Fire Sprites):** Tiny, skittering fragments of flame deposited into the basins. If left alone, they accumulate, wandering between basins and threatening to overwhelm the party.
  - **The Sower (The Fire Elemental):** The kinetic engine of the floor. It traverses the circuit in a strict clockwise loop, shedding sparks and dropping sprites.
  - **The Heart-Pit Bypass (`oh`):** The central hub of the board, connecting directly to both `oan` (North Store) and `oas` (South Store). It serves as Layla's tactical shortcut to cross the board and get ahead of the boss.

---

## The Circuit: Board Architecture

The floor consists of 15 interconnected chambers:

```
                  [ oan ] (North Offering / Store)
                 ^   |   \
                /    |    \
     (ccw)     /     |     \  (cw)
              /      |      \
        [ o12 ]      |      [ o1 ]
          ^          |        |
        [ o11 ]      |      [ o2 ]
          ^          v        |
        [ o10 ]   [ oh ]    [ o3 ]
          ^     (Heart-Pit)   |
        [ o9  ]      ^        |
          ^          |        |
        [ o8  ]      |      [ o4 ]
          ^          |        |
        [ o7  ]      |      [ o5 ]
              \      |      /
     (cw)      \     |     /  (cw)
                \    |    /
                 v   |   v
                  [ oas ] (South Store)
```

- **Perimeter Ring (14 chambers):**
  `oan` $\rightarrow$ `o1` $\rightarrow$ `o2` $\rightarrow$ `o3` $\rightarrow$ `o4` $\rightarrow$ `o5` $\rightarrow$ `o6` $\rightarrow$ `oas` $\rightarrow$ `o7` $\rightarrow$ `o8` $\rightarrow$ `o9` $\rightarrow$ `o10` $\rightarrow$ `o11` $\rightarrow$ `o12` $\rightarrow$ `oan`.
- **Clockwise Flow:** The primary exit in every room is labeled `Clockwise` (`cw`), leading to the next basin in the sowing sequence. A reverse `Counter-clockwise` (`ccw`) exit allows the player to backtrack.
- **The Central Bypass (`oh`):** Connects `oan` and `oas`. When the elemental is circling the eastern basins (`o1`–`o6`), Layla can dive through `oh` to reach `oas` in two steps, cutting off the elemental and preparing the southern arc.

---

## Cast & Power Economy

| Actor | Count | Level | HP | Str | Def | Res | Move Cadence | Key Drop | Role |
|---|---|---|---|---|---|---|---|---|---|
| **Fire Elemental** | 1 | 5 | 18 | 3 | 2 | Physical: 100,000 | 2 ticks (Clockwise) | `spawn-scroll` | The Sower / L3 Boss |
| **Fire Sprites** | 3–8 | 3 | 4 | 2 | 0 | None | 1 tick (Wandering) | None | The Seeds / Swarm |

### The Fire Elemental (`fire-elemental`)
- **Nature:** An unshaped, patient entity made of the board's old fire. It holds itself together out of ancient memory.
- **Physical Resistance:** `resistances: { "physical": 100000 }`. Conventional weapons, spears, and golem fists glance harmlessly off its core. Physical strikes deal 0 damage.
- **Movement:** Traverses the perimeter circuit clockwise, moving one basin every **2 ticks**.
- **Inspect text:** *"It is not quite a body, not quite a flame. The shape holds itself together out of memory: a wide, patient form, made of the board's old fire. It does not flicker. It holds. A spiral of warmth beats slow inside it."*

### Fire Sprites (`fire-sprite-1`..`n`)
- **Nature:** Small sparks cast off from the elemental's mantle. Fragile, erratic, and aggressive.
- **Vulnerability:** HP 4, Def 0. A single solid strike from Layla or an assist-golem will dispatch them.
- **Threat:** While weak individually, they wander rapidly (cadence 1). Their real danger is **resource sabotage**: entering rooms where Layla has placed Drain Sigils and absorbing the trap's charges.
- **Inspect text:** *"A small twist of heat shaped like a child of the old fire, leaking embers onto the black glass. It crackles once, watching, then settles back to a banked glow."*

---

## The Trap & Resource Economy (How the Puzzle Works)

This floor is a tactical trap puzzle governed by **three competing pressures**:

### 1. The Drain Sigil as the Win Condition
- Traced using Layla's magic chalk (`trace drain-sigil`).
- **Charge Budget:** Each sigil has `max_activations: 5`.
- **Effect:** Every tick, it targets `hostile_living` creatures in the room, dealing **2 points of non-physical damage** (`combat.actor_drained`).
- **Math:** 5 activations $\times$ 2 damage = **10 total damage** per fully expended sigil.
- Since the Fire Elemental has **18 HP**, Layla must successfully trap the elemental in **at least two distinct Drain Sigils** (or keep it inside a single sigil for 5 ticks, then lure it into a second for 4 ticks) to achieve victory.

### 2. The Spoiler Dynamic (Sprite Interference)
- Because `drain-sigil` targets *any* living hostile, a wandering **Fire Sprite** (HP 4) entering a trapped room will trigger the sigil!
- A sprite takes 2 ticks (4 damage) to die, burning **2 of the sigil's 5 charges**.
- If two sprites wander into the room, **4 of the 5 charges are consumed**, leaving only 1 tick (2 damage) for the elemental — ruining the trap.
- **Player Imperative:** Layla cannot simply lay sigils randomly. She must maintain **lane control**, actively clearing fire sprites out of basins before laying her sigil.

### 3. The Race (Predictive Movement & The Bypass)
- Layla cannot stay in the same room as the elemental to fight it directly — its fire aura and attacks will wear her down, and tracing chalk takes time.
- Chasing behind the elemental is futile because the elemental leaves spawned sprites in its wake.
- **Winning Flow:**
  1. Observe the elemental's current position and clockwise direction.
  2. Use the **Heart-Pit (`oh`) bypass** or sprint ahead on the outer ring to get 2–3 basins ahead.
  3. Clear any existing sprites from the target basin with the party.
  4. Trace a `drain-sigil` on the black glass.
  5. Fall back one room clockwise, waiting for the elemental to enter the trapped basin.
  6. Repeat on the opposite arc until the elemental's 18 HP is exhausted.

---

## The Role of the Party (Golems on the Fire Board)

Layla enters Floor 3 with her surviving golems from Floor 1, leveled through Floor 2. Their party orders become indispensable:

- **Dark Golems on `guard` (Interception):**
  When Layla is tracing a sigil on the floor or navigating through rooms with active fire sprites, `guard` golems intercept incoming strikes (`combat.guard_intercepts`). Made of cold river stone and dense volcanic granite, they withstand fire attacks far better than Layla.
- **Pale Golems on `assist` (Sweepers):**
  Set to `assist`, pale golems retaliate against sprites the moment combat begins (`combat.party_counterattack`). With their raw attack power, they can one-shot 4 HP sprites, clearing the lane before traps can be spoiled.

---

## Feature Surface Order & Progression Beats

| # | Trigger / Action | Feature / Story Beat Unlocked | Mechanic / Engine Rule |
|---|---|---|---|
| 1 | **Descend to Floor 3** (`oan`) | Reveal the Mancala ring & heat mechanics | Enters `oan` from `d8c5`. The air shifts to caustic dry heat. |
| 2 | **Encounter the Fire Elemental** | Discover physical weapon immunity | Attack glances off (`resistances.physical: 100000`). Narrative feedback reinforces that blades cannot cut memory. |
| 3 | **Execute the Trap Strategy** | Trace Drain Sigils ahead of the boss; sweep sprites | Uses `trace drain-sigil` learned from Floor 2; manages 5-charge limit against sprite interference. |
| 4 | **Exhaust the Elemental's HP** | **The Release (`elemental.release`)** | Elemental lets go; hook `actor.defeated` on `fire-elemental` triggers `elemental.release` narration and sets `elemental_released = true`. |
| 5 | **Sprite Pacification** | Surviving sprites become peaceful | Hook sets tag `sprite` to `stance: neutral`. The floor cools. |
| 6 | **Loot the Ember Scroll** | **Spawn Sigil (`spawn-sigil`)** unlocked | Reading `spawn-scroll` (`item.spawn_scroll_read`) teaches the third and final dungeon-master power. |

---

## Freedom vs. Binding: The Release Arc

The defeat of the Fire Elemental resolves the third phase of Layla's internal arc:

1. **Floor 1 (Golems):** Stone pieces shaped by a master, bound to service by chalk rings and the shaman's bone ring (**Subjugation**).
2. **Floor 2 (Elves):** Free minds who decline binding, lowering their weapons by conscious choice when their sovereign falls (**Choice**).
3. **Floor 3 (The Elemental):** A manifestation of the board's ancient fire held captive in a closed cycle of sowing and burning. It is not destroyed; it is **freed**:
   > *"The fire elemental does not fall — it lets go. Its fire unknots, loosens, and the spiral in its chest unwinds like a long-held breath. It is released back to the elemental realm, and the board is suddenly, quietly cool."*
4. **The Echo in Layla:** Layla sees that every floor was designed to teach her how to bind and dominate (Dungeon Master training) — yet at every stage, the true victory has come from understanding the game's rules and choosing to let things go free.

---

## Room Feature Architecture (Inspectable Surfaces)

Every room on Floor 3 features high-detail inspectable surfaces grounding the Mancala board fiction:

| Room ID | Room Title | Primary Feature | Secondary Feature | Prose Anchor |
|---|---|---|---|---|
| `oan` | The North Offering | `the fire-bowl` (`oan-fire-bowl`) | `the black glass` (`oan-glass`) | Deep north store; scored lip where embers pool; cool obsidian floor. |
| `oas` | The South Store | `the fire-bowl` (`oas-fire-bowl`) | `the banked ember` (`oas-ember`) | Shallow south store; a single glowing seed resting in the basin. |
| `o1`–`o6` | Basins of the East Arc | `the glass basin` (`<rid>-basin`) | `the thermal cracks` (`<rid>-cracks`) | Smooth circular pits; cooling embers; spiderweb fractures in the glass floor. |
| `o7`–`o12` | Basins of the West Arc | `the glass basin` (`<rid>-basin`) | `the ash drifts` (`<rid>-ash`) | Wide circular pits; fine grey ash settling in the grooves; heat hazes. |
| `o8` | A Basin Where Seeds Were Slid | `the glass basin` (`o8-basin`) | `the seed scores` (`o8-scores`) | Deeply grooved glass where countless seeds of fire were slid in play. |
| `oh` | The Heart-Pit | `the coals` (`oh-coals`) | `the obsidian cradle` (`oh-cradle`) | Central hub; circular cradle of black glass; watching coals; bypass north/south. |

---

## Engine & Content Requirements

To fully power this redesign in the engine and content pack:

1. **Clockwise Movement Mode for `movement.json`:**
   - In `cinder-core/src/content/types/world_defs/movement.rs` and `cinder-core/src/engine/actor_tick/movement.rs`:
   - Introduce `WanderMode::ExitLabel { label: String }` (or `Route { waypoints: Vec<String> }`).
   - For `fire-elemental`, configure `mode: "exit_label", label: "Clockwise"`, with `cadence_ticks: 2`.
2. **Actor Turn Tick Scope:**
   - Ensure `actor_tick_scope: "current_board"` in `settings.json` keeps the elemental moving even when Layla is 2–3 rooms away setting traps.
3. **Sprite Interception Verification:**
   - Verify that `periodic_actor_effects` correctly decrements activations when a sprite takes damage, ensuring sprite interference functions as intended.
4. **Boss Physical Resistance:**
   - Confirm `fire-elemental` in `actors.json` retains `resistances: { "physical": 100000 }` so standard attacks deal 0 damage.
