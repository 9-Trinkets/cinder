# Floor 6 — The High Sanctuary (The Clergy & The Inquisition)

Living design plan for **Level 6** of Layla. Like earlier levels (the deep forest on Floor 2, the geothermal valley on Floor 4, and the snow peaks and glaciers on Floor 5), Floor 6 is a distinct magical partitioned world: a breathtaking sacred acropolis set under open skies, featuring soaring white limestone basilicas, sunlit colonnades, open cloister courtyards, terraces overlooking endless seas of clouds, towering bell towers, and silent embalming crypts below.

Here, Layla discovers the terrifying truth of the dungeon: it is a **closed-loop bio-computational factory** that harvests living humans from the worker towns, executes them in Floor 5's citadel, and revives and brainwashes them in Floor 6 into dungeon monsters.

The gameplay genre for Floor 6 is **Social Deduction / Infiltration (Werewolf / Mafia)**. Layla infiltrates the cathedral alone as the "Werewolf" in sheep's clothing, operating under a fast-paced **Cathedral Watch Rotation (Guard Shifts)** to deduce the identities of the **Seer** and the **Witch**, learn the dark **Revival Sigil**, and unlock the main teleport platform to bring her party through.

All player-facing text must follow the writing guide: **simple sentences and everyday words for teenagers**. Avoid archaic or pseudo-academic jargon (use "priest" instead of "hierophant", "hall" instead of "peristyle", "pill" instead of "tincture", "spirit" instead of "wraithly apparition").

---

## 1. The Lore & Narrative Spine: The Monster Assembly Line

Up until Floor 6, the player has encountered monsters as combat obstacles and seen people taken away as "offerings." Floor 6 exposes the industrial pipeline that connects them:

```
[Floor 4 / Worker Towns]  ──(Harvest)──────>  Civilians selected as "Offerings"
                                                   │
                                                   ▼
[Floor 5 / Frost Citadel] ──(Execution)────>  Caged in Courtyard & Systematically Executed
                                                   │
                                                   ▼
[Floor 6 / High Sanctuary]──(Reprogramming)─>  Corpses Revived via Revival Sigil
                                              Memories Erased via "Rite of Oblivion"
                                              Bound into Dungeon Monsters (Golems, Elves, Beasts)
                                                   │
                                                   ▼
[Floors 1, 2, 3]          ──(Deployment)───>  Stocked on lower floors as game pieces
```

### The Truth of the Disappeared
In the High Sanctuary’s mortuary records and embalming vaults, Layla finds the ledger of transitions:
- **Sakhra** (Zayd's father) was an executed miner from Deepwell, revived and sealed in steam-driven stone.
- **The Elf Queen** on Floor 2 was Zayd's mother, revived and placed in the chess ranks.
- **The Goblin Golems** on Floor 1 were the earliest batches of commoners.
- Every monster on the board was once a living person from the worker towns.

### The Spirits Mechanic
When an actor is defeated anywhere in the game, their physical shell collapses, but a **spirit** remains anchored in the room where they fell.
- Spirits are completely invisible, silent, and intangible to normal senses.
- They linger quietly in the room state, hidden until Layla consumes the **Spiritual Enhancer pill** on Floor 6.

---

## 2. Einar & Astrid's Conspiracy: The Infiltration Mission & Branching Descent

### Einar's Survival Dynamic: Fail-Forward Branching
Einar is a dedicated healer (`hp: 20`, `heal`, `hold`). During the Floor 5 siege, he is stationed in the central courtyard (`courtyard_center`) tending the wounded offerings. If the defense lanes hold, Einar survives. However, if a lane is overwhelmed and siege forces breach the courtyard center, Einar can be defeated.

Rather than a hard failure or immersion-breaking plot armor, Einar's survival status drives a **dynamic branching infiltration into Floor 6**:

```
                              【End of Floor 5 Siege】
                                         │
                    ┌────────────────────┴────────────────────┐
                    ▼                                         ▼
         【Path A: Scholar's Guidance】              【Path B: Shadow Hard Route】
               (Einar Survives)                           (Einar Fallen)
       • Einar explains the assassination pact    • Einar crumbles; retrieves satchel
       • Einar gifts Clandestine Token            • Discovers sealed Token & torn notes
       • Coaches Layla on Liturgical Dogma        • Enters Floor 6 blind (no quiz coaching)
       • Shares secret counter-phrase for Witch   • Witch is hostile (must be subdued)
                    │                                         │
                    ▼                                         ▼
       [Inspection Shift: answers quizzes]        [Inspection Shift: quizzes trigger alarms]
       [Free cloister navigation]                 [Must navigate crypts/bell towers/shadows]
       [Peaceful Witch contact]                   [Relief Shift Scriptorium heist for dogma]
```

### Path A: The Scholar's Guidance (Einar Survives)
- **Einar's Confession:** Before Layla teleports, Einar reveals his and Astrid's past:
  > *"Astrid and I didn't end up in those cages by accident. We formed a pact to cut the snake's head off. We planned to assassinate the **Seer**—the high priest who conducts the reanimation rites and holds the revival scroll. Without the Seer, the entire monster-making pipeline stops cold. We were betrayed before we could strike."*
- **The Clandestine Solo Token:** Einar gifts Layla a smuggled, single-use infiltration token (`item.clandestine_sanctuary_token`). It bypasses the High Sanctuary's security wards, but it can **only transport Layla alone**.
- **Liturgical Coaching:** Einar quizzes Layla on Floor 5, teaching her the answers to the High Sanctuary's dogma quizzes (liturgical hours, saintly titles, prayer responses). When Templar patrol guards challenge Layla at checkpoint archways, the player can answer correctly and walk unhindered.
- **The Secret Counter-Phrase:** Einar teaches Layla his apothecary greeting (*"mint, crushed moss, and mountain snow"*), allowing Layla to peacefully identify and converse with the Witch.

### Path B: The Shadow Hard Route (Einar Dies)
- **Retrieving the Token:** When Einar falls in combat, his physical shell crumbles into grey dust like all fallen beings in the dungeon. Left behind among the settling dust on the cobblestones is his leather satchel (`item.einar_satchel`), holding the clandestine token and his torn notes. Layla (or Astrid) picks it up directly from the ground.
- **Blind Infiltration:** Layla enters the High Sanctuary with zero coaching on monastic doctrine or guard rituals.
- **Brutal Checkpoint Quizzes:**
  - When Templar patrol guards stop Layla at checkpoint arches during Inspection Shifts, Layla has no answers.
  - Guessing wrong raises the Sanctuary **Alert Level**, locking down holy cloisters and dispatching Inquisitor hounds.
  - To avoid guards, Layla must take dangerous alternate routes: claustrophobic subterranean mortuary crypts, high-altitude bell tower catwalks, and smoke flues.
- **The Scriptorium Heist (Relief Shift):**
  - To survive subsequent Inspection Shifts, Layla must break into the heavily guarded Cathedral Scriptorium during a Relief Shift to steal liturgical catechism scrolls and learn the answers herself.
- **The Witch Confrontation:**
  - Without Einar's counter-phrase, the Witch assumes Layla is an Inquisitor assassin sent to silence her.
  - The Witch attacks with toxic vapors and flash powder; Layla must duel and subdue her to half HP before presenting Einar's keepsake to prove she is an ally.
- **Spiritual Resonance:**
  - Later, when Layla earns the Revival Sigil and Spiritual Sight, Einar's spirit is found lingering in the Floor 5 courtyard. Layla can revive him—but only as a monster thrall, delivering an unforgettable emotional climax.

### The Party Disconnection Rule
- When Layla is on a different floor from her party members, the party enters a **`[Disconnected]`** state.
- In the UI, companions are marked as *Off-Floor / Out of Range*.
- **Gameplay Constraint:** Layla **cannot issue party orders** (`guard`, `follow`, `support`, `strike`) to companions who are on a different floor.
- Companions can only cross between floors when Layla reaches and activates a **Main Teleportation Platform / Resonance Gate**.

---

## 3. Gameplay Genre: Social Deduction / Werewolf Infiltration & Cathedral Watches

The High Sanctuary is a soaring sacred acropolis under open sky, but the clergy and Templar garrison live in lockstep with the **Cathedral Chimes and Canonical Watches**. 

Rather than a sluggish, macro Day/Night cycle that strands the player waiting for hours, Floor 6 operates on **Cathedral Watch Rotations (Guard Shifts)** with tight, alternating micro-intervals:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   THE CATHEDRAL WATCH ROTATION                         │
├───────────────────────────────────┬────────────────────────────────────┤
│  INSPECTION SHIFT (~8-10 turns)   │   RELIEF SHIFT (~4-6 turns)        │
│  • Checkpoints manned by guards   │  • Guards rotate to garrison/mess  │
│  • Acolytes & monks in colonnades │  • Checkpoints UNMANNED            │
│  • Dogma Quizzes at major arches  │  • Free passage without quizzes    │
│  • Social inquiries & clues       │  • Scriptorium & Mortuary heists   │
│  • OPEN COMBAT FORBIDDEN          │  • Werewolf strikes / assassinations│
└───────────────────────────────────┴────────────────────────────────────┘
```

### The Inspection Shift: Investigation & Checkpoint Quizzes (~8–10 Turns)
- **Sanctuary Life:** Sunlight floods through stained glass and colonnades; incense smoke curls from bronze censers, and choir chants echo off marble walls. Layla moves freely through public basilicas, courtyards, and colonnaded walks.
- **The Peace of the Sanctuary (No Combat):** Drawing a weapon or attacking in public halls rings alarm bells, immediately swarming the room with invulnerable Templar vanguards.
- **Checkpoint Arches & Dogma Quizzes:**
  - Templar guards man the archways connecting major wings (`checkpoint_nave`, `checkpoint_cloisters`, `checkpoint_undercrypt`).
  - Guards challenge anyone seeking passage with faith quizzes on canonical hours, saintly titles, and orthodox chants.
  - **Path A (Einar Alive):** With Einar's prior coaching, the dialogue choices supply the right answers; Layla passes without friction.
  - **Path B (Einar Fallen):** Layla lacks the answers. Wrong answers raise the **Alert Level**, calling Inquisitor hounds and sealing doors. Layla must either sneak through high-altitude bell catwalks and crypt flues, or wait for the next Relief Shift.
- **Social Deduction:**
  - The high clergy wear uniform white vestments and silver masks concealing their faces behind monastic titles (*Curate*, *Precentor*, *Almoner*, *Archdeacon*, *Sacristan*).
  - Layla questions novices, inspects offering ledgers, and studies habits to deduce:
    1. **Who is the "Witch"?**
    2. **Who is the "Seer"?**

### The Relief Shift: The Blind Window (~4–6 Turns)
- **The Watch Bell Chimes:** The massive iron bells in the skyward towers toll for the change of watch. Guards step down from their posts and march in squads to the barracks and mess halls for roll call.
- **Unmanned Checkpoints:** Checkpoint archways are left empty! Layla can walk right through without facing any dogma quizzes—a critical infiltration window, especially for Path B.
- **The Werewolf Strike (Silent Assassinations):**
  - Combat restrictions lift in private or unmonitored chambers.
  - Layla can stalk corrupt clergy, eliminate lone night sentries, or corner her primary targets.
  - As long as Layla eliminates targets in isolated rooms before any witness flees to sound an alarm, the sanctuary remains oblivious.
- **Heists and Infiltration:**
  - The Scriptorium and lower Mortuary vaults become vulnerable to break-ins while scribes and embalmers attend services.

### Zero Downtime: Fast-Forward & Sabotage Mechanics
To ensure Layla never sits idle waiting for shifts to change:
1. **Prayer Benches (`rest` at Prie-Dieu):**
   - Side chapels throughout the basilicas contain prie-dieu prayer desks. Using `rest` allows Layla to advance the clock directly to the next watch bell chime.
2. **Shift Sabotage (Tampering):**
   - Interacting with the **Cloister Water Clock** or snuffing central **Incense Flues** creates procedural anomalies that trigger early guard rotations or prolong a relief window.

---

## 4. The Witch, The Seer, and The Revival Sigil

### 1. The Witch $\rightarrow$ The Spiritual Enhancer Pill
- **Deduction:** Clues reveal a temple apothecary who secretly harbors sympathy for the victims, smelling of mint and crushed moss rather than ceremonial incense.
- **Confrontation:**
  - **Path A (Einar Alive):** Approached in secret using Einar's counter-phrase (*"mint, crushed moss, and mountain snow"*), the Witch recognizes an ally and willingly yields the **Spiritual Enhancer Pill** (`item.spiritual_enhancer`).
  - **Path B (Einar Fallen):** Without the counter-phrase, the Witch believes Layla is an Inquisitor executioner sent to eliminate her and attacks with toxic vapors and flash powder. Layla must duel and subdue her to half HP, then display Einar's keepsake/satchel to de-escalate the fight and earn the pill.
- **Spiritual Sight:** Swallowing the pill permanently alters Layla's vision:
  - Rooms where actors were defeated now display faint, translucent spirits lingering near the walls.
  - Layla can inspect these spirits to read their residual memories and identify who they were before they died.

### 2. The Seer $\rightarrow$ The Revival Sigil
- **Deduction:** Clues point to the high priest whose fingers are stained with mortuary bitumen, who alone possesses the bronze key to the Grand Mortuary.
- **Assassination:** Layla corners and defeats the Seer during a Relief Shift (or within the sealed Mortuary).
- **The Loot:** The Seer drops the **Torn Parchment of Reanimation**, teaching Layla the **Revival Sigil** (`revive-sigil`).

### 3. The Monster Twist: "Revived, But Bound"
- Layla can trace the Revival Sigil in any room containing a lingering spirit to bring it back to life.
- **The Moral & Mechanical Catch:** The sigil is a dungeon-master tool designed for the monster factory. Layla **cannot restore human form**—she can only revive the spirit as a **monstrous thrall** (a stone golem, a shadow stalker, or an animated soldier) bound to her party.
- This creates deep thematic weight: Layla is wielding the very power the corporation uses, forcing her to confront whether she is becoming the Dungeon Master or using their weapons to destroy the system.

---

## 5. Climax: Cathedral Chaos & Party Convergence

1. **The Fall of the Seer:**
   - Slain by Layla, the Seer's death shatters the High Sanctuary's command structure.
   - The liturgical clock breaks down; alarm sirens scream, acolytes scatter in panic, and the Inquisitors lose control of the cathedral.
2. **Breaching the Main Teleport Platform:**
   - In the chaos, Layla fights her way to the **Cathedral Sanctum Gate / Main Teleportation Platform** (`sanctuary_main_platform`).
   - Layla traces the master resonance anchor, establishing a permanent cross-floor link back to Floor 5.
3. **The Party Arrives:**
   - Commander Astrid, Sakhra, and surviving allies materialize on the platform!
   - If Einar survived Floor 5, he teleports in beside Astrid and immediately sets up a triage post to treat traumatized acolytes.
   - If Einar fell on Floor 5, Astrid solemnizes his memory as she locks shields in the vanguard. (Furthermore, Layla now holds the power to return to Floor 5 to view or revive Einar's lingering spirit).
   - The **`[Disconnected]`** status clears.
   - Astrid establishes a defensive perimeter with her great tower shield, and the united party punches through to **Floor 7 (The Royal Core / Dungeon Master)**.

---

## 6. Quests Architecture (Floor 6)

Following our established quest principles (*clear goals, no step-by-step handholding, 'What, Not How'*):

| Quest Type | Quest Title | Summary | Goal |
|---|---|---|---|
| **Main** | **The High Sanctuary Infiltration** | Infiltrate the Cathedral alone, assassinate the Seer to stop the monster conversion rites, and unlock the Main Teleport Platform. | Slay the Seer and activate the main platform to bring your party through. |
| **Side** | **The Witch's Formula** | Track down the rogue temple apothecary hidden among the clergy and obtain the formula for spiritual sight (peacefully via Einar's phrase during a Relief Shift, or by subduing her in Path B). | Locate the Witch and acquire the Spiritual Enhancer pill. |
| **Side (Path B)** | **The Scriptorium Catechisms** | *(Active only if Einar died)* Break into the locked Scriptorium during a Relief Shift to steal theological manuscripts and master the checkpoint quiz answers. | Recover the High Sanctuary Catechisms to pass Inspection Shift checkpoints. |
| **Secret** | **The Archive of the Disappeared** | Locate the confidential mortuary ledgers in the Cathedral Undercrypt to uncover the original human identities of every monster in the dungeon. | Discover the true origins of Sakhra, the Elf Queen, and the goblin golems. |

---

## 7. Deferred Implementation Note
This document serves as the approved blueprint for Floor 6. Active development continues on **Floor 5** (the five worker towns, survivor interactions, local items, and siege tuning) before building Floor 6's rooms and mechanics.
