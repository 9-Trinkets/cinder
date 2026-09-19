# Floor 4 — The Commoners (The Mine, The Village, and The Military Complex)

Living design plan for **Level 4** of Layla. This floor covers the Noor-stone mines, the steampunk worker village, and the fortified military complex guarding the teleportation gate.

All text for players must use **simple sentences and everyday words for teenagers**. Avoid unusual, archaic, or fancy words (use "offering" instead of "tithe", "gate" instead of "portal", "platform" instead of "dais", "furnace" instead of "crucible").

---

## 1. Story and Setting

After the fire rooms on Floor 3, Layla steps out of the ancient border traps and into a noisy, steam-filled underground civilization.

This is not an abandoned ruin. Thousands of commoners live and work here under the watchful eye of an armored garrison.

- **The Setting — Steampunk Underworld:** 
  The caverns are crisscrossed by thick copper and iron pipes, hissing steam valves, rattling ore carts, and brass pressure gauges. High-pressure steam engines power the heavy mining equipment and heat the underground town.
- **The Mineral — Noor-Stone:**
  The miners excavate **Noor-stone**, a soft, glowing mineral found in deep geothermal veins.
  - *The Magic Chalk Connection:* When pulverized, purified, and pressed, Noor-stone is the exact raw material used to craft the magic chalk Layla holds in her hand.
  - *The Industrial Power:* In its raw rock form, Noor-stone vibrates with heat and arcane resonance, superheating boilers to power the steam machinery on Floor 4 and the spellcraft on the floors above.
- **The Problem — The Barred Gate:**
  There are no stairs leading up. The only way forward is a massive **teleportation gate** powered by steam boilers and Noor-stone conduits. It sits inside a fortified military complex in the center of the floor. Commoners are forbidden from entering—unless the guards select them as a living **offering** for the temple priests above.

---

## 2. Floor Layout: 3 Concentric Triangles

The entire floor is structured in three concentric triangles, with security and control tightening the closer you get to the center:

```
                  /\
                 /  \
                /    \            OUTER TRIANGLE: THE NOOR-STONE MINES
               /  /\  \           (Steam drills, open pits, ore carts, heavy dust)
              /  /  \  \
             /  / /\ \  \         MIDDLE TRIANGLE: THE VILLAGE
            /  / /  \ \  \        (Brass pipe homes, bakeries, steam-mushroom beds)
           /  / / /\ \ \  \
          /  / / /  \ \ \  \      INNER TRIANGLE: MILITARY COMPLEX
         /  / / /GATE\ \ \  \     (Bastions, prison cage, steam teleport gate)
        /  / / /______\ \ \  \
       /  / /____________\ \  \
      /  /__________________\  \
     /__________________________\
```

### Outer Triangle: The Noor-Stone Mines (Heavy Industry)
The outermost ring is a massive, echoing network of open quarries and mine shafts wrapping around the entire floor.
- **Look & Feel:** Fine white stone dust coats every surface. Steam drills pound against rock walls (*thump... thump... hiss*). Red boiler lights glow through dense steam clouds. Bright, pale veins of raw Noor-stone glow softly inside deep crevices.
- **Key Locations:**
  - `mine_apex_pit`: The northern tip. An immense open quarry where steam-powered mechanical picks break off raw Noor-stone boulders.
  - `steam_sorting_plant`: The southwest corner. Chutes, shaking screens, and steam conveyors sort Noor-stone chunks by grade.
  - `cart_rail_terminus`: The southeast corner. Iron tracks where steam rail engines pull heavy ore wagons.
  - `drainage_flue`: An old, rusted steam-drain conduit running beneath the mine floor directly into the middle village ring.

### Middle Triangle: The Worker Village (Living Quarters)
The middle ring is where the miners, artisans, and their families live. Homes are built from rough stone blocks retrofitted with exposed brass steam heating pipes and glowing Noor-stone lanterns.
- **Look & Feel:** Warm yellow lantern light cuts through the mist. The smell of fresh flatbread, mushroom broth, and machine oil. Children running between laundry lines; tired workers resting on wooden benches.
- **Key Locations:**
  - `boiler_square`: The central town plaza built around a huge brass geothermal manifold that distributes steam across town.
  - `communal_hall`: A large tavern where miners eat, argue, and share rumors about the guards.
  - `farida_bakery`: The busiest bakery in town, filled with the aroma of sesame flatbread and spiced tea.
  - `elder_residence`: The home of Elder Tariq, the village leader who signed the order surrendering Zayd.
  - `mushroom_steambeds`: Tiered planting beds heated by low-pressure steam pipes to grow food in the dark.

### Inner Triangle: The Military Complex (The Fortress)
The fortified center of the floor, enclosed behind high iron fences, steam-powered security gates, and searchlight towers. Commoners caught here are locked up or shot on sight.
- **Look & Feel:** Polished iron plating, barbed wire, clean steam vents, and the hum of high-voltage mana dynamos. Armed soldiers patrol the ramparts carrying steam crossbows.
- **Key Locations:**
  - `fortress_gate`: The main checkpoint with hydraulic gates, guard dogs, and sentry barricades.
  - `command_bastion`: Commander Malik's headquarters. Inside is a brass combination safe holding the **Teleportation Scroll**.
  - `steam_prison_cage`: A reinforced iron cage suspended by chains over a geothermal vent. This is where young Zayd is held.
  - `teleport_platform`: A massive circular brass platform ringed with steam valves and Noor-stone conduits that leads to Floor 5.

---

## 3. The Quests

### Main Quest: "The Teleportation Scroll"
- **Goal:** Infiltrate the Inner Military Complex, break into Commander Malik's safe, take the **Teleportation Scroll**, and learn the **Teleportation Sigil**.
- **Why You Need It:** The fortress gates are locked by hydraulic deadbolts that cannot be pried open. The Teleportation Sigil lets Layla blink through solid iron bars, slip past sentries, and activate the teleport platform.
- **How to Complete It:**
  1. Gather intel in the village from Farida and the miners about the fortress layout and Malik's habits.
  2. Use the `drainage_flue` from the mine or create a diversion with the steam pressure valves in `boiler_square`.
  3. Crack the commander's safe, take the scroll, and read it.
- **The Sigil:**
  - Layla draws a double triangle with intersecting lines in magic chalk.
  - Tracing it while standing before a barred gate or obstacle blinks the party instantly to the other side.

---

### Side Quest: "Save the Boy Zayd"
- **The Orphan (Zayd, age 11):**
  - Zayd lost both parents in a steam drill collapse years ago. With no family left, the entire village took turns looking after him: Farida gave him bread, the miners taught him the machinery, and the elders looked the other way when he played pranks.
  - He was wild, loud, and constantly climbed on steam pipes and threw pebbles at the soldiers. But he had a loyal heart, always carrying heavy coal buckets for the elderly and tending to injured stray animals.
- **The Tragedy:**
  - Temple priests sent an order demanding one youth as an **offering** for the rituals above.
  - Paralyzed with fear for their own children, the village council made a cowardly decision: they gave up Zayd, rationalizing that *"he has no mother to cry for him."*
  - The moment Malik's soldiers dragged Zayd away in brass shackles, heavy guilt crushed the village. Nobody can look at one another without shame.
- **Rescuing Zayd:**
  - Zayd is locked in the suspended `steam_prison_cage`, awaiting the priest's transport wagon.
  - Once Layla masters the **Teleportation Sigil**, she can blink inside the cage, grab Zayd, and blink both of them out before guards can sound the alarm.
  - Alternatively, Layla can sabotage a nearby steam valve to blind the guards with a cloud of hot vapor and pick the cage lock.
- **The Reward & The Family Heirloom:**
  - Zayd is escorted back to the village and hidden safely in the steam mushroom caves.
  - The villagers are overjoyed and deeply indebted to Layla, providing healing salves, chalk refills, and local maps.
  - Zayd holds out his most cherished possession—a small, brass-bound miner's lantern fueled by a glowing piece of raw Noor-stone, the only thing his parents left behind before the cave-in:
    > *"My mom and dad gave me this before the rocks fell. They said light always finds a way through stone. Take it... wherever you're going in the deep dark, I hope their light guides your way."*
  - Layla receives **Zayd's Lantern** (+5 Wisdom accessory).

---

## 4. Characters (Arabian Names)

| Name | Role | Location | Personality & Voice |
|---|---|---|---|
| **Elder Tariq** | Village Elder | Elder's Residence | Exhausted, burdened by grief and guilt. Speaks in quiet, regretful sentences. Desperate to undo his betrayal of Zayd. |
| **Farida** | Town Baker | Farida's Bakery | Passionate, outspoken, and furious at the elders. She baked for Zayd every morning and demands someone save him. |
| **Zayd** | Orphan Boy (Age 11) | Steam Prison Cage | Scrappy, soot-stained, and defiant. Even trapped in an iron cage, he glares at the guards and clutches the memory of his parents. |
| **Commander Malik** | Garrison Captain | Command Bastion | Cold, proud, and precise. Wears brass officer armor and views the miners as replaceable labor. |
| **Inquisitor Bashir** | Temple Emissary | Command Bastion | Eerie, soft-spoken priest from the upper floors. Talks smoothly about "sacred duty" while waiting to take Zayd away. |
| **Sakhra** | Heavy Steam Golem | The Noor-Stone Mines | A massive basalt golem retrofitted with brass steam pistons. Sluggish until awakened with Wisdom, becoming an expert tunnel builder. |

---

## 5. Party Wisdom & Awakening

- Follower golems and charmed creatures start with low Wisdom (3 to 5).
- Equipping **Zayd's Lantern** shines its warm, steady Noor-stone light into a follower's mind, raising their Wisdom to 10+.
- Reaching Wisdom 10 triggers an **Awakening**:
  - The follower breaks free from mindless stone slumber and speaks in clear prose.
  - They provide hints about steam pipe puzzles, hidden bypasses, and enemy attack patterns.
  - They become immune to the fear and charm spells used by temple inquisitors.

---

## 6. Party Orders in the Steam Caverns

With multi-member parties, players can assign specialized tactical roles:
- **`guard` / `sentry`:** A follower anchors at a doorway or steam valve, preventing roaming soldier patrols from flanking the party.
- **`scout`:** A nimble follower slips ahead through steam clouds or narrow pipe tunnels to reveal room hazards and enemy numbers without triggering combat.
