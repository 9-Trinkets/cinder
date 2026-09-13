# Floor 4 — The Commoners (The Village, The Mine, and The Guard Camp)

Living design plan for **level 4** of Layla. This floor covers the villagers' homes, the crystal mine, and the military camp that guards the teleportation gate. 

All text for players must use **simple sentences and everyday words for teenagers**. Avoid unusual, archaic, or fancy words (use "offering" instead of "tithe", "gate" instead of "portal", "platform" instead of "dais", "furnace" instead of "crucible").

---

## 1. Story and Setting

After the fire rooms on Floor 3, Layla steps out of the old border traps and into an underground kingdom. 

This is not an empty dungeon. People live here.

Floor 4 is where **The Commoners** live and work:
- **The Work:** They mine **mana crystals**. These glowing purple and blue stones power the whole underground kingdom. The nobles on Floor 5 and the priests on Floor 6 need these crystals to keep their machines running and cast their spells.
- **The Problem:** People cannot just walk from floor to floor. There are no stairs. The only way up is a **magic teleportation gate**, and it sits behind the walls of a heavily guarded military camp. The commoners are never allowed through, unless the guards pick them as an **offering** for the priests.
- **The Zones:** Floor 4 has three simple areas:
  1. **The Village:** Where the miners and their families live.
  2. **The Mana Mine:** Where they dig out the glowing crystals.
  3. **The Guard Camp:** A stone fort that holds the teleportation gate, the stolen teleport scroll, and a locked prison cage.

---

## 2. The 3 Zones

```
                     +-----------------------------------+
                     |       ZONE 3: THE GUARD CAMP      |
                     |  (Stone walls, guard towers,      |
                     |   prison cage, teleport gate)     |
                     |  [Main Quest: Get Teleport Scroll]|
                     +-----------------+-----------------+
                                       ^
                    Guarded gate /     | \ Secret drain pipe
                                       |
+--------------------------------------+-----------------------------------+
|          ZONE 1: THE VILLAGE         |        ZONE 2: THE MANA MINE      |
|  (Small stone homes, dining hall,    |  (Deep open pits, mine cart rails,|
|   mushroom gardens, quiet streets)   |   glowing crystal caves)          |
|  [Side Quest: Rescue the boy Kip]    |  [Mine hazards, sneak paths]      |
+--------------------------------------+-----------------------------------+
```

### Zone 1: The Village (Living Quarters)
A small, crowded underground town built against the rock walls. Simple stone huts sit next to mushroom gardens and a clean water spring.
- **What You Sense:**
  - *Smell:* Wood smoke, mushroom soup, damp earth, old candles.
  - *Sound:* Quiet murmurs from tired workers, wooden bowls clattering at dinner, soft crying behind closed doors.
  - *Look:* Warm yellow candlelight against dark cave walls.
  - *Main Rooms:*
    - `village_square`: The town center with a stone well and a big iron fire bowl.
    - `village_hall`: A wooden hall where miners eat together and talk about their work.
    - `mushroom_beds`: Stepped gardens where the village grows food mushrooms in the dark.
    - `elder_hut`: The home of Valen, the tired village elder who had to choose Kip.

### Zone 2: The Mana Mine
A huge, noisy cavern where workers cut glowing crystals from the stone walls.
- **What You Sense:**
  - *Smell:* Dust, stone powder, cold damp air, and sharp static electricity from broken crystals.
  - *Sound:* Pickaxes hitting rock (*clink... clink*), heavy ore carts rolling on iron rails, shouts of miners calling out rockfalls.
  - *Look:* Bright violet and blue crystals glowing in the dark rock like cold fire.
  - *Main Rooms:*
    - `mine_entrance`: Big wooden beams holding up the stone ceiling, with carts full of raw rock.
    - `crystal_pit`: A deep hole in the floor where the richest purple crystals grow.
    - `cart_tracks`: A rail track where miners push heavy wooden carts.
    - `old_drain_pipe`: An abandoned drainage tunnel under the mine that leads right under the guard camp walls.

### Zone 3: The Guard Camp
A dark stone fort built by the Floor 5 soldiers. It cuts off the rest of the floor and guards the exit.
- **What You Sense:**
  - *Smell:* Lamp oil, hot iron from the smithy, roasted meat, smoke.
  - *Sound:* Heavy boots marching on stone, soldiers laughing, the deep electrical hum of the teleport gate.
  - *Look:* High iron fences, bright guard torches, and a big stone ring glowing with white light on the back wall.
  - *Main Rooms:*
    - `camp_gate`: Heavy wooden gates with iron spikes, guarded by armored soldiers with crossbows.
    - `command_tent`: The head officer's tent. Inside is an iron chest holding the **Teleportation Scroll**.
    - `prison_cage`: An iron cage hanging over a dry pit. This is where they lock up people chosen as an offering for the priests.
    - `teleport_gate`: A massive stone circle carved with runes that glows bright white. This is the way up to Floor 5.

---

## 3. The Quests

### Main Quest: "Take the Teleport Scroll"
- **Goal:** Sneak or fight your way into the Guard Camp, take the **Teleportation Scroll**, and learn the **Teleportation Sigil**.
- **Why You Need It:** The soldiers will not open the gate for anyone from the village. You cannot climb the walls. Learning the teleportation sigil gives you the power to blink through locked bars, slip past the guards, and use the teleport gate.
- **How You Do It:**
  1. Talk to miners in the village to learn that the guard captain keeps an old magic scroll locked in his tent.
  2. Crawl through the old drain pipe in the mine to bypass the front gate and pop up inside the camp.
  3. Find the captain's chest, grab the scroll, and read it.
- **The Sigil:**
  - Layla draws a double ring with crossing lines in magic chalk.
  - Standing in front of iron bars or a locked gate, the sigil blinks you straight to the other side.

---

### Side Quest: "Save Kip"
- **The Boy (Kip, age 11):**
  - Kip's parents died in a mine accident when he was small. He had no family left, so the whole village took turns feeding him, fixing his clothes, and keeping an eye on him.
  - Because he did not have parents to keep him in line, Kip was wild and rowdy. He skipped chores, ran on top of the mine carts, and threw pebbles at the guards. But he also had a huge heart. He carried heavy water buckets for old folks and shared his bread with stray dogs. Everybody in the village loved him like their own kid.
- **The Conflict:**
  - The temple priests ordered the village to hand over one young worker as an **offering** for the Demon King.
  - The village leaders panicked. Every family wanted to protect their own children. In the end, they made a terrible, cowardly choice: they gave up Kip. They told themselves, *"He gets into trouble anyway, and he has no parents to cry for him."*
  - The moment the soldiers dragged Kip away in chains, the village fell apart with guilt. Nobody can look each other in the eye.
- **Starting the Quest:**
  - You hear the villagers arguing the moment you walk into town:
    - *A baker crying:* "We baked bread for that boy for six years! Then the guards showed up, and we just looked down at our boots. How could we do that to our own boy?"
    - *An elder shouting:* "What were we supposed to do? Give them two kids instead? He was always headed for trouble!"
    - *A young miner:* "He was our kid! Every single house raised him, and every single house let them take him!"
- **Rescuing Kip:**
  - Kip is locked in the iron cage inside the Guard Camp, waiting for the priest wagon to take him away.
  - Once you learn the **Teleportation Sigil** from the main quest, you can step up to the cage and blink Kip right out through the bars. (Or, you can roll an explosive crystal cart into the camp wall to make the guards run away, then pick the lock.)
- **The Reward:**
  - Kip gets taken back to the village and hidden safely in the mushroom caves.
  - The village unites again, and they thank you with supplies and tips.
  - Kip hugs you and gives you his lucky charm: **Kip's Lucky Crystal** (+5 Wisdom accessory).

---

## 4. Characters

| Name | Place | Who They Are | How They Talk |
|---|---|---|---|
| **Elder Valen** | Village | The village leader. He signed the paper that gave Kip away. | Tired, sad, speaks in short, heavy sentences. Desperately wants to fix his mistake. |
| **Bess, the Baker** | Village | A town baker who fed Kip every morning. | Warm, tearful, angry at the village leaders. |
| **Kip** | Guard Camp Cage | The 11-year-old orphan boy. | Tough, dirty face, defiant. Even locked in a cage, he yells at the guards. |
| **Captain Ronald** | Guard Camp | The arrogant boss of the soldiers. | Smug, loud, talks down to the miners like they are dirt. |
| **Priest Malas** | Guard Camp | A quiet temple priest waiting to take Kip away. | Creepy, calm, talks in a soft whisper about "holy duty." |
| **Orin (Stone Golem)**| Mana Mine | A tired stone golem working in the deep rocks. | Silent until you give him a Wisdom boost, then wakes up as an old master builder. |

---

## 5. Party Wisdom & Awakening

- Followers you charm start out quiet and slow (Wisdom 3 to 5).
- If you give a follower **Kip's Lucky Crystal** or a clear-mind potion, their Wisdom goes up to 10 or higher.
- When their Wisdom hits 10, they **wake up**:
  - They remember their real name and their real past.
  - They can talk to you and give you advice about the dungeon.
  - The priests can no longer control or scare them.

---

## 6. Sentry and Scout Orders

With a bigger party, you can give your team two new jobs:
- **`guard` / `sentry`:** Tell a follower to stand at a doorway and hold it. They stop roaming guards from walking in on you.
- **`scout`:** Send a fast follower down a dark tunnel. They run ahead, look around, and come back to tell you what enemies and treasures are waiting.
