# Floor 6 — The High Sanctuary (The Clergy & The Inquisition)

Living design plan for **Level 6** of Layla. Like earlier levels (the deep forest on Floor 2, the geothermal valley on Floor 4, and the snow peaks and glaciers on Floor 5), Floor 6 is a distinct magical partitioned world: a breathtaking sacred acropolis set under open skies, featuring soaring white limestone basilicas, sunlit colonnades, open cloister courtyards, terraces overlooking endless seas of clouds, towering bell towers, and quiet soul vaults below.

Here, Layla discovers the terrifying truth of the dungeon: it is a **closed-loop monster creation pipeline** that takes workers from the worker towns, separates their spirits into soul containers in Floor 5's citadel, and re-binds them in Floor 6 into dungeon monsters, workers, and utility guardians.

The gameplay genre for Floor 6 is **Social Deduction / Infiltration (Werewolf / Mafia)**. Layla infiltrates the cathedral alone as the "Werewolf" in sheep's clothing, operating under a fast-paced **Cathedral Watch Rotation (Guard Shifts)** to deduce the identities of the **Seer** and the **Witch**, learn the arcane **Bind Sigil** (`bind-sigil`), and unlock the main teleport platform to bring her party through.

All player-facing text must follow the writing guide: **simple sentences and everyday words for teenagers**. Avoid archaic or pseudo-academic jargon (use "priest" instead of "hierophant", "hall" instead of "peristyle", "pill" instead of "tincture", "spirit" instead of "wraithly apparition").

---

## 1. The Lore & Narrative Spine: The Monster Creation Pipeline

Up until Floor 6, the player has encountered monsters as combat obstacles and seen people taken away as "offerings." Floor 6 exposes the factory pipeline that connects them. The conversion process consists of three distinct stages:

```
[The 3-Step Monster Conversion Pipeline]

1. Kill the Offering       ──> Physical shell is eliminated; spirit/ether is stripped of secular impurities.
         │
         ▼
2. Spawn Monster Shell    ──> High Sanctuary priests trace the Spawn Sigil (生紋) to manifest a vessel.
         │
         ▼
3. Bind Spirit to Shell   ──> Priests execute the Rite of Oblivion and trace the Bind Sigil (縛紋),
                              anchoring the amnesiac spirit into the monster construct.
```

```
[Floor 4 / Worker Towns]  ──(Harvest)──────>  Civilians selected as "Offerings"
                                                   │
                                                   ▼
[Floor 5 / Frost Citadel] ──(Sorting)─────>   Caged in Courtyard; Prepared for Transit
                                                   │
                                                   ▼
[Floor 6 / High Sanctuary]──(Conversion)──>   1. Execution (Flesh dissolved)
                                              2. Spawn Shell (Spawn Sigil / 生紋)
                                              3. Memory Erased & Spirit Bound (Bind Sigil / 縛紋)
                                                   │
                                                   ▼
[Entire Kingdom & Dungeon]──(Deployment)───>  Deployed as Workers, Guards, and Utilities
                                              (Deepwell, Worker Towns, and Dungeon Floors 1-3)
```

### The Half-Ling Servants & Infiltration Reality
- The cathedral's servants and laborers are a distinct demi-human race: **Halflings (半身人)**, standing only 1.2–1.3m tall, wearing white porcelain half-masks (`白瓷半面具`) and sweeping hooded robes, with nimble fingers and rounded furry ear tips.
- Layla (1.65m human) cannot simply disguise herself in servant robes—she would be spotted instantly. She must move across the high vaulted rafters, ventilation conduits, and shadows using finite **Invisibility Potions**.
- **Why Einar was condemned**: In the strict, purity-obsessed clergy, a human healer falling in love with an enslaved Halfling herbalist was branded the ultimate heresy. They branded Orva a "Witch" and cast Einar into the dungeon cages.

### The Truth of the Disappeared: The Black Binding Ledger
In the High Sanctuary’s soul vaults (`mortuary_reanimation_sanctum`) and secret archives (`scriptorium_secret_archive`), Layla reads the black leather binding ledgers:
- **Siraj** (西拉傑, miner from Deepwell, spouse Nur, son Zayd): Bound into steam-driven stone guardian **Sakhra** (石守).
- **Nur** (努爾, miner from Deepwell, spouse Siraj, son Zayd): Bound into the **Elf White Queen** (白棋王后) on Floor 2.
- **Orin** (歐林, stonecutter): Bound into Black Golem #1 (**Mr. Black** / 黑先生).
- **Mari** (瑪莉, miller): Bound into Black Golem #2 (**Miss Black** / 黑小姐).
- **Ferid** (法瑞德, miner): Bound into White Golem #1 (**Mr. White** / 白先生).
- **Hana** (哈娜, weaver): Bound into White Golem #2 (**Miss White** / 白小姐).

*The Guilt of the Player/Layla*: Layla realizes she had ordered "Mr. Black" to sacrifice his granite body against the steam battering ram on Floor 5, calculating his loss as a game piece, only to discover here that he was Orin, a flesh-and-blood stonecutter.

---

## 2. Einar & Astrid's Conspiracy: The Infiltration Mission & Branching Descent

### The Lovers & The Infiltration Mission
- **The Star-Crossed Lovers (Werewolf Cupid Role):**
  - Einar and the Witch (Sister Orva—the Halfling cathedral herbalist) fell in love before the cathedral gates were shut.
  - She remained inside as an undercover ally while Einar fought alongside Astrid.
  - Their shared keepsake: **The Twin Arrow** (a pair of matching silver arrow hairpins).
  - Their secret phrase: **"The arrow always finds home."**
- **Einar's Gift (Before Teleporting to Floor 6):**
  - **1x Clandestine Sanctuary Token:** Teleports Layla alone onto the High Sanctuary grand terrace.
  - **3x Invisibility Potions:** Cool glass bottles that hide Layla from cathedral guards for 8 turns each.
  - **The Keepsake (Silver Arrow Hairpin):** Einar's half of the twin arrow.
- **Fail-Forward Path (If Einar Fell on Floor 5):**
  - When Einar falls in the siege, his physical shell crumbles into grey dust. Layla retrieves his leather satchel (`einar-satchel`) containing the token, potions, silver arrow, and farewell note.

---

## 3. Gameplay: Invisibility Stealth & Finding the Witch

Floor 6 features tense stealth gameplay and social deduction:

1. **Cathedral Patrols & Invisibility:**
   - White-armored cathedral guards patrol the major crossroads and basilicas.
   - Layla drinks an **Invisibility Potion** (`drink potion` / `use invisibility-potion`) for 8 turns of stealth.
2. **Deducing the Witch Among the Herbarium Staff:**
   - In the glass solarium (`herbarium_garden`), Halfling servants in porcelain masks work:
     - **Flora**: Apprentice with a bronze leaf pin in her red hair.
     - **Silvia**: Gardener with a plain leather cord tying back her blonde braid.
     - **Orva**: Quiet herbalist with soft dark hair, rounded furry ear tips, and the matching **silver arrow hairpin** tucked inside her collar. Her straw shoes are flecked with alpine dark soil and fresh cliff moss.
   - Layla delivers Einar's code phrase: *"The arrow always finds home"*.
3. **The Secret Cliff Balcony & The Spirit Capsule:**
   - Through a hidden vent flue beneath the water basin, Layla accesses **The Secret Cliff Balcony** (`herbarium_cliff_balcony`).
   - Nestled beneath overhanging buttresses above the sea of clouds, Orva cultivates wild soul grass warmed by a steam pipe.
   - Orva collects fresh luminous dew into a thumb-sized metal capsule: the **Spirit Capsule** (`spirit-pill` / `spirit capsule`).
   - Through a chained iron door on the balcony, a secret spiral stair descends directly into the Reanimation Sanctum (`mortuary_reanimation_sanctum`).

---

## 4. The Spirit Capsule, The Dragon Chimera Boss, and Bind Sigil

1. **Swallowing the Spirit Capsule:**
   - Layla swallows the capsule (`swallow capsule` / `use spirit-pill`).
   - Gains **Spiritual Sight** (`spiritual_sight = "true"`).
2. **The Reanimation Sanctum Reveal:**
   - The chamber is filled with hundreds of glowing, translucent human spirits floating silently above altars.
   - Standing before the obsidian altar is **Seer Valerius**.
3. **The Boss Fight: The 20-Meter Stone & Dragon Bone Chimera (`sanctuary_dragon`):**
   - Valerius channels the **Spawn Sigil** (生紋) to fuse ancient dragon bones and obsidian rocks into a colossal 20m dragon construct, then uses the **Bind Sigil** (縛紋) to rip captive spirits from the altar and chain them into its heart.
   - **4-Sigil Tactical Showdown:**
     - **Teleport Sigil (傳紋)**: Displaces Layla away from crushing claw slams.
     - **Spawn Sigil (生紋)**: Manifests spinning fire sprites to blind the dragon's ghostfire eyes and misdirect high-temperature breath.
     - **Drain Sigil (噬紋)**: Traps the rear foot in an inverse spiral, draining mana and crumbling the support bones.
     - **Charm Sigil (魅紋)**: Layla leaps onto the dragon's spine, closing a geometric boundary ring over the chest Bind Sigil to sever Valerius's red mana threads and liberate the captive souls.
   - The dragon collapses into grey dust like falling snow. Valerius suffers fatal magic recoil, his bone staff shatters, he crumbles to dust, and drops the **Bind Sigil Scroll** (`bind-scroll`).

---

## 5. Climax: Main Platform Activation, True Names, and Harun's Capture

1. **Activating the Platform:**
   - On the Grand Platform (`sanctuary_main_platform`), Layla inserts the five town tokens into the console and channels the master anchor, launching a massive pillar of light connecting to Floor 5.
2. **The Party Arrives & Awakenings:**
   - Astrid, Elwin, Queen Nur, Sakhra (Siraj), Einar, and the golems step onto the platform.
   - **Awakening Siraj & Nur**: Layla channels the **Bind Sigil** through **Zayd's Amber Miner Lamp**, reconnecting their memories. Siraj ("I am Zayd's father; my name is Siraj") and Nur ("I am Zayd's mother; my name is Nur") tearfully embrace.
   - **Freeing the Golems**: Layla unbinds Mari, Ferid, and Hana from servitude. Orin's golden spirit spark smiles from the lantern and dissipates peacefully into the cloud sea.
   - **Harun Captured**: Priest Harun attempts to flee across the terrace to the freight elevator. Astrid intercepts him with a heavy tower shield slam, Elwin pins him with an arrow, and fortress guards bind him in chains.
   - **Einar & Orva Reunion**: Einar and Orva reunite, joining their two halves of the silver arrow into feathered wings.
   - **The Operator's Signal**: "The staircase is open. I will wait for you on Floor 7, Layla." A magnificent white staircase rises towards Floor 7: The Royal Core.

---

## 6. Quests Architecture (Floor 6)

| Quest Type | Quest Title | Summary | Goal |
|---|---|---|---|
| **Main** | **The High Sanctuary Infiltration** | Infiltrate the cathedral alone, find the Witch, conquer the Seer's Dragon construct, and open the main platform. | Slay the Dragon Chimera & Seer, and activate the main teleport platform to bring your party through. |
| **Side** | **The Witch's Keepsake** | Find the Halfling herbalist wearing the silver arrow hairpin and deliver Einar's code word to receive the Spirit Capsule. | Find Orva and speak the secret phrase. |
| **Secret** | **The Memory of the Offerings** | Inspect the black binding ledger in the Reanimation Sanctum with Spiritual Sight to uncover the true names of the golems and Zayd's parents. | Discover the human origins of the dungeon monsters and the true identities of Orin, Mari, Ferid, Hana, Siraj, and Nur. |
