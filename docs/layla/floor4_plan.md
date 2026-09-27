# Floor 4 — The Commoners (The Mine, The Village, and The Military Complex)

Living design plan for **Level 4** of Layla. This floor covers the geothermal mines, the steampunk worker village, and the fortified military complex guarding the teleportation gate. It is the first floor with **quests** and the floor where the game's theme stops being abstract: *control vs. freedom* becomes a boy in a cage and two parents turned into dungeon pieces.

All text for players must use **simple sentences and everyday words for teenagers**. Avoid unusual, archaic, or fancy words (use "offering" instead of "tithe", "gate" instead of "portal", "platform" instead of "dais", "furnace" instead of "crucible").

> Status: **plan rewritten to canonical truth.** The village ring is built in content (rooms `village_*`, actors `elder_rashid`, `yasmin`, `tariq`, `zayd`, `priest_harun`, `captain_malik`, garrison, `sakhra`). The mines and military complex are the open implementation work. This revision settles the canonical backstory of the "offering" (the conversion), re-casts **Sakhra as Zayd's father**, and keys the **Wisdom (WIS) stat** that the pack now implements for real.

---

## 1. Story and Setting

After the fire rooms on Floor 3, Layla steps out of the ancient border traps and into a noisy, steam-filled underground civilization.

This is not an abandoned ruin. Thousands of commoners live and work here under the watchful eye of an armored garrison.

- **The Setting — Steampunk Underworld:**
  The caverns are crisscrossed by thick copper and iron pipes, hissing steam valves, rattling ore carts, and brass pressure gauges. High-pressure steam engines power the heavy mining equipment and heat the underground town.
- **The Industry — Geothermal Steam & Mining:**
  The miners drill into rich volcanic veins and deep geothermal shafts. High-pressure steam and boiling underground springs superheat heavy boilers, driving the pistons, dynamos, and industrial machinery across the floor.
- **The Problem — The Barred Gate:**
  There are no stairs leading up. The only way forward is a massive **teleportation gate** powered by high-pressure steam boilers and pneumatic conduits. It sits inside a fortified military complex in the center of the floor. Commoners are forbidden from entering — unless the guards select them as a living **offering** for the temple priests above.

### The Lie the Floor Tells

The village runs on a cover story: the chosen ones are taken to the temple as an **offering** for the rituals upstairs. The truth is worse and plainer — the offering is the **reprogramming**. Up on the temple floors, a device (later floors will build it in its final, ritual form) **wipes a person's memory and reshapes their body into a dungeon piece** that protects the machine. That is where the shaman's golems came from, where the elves of Floor 2 came from, and where the village's "disappeared" go — including Zayd's parents. Zayd's father, told he died in a "steam drill collapse," was turned into the guardian golem that has knelt at the village edge ever since. Zayd's mother was reshaped into a chess queen who now wanders the ranks on Floor 2, searching for a son she cannot name.

Layla does not realize — yet — that she is moving through *her own* process. She is a being being reprogrammed toward the dungeon-master role, meeting the finished product of that process everywhere she looks.

---

## 2. Floor Layout: 2 Concentric Triangles (With Surrounding Mines in Lore)

The floor is structured in two concentric triangles, with the massive geothermal mines forming the surrounding industrial world outside the perimeter:

```
              /\
             /  \
            / /\ \           OUTER RING: THE WORKER VILLAGE (9 Rooms)
           / /  \ \          (Brass pipe homes, bakeries, workshops, town square)
          / / /\ \ \         *Surrounded by the deep steam mines in lore*
         / / /  \ \ \
        / / /GATE\ \ \       INNER TRIANGLE: MILITARY COMPLEX (7 Rooms)
       / / /______\ \ \      (Bastions, prison cage, steam teleport platform)
      / /____________\ \
     /__________________\
```

### The Surrounding Mines (World Lore & Sensory Backdrop)
The deep steam mines surround the entire floor in the narrative fiction, rather than being empty playable map space.
- **Sensory Presence:** Fine stone dust and soot coat roofs and garments. Heavy steam drills pound against distant rock walls (*thump... thump... hiss*). Red boiler glow reflects off drifting steam, and iron rails carry loaded ore carts directly to the village perimeter gates.
- **The Conversion Connection:** The villagers know the mines as their livelihood, but the deep shafts conceal where the "offerings" are escorted before being shipped upward.

### Outer Ring: The Worker Village (Living Quarters — 9 Rooms Built)
The outer playable ring is where the miners, artisans, and their families live. Homes are built from rough stone blocks retrofitted with exposed brass steam heating pipes and warm oil lanterns.
- **Look & Feel:** Warm yellow lantern light cuts through the mist. The smell of fresh flatbread, mushroom broth, and machine oil. Children running between laundry lines; tired workers resting on wooden benches.
- **Key Locations:**
  - `village_square`: The central town plaza (the stage for gossip, Tariq's observations, and Yasmin's bakery).
  - `village_south_1`: Elder Rashid's home — the elder who signed the order surrendering Zayd.
  - `village_east_2`: **Yasmin's bakery**, the busiest in town (sesame flatbread, spiced tea).
  - `village_west_2`: **Tariq's workshop** — the clockmaker and tinkerer whose knowledge guides the steam diversion.
  - `village_west_1`: **Wash Basin Terrace** — the site of the overpressure valve used to divert steam and pop the fortress gate locks.
  - `village_north_gate`: The heavy northern checkpoint leading into the inner fortress.

### Inner Triangle: The Military Complex (The Fortress — 7 Rooms Built)
The fortified center of the floor, enclosed behind high iron fences, steam-powered security gates, and searchlight towers. Commoners caught here are locked up or shot on sight.
- **Look & Feel:** Polished iron plating, barbed wire, clean steam vents, and the hum of high-voltage mana dynamos. Armed soldiers patrol the ramparts carrying steam crossbows.
- **Key Locations:**
  - `fortress_gate`: The main checkpoint with hydraulic gates and sentry barricades.
  - `command_bastion`: Commander Malik's headquarters and Priest Harun's station, holding the safe and the Teleportation Scroll.
  - `steam_prison_cage`: A reinforced iron cage suspended by chains over a geothermal vent where young Zayd was held.
  - `teleport_platform`: A massive circular brass platform ringed with steam valves and arcane bronze conduits that leads to Floor 5. Accessible exclusively through Commander's Bastion once Commander Malik is defeated.

---

## 3. The Characters

| Name | Actor id | Role | Location | Personality & Voice |
|---|---|---|---|---|
| **Elder Rashid** | `elder_rashid` | Village Elder | `village_south_1` | Exhausted, burdened by grief and guilt. Quiet, regretful sentences. Desperate to undo his betrayal of Zayd. |
| **Yasmin** | `yasmin` | Town Baker | `village_east_2` | Passionate, outspoken, furious at the elders. Baked for Zayd every morning; demands someone save him. |
| **Tariq** | `tariq` | Clockmaker & Tinkerer | `village_west_2` | Clever, watchful, the village's sceptic. The wires of the collapse lie are easiest to trip around him. Doubts the "steam drill collapse" story — the doorway to the truth. |
| **Zayd** | `zayd` | Boy (age 11) | `steam_prison_cage` (to be placed) | Scrappy, soot-stained, defiant. Even caged, he glares at the guards and clutches his parents' lantern. Still talks to the guardian golem. |
| **Commander Malik** | `captain_malik` | Garrison Captain | `command_bastion` | Cold, proud, precise. Brass officer armor. Views miners as replaceable labor; follows the priests' orders without questions. |
| **Priest Harun** | `priest_harun` | Temple Emissary | `command_bastion` | Eerie, soft-spoken, from the upper floors. Smooth talk about "sacred duty" while he waits to take Zayd away. On defeat he narrates the truth of the offering (hook `priest_harun.defeat`). |
| **Sakhra** | `sakhra` | Village Guardian Golem | Village edge → the party | An ordinary guardian golem the village retrofitted with steam pistons long ago — **secretly Zayd's father.** He has guarded this village, and his son, since before Zayd could walk. He does not remember that. |
| **The Queen** | `elf-queen-4` | Wandering Convert (Floor 2) | Floor 2 ranks | **Secretly Zayd's mother.** The piece that remembers a son it cannot name; the reason the elves stand close to waking. |
| **The Handler** | — | External System | Voice | His friction with Layla on this floor is where he turns hostile: he will order her to "let the tithe ship." Saving Zayd is her first open act against the machine. |

> **Name reconciliation:** the earlier plan named the elder "Elder Tariq" and the baker "Farida"; content now uses `elder_rashid`, `yasmin` (baker), and `tariq` (clockmaker). This plan is authoritative under the content ids.

---

## 4. The Quests

### Main Quest: "The Teleportation Scroll"
- **Goal:** Infiltrate the Inner Military Complex, break into Commander Malik's safe, take the **Teleportation Scroll**, and learn the **Teleportation Sigil**.
- **Why You Need It:** The fortress gates are locked by hydraulic deadbolts that cannot be pried open. The Teleportation Sigil lets Layla blink through solid iron bars, slip past sentries, and activate the teleport platform.
- **How to Complete It:**
  1. Gather intel in the village from Yasmin, the miners, and Tariq (and, in one quiet thread, Sakhra) about the fortress layout and Malik's habits.
  2. Create a diversion by releasing the steam overpressure valve in the wash basin terrace (`village_west_1`) to pop the northern gate hydraulic locks.
  3. Crack the commander's safe, take the scroll, and read it (`item.teleport_scroll_read` → `knows_teleport`).
- **The Sigil:** a double triangle with intersecting lines in magic chalk. Tracing it while standing before a barred gate or obstacle blinks the party instantly to the other side.

**Beat stages:** `mq_hear_gate` → `mq_find_scroll` → `mq_use_teleport_platform` (already authored in `beats.json`).

### Side Quest: "Save the Boy Zayd"
- **The Boy (Zayd, age 11):**
  - The village believes Zayd lost both parents in a steam drill collapse years ago. With no family left, the village took turns looking after him: Yasmin gave him bread, miners taught him the machinery, Tariq let him fiddle with gears, and the elders looked the other way when he played pranks. Only Sakhra, the guardian golem, was *always* beside him.
  - He was wild, loud, and constantly climbed on steam pipes and threw pebbles at the soldiers. But he had a loyal heart, always carrying heavy coal buckets for the elderly and tending to injured stray animals. He still brings his lamp to the old golem and talks to it about his parents.
- **The Tragedy:**
  - The temple priests sent an order demanding one youth as an **offering** for the rituals above.
  - Paralyzed with fear for their own children, the village council made a cowardly decision: they gave up Zayd, rationalizing that *"he has no mother to cry for him."*
  - The moment Malik's soldiers dragged Zayd away in brass shackles, heavy guilt crushed the village. Nobody can look at one another without shame.
- **Rescuing Zayd:**
  - Zayd is locked in the suspended `steam_prison_cage`, awaiting the priest's transport wagon (`priest_harun`).
  - Once Layla masters the **Teleportation Sigil**, she can blink inside the cage, grab Zayd, and blink both of them out before guards can sound the alarm.
  - Alternatively, Layla can sabotage a nearby steam valve to blind the guards with a cloud of hot vapor and pick the cage lock.
  - **The handler turns.** The handler's route sheet does not cover a missing offering. He will order Layla to leave the boy: *"Let the tithe ship, Layla. That's the job."* Saving Zayd (with or against him) is Layla's first deliberate act of rebellion.
- **The Reward & The Family Heirloom:**
  - Zayd is escorted back to the village and hidden safely in the steam mushroom caves (beat stages: `sq_hear_sacrifice` → `sq_save_zayd` → `sq_return_zayd`).
  - The villagers are overjoyed and deeply indebted, providing healing salves, chalk refills, and local maps.
  - Zayd hands Layla his most cherished possession — the brass-bound miner's lantern with a warm amber filament that never goes cold, and the only clue to the secret his parents share:
    > *"My mom and dad gave me this before the rocks fell. They said light always finds a way through stone. Take it... wherever you're going in the deep dark, I hope their light guides your way."*
  - Layla receives **Zayd's Lantern** — the key item of the floor's secret.

### Secret Quest: "Return the Light" (Freeing Zayd's Parents)
The scavenged truth: with Zayd safe, the pieces fall into place — the old golem at the village edge, and the wandering queen on the lower-forest ranks. The clues:

1. **The golem (Sakhra).** Tariq finds the plaster-and-pipe joints on the guardian golem odd — that isn't how a guardian is built, that's a *person,* bricked in stone and holding steam pipes over old scars. Zayd, who played on Sakhra's knees, is the only one who never needed the clue: *"He's been here my whole life. He feels like mine."*
2. **The queen.** On Floor 2, the queen's inspect_text reads like a worried mother's patrol — she is always moving, always looking, retracing the same file. A player with `zayd` freed can find her circling, and her description shifts to a person searching for someone whose face she can't hold onto.
3. **The lantern.** Zayd's Lantern (+WIS) is the one light that reaches a piece. Its description now carries the truth of its maker: it was his parents' lamp — of course it was the first thing the conversion works could not quite put out.

**The awakening — one light, two ways to wake:**

- **The father wakes by light.** Bring the lantern to Sakhra. The lantern's warm glow is the light his son kept carrying to him — the only warmth a guardian's fog keeps. At WIS 10, the stone eyes clear and a voice comes out of the golem for the first time: his pipe hiss gives way to speech, and he says Zayd's name. He has been beside his son the whole floor, and he remembers.
- **The mother wakes by voice.** The queen is already at the edge of waking (her wisdom sits highest of any piece on the board — she is a mother who never fully stopped). The extra point of light is not a tool but a voice: Zayd's, calling her name across the ranks. The son wakes what light cannot reach. *(One lantern only, and no second lamp in this design — the family wakes by different lights, on purpose.)*

- **The payoff.** The awakened parents stay with Layla as free allies — two humans who remember the flavour of bread and the weight of a child — the proof the whole game has been circling: the program can be *undone*, and she is not the only one being unmade.

---

## 5. Party Wisdom & Awakening

Wisdom (WIS) is now a **real stat** in `content/layla/stats.json` (default 3, range 0–10, seeded to every actor, surfaced generically in the vitals/sidebar and inspect UI).

- **What wisdom means:** how much of their *pre-machine* self a converted piece still holds. Golems converted and bricked in stone carry the least (3–5). Elves converted into ranked pieces carry more (7–9) — the army's refusal to be bound and its final choice to stand down are *this* stat leaking through. The queen is the closest to waking of all.
- **Raising wisdom:** Equipping **Zayd's Lantern** (+6 WIS) shines its warm, steady light into a follower's mind. Raisable to the 10-line.
- **Reaching Wisdom 10 triggers an Awakening:**
  - The follower breaks free from its mind-wiped role and remembers who it was.
  - Its generic name changes to its real name; full speech unlocks.
  - It stops being a piece and teaches the party: floor secrets, pipe-puzzle shortcuts, enemy patrol habits — and, for the parents, *the truth about the offering.*
- **Hard limit:** the **goblin shaman** and the **elf king** cannot be awakened — their conversion took deepest, or they were never pieces at all. They can only be defeated (established in Floor 1/2 canon; the king's real name is a story the parents may one day tell on Floor 6).

### Who They Were (awakening storylines for surviving pieces)

Only pieces that were **not destroyed** can wake, and only if they receive enough wisdom (10+). Simple one-line storylines, to be authored into `actors.json` `prompt_context` and awakening narration as the mechanic grows:

| Piece (actor id) | Woken name | Who they were | Fragment on waking |
|---|---|---|---|
| `golem-dark-nw` | Orin | Stonecutter who helped build the first guard rooms | *"I remember the rings. I was ringing the stone before they rang me."* |
| `golem-pale-ne` | Mari | Millwright who kept the ore mill turning | *"The mill... is it still standing? I left a plate loose on the brace."* |
| `golem-dark-sw` | Ferid | Miner who worked the deepest shaft, swallowed first | *"They came down here and put the dark in my chest. It's quieter now."* |
| `golem-pale-se` | Hana | Weaver of the village's first cloth | *"I was counting threads. They Counted me instead."* |
| `sakhra` | (Zayd's father) — Jamil | Miner, father of Zayd, the village's quietest man | *"Zayd. My boy. Run your lamp over me — I will follow it the whole way home."* |
| `elf-pawn-1` | Ays | Stoker's girl who ran hot coal buckets | *"I was small and fast. They made me fast where it hurt."* |
| `elf-pawn-2` | Barak | Apprentice machinist, never off the steam drill | *"Hiss and ping and clank — I know that drum. That's a boiler, not a heart."* |
| `elf-pawn-3` | Dunya | Seed keeper of the mushroom beds | *"I used to know which dark grows grass. This dark grows orders."* |
| `elf-pawn-4` | Qamar | Night watch, the one who saw them take the first wagon | *"I saw it. I saw the loading — and they put a spear in my hand for seeing."* |
| `elf-pawn-5` | Sari | Cook's child who carried flatbread to the works | *"Warm bread. That's the sharpest memory I've got left, and it's worth more than this."* |
| `elf-pawn-6` | Nadim | Gear-oiler, knew every joint by its squeal | *"They oiled nothing. Of course I remember — a dry joint shrieks."* |
| `elf-pawn-7` | Rima | Basket weaver who made the village's laundry lines | *"I can still weave. Give me reeds, I'll grow you a way out of this."* |
| `elf-pawn-8` | Yusuf | Sand-hauler at the sorting plant | *"Sand. All that sand, and I never once found the bottom of it."* |
| `elf-rook-1` | Farhan | Quarry boss, the one who first refused the priests | *"I refused. That's when they built the walls around me. Walls can be walked around."* |
| `elf-rook-8` | Salma | Foundry forewoman who held the furnace line | *"I held the line. They turned my line into a wall. A wall still stands up — good luck punching it."* |
| `elf-knight-2` | Aziz | Messenger rider who outran the summons once too often | *"I can still run the gaps. Pacen your steps and the board forgets you."* |
| `elf-knight-7` | Layali | Acrobat of the travelling lamps | *"They untrained my tumbles into trots. Give me a crossbar and I'll show you old me."* |
| `elf-bishop-3` | Istawa | Temple caller who saw the truth behind the Rite and was silenced | *"I called. They rewired my calling into a weapon. I can call again."* |
| `elf-bishop-6` | Ruwan | Healer's hand who set the sick by lamp and bell | *"Bell and root and warm hand — no, child, that's not a crook, that's a staff for walking."* |
| `elf-queen-4` | (Zayd's mother) — Layla's mirror | Miner's wife, mother of Zayd, the village's sharpest eye | *"I keep looking for the face I lost. A boy. He called me something. Will you say it so I can keep it?"* |

> **On the queen and the easter egg:** the queen is a boss — she **may be destroyed by chance** (any fight can end a piece). That is canon, not a soft-lock: if she falls, she does not wake, and the family's story narrows to the father only, her *"storylines"* above simply never fire. The game should never hold the ending hostage to a single piece; the parent-thread resolves as long as *either* parent wakes. The queen is a *chance* easter egg, not a requirement.

---

## 6. Party Orders in the Steam Caverns

With multi-member parties, players can assign specialized tactical roles:

- **`guard` / `sentry`:** A follower anchors at a doorway or steam valve, preventing roaming soldier patrols from flanking the party.
- **`scout`:** A nimble follower slips ahead through steam clouds or narrow pipe tunnels to reveal room hazards and enemy numbers without triggering combat.

---

## 7. Implementation Status & Next Steps

**Built in content:**
- **Middle Triangle (Worker Village):** 9-room perimeter loop (`village_*`) with the three named villagers (`elder_rashid`, `yasmin`, `tariq`), two sentries at `village_north_gate`, and `sakhra`.
- **Inner Triangle (Military Complex):** 7-room concentric fortress loop (`fortress_gate`, `west_iron_walkway`, `steam_prison_cage`, `south_steam_gantry`, `command_bastion`, `east_sentry_walk`, and central `teleport_platform`).
- **Actors Placed:** `zayd` and `garrison_warden` in `steam_prison_cage`; `captain_malik` and `priest_harun` in `command_bastion`.
- **Fortress Entry Diversion (Clockmaker Route):**
  - Massive brass bulkhead exit at `village_north_gate` is gated behind story var `fortress_gate_open`.
  - Added overpressure valve feature (`village_west_1-valve`) to `village_west_1` (Wash Basin Terrace).
  - Authored player command `turn_steam_valve` (`TURN_VALVE`), restricted to `village_west_1`.
  - Hook `valve.diverted` triggers on valve turning, setting `fortress_gate_open = "true"` and releasing the hydraulic lock pins to unlock the northern bulkhead.
  - Updated Tariq's prompt context in `actors.json` with knowledge and dialogue clues pointing Layla to the wash terrace valve.
  - Verified via full end-to-end integration test `floor4_diversion_opens_fortress_gate`.
- **Zayd Rescue, Escort, and Quest Reward Lantern (Side Quest `save_zayd`):**
  - Cleared `zayd`'s initial inventory to prevent premature gifting via dialogue grounding. Zayd's prompt context instructs him to clutch his parents' lantern close and only grant it once safe in the village.
  - Authored player command `unlock_cage` (`UNLOCK_CAGE`, requires `iron-cage-key` at `steam_prison_cage`).
  - Hook `zayd.freed` sets `zayd_rescued = "true"`, converts Zayd to an ally with `follows_player = true`, and emits the Handler's warning (*"Let the tithe ship. That's the job"*).
  - Escorting Zayd back to `village_square` triggers safe arrival hook in `player.moved` (gated on `zayd_safe` `not_exists`): sets `zayd_safe = "true"`, converts Zayd back to static village ally (`follows_player = false`), awards `zayd-lantern` to Layla's inventory, narrates the reunion with Yasmin, and completes side quest `sq_return_zayd`.
  - Added `WorldHookEffect::AcquireItem` engine effect to cinder-core and wired `SetStoryVar` to emit beat advancement signals.
  - Verified via full end-to-end integration test `floor4_zayd_rescue_and_village_escort`.

**To build (next passes):**
1. Bastion & Command encounters: Commander Malik safe interaction/defeat and Priest Harun encounter to acquire `teleport-scroll`;
2. Central Teleport Platform activation: Tracing `teleport-sigil` on `teleport_platform` to activate the steam gate and open the exit down to Floor 5;
3. Quest line beat objectives (`beat_objectives.json`) and progression verification;
4. Awakening runtime: WIS ≥ 10 name change + speech unlock (engine/UI follow-up; stat is real today, the threshold behavior comes with the content above).