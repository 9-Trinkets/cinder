# Character Levels and Stat Mechanics

Every character in the dungeon—Layla, her companions, common dungeon monsters, and powerful bosses—is placed on a unified scale from **Level 1 to Level 10**.

---

## 1. Floor Level Progression

Each floor challenges the party with higher-level enemies and rewards deeper exploration:

| Floor | Setting / Theme | Typical Actor Levels | Key Bosses / Allies |
| :---: | :--- | :---: | :--- |
| **Floor 1** | Stone Grid & Golems | **Level 1** | Layla (L1), Golems (L1), Shaman (L2), Fire Elemental (L2) |
| **Floor 2** | Royal Elf Garden | **Level 2–3** | Elf Pawns (L2), Elf Knights & Bishops (L3), Elf King (L3) |
| **Floor 3** | Fire Chasm & Village | **Level 3** | Fire Sprites (L3), Kamil & Zayd (L2), Village Elders (L3) |
| **Floor 4** | Steam Fortress & Mines | **Level 4–5** | Steam Automatons (L4), Fortress Guards (L4), Captain Malik (L5), Sakhra (L5) |
| **Floor 5** | Frozen Citadel | **Level 5–6** | Frost Raiders (L5), Astrid (L6), Einar (L6), Citadel Captain (L6) |
| **Floor 5.5** | 5 Worker Towns | **Level 6–7** | Rescued Civilians (L1), Cultural Golems & Town Guardians (L6–7) |
| **Floor 6** | Gilded Cathedral | **Level 7–8** | Cathedral Sentries (L7), House Heads (L8), Seer Valerius (L8) |
| **Floor 7** | The Antechamber | **Level 8–10** | Layla Ascended (L9–10), The Handler (L10) |

---

## 2. Core Stats and Stat Budgets

Every character has four core primary stats:
- **Strength (STR)**: Physical hitting power and close-quarters force.
- **Endurance (END)**: Physical defense and body toughness.
- **Dexterity (DEX)**: Speed and combat action frequency.
- **Intelligence (INT)**: Tactical mark mastery and spell power.

### The Stat Budget Formula
To keep encounters balanced, the sum of a character's base stats is strictly determined by their level:

$$\mathbf{Stat\ Budget} = 12 + (\mathbf{Level} \times 3)$$

- **Level 1**: 15 points
- **Level 2**: 18 points
- **Level 3**: 21 points
- **Level 4**: 24 points
- **Level 5**: 27 points
- **Level 6**: 30 points
- **Level 7**: 33 points
- **Level 8**: 36 points
- **Level 9**: 39 points
- **Level 10**: 42 points

When leveling up, characters receive **+3 primary stat points** to distribute among their core attributes.

---

## 3. Derived Vitals (HP and MP)

Health and magic pools are calculated directly from a character's level and core stats:

### Max HP (Health Points)
$$\mathbf{Max\ HP} = 8 + (\mathbf{Level} \times 4) + (\mathbf{Endurance} \times 3) + (\mathbf{Strength} \times 1)$$

- Every level gained gives **+4 base HP**.
- Each point of **Endurance** gives **+3 HP**.
- Each point of **Strength** gives **+1 HP**.

### Max MP (Magic Points)
$$\mathbf{Max\ MP} = 4 + (\mathbf{Level} \times 2) + (\mathbf{Intelligence} \times 3)$$

- Every level gained gives **+2 base MP**.
- Each point of **Intelligence** gives **+3 MP**.

---

## 4. Default Level-Up Growth Table

When Layla and party members level up, their stats grow along a steady progression curve:

| Level Transition | EXP Needed | Primary Stat Growth | HP Gain | MP Gain |
| :---: | :---: | :--- | :---: | :---: |
| **1 → 2** | 50 | +1 STR, +1 END, +1 DEX | +8 HP | +2 MP |
| **2 → 3** | 100 | +1 END, +1 INT, +1 WIS | +7 HP | +6 MP |
| **3 → 4** | 180 | +1 STR, +1 END, +1 DEX | +8 HP | +2 MP |
| **4 → 5** | 300 | +1 STR, +1 END, +1 INT | +8 HP | +5 MP |
| **5 → 6** | 450 | +1 END, +1 DEX, +1 WIS | +7 HP | +3 MP |
| **6 → 7** | 650 | +1 STR, +1 END, +1 INT | +8 HP | +5 MP |
| **7 → 8** | 900 | +1 STR, +1 END, +1 WIS | +8 HP | +3 MP |
| **8 → 9** | 1200 | +1 END, +1 DEX, +1 INT | +7 HP | +5 MP |
| **9 → 10** | 1600 | +1 STR, +1 INT, +1 WIS | +5 HP | +6 MP |

Cumulative growth from Level 1 to 10 adds **+27 primary stat points** (+6 STR, +8 END, +4 DEX, +5 INT, +4 WIS), **+66 Max HP**, and **+37 Max MP**.

---

## 5. Roster Reference Table

| Actor ID | Name | Floor | Level | STR | END | DEX | INT | WIS | HP | MP |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| `player` | Layla | 1–7 | 1 | 3 | 1 | 5 | 3 | 3 | 18 | 18 |
| `golem-dark-nw` | Dark Golem (Orin) | 1 | 1 | 4 | 4 | 2 | 1 | 4 | 28 | 13 |
| `goblin-1` | Goblin | 1 | 1 | 3 | 2 | 6 | 2 | 2 | 21 | 14 |
| `goblin-shaman` | Goblin Shaman | 1 | 2 | 2 | 2 | 4 | 6 | 4 | 24 | 30 |
| `fire-elemental` | Fire Elemental | 1 | 2 | 4 | 3 | 3 | 5 | 3 | 29 | 26 |
| `elf-pawn-1` | Elf Pawn | 2 | 2 | 4 | 3 | 4 | 3 | 4 | 29 | 21 |
| `elf-knight-2` | Elf Knight | 2 | 3 | 5 | 4 | 5 | 3 | 4 | 37 | 23 |
| `elf-bishop-3` | Elf Bishop | 2 | 3 | 3 | 3 | 4 | 6 | 5 | 32 | 33 |
| `elf-rook-1` | Elf Rook | 2 | 3 | 6 | 6 | 3 | 3 | 3 | 44 | 21 |
| `elf-king-5` | Elf King | 2 | 3 | 5 | 5 | 4 | 4 | 3 | 40 | 27 |
| `fire-sprite-1` | Fire Sprite | 3 | 3 | 3 | 2 | 7 | 5 | 4 | 29 | 29 |
| `zayd` | Zayd | 3 | 2 | 3 | 2 | 5 | 5 | 3 | 25 | 27 |
| `steam_automaton_west` | Driller Automaton | 4 | 4 | 6 | 6 | 4 | 4 | 4 | 48 | 28 |
| `garrison_sentry_gate` | Garrison Sentry | 4 | 4 | 5 | 5 | 5 | 5 | 4 | 44 | 31 |
| `sakhra` | Sakhra (Siraj) | 4 | 5 | 7 | 7 | 3 | 4 | 6 | 56 | 32 |
| `captain_malik` | Captain Malik | 4 | 5 | 7 | 7 | 5 | 4 | 4 | 56 | 30 |
| `commander_astrid` | Astrid | 5 | 6 | 7 | 7 | 6 | 5 | 5 | 60 | 36 |
| `einar` | Einar | 5 | 6 | 3 | 5 | 5 | 9 | 8 | 50 | 51 |
| `lord_vane` | Lord Vane | 5/6 | 8 | 8 | 6 | 8 | 8 | 6 | 66 | 50 |
| `warmaster_torin` | Warmaster Torin | 5/6 | 8 | 10 | 8 | 7 | 6 | 5 | 74 | 43 |
| `lady_sylvan` | Lady Sylvan | 5/6 | 8 | 6 | 4 | 10 | 9 | 7 | 58 | 54 |
| `the_seer` | Seer Valerius | 6 | 8 | 5 | 7 | 6 | 10 | 8 | 66 | 58 |
| `handler` | The Handler | 7 | 10 | 8 | 10 | 8 | 8 | 8 | 86 | 56 |
