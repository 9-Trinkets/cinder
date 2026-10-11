# Floor 6 — The High Sanctuary (The Clergy & The Inquisition)

Living design plan for **Level 6** of Layla. Like earlier levels (the deep forest on Floor 2, the geothermal valley on Floor 4, and the snow peaks and glaciers on Floor 5), Floor 6 is a distinct magical partitioned world: a breathtaking sacred acropolis set under open skies, featuring soaring white limestone basilicas, sunlit colonnades, open cloister courtyards, terraces overlooking endless seas of clouds, towering bell towers, and quiet soul vaults below.

Here, Layla discovers the terrifying truth of the dungeon: it is a **closed-loop monster creation pipeline** that takes workers from the worker towns, separates their spirits into soul containers in Floor 5's citadel, and re-binds them in Floor 6 into dungeon monsters, workers, and utility guardians.

The gameplay genre for Floor 6 is **Social Deduction / Infiltration (Werewolf / Mafia)**. Layla infiltrates the cathedral alone as the "Werewolf" in sheep's clothing, operating under a fast-paced **Cathedral Watch Rotation (Guard Shifts)** to deduce the identities of the **Seer** and the **Witch**, learn the dark **Revival Sigil**, and unlock the main teleport platform to bring her party through.

All player-facing text must follow the writing guide: **simple sentences and everyday words for teenagers**. Avoid archaic or pseudo-academic jargon (use "priest" instead of "hierophant", "hall" instead of "peristyle", "pill" instead of "tincture", "spirit" instead of "wraithly apparition").

---

## 1. The Lore & Narrative Spine: The Monster Creation Pipeline

Up until Floor 6, the player has encountered monsters as combat obstacles and seen people taken away as "offerings." Floor 6 exposes the factory pipeline that connects them:

```
[Floor 4 / Worker Towns]  ──(Harvest)──────>  Civilians selected as "Offerings"
                                                   │
                                                   ▼
[Floor 5 / Frost Citadel] ──(Separation)───>  Caged in Courtyard; Spirits Separated from Shells
                                                   │
                                                   ▼
[Floor 6 / High Sanctuary]──(Re-binding)───>  Spirits Sealed in Anchored Soul Containers
                                              Memories Erased via "Rite of Oblivion"
                                              Bound into Monster Shells (Golems, Elves, Beasts)
                                                   │
                                                   ▼
[Entire Kingdom & Dungeon]──(Deployment)───>  Deployed as Workers, Guards, and Utilities
                                              (Deepwell, Worker Towns, and Dungeon Floors 1-3)
```

### The Truth of the Disappeared
In the High Sanctuary’s soul vaults and binding records, Layla finds the ledger of transitions:
- **Sakhra** (Zayd's father, Jamil) was a miner from Deepwell, his spirit bound into steam-driven stone to guard his home town.
- **The Elf Queen** on Floor 2 was Zayd's mother, her spirit bound into an elf chess piece.
- **The Goblin Golems** on Floor 1 were early batches of workers bound to stone to guard the borders.
- Every monster on the board and in the kingdom was once a living person from the worker towns.

### The Spirits & Defeat Mechanics: Dust, Drops, and Anchored Spirits

1. **The Crumble-to-Dust Rule (No Human Corpses):**
   - In Cinder and Layla lore, living beings, constructs, and monsters are digital constructs running on partitioned world threads. When an actor's HP reaches 0, their physical shell dissolves into grey dust (`combat.actor_defeated`: *"The {actor} crumbles to dust."*).
   - There are **no human corpses, bloodied cadavers, or anatomical remains left on the battlefield** to loot or autopsy.
2. **Defeat Drops & Loose Items:**
   - When an actor crumbles away, physical possessions do not vanish. The engine's defeat system (`spawn_defeat_drops`) drains their inventory onto the cobblestones of the room (`combat.defeat_drop`: *"From the settling dust, something remains: {items}."*).
   - Any dropped satchel, weapon, or key becomes a loose room object that can be picked up with `take <item>` or collected in post-combat dialogue.
3. **The Anchored Spirit:**
   - While the physical shell dissolves into dust, the metaphysical data container—the **spirit**—remains anchored invisibly in the exact room where the actor fell.
   - Spirits are completely silent and intangible to normal senses. They linger quietly in the room state, hidden until Layla consumes the **Spiritual Enhancer pill** on Floor 6.

---

## 2. Einar & Astrid's Conspiracy: The Infiltration Mission & Branching Descent

### Einar's Survival Dynamic: Fail-Forward Branching
Einar is a dedicated healer (`hp: 20`, `heal`, `hold`). During the Floor 5 siege, he is stationed in the central courtyard (`courtyard_center`) tending the wounded offerings. If the defense lanes hold, Einar survives. However, if a lane is overwhelmed and siege forces breach the courtyard center, Einar can be defeated.

Rather than a hard failure or immersion-breaking plot armor, Einar's survival status drives a **dynamic branching infiltration into Floor 6**:

```
### The Lovers & The Infiltration Mission
- **The Star-Crossed Lovers (Werewolf Cupid Role):**
  - Einar and the Witch (Sister Orva—named for the Old Norse word for "arrow"—the cathedral herbalist) fell in love before the cathedral gates were shut.
  - She remained inside as an undercover ally while Einar fought alongside Astrid.
  - Their shared keepsake: **The Twin Arrow** (a pair of matching silver arrow hairpins).
  - Their secret phrase: **"The arrow always finds home."**
- **Einar's Gift (Before Teleporting to Floor 6):**
  - **1x Clandestine Sanctuary Token:** Teleports Layla alone into the High Sanctuary terrace.
  - **3x Invisibility Potions:** Cool glass bottles that hide Layla from cathedral guards for 8 turns each.
  - **The Keepsake (Silver Arrow Hairpin):** Einar's half of the twin arrow.
  - **The Mission Briefing:**
    > *Astrid: "The Frost Citadel is secure, Layla, but our path to the Royal Core is blocked. High above us sits Floor 6—the High Sanctuary. Its brass gates are sealed from within. Our army cannot storm those walls from the outside. If we want to reach the throne, someone must slip inside and activate their master teleport platform."*
    > *Einar: "That cathedral is not a place of prayer, Layla. Before they threw me into the cages down here, I worked in their dispensary. The high clergy takes the spirits separated from the people in this citadel, wipes their memories, and binds them into monster shells that guard the kingdom and the lower floors. The man conducting this horror is their high priest, Seer Valerius. Valerius hides the ritual behind thick illusions, but someone inside can help. My partner, Orva, is trapped in the cathedral herb gardens. The priests call her 'the Witch' because she secretly brews remedies to resist their mind-wiping magic. She is also... the woman I love. Take this token to slip past their outer wards, and use these invisibility potions to walk past the guards. Find Orva, show her this arrow pin, and say: 'The arrow always finds home.' She will give you the spirit pill to reveal Valerius's ritual and defeat him."*
- **Fail-Forward Path (If Einar Fell on Floor 5):**
  - When Einar falls in the siege, his physical shell crumbles into grey dust. Layla or Astrid retrieves his leather satchel (`einar-satchel`) from the dust.
  - Inside the satchel: the Clandestine Token, the 3 Invisibility Potions, Einar's silver arrow hairpin, and his farewell note with the secret phrase.
  - When Layla meets Orva in the garden and presents Einar's arrow, Orva realizes Einar didn't make it. Though heartbroken, she resolves to help Layla finish what they started and yields the Spirit Pill.

---

## 3. Gameplay: Invisibility Stealth & Finding the Witch

Floor 6 cuts out complicated shift schedules and trivia tests in favor of direct, tense stealth gameplay and social deduction:

1. **Cathedral Patrols & Invisibility:**
   - White-armored cathedral guards patrol the major crossroads and basilicas. If they spot Layla, they attack as hostile enemies.
   - Layla drinks an **Invisibility Potion** (`drink potion` / `use invisibility-potion`).
   - For **8 turns**, she fades from sight. Cathedral guards cannot target or attack her while she is invisible.
   - Layla can freely walk through halls, explore rooms, and search for the Witch. Attacking an enemy breaks invisibility immediately.
   - With 3 finite bottles, the player must choose their stealth windows wisely.
2. **Deducing the Witch Among the Herbarium Staff:**
   - In the cathedral herb garden (`herbarium_garden`) and adjacent dispensary (`herbarium_apothecary`), multiple botanical staff work:
     - **Flora**: An apprentice botanist sorting dried flower petals with a bronze leaf pin in her red hair.
     - **Silvia**: A gardener tending damp moss beds with a plain leather cord tying back her braid.
     - **Elder Fabian**: An elderly apothecary in the dispensary hall grinding roots in spectacles with no hair ornaments.
     - **Orva**: A quiet herbalist with dark hair, wearing the distinct **silver arrow hairpin** matching Einar's keepsake.
   - Layla must inspect the workers (`look at <person>`) to spot the silver arrow clue, then speak to Orva with the code phrase: *"The arrow always finds home"*.
   - Orva recognizes the token and phrase, confides in Layla, and gives her the **Spirit Pill** (`spirit-pill`).

---

## 4. The Spirit Pill, Seer Valerius, and The Reanimation Sanctum

1. **Swallowing the Spirit Pill:**
   - Layla swallows the pill (`use spirit-pill` / `swallow pill`).
   - Her eyes tingle as she gains **Spiritual Sight** (`spiritual_sight = "true"`).
2. **The Reanimation Sanctum Reveal:**
   - Layla drinks a potion and slips down into the dark stone reanimation sanctum (`mortuary_reanimation_sanctum`).
   - Before taking the pill, the room appeared to be a quiet, cold hall of stone tables and lanterns.
   - With **Spiritual Sight** active, the room is revealed in all its horror:
     > *The chamber is filled with dozens of glowing, translucent human spirits floating silently above the altar—the souls of offerings waiting to be wiped and bound into monster shells.*
     > *In the center of all the glowing spirits stands **a single living man** in dark robes conducting the binding rite: **Seer Valerius**.*
3. **The Immediate Deduction:**
   - Layla knows at once: *Every other figure in this room is a ghost. The only living person standing among them is Seer Valerius!*
4. **The Boss Fight & Revival Sigil:**
   - Layla engages and defeats Seer Valerius in combat.
   - Seer Valerius drops the **Revival Sigil Scroll**, teaching Layla the `revive-sigil`.

---

## 5. Climax: Main Platform Activation & Party Reunion

1. **Activating the Platform:**
   - In the adjoining chamber (`sanctuary_main_platform`), Layla activates the master brass teleport platform.
   - This opens the permanent cross-floor portal back to Floor 5's courtyard (`courtyard_center`).
2. **The Party Arrives:**
   - Astrid, Einar (if he survived), and Layla's party materialize on the platform!
   - The party disconnection state clears, restoring full party orders and dialogue.
   - If Einar survived, he and Orva share a heartfelt reunion in the herb garden.
   - Astrid plants her great shield, securing the high acropolis as the party prepares to breach **Floor 7 (The Royal Core / Dungeon Master)**.

---

## 6. Quests Architecture (Floor 6)

| Quest Type | Quest Title | Summary | Goal |
|---|---|---|---|
| **Main** | **The High Sanctuary Infiltration** | Infiltrate the cathedral alone, find the Witch in the herb garden, discover the Seer, and open the main platform. | Slay the Seer and activate the main teleport platform to bring your party through. |
| **Side** | **The Witch's Keepsake** | Find the herbalist wearing the silver arrow hairpin in the herb garden and deliver Einar's code word to receive the Spirit Pill. | Find Orva in the herb garden and speak the secret phrase. |
| **Secret** | **The Memory of the Offerings** | Inspect the floating spirits in the reanimation sanctum with Spiritual Sight to recognize the lost townspeople. | Discover the true human origins of the dungeon monsters. |
