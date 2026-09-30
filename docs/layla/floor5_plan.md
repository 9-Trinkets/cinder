# Floor 5 — The Frost Citadel (The Hexagon Courtyard & The Three Houses)

Living design plan for **Level 5** of Layla. This floor covers the ice world of the mountain castle, the 7-room hexagonal courtyard, the mass suspension cages of virtual "offerings," and the strategic siege by three rival noble houses.

The gameplay genre for Floor 5 is **Tower Defense & Decapitation Strike (Real-Time Strategy)**. While Layla's allies hold the three defensive chokepoints of the courtyard to protect vulnerable civilians, Layla breaches the surrounding house estates to defeat or charm the heads of each house, breaking the enemy army's morale.

All player-facing text must use **simple sentences and everyday words for teenagers**. Avoid unusual, archaic, or overly technical jargon (use "cage" instead of "oubliette", "healer" instead of "thaumaturge", "guard" instead of "interdict", "pill" instead of "elixir").

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
2. **The Sight:** Grouped across the courtyard are heavy iron cages labeled with white stenciled code:
   - `BLOCK 01 — TOWN 01`
   - `BLOCK 04 — TOWN 04` (The mining village of Zayd, Rashid, and Yasmin!)
   - `BLOCK 07 — TOWN 07`
   - `BLOCK 12 — TOWN 12`
   Inside the cages are hundreds of huddled virtual people—the vast majority being **frail old people and frightened children** marked for reformatting.
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
Three points connect to the estates of the three rival noble houses; the alternating three points are defensive chokepoints where soldiers assault the courtyard:

```
                            [ HOUSE FROST-WOLF ]
                               (North Estate)
                                     |
                               [courtyard_north]
                                  /     \
    [courtyard_northwest] -------+       +------- [courtyard_northeast]
     (Defensive Choke 1)        /         \        (Defensive Choke 2)
              |                /           \                |
              |     [courtyard_center]      \               |
              |       (Cages & Allies)       \              |
              |                \              \             |
    [courtyard_southwest]       \              +-- [courtyard_southeast]
     (House Frost-Leopard)       +-------+                     |
              |                          |             [ HOUSE IRON-RAM ]
   [ HOUSE FROST-LEOPARD ]       [courtyard_south]       (East Estate)
       (West Estate)            (Defensive Choke 3)
```

### The 7 Courtyard Rooms
1. **`courtyard_center` (The Heart):**
   - The central teleport pad, the unlocked cages, and the safe haven where the elderly and children shelter.
   - If enemy soldiers reach this room uncontested, they begin slaughtering the civilians. If civilian casualties reach the limit, the mission fails.
2. **`courtyard_north` (Frost-Wolf Approach):**
   - The grand arched gateway leading north into **House Frost-Wolf Manor**.
3. **`courtyard_northeast` (North-East Rampart — Choke 1):**
   - An open stone battlement overlooking snowy drop-offs. Flanking soldiers try to breach the courtyard through this lane.
4. **`courtyard_southeast` (Iron-Ram Approach):**
   - The reinforced portcullis leading east into **House Iron-Ram Bastion**.
5. **`courtyard_south` (South Fortress Gate — Choke 2):**
   - The massive frozen fortress iron gates. Battering rams and hammer-wielding shock troops assault this bottleneck.
6. **`courtyard_southwest` (Frost-Leopard Approach):**
   - The frosted glass conservatory doors leading west into **House Frost-Leopard Hall**.
7. **`courtyard_northwest` (North-West Parapet — Choke 3):**
   - A narrow catwalk along the outer curtain wall. Agile skirmishers and snipers attempt to infiltrate from this angle.

---

## 3. The Three Noble Houses

The citadel is ruled by three rival noble families. The Handler promised each house special computational privileges and domain expansion if their private armies crush Layla.

Each house possesses a unique crest, banner, combat style, and a commanding **Head of House**:

### 1. House Frost-Wolf (The North Estate)
- **Crest & Banner:** A howling silver wolf against a deep navy-blue field with white frost spikes.
- **Combat Style:** Lightning-fast melee duelists, frost rapiers, and trained cybernetic snow hounds. High speed, high dodge, bleed attacks.
- **Head of House:** **Lady Lyra Frost-Wolf** (`lady_lyra`).
  - *Personality:* Cold, elegant, obsessed with fencing technique and aristocratic bloodlines.
  - *Location:* The High Solarium in Frost-Wolf Manor.
  - *Combat:* Wields twin frost rapiers. Blinks around the room with rapid thrusts.

### 2. House Iron-Ram (The Southeast Estate)
- **Crest & Banner:** A horned iron ram's skull over cross-hammers on a crimson and charcoal banner.
- **Combat Style:** Unstoppable heavy armor, tower shields, steam-powered warhammers, and crushing battering sleds. Slow movement, massive physical hit points and defense.
- **Head of House:** **Baron Torvald Iron-Ram** (`baron_torvald`).
  - *Personality:* Boisterous, brutal, armored from head to toe in blackened iron plates. Laughs at small blades.
  - *Location:* The Grand Foundry Forge in Iron-Ram Bastion.
  - *Combat:* Swings a two-handed volcanic maul that shatters stone and inflicts area shockwaves.

### 3. House Frost-Leopard (The Southwest Estate)
- **Crest & Banner:** A snarling silver snow leopard leaping across crags on an ash-grey and ice-blue banner.
- **Combat Style:** Silent cliff-stalkers, rime-crossbow snipers, frost-oil alchemists, concealed claw-traps, and shock-harpoon launchers. Highly tactical, ranged ambush focus, movement-slowing poisons.
- **Head of House:** **Margrave Varis Frost-Leopard** (`margrave_varis`).
  - *Personality:* Soft-spoken, calculating, masked behind a silver leopard helm. Views combat as cold statistical math.
  - *Location:* The Alchemical Observatory in Frost-Leopard Hall.
  - *Combat:* Fires poisoned repeater crossbow darts from high perches and drops explosive frost-oil canisters.

---

## 4. Characters & Combat Roles

### 1. Commander Astrid (`commander_astrid`) — The Unyielding Shield
- **Role:** Premier frontline tank.
- **Visuals:** A tall, muscular woman in scarred steel plate armor, bearing a massive kite shield and a notched broadsword. Her hair is braided tightly in northern warrior fashion, and her gaze is fierce and unwavering.
- **Backstory:** Astrid was the commander of the citadel's defense garrison. When the Handler ordered her to round up elderly commoners and children for the reformatting cages, she refused the order and led a mutiny. Her troops were overwhelmed by automated drones, and she was dragged into the cages in chains.
- **Mechanics:**
  - Extremely high HP and physical defense.
  - Built specifically for `guard` + counterattack.
  - When ordered to `guard` a chokepoint (e.g. `courtyard_south`), she intercepts every hostile strike aimed at allies and delivers a punishing retaliatory blow.
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
- **Synergy:** If the player brought Sakhra from Floor 4, Layla now has **two heavy tanks** (Astrid and Sakhra). Astrid can hold one chokepoint while Sakhra locks down another, giving Layla immense defensive security while she goes on offense!
- **Civilian Interaction:** Town 04 civilians in the cages recognize Sakhra's lantern, unlocking his passive **"Father's Vow"** (+3 Defense).

### 4. The Civilian Offerings
- Dozens of elderly villagers and children from Towns 01, 04, 07, and 12 sheltered in `courtyard_center`.
- They do not fight, but they offer gratitude, bandaging items, and warm dialogue. If any enemy reaches `courtyard_center`, the player receives urgent alerts to teleport back and clear the threat before civilians take damage.

---

## 5. Gameplay Mechanics: Tower Defense & Decapitation

### The Dual Flow: Defend the Center, Strike the Houses
Floor 5 plays out in an active, dynamic RTS loop:
1. **Assign Defenders to Chokepoints:**
   - The player stations Astrid at one chokepoint (e.g. `order astrid guard` at `courtyard_south`).
   - The player stations Sakhra, Dark Golem, or summoned spirits at the other chokepoints (`courtyard_northeast`, `courtyard_northwest`).
   - Einar can be placed with Astrid or in `courtyard_center` to heal anyone retreating.
2. **Layla's Decapitation Strikes:**
   - While the defenders hold the lines, Layla runs or teleports into one of the three House buildings:
     - North into Frost-Wolf Manor.
     - East into Iron-Ram Bastion.
     - West into Frost-Leopard Hall.
   - She fights through the estate rooms, reaches the House Head, and resolves the confrontation:
     - **Kill:** Slay the Head of House in combat.
     - **Charm / Convert:** Use `charm-sigil` or high WIS dialogue to convert the Head of House.
3. **The Demoralization Effect:**
   - The moment a Head of House falls or converts, **their entire faction's army demoralizes**!
   - Their troops immediately rout, drop their weapons, or switch to defending the courtyard!
   - This permanently neutralizes that house's assault lane, easing pressure on the courtyard defenders!
4. **Teleportation Micro:**
   - If an alert warns that a courtyard chokepoint is buckling while Layla is inside an estate, Layla casts `teleport courtyard_center` or `teleport <choke_anchor>`, blinks back instantly, drops a `drain-sigil` to wipe out the breach, and then teleports back to resume her attack!

### The Spawning Cadence: 30 Soldiers, 3 at a Time
To deliver authentic Tower Defense pacing, each house deploys its forces in a disciplined marching cadence:
- **House Quota (30 Soldiers):** Each mobilized house queues a total contingent of **30 soldiers** per wave.
- **Dispatch Cadence (3 at a Time):** Soldiers spawn and march out in squads of **3 units at a fixed tick interval** (e.g. every 3–4 turns).
- **The Marching Lanes:** 
  - Squads emerge from the house estate gates and march down their designated approach lane toward `courtyard_center`.
  - When they hit a chokepoint held by a companion on `guard`, the guard intercepts the 3-man squad, soaking their strikes and counterattacking them into scrap.
- **Exponential Escalation by Wave:**
  - **Wave 1 (30 soldiers total):** House Frost-Wolf dispatches alone — **3 units per interval** down the North lane.
  - **Wave 2 (60 soldiers total):** House Frost-Wolf and House Iron-Ram mobilize together — **6 units per interval** (3 North, 3 Southeast) pinching the courtyard!
  - **Wave 3 (90 soldiers total + Colossus):** All three houses mobilize — **9 units per interval** (3 North, 3 Southeast, 3 Southwest) converging on all fronts, followed by the **Rime Colossus**!
- **Strategic Impact of Decapitation Strikes:**
  - Slaying or charming a Head of House **instantly halts that house's spawn queue**!
  - Any remaining soldiers from their 30-unit quota are canceled immediately.
  - The faster Layla infiltrates and neutralizes a house leader, the fewer 3-man squads her courtyard defenders have to endure!

---

## 6. The Three Waves & The Handler's Breakdown

The defense occurs across **three exponentially harder waves**, with the noble houses compounding their forces on each wave:
- **Wave 1:** House Frost-Wolf (Single front: North).
- **Wave 2:** House Frost-Wolf + House Iron-Ram (Dual front: North & South/East).
- **Wave 3:** House Frost-Wolf + House Iron-Ram + House Frost-Leopard + The Rime Colossus (Triple front: All chokepoints under siege!).

> **Tactical Decapitation Dynamic:** If Layla ventures into an estate and eliminates or charms its Head of House, that house's forces are **permanently demoralized** and will drop out of subsequent waves! For example, defeating Lady Lyra in Wave 1 removes House Frost-Wolf from Waves 2 and 3, turning an impossible three-front assault into a manageable fight.

| Wave | Mobilized Factions | Tactical Threat & Spawning Cadence | Handler Announcement | Handler Defeat Reaction |
|---|---|---|---|---|
| **Wave 1: The Frost-Wolf Hunt** | House Frost-Wolf | **Single Front (North):**<br>**30 soldiers total** (10 squads of 3 dispatched at fixed intervals). Fast Frosthounds & Rapier Duelists probing the North and NE chokepoints. | **Cold Administrative Tone:**<br>*"Attention, Subject Layla. You have breached quarantine parameters in Sector 5. Purge Directive 14 is active. House Frost-Wolf has been authorized to sanitize the courtyard. Cease execution and submit to reformatting."* | **Mild Irritation:**<br>*"Frost-Wolf vanguard eliminated? ...A minor routing anomaly. Adjusting threat matrix. Authorizing heavy asset deployment."* |
| **Wave 2: The Two-Front Pincer** | House Frost-Wolf + House Iron-Ram | **Dual Front (North & South/East):**<br>**60 soldiers total** (30 per house, **6 dispatched per interval**: 3 North, 3 Southeast). Frost-Wolf duelists swarm North while heavy Iron-Ram Battering Sleds and Iron Maulers hammer the South and SE gates! | **Frustrated, Bitter Threats:**<br>*"Look at those cages, Layla! They are 1s and 0s! Obsolete data packets scheduled for memory recycling! You are a machine—an algorithm! Why are you fighting for deleted files?! Baron Torvald, crush the gates! Wipe the courtyard!"* | **Cracking Composure & Anger:**<br>*"Baron Torvald is down?! How did you breach his armor values?! Stop it! Stop using high-level tactical commands! You're an amnesiac test subject! Who unlocked your strategy routines?!"* |
| **Wave 3: The All-House Cataclysm** | House Frost-Wolf + House Iron-Ram + House Frost-Leopard + Rime Colossus | **Triple Front (All Chokepoints):**<br>**90 soldiers total** (30 per house, **9 dispatched per interval**: 3 North, 3 Southeast, 3 Southwest) converging on all fronts, spearheaded by an overclocked **Rime Colossus**! | **Unhinged Hysterical Panic:**<br>*(Static screaming and desk slamming)*<br>*"Listen to me, you defective, miserable glitch! I will NOT be deleted because of your error logs! The overseers are auditing my sector! Rime Colossus, override safety limiters! Overclock cores! CRUSH THE COURTYARD! DELETE HER TO ASH!"* | **Complete Psychological Collapse:**<br>*"The Colossus core is dead... The terminal is flashing red... The retrieval program is pinging MY core! No... no, NO! I served the company! Don't wipe my neural tree! Layla... what ARE you?! Please, don't let them delete m—"*<br>*(An ear-splitting burst of white noise shrieks, followed by a dull crunch of terminating code, and complete silence).* |

---

## 7. The Quests

### Main Quest: "The Frost Siege & The Three Houses"
- **Goal:** Defend the vulnerable civilians in `courtyard_center`, neutralize the three noble houses (Frost-Wolf, Iron-Ram, Frost-Leopard), survive the Handler's purge waves, and unlock the descent path to Floor 6.
- **Key Milestones:**
  1. Arrive at `courtyard_center`, defeat the sentry ambush, and loot the **Courtyard Cage Key**.
  2. Free the captives, recruiting **Commander Astrid** (tank) and **Einar** (healer).
  3. Talk to Einar to receive his smuggled **Sensory Enhancer**, and swallow it to heighten Layla's senses and upgrade the map interface with live ally positions and teleport anchors.
  4. Station Astrid and companions at the courtyard chokepoints.
  5. Infiltrate House Frost-Wolf Manor, confront **Lady Lyra**, and neutralize her forces.
  6. Infiltrate House Iron-Ram Bastion, defeat **Baron Torvald**, and neutralize the heavy battering sleds.
  7. Infiltrate House Frost-Leopard Hall, confront **Margrave Varis**, and neutralize the ranged sniper battery.
  8. Repel the final Rime Colossus wave at the courtyard.
  9. The Handler's terminal shuts down. Open the grand iron portcullis leading to Floor 6 (`citadel_sanctum_gate`).

### Side Quest / Secret: "The Lantern in the Dark"
- **Trigger:** Bring **Sakhra** to talk to the Town 04 civilians gathered in `courtyard_center`.
- **Resolution:** A small girl notices Sakhra's battered miner's lantern: *"That lantern... it's Master Sakhra's mark from the village forge!"* Sakhra's stone core blazes with memory.
- **Reward:** Sakhra gains **"Father's Vow"** (+3 Defense, boosted counterattack damage).

### Side Quest: "Heirloom Banners of the Citadel"
- **Trigger:** Inspect the crest banners inside each house estate after defeating or charming its leader.
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
