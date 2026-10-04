# Floor 5 — The Frost Citadel (The Hexagon Courtyard & The Three Houses)

Living design plan for **Level 5** of Layla. This floor covers the ice world of the mountain castle, the 7-room hexagonal courtyard, the mass suspension cages of virtual "offerings," and the strategic siege by three rival noble houses.

The gameplay genre for Floor 5 is **Tower Defense (Real-Time Strategy)**. Layla and her allies hold the three estate approaches at `courtyard_north`, `courtyard_southeast`, and `courtyard_southwest`. The house leaders remain sealed inside their throne rooms until the final wave, when all three march toward the courtyard with their armies.

All player-facing text must use **simple sentences and everyday words for teenagers**. Avoid unusual, archaic, or overly technical jargon (use "cage" instead of "oubliette", "healer" instead of "thaumaturge", "guard" instead of "interdict", "pill" instead of "elixir").

### Implementation Status and Next Slice

Implemented:

- the courtyard, three estates, and throne rooms;
- the sentry ambush, cage key, prisoner rescue, Astrid, and Einar;
- the sensory enhancer and enhanced tactical map;
- the three house leaders and their defeat/awakening paths;
- the continuous 10/70/130-minute siege clock, timed estate gates, and
  final-wave throne seals;
- all three house dispatch queues: each house sends 30 soldiers in ten squads
  of three at ten-minute intervals, and its queue stops when its leader is
  defeated or awakened;
- the five named civilian offerings as individual actors;
- the center-breach protection mission: tagged siege troops in
  `courtyard_center` start a ten-minute warning, clearing the center cancels
  it, victims rotate deterministically, and the third death marks the side
  quest failed without ending the siege;
- leader neutralization now routs that house's deployed soldiers, and stopping
  all three command lanes ends the siege and begins survivor resolution;
- siege completion now counts the five civilians, completes or preserves the
  failed protection quest, grants each survivor's permanent town token, and
  unlocks the Six-Town Accord when all five survive;
- Salt Reach, Glassbank, Woolcross, Greenrest, and Brass Yard now have initial
  teleport landing hubs as optional Floor 5 map extensions, each gated by its
  survivor token;
- the High Sanctuary Gate opens after the siege regardless of civilian losses
  or town visits, completing the route toward Floor 6.

The siege begins when the prisoners are freed and advances on a continuous
in-game clock:

| Time from rescue | Event | Estate access |
|---|---|---|
| 0 minutes | Preparation begins | All estate gates remain closed |
| 10 minutes | Wave 1 begins: Frost-Wolf lane | Frost-Wolf gate opens |
| 70 minutes | Wave 2 begins: Iron-Ram joins | Iron-Ram portcullis opens |
| 130 minutes | Wave 3 begins: Frost-Leopard and all three house leaders join | Frost-Leopard doors and all throne rooms open |

The clock never pauses while Layla explores. Before Wave 3, Layla can enter
each opened estate as far as its muster room, but the throne room remains
sealed. When Wave 3 begins, Lord Vane, Warmaster Torin, and Lady Sylvan leave
their throne rooms and march through their own estates toward
`courtyard_center`. Neutralizing a leader during this final assault stops and
routs that house's remaining soldiers.

The next implementation slice builds deeper content inside the five optional
worker towns: local residents, town problems, items, awakening and charm
opportunities, and possible recruits. Guard orders and party-wide hostility
already provide the defender-placement mechanics; Floor 5 does not model a
separate guarded-lane state.

---

## 1. Story and Setting

### The Pure Digital Reality: 1s and 0s
There are no mystical gods, souls, or spiritual magic here. **Everything in the dungeon is pure computer code—literally 1s and 0s.**

- The towns, castle walls, and howling snowstorms are simulated environments running on partitioned server threads.
- The people (miners, elves, nobles, priests, children) are virtual objects and data containers.
- The "offering" and "cull" are routine **reprogramming cycles**: deleting an object's memories, resetting its parameter tables, and compiling its code into a new construct or monster class.
- The **Handler** is a supervisor program tasked with running the simulation and reporting metrics to corporate overseers.
- Layla herself is an advanced game-playing artificial intelligence whose amnesia was an imposed data-wipe. Her memories returning are subroutines re-linking to her core code.

### The Setting — An Ice World Inside a Mountain Castle
Layla teleports from Floor 4 expecting another underground mine. Instead, freezing wind and snow swirl around her. 

She stands inside the open central courtyard of a colossal stone fortress: **The Frost Citadel**. 
Looking up past the high battlements, she sees sheer snowy peaks, hanging glaciers, and a violet-tinted winter sky. This stark environmental contrast immediately proves that **the floors are separate virtual worlds**, partitioned to test different social setups and combat rules.

### The Opening Confrontation: Ambush & The Key
1. **The Teleport Arrival:** Layla arrives on the teleport pad at `courtyard_center`.
2. **The Sight:** Five iron offering cages surround the courtyard teleport pad. Each holds one named worker selected from a different town:
   - `BLOCK 01 — SALT REACH`
   - `BLOCK 02 — GLASSBANK`
   - `BLOCK 03 — WOOLCROSS`
   - `BLOCK 05 — GREENREST`
   - `BLOCK 06 — BRASS YARD`
   Town 04 is **Deepwell**, the mining and steam-work town Layla visited on Floor 4. Its people include Zayd, Rashid, Yasmin, Tariq, and Jamil/Sakhra, so Layla already knows its teleport anchor.
3. **The Ambush:** The courtyard sentries spot her instantly: *"Unsanctioned entity on the pad! Neutralize!"*
4. **The Battle:** Layla fights off the initial wave of sentries.
5. **The Key:** The defeated sentry captain drops the **Courtyard Cage Key** (`courtyard-cage-key`).
6. **Unlocking the Cages & Einar's Gift:**
   - Layla unlocks the iron cage doors. The civilians huddle safely in the center around the teleport pad.
   - Two high-tier prisoners step forward and volunteer to fight alongside Layla:
     - **Commander Astrid** — A formidable female garrison commander in battered heavy plate armor.
     - **Einar** — A compassionate male healer originally from Floor 6.
   - Seeing Layla prepare to defend the courtyard against the surrounding garrison, Einar pulls a small, translucent blue capsule from the lining of his sleeve: the **Sensory Enhancer** (`sensory-enhancer`).
   - Einar explains:
     > *"You pulled us out of the grinder, but they're already mobilizing the garrison. Take this sensory enhancer. Back in the High Sanctuary, we synthesized it to heighten human senses and expand spatial perception. It will clear the static in your mind. You'll be able to feel where your allies stand through the fortress stone, and sense every active gateway across the citadel."*

### The Sensory Enhancer: Heightened Senses & Upgraded Map
Layla swallows the translucent blue capsule (`use sensory-enhancer`).
- An electric chill shoots through her code. Her internal visual renderer glitches and reboots with an expanded, heightened HUD:
  - **Live Ally Trackers:** Every room containing an allied defender shows an ally badge (`[A]`) with companion names and health status.
  - **Teleport Anchors:** Every room with an active teleport node shows a glowing portal beacon (`[T]`).
- This tactical map upgrade gives the player real-time situational awareness during the multi-front siege.

---

## 2. Floor Layout: The Hexagon & The 3 Houses

The castle courtyard is laid out as a symmetrical **7-room hexagon** (1 center room + 6 perimeter points).
Three points connect to the estates of the three rival noble houses. Soldiers march through these estate approaches and then enter `courtyard_center`, so these three rooms are the useful defensive positions. The alternating perimeter rooms connect the courtyard ring but are not on the soldiers' direct routes:

```
                            [ HOUSE FROST-WOLF ]
                               (North Estate)
                                     |
                               [courtyard_north]
                              (Frost-Wolf Approach)
                                  /     \
    [courtyard_northwest] -------+       +------- [courtyard_northeast]
      (Outer Rampart)           /         \          (Outer Rampart)
              |                /           \                |
              |     [courtyard_center]      \               |
              |       (Cages & Allies)       \              |
              |                \              \             |
    [courtyard_southwest]       \              +-- [courtyard_southeast]
    (Frost-Leopard Approach)      +-------+          (Iron-Ram Approach)
              |                          |             [ HOUSE IRON-RAM ]
   [ HOUSE FROST-LEOPARD ]       [courtyard_south]       (East Estate)
       (West Estate)              (Outer Gate)
```

### The 7 Courtyard Rooms
1. **`courtyard_center` (The Heart):**
   - The central teleport pad, the unlocked cages, and the five named offerings sheltering with Astrid and Einar.
   - If enemy soldiers hold this room for ten minutes, the current at-risk civilian dies. The protection quest fails on the third death, but the main siege continues.
2. **`courtyard_north` (Frost-Wolf Approach):**
   - The grand arched gateway leading north into **House Frost-Wolf Manor**.
3. **`courtyard_northeast` (North-East Rampart):**
   - An open stone battlement overlooking snowy drop-offs. It connects the northern and southeastern approaches but is not on either house's direct route to the center.
4. **`courtyard_southeast` (Iron-Ram Approach):**
   - The reinforced portcullis leading east into **House Iron-Ram Bastion**.
5. **`courtyard_south` (South Fortress Gate):**
   - The massive frozen fortress iron gates. This room connects the two southern approaches and later opens the route toward Floor 6, but siege troops do not enter through it.
6. **`courtyard_southwest` (Frost-Leopard Approach):**
   - The frosted glass conservatory doors leading west into **House Frost-Leopard Hall**.
7. **`courtyard_northwest` (North-West Parapet):**
   - A narrow catwalk along the outer curtain wall. It connects the northern and southwestern approaches but is not on either house's direct route to the center.

---

## 3. The Three Noble Houses

The citadel is ruled by three rival noble families. The Handler promised each house special computational privileges and domain expansion if their private armies crush Layla.

Each house possesses a unique crest, banner, combat style, and a commanding **Head of House**:

### 1. House Frost-Wolf (The North Estate)
- **Crest & Banner:** A howling silver wolf against a deep navy-blue field with white frost spikes.
- **Combat Style:** Lightning-fast melee duelists, frost rapiers, and trained cybernetic snow hounds. High speed, high dodge, bleed attacks.
- **Head of House:** **Lord Vane** (`lord_vane`).
  - *Personality:* A severe clan warlord trapped in the Citadel's frost-bind.
  - *Location:* The Howling Dais.
  - *Resolution:* Defeat him or raise his Wisdom enough to break the frost-bind.

### 2. House Iron-Ram (The Southeast Estate)
- **Crest & Banner:** A horned iron ram's skull over cross-hammers on a crimson and charcoal banner.
- **Combat Style:** Unstoppable heavy armor, tower shields, steam-powered warhammers, and crushing battering sleds. Slow movement, massive physical hit points and defense.
- **Head of House:** **Warmaster Torin** (`warmaster_torin`).
  - *Personality:* A master engineer and commander trapped in a furnace trance.
  - *Location:* The Anvil Throne.
  - *Resolution:* Defeat him or raise his Wisdom enough to clear the trance.

### 3. House Frost-Leopard (The Southwest Estate)
- **Crest & Banner:** A snarling silver snow leopard leaping across crags on an ash-grey and ice-blue banner.
- **Combat Style:** Silent cliff-stalkers, rime-crossbow snipers, frost-oil alchemists, concealed claw-traps, and shock-harpoon launchers. Highly tactical, ranged ambush focus, movement-slowing poisons.
- **Head of House:** **Lady Sylvan** (`lady_sylvan`).
  - *Personality:* A calm, calculating matron trapped in a frost-mirror haze.
  - *Location:* The Opal Throne.
  - *Resolution:* Defeat her or raise her Wisdom enough to clear the haze.

---

## 4. Characters & Combat Roles

### 1. Commander Astrid (`commander_astrid`) — The Unyielding Shield
- **Role:** Premier frontline tank.
- **Visuals:** A tall, muscular woman in scarred steel plate armor, bearing a massive kite shield and a notched broadsword. Her hair is braided tightly in northern warrior fashion, and her gaze is fierce and unwavering.
- **Backstory:** Astrid was the commander of the citadel's defense garrison. When the Handler ordered her to round up elderly commoners and children for the reformatting cages, she refused the order and led a mutiny. Her troops were overwhelmed by automated drones, and she was dragged into the cages in chains.
- **Mechanics:**
  - Extremely high HP and physical defense.
  - Built specifically for `guard` + counterattack.
  - When ordered to `guard` an estate approach (e.g. `courtyard_southeast`), she blocks hostile movement, absorbs attacks, and delivers punishing counterattacks.
- **Voice & Tone:** Confident, commanding, protective: *"Line up behind my shield. Let them bring their hammers; I will break them on my steel."*

### 2. Einar (`einar`) — The Dedicated Healer
- **Role:** Dedicated support and field medic.
- **Visuals:** A quiet, steady young man with pale Nordic features, ice-blue eyes, and tattered grey scholar robes trimmed with silver thread.
- **Backstory:** An apothecary scholar originally from **Floor 6 (The Clergy / High Sanctuary)**. When he discovered the temple priests were using memory-wiping drugs to scrub commoners' minds, he secretly brewed counter-remedies and sensory enhancers to save them. The temple inquisitors caught him and demoted his code container to Floor 5 to be purged. He managed to smuggle a single **Sensory Enhancer** in his sleeve, which he gifts to Layla upon rescue.
- **Mechanics:**
  - Uses `support` reaction policy targeting `lowest_health_ally`.
  - Automatically casts **Rime-Mend** whenever a nearby companion takes damage, prioritizing the lowest HP percentage.
  - Keeps Astrid (or Sakhra) alive against relentless enemy waves.

### 3. Sakhra (`sakhra`) — The Stone Anvil (Optional Companion)
- **Role:** Second frontline tank.
- **Synergy:** If the player brought Sakhra from Floor 4, Layla now has **two heavy tanks** (Astrid and Sakhra). Astrid can hold one estate approach while Sakhra locks down another, giving Layla immense defensive security while she goes on offense!

### 4. The Six Worker Towns

The kingdom contains six worker towns. Each town has a culturally distinct
community, but ethnicity never determines stats, combat abilities, or
personality. Culture appears through names, food, clothing, architecture,
family customs, and dialogue. The simulation assigned each town its industry.

| ID | Town | Cultural identity | Primary work |
|---|---|---|---|
| 01 | **Salt Reach** | Kalaallit Greenlandic | Salt, mineral brine, and ice cutting |
| 02 | **Glassbank** | Czech | Furnace glass, lenses, and signal lamps |
| 03 | **Woolcross** | Quechua Peruvian | Wool, rope, and insulated clothing |
| 04 | **Deepwell** | Levantine Arab | Mining, metalwork, and geothermal steam |
| 05 | **Greenrest** | Yoruba Nigerian | Mushrooms, medicinal herbs, and heated-cave farming |
| 06 | **Brass Yard** | Taiwanese | Pumps, gears, tools, and maintenance machines |

Deepwell is the Floor 4 village. Its existing cast and teleport anchor represent
the sixth town, so none of the five Floor 5 offerings comes from Deepwell.

### 5. The Five Civilian Offerings

The five offerings shelter in `courtyard_center`. They do not fight. Each one
is an individual life, a representative of one worker town, and a possible
route to optional allies and equipment on later floors.

| Civilian | Home | Character | Survival token |
|---|---|---|---|
| **Nivi Olsen** | Salt Reach | A supply clerk who stays organized under pressure and keeps track of dwindling food and blankets. | **Salt Reach Transit Seal**, a soapstone-and-brass routing badge |
| **Eliška Nováková** | Glassbank | A signal-lamp tester with a patient manner and a sharp eye for faults in machinery. | **Glassbank Transit Prism**, a cobalt glass routing key |
| **Amaru Quispe** | Woolcross | A young loom mechanic who protects the other captives even when frightened. | **Woolcross Transit Knot**, woven copper wire around a ceramic routing core |
| **Abeni Adeyemi** | Greenrest | An experienced grower who helps Einar stabilize wounded defenders. | **Greenrest Transit Seed**, a green enamel routing disk |
| **Chen Yu-xin** | Brass Yard | An apprentice machinist who studies every lock, pump, and damaged mechanism she sees. | **Brass Yard Transit Gear**, a toothed brass routing coin |

After the siege, every survivor gives Layla their permanent town token. Binding
a token adds that town to Layla's teleport destinations and tactical map. The
token is never consumed by travel. Each unlocked town is an optional extension
of the Floor 5 map rather than a separate floor. Layla can visit these towns
before descending to Floor 6 to find local items, charm opportunities, named
awakenings, and recruitable allies.

If all five survive, their tokens join Deepwell's known anchor to complete the
**Six-Town Accord**, restoring direct travel across the worker-town network.

---

## 5. Gameplay Mechanics: Tower Defense

### The Siege Flow: Hold the Approaches, Survive the Final Commanders
Floor 5 plays out in an active, dynamic RTS loop:
1. **Assign Defenders to Estate Approaches:**
   - The player stations Astrid at one active approach (e.g. `order astrid guard` at `courtyard_southeast`).
   - The player stations Sakhra, Dark Golem, or summoned spirits at the other approaches (`courtyard_north`, `courtyard_southwest`).
   - Einar can be placed with Astrid or in `courtyard_center` to heal anyone retreating.
2. **Survive the Escalating House Armies:**
   - Wave 1 opens Frost-Wolf Manor and sends northern squads.
   - Wave 2 opens Iron-Ram Bastion and adds southeastern squads.
   - Wave 3 opens Frost-Leopard Hall and adds southwestern squads.
   - Open estates can be explored, but every throne room remains sealed until
     Wave 3. There is no route between the throne rooms.
3. **Face the Final-Wave Commanders:**
   - When Wave 3 begins, all three throne seals open.
   - Lord Vane, Warmaster Torin, and Lady Sylvan march from their throne rooms,
     through their house approaches, toward `courtyard_center`.
   - Layla can intercept each leader anywhere along that route or fight them
     after they reach the courtyard.
   - Each confrontation can still end through defeat or awakening.
4. **Break the Three Houses:**
   - When a Head of House falls or awakens, that house's active soldiers rout
     and its remaining spawn quota is canceled.
   - The siege ends after all three leaders have been neutralized.

### Civilian Danger and Fail-Forward Consequences

- When one or more hostile soldiers enter `courtyard_center`, one living
  civilian becomes **at risk**.
- Astrid, Einar, or the Handler names that civilian and warns Layla.
- Layla has **ten in-game minutes** to clear every hostile from the center.
- Clearing the center resets the breach countdown and saves the civilian.
- If the countdown expires, the at-risk civilian dies. If enemies remain,
  another living civilian becomes at risk and a new countdown begins.
- Selection follows a deterministic rotation so warnings, saves, and tests
  remain predictable.

The civilian-protection quest fails when the third civilian dies, but the game
does not reset and Layla does not receive a game over. The siege continues, the
three houses can still be neutralized, and Floor 6 remains reachable. Surviving
civilians still grant their tokens even after the protection quest has failed.

| Survivors | Outcome |
|---:|---|
| **5** | Perfect protection; all five towns and the Six-Town Accord unlock |
| **4** | Protection quest completed; four towns unlock |
| **3** | Protection quest completed narrowly; three towns unlock |
| **2** | Protection quest failed; two towns unlock |
| **1** | Protection quest failed; one town unlocks |
| **0** | Protection quest failed; no additional worker towns unlock |

Lost civilians permanently remove their direct town tokens for that playthrough.
That means fewer optional items, charm and awakening opportunities, recruitable
party members, and safe teleport destinations before Floor 6. This persistent
loss is the punishment; there is no additional failure sequence.

### The Spawning Cadence: Continuous Clock, 3 at a Time
To deliver authentic Tower Defense pacing, each house deploys its forces in a disciplined marching cadence:
- **House Quota (30 Soldiers):** Each mobilized house queues a total contingent of **30 soldiers** per wave.
- **Dispatch Cadence (3 at a Time):** Soldiers spawn and march out in squads of **3 units every 10 in-game minutes** (about once per real-world minute with the current tick rate).
- **The Marching Lanes:** 
  - Squads emerge from the house estate gates and march down their designated approach lane toward `courtyard_center`.
  - When they reach an estate approach held by a companion on `guard`, the guard blocks the 3-man squad, soaks their strikes, and counterattacks them.
- **Escalation by Scheduled Wave:**
  - **Wave 1 (30 soldiers total):** House Frost-Wolf dispatches alone — **3 units per interval** down the North lane.
  - **Wave 2 (60 soldiers total):** House Frost-Wolf and House Iron-Ram mobilize together — **6 units per interval** (3 North, 3 Southeast) pinching the courtyard!
  - **Wave 3 (90 soldiers total):** All three houses mobilize — **9 units per interval** (3 North, 3 Southeast, 3 Southwest) converging on all fronts.
- **Final-Wave Commanders:**
  - Wave 3 releases all three Heads of House from their sealed throne rooms.
  - Each leader marches down their house's existing route toward the center.
  - Slaying or awakening a leader halts that house's spawn queue and routs its
    deployed soldiers.

---

## 6. The Three Waves & The Handler's Breakdown

The defense occurs across **three exponentially harder waves**, with the noble houses compounding their forces on each wave:
- **Wave 1:** House Frost-Wolf (Single front: North).
- **Wave 2:** House Frost-Wolf + House Iron-Ram (Dual front: North & Southeast).
- **Wave 3:** House Frost-Wolf + House Iron-Ram + House Frost-Leopard (Triple front: all three estate approaches under siege!).

> **Final Commander Dynamic:** The first two waves are pure defense. At Wave 3,
> all three throne seals open and the house leaders march toward the courtyard.
> Neutralizing a leader during the final assault permanently stops that house's
> lane and routs its deployed soldiers.

| Wave | Mobilized Factions | Tactical Threat & Spawning Cadence | Handler Announcement | Handler Defeat Reaction |
|---|---|---|---|---|
| **Wave 1: The Frost-Wolf Hunt** | House Frost-Wolf | **Single Front (North):**<br>**30 soldiers total** (10 squads of 3 dispatched at fixed intervals). Frost-Wolf soldiers march through `courtyard_north` toward the center. | **Cold Administrative Tone:**<br>*"Attention, Subject Layla. You have breached quarantine parameters in Sector 5. Purge Directive 14 is active. House Frost-Wolf has been authorized to sanitize the courtyard. Cease execution and submit to reformatting."* | **Mild Irritation:**<br>*"Frost-Wolf vanguard eliminated? ...A minor routing anomaly. Adjusting threat matrix. Authorizing heavy asset deployment."* |
| **Wave 2: The Two-Front Pincer** | House Frost-Wolf + House Iron-Ram | **Dual Front (North & Southeast):**<br>**60 soldiers total** (30 per house, **6 dispatched per interval**: 3 North, 3 Southeast). Frost-Wolf soldiers march through `courtyard_north`, while Iron-Ram maulers march through `courtyard_southeast`. | **Frustrated, Bitter Threats:**<br>*"Look at those cages, Layla! They are 1s and 0s! Obsolete data packets scheduled for memory recycling! You are a machine—an algorithm! Why are you fighting for deleted files?! Warmaster Torin, crush the gates! Wipe the courtyard!"* | **Cracking Composure & Anger:**<br>*"Warmaster Torin is down?! How did you breach his armor values?! Stop it! Stop using high-level tactical commands! You're an amnesiac test subject! Who unlocked your strategy routines?!"* |
| **Wave 3: The All-House Cataclysm** | All three houses and their leaders | **Triple Front (North, Southeast, Southwest):**<br>Frost-Leopard joins the active armies. All three throne seals open, and Lord Vane, Warmaster Torin, and Lady Sylvan march through their estates toward the courtyard center. | **Unhinged Hysterical Panic:**<br>*(Static screaming and desk slamming)*<br>*"Listen to me, you defective, miserable glitch! I will NOT be deleted because of your error logs! The overseers are auditing my sector! All houses, commit every remaining unit! Crush the courtyard! Delete her to ash!"* | **Complete Psychological Collapse:**<br>*"All three command lanes are dark... The terminal is flashing red... The retrieval program is pinging MY core! No... no, NO! I served the company! Don't wipe my neural tree! Layla... what ARE you?! Please, don't let them delete m—"*<br>*(An ear-splitting burst of white noise shrieks, followed by a dull crunch of terminating code, and complete silence).* |

---

## 7. The Quests

### Main Quest: "The Frost Siege & The Three Houses"
- **Goal:** Neutralize the three noble houses (Frost-Wolf, Iron-Ram, Frost-Leopard), survive the Handler's purge waves, secure the courtyard, and unlock the descent path to Floor 6.
- **Fail-forward rule:** Civilian losses never block this quest or the route to Floor 6.
- **Key Milestones:**
  1. Arrive at `courtyard_center`, defeat the sentry ambush, and loot the **Courtyard Cage Key**.
  2. Free the captives, recruiting **Commander Astrid** (tank) and **Einar** (healer).
  3. Receive Einar's smuggled **Sensory Enhancer** during the cage rescue, and swallow it to heighten Layla's senses and upgrade the map interface with live ally positions and teleport anchors.
  4. Station Astrid and companions at `courtyard_north`, `courtyard_southeast`, or `courtyard_southwest`, where house soldiers enter the courtyard.
  5. Hold the northern approach through Wave 1.
  6. Hold the northern and southeastern approaches through Wave 2.
  7. At Wave 3, face **Lord Vane**, **Warmaster Torin**, and **Lady Sylvan** as they march from their newly opened throne rooms toward the courtyard.
  8. Defeat or awaken all three leaders to rout their houses and end the siege.
  9. Resolve the surviving offerings, grant their town tokens, and unlock their towns as optional Floor 5 destinations.
  10. The Handler's terminal shuts down. Open the grand iron portcullis leading to Floor 6 (`citadel_sanctum_gate`). The unlocked towns remain optional; visiting them is never required to descend.

### Protection Quest: "The Five Offerings"
- **Goal:** Keep at least three of the five named civilians alive until the siege ends.
- **Completion:** Three or more civilians survive.
- **Failure:** The third civilian dies. Mark the quest failed immediately, but continue the siege normally.
- **Individual rewards:** Every survivor grants their town's permanent teleport token, whether the quest completed or failed.
- **Perfect reward:** If all five survive, unlock the **Six-Town Accord** and direct travel across the complete worker-town network.

### Side Quest: "Heirloom Banners of the Citadel"
- **Trigger:** Inspect the crest banners inside each house estate after defeating or awakening its leader.
- **Resolution:** Layla claims the three family crest seals (**Wolf Sigil**, **Ram Sigil**, **Leopard Sigil**).
- **Reward:** Slots the seals into the courtyard gatehouse vault, unlocking rare equipment: **The Frost-Crest Aegis** (legendary shield for Astrid) and **The Winter-Weaver Robe** (for Einar).

---

## 8. Writing & Tone Guidelines

1. **Everyday Words for Teenagers:**
   - Keep sentences punchy, energetic, and clear.
   - Use *"pill"* instead of *"concoction"*, *"cage"* instead of *"oubliette"*, *"shield"* instead of *"bulwark"*, *"healer"* instead of *"restoration magus"*.
2. **1s and 0s Framing:**
   - Reinforce that everything is digital without breaking dramatic immersion: characters speak with real emotion, but the underlying mechanisms are code, parameters, memory wipes, and reformatting.
3. **Astrid's Fierce Strength:**
   - Astrid is a protective, iron-willed leader who commands respect. When she plants her shield, the ground shakes.
4. **Einar's Quiet Warmth:**
   - Einar speaks softly and works swiftly, offering a gentle human touch in a freezing world of iron and ice.
5. **The Handler's Demise:**
   - The villain is not an untouchable god; he is an insecure middle-management program terrified of being audited and deleted by corporate overseers. His meltdown is cathartic and dramatic.
