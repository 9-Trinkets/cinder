# Unified Skills & Progression — Design Specification

Living design specification for a **unified skill and progression system** in Cinder (with Layla as the reference implementation).

> **Status:** Specification revised and grounded. This document replaces the previous overcomplicated draft, resolves the architectural drift, unifies combat capabilities (`attack`, `defend`, `heal`, `spell`, `passive`), and restores `LevelDefinition.unlocks` to connect skills directly to level-up rewards.

---

## 1. Executive Summary & Why the Previous Design Drifted

The previous draft of this document attempted to solve capability grouping by introducing a massive meta-layer. Over successive revisions, it drifted away from Cinder's declarative engine architecture in several critical ways:

1. **Retiring `LevelDefinition.unlocks`:** The old doc argued to *retire* the engine's existing `LevelDefinition.unlocks` field, claiming progression was purely scroll-driven. This broke the natural, intuitive loop where leveling up grants new combat techniques.
2. **The 7-System Meta-Grant Abstraction:** It proposed a meta-wrapper (`grants: { action, craftable_item, healing, behavior_rule, stat_bonus, story_var, passive }`) that tried to puppet seven disjoint legacy subsystems instead of defining what a skill actually is.
3. **Shoehorning 60 Dungeon Mobs into JRPG Classes:** It attempted to force every dungeon creature (goblins, frost pawns, rooks, sprites) into player-like character classes (`thief`, `warrior`, `mage`, `priest`) with theoretical skills (`vanish`, `sanctuary`) that had no engine presence.
4. **Endless Semantic Debates:** Over 200 lines were spent debating 11 different titles for Layla (`apprentice`, `dungeon_master`, `inker`, `surveyor`, `boardwright`, etc.) rather than establishing mechanics.
5. **No Solution for Skill Visibility:** Players had no consistent way to view their current skills, inspect companion skills (e.g. what Einar or Astrid can do), or preview upcoming unlocks.

### The Grounded Realignment

Instead of inventing an abstract meta-class framework, this specification addresses the **four fundamental needs** of the engine:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        UNIFIED SKILL ARCHITECTURE                      │
├────────────────────────────────────────────────────────────────────────┤
│ 1. Define Skills:     Unified declarative schema in skills.json        │
│                       (attack / defend / heal / spell / passive)       │
├────────────────────────────────────────────────────────────────────────┤
│ 2. See Skills:        Visible in Player Status UI, Companion Profiles, │
│                       Party Sidebar, and Action Prompts                │
├────────────────────────────────────────────────────────────────────────┤
│ 3. Reward Skills:     Level-up unlocks via LevelDefinition.unlocks     │
│                       in levels.json (plus diegetic scrolls/sigils)    │
├────────────────────────────────────────────────────────────────────────┤
│ 4. Execute Skills:    Companions react using equipped skills;          │
│                       Player invokes skills via actions/commands       │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. The Core Problem: Fragmented Capabilities Today

Prior to this specification, combat actions and abilities lived in three completely unrelated, incompatible subsystems:

| Capability | Where it Lives Today | How it Works | Gaps / Inconsistencies |
|---|---|---|---|
| **Attack** | `actions.json` (Player)<br>`behavior.json` (NPCs) | Player uses `ATTACK` command.<br>NPCs use neuron `effect_table` rules (`strike`). | Player attacks cannot scale with special techniques. NPC attacks are disjoint from party allies. |
| **Defend / Guard** | `settings.json` (`party.combat_rules`) | Companions intercept or hold based on hardcoded party rules with priority weights. | Hardcoded to entire party; not tied to companion capabilities or progression. |
| **Heal** | `ActorDefinition.healing`<br>`settings.json` (`healer-support-order`) | Hostiles check `ActorHealingSpec { amount, message }`.<br>Allies check `actor_has_tag: "healer"`. | Disjoint definitions. Healing amount is hardcoded in two separate places. Companions cannot learn better heals. |
| **Spells / Sigils** | `actions.json` (`TRACE`)<br>`settings.json` (`periodic_actor_effects`) | Gated by story-var booleans (`knows_drain`, `knows_spawn`, etc.). | Invisible as "skills"; treated purely as crafting items or room effects. |

This fragmentation makes it hard for content creators to add a new ability, impossible for players to inspect what an ally can do, and disconnects leveling from ability acquisition.

---

## 3. The Unified Skill Model (`skills.json`)

All skills in Cinder are declared in a top-level `skills.json` file inside the content pack. 

### 3.1 The Skill Schema

```jsonc
// content/<pack>/skills.json
{
  "skills": [
    {
      "id": "strike",
      "label": "Strike",
      "kind": "attack",
      "target": "single_enemy",
      "description": "A focused physical strike against a target in the room.",
      "attack": {
        "stat": "strength",
        "power_multiplier": 1.0,
        "variance": 0.1
      },
      "cost": { "readiness": 100 },
      "narration_key": "combat.strike"
    },
    {
      "id": "shield_guard",
      "label": "Shield Guard",
      "kind": "defend",
      "target": "single_ally",
      "description": "Intercept incoming physical blows aimed at a vulnerable ally, reducing damage taken.",
      "defend": {
        "interception": true,
        "damage_reduction_percent": 30,
        "trigger_condition": {
          "ally_health_at_most_percent": 50
        }
      },
      "cost": { "cooldown_turns": 1 },
      "narration_key": "combat.guard_intercepts"
    },
    {
      "id": "field_mending",
      "label": "Field Mending",
      "kind": "heal",
      "target": "single_ally",
      "description": "Applies soothing herbal compresses to restore health to a wounded ally.",
      "heal": {
        "amount": 5,
        "trigger_condition": {
          "ally_health_at_most_percent": 60
        }
      },
      "cost": { "cooldown_turns": 2 },
      "narration_key": "combat.einar_heal"
    },
    {
      "id": "drain_sigil",
      "label": "Drain Sigil",
      "kind": "spell",
      "target": "room",
      "description": "Inscribes a siphon sigil in chalk that leeches vitality from hostile occupants each turn.",
      "spell": {
        "creates_room_item": "drain-sigil",
        "requires_item": "magic-chalk"
      },
      "narration_key": "combat.drain_sigil_traced"
    },
    {
      "id": "iron_resolve",
      "label": "Iron Resolve",
      "kind": "passive",
      "target": "self",
      "description": "Hardened discipline grants +2 Defense and immunity to fear.",
      "passive": {
        "stat_modifiers": { "defense": 2 }
      }
    }
  ]
}
```

### 3.2 The Five Core Skill Kinds

| Kind | Purpose | Execution Behavior |
|---|---|---|
| `attack` | Direct offensive strikes | Computes damage from actor stats (Strength/Intelligence), weapon bonuses, and power multiplier. Target takes damage modified by target defense. |
| `defend` | Defensive stances & ally interception | Triggers during the pre-damage combat reaction window. Can intercept strikes for an ally or reduce incoming damage to self. Replaces ad-hoc `party.combat_rules`. |
| `heal` | Vitality recovery for self or allies | Restores HP to the target. For autonomous allies, triggers when an ally falls below the specified health threshold. Replaces `ActorHealingSpec`. |
| `spell` | Inscriptions, sigils, and magic effects | Active tactical abilities (tracing floor sigils, teleporting, charming). Can require tools (e.g. magic chalk) or resources. |
| `passive` | Innate stat boosts and behavioral perks | Always active. Directly modifies effective stats or resistance profiles. |

---

## 4. Seeing Current Skills (Visibility & UI)

A major gap in the engine was player visibility: players could not see what their character was capable of, nor could they inspect companion abilities.

### 4.1 Player Skills Display

1. **Status Panel (`StatusPanel.tsx` / `LOOK STATUS`):**
   - Displays a clear **Skills** block alongside stats:
     ```
     ── Skills ──────────────────────────────────────────
     [Attack]  Strike           Deals Strength-based physical damage.
     [Spell]   Trace Sigil      Inscribe chalk rings (Charm, Drain).
     [Passive] Iron Will        +2 Defense in combat.
     ── Next Level (Lv 2) ──────────────────────────────
     [Defend]  Shield Guard     Intercept strikes aimed at allies.
     ```
2. **Dedicated Command / Inspector:**
   - Typing `skills` (or `sk`) outputs the full list of learned abilities with descriptions, readiness costs, and cooldowns.
3. **Action Bar Integration:**
   - Active skills appear dynamically in the combat action bar or sub-menus, replacing hardcoded action IDs with the actor's current skill repertoire.

### 4.2 Companion & Ally Skills Display

Companions have clear, distinct combat profiles. Players can view companion abilities in two ways:

1. **Companion Inspection (`look astrid`, `look einar`):**
   - Inspecting an ally displays their role, equipment, and active combat skills:
     ```
     Commander Astrid — Level 3 Guardian
     Stance: Following, Guarding Player
     Health: 26/26 | Defense: 6 | Strength: 6
     Active Skills:
     • Strike (Attack): High-damage physical attack.
     • Shield Guard (Defend): Intercepts attacks when player health < 50%.
     • Bastion (Passive): +2 Armor when standing in front.
     ```
2. **Party Sidebar HUD:**
   - In the HUD party display, each companion card shows their primary active skill badge next to their health bar:
     - `Astrid [Guardian: Shield Guard]`
     - `Einar [Healer: Field Mending]`

### 4.3 Inspecting Enemies

When inspecting a hostile creature (`look goblin-shaman`, `look lady-sylvan`):
- Visible combat traits are surfaced diegetically:
  - *"Carries a crooked healing branch and weaves restorative hexes."* (Healer)
  - *"Wields a heavy tower shield, ready to intercept attacks on allies."* (Guardian)

---

## 5. Tying Skills to Level-Up Rewards (`LevelDefinition.unlocks`)

The engine already has `LevelDefinition.unlocks: Vec<String>` in `cinder-core/src/content/types/leveling.rs`, but it was previously unused. We activate this exact mechanism.

### 5.1 Authoring Level Rewards in `levels.json`

In `content/<pack>/levels.json`, level thresholds declare stat gains **and** unlocked skill IDs:

```jsonc
// content/layla/levels.json
{
  "default": [
    {
      "exp_required": 50,
      "stat_changes": { "hp": 3, "strength": 1, "defense": 1 },
      "unlocks": ["shield_guard"]
    },
    {
      "exp_required": 100,
      "stat_changes": { "hp": 3, "strength": 1, "intelligence": 1 },
      "unlocks": ["flanking_strike"]
    },
    {
      "exp_required": 180,
      "stat_changes": { "hp": 4, "strength": 2, "defense": 1 },
      "unlocks": ["iron_resolve"]
    }
  ],
  "actors": {
    "einar": [
      {
        "exp_required": 50,
        "stat_changes": { "hp": 2, "wisdom": 1 },
        "unlocks": ["soothing_draught"]
      },
      {
        "exp_required": 100,
        "stat_changes": { "hp": 3, "wisdom": 2 },
        "unlocks": ["greater_mending"]
      }
    ]
  }
}
```

### 5.2 Engine Level-Up Reducer Flow (`reducer/combat/defeat.rs`)

When an actor accumulates enough XP to cross a level boundary:
1. **Apply Stat Changes:** Increment HP, Strength, Defense, etc. (existing logic).
2. **Grant Unlocks:** For every skill ID in `definition.unlocks`:
   - Insert the skill ID into `WorldState.actor_skills.entry(actor_id).or_default()`.
3. **Emit Level-Up Feedback:**
   - Render `combat.level_up` for the level gain.
   - For each granted skill, render `combat.skill_unlocked`:
     - Player: *"You mastered {skill_label}! {skill_description}"*
     - Companion: *"{actor_name} learned {skill_label}!"*

### 5.3 Coexistence with Diegetic Unlocks (Scrolls & Sigils)

Leveling up is not the only way to gain skills:
- **Scrolls & Items:** Reading the `drain-scroll` directly unlocks `drain_sigil` via the unified item handler.
- **Story Beats / Quests:** Freeing a prisoner or completing a floor trial can award a skill.
- The state representation is identical: once acquired, a skill sits in `WorldState.actor_skills` regardless of whether it arrived from a level-up or a scroll.

---

## 6. Companion Roles Grounded in Live Gameplay

Instead of abstract JRPG classes applied to 60 dungeon mobs, roles exist for the characters who actually participate in party tactics:

```
┌────────────────────────────────────────────────────────────────────────┐
│                          COMPANION ROLES                               │
├──────────────────┬─────────────────┬───────────────────────────────────┤
│ Role             │ Key Companions  │ Primary Skill Focus               │
├──────────────────┼─────────────────┼───────────────────────────────────┤
│ **Guardian**     │ Astrid, Sakhra  │ `defend` (Interception, Guard)    │
│ **Healer**       │ Einar           │ `heal` (Field Mending, Draughts)  │
│ **Skirmisher**   │ Malik, Awakened │ `attack` (Counters, Multi-strike) │
│ **Apprentice**   │ Layla (Player)  │ `spell` (Sigils, Inscriptions)    │
└──────────────────┴─────────────────┴───────────────────────────────────┘
```

### 6.1 Astrid (Guardian)
- **Role:** High defense frontline protector.
- **Starting Skills:** `strike` (Attack), `shield_guard` (Defend: intercepts attacks when player HP < 50%).
- **Progression Unlocks:** 
  - Level 2: `iron_wall` (Self Defense +2)
  - Level 3: `bastion_stand` (Intercepts damage at 50% reduction)

### 6.2 Einar (Healer)
- **Role:** Midline herbalist and combat medic.
- **Starting Skills:** `strike` (Attack), `field_mending` (Heal: restores 5 HP to allies below 60% HP).
- **Progression Unlocks:**
  - Level 2: `soothing_draught` (Heal: restores 8 HP and clears status ailments)
  - Level 3: `revitalize` (Heal: party-wide regenerative pulse)

### 6.3 Layla (Apprentice / Sigilist)
- **Role:** Tactical field commander utilizing chalk marks and sigil manipulation.
- **Starting Skills:** `strike` (Attack), `trace` (Spell: inscribe chalk marks).
- **Progression Unlocks:**
  - Scrolls / Leveling: `drain_sigil`, `spawn_sigil`, `teleport_sigil`.

---

## 7. Content Authoring Guide: Adding a New Skill

Content authors can define a new skill in 4 simple steps without modifying Rust engine code:

### Step 1: Declare the Skill in `skills.json`
Add the skill definition with its kind, target, cost, and parameters:
```json
{
  "id": "power_slash",
  "label": "Power Slash",
  "kind": "attack",
  "target": "single_enemy",
  "description": "A heavy two-handed slash dealing 150% damage.",
  "attack": {
    "stat": "strength",
    "power_multiplier": 1.5
  },
  "cost": { "cooldown_turns": 2 },
  "narration_key": "combat.power_slash"
}
```

### Step 2: Add Narrative Messages in `messages.json`
```json
"combat.power_slash": "{actor} winds up and unleashes a crushing Power Slash upon {target}, dealing {damage} damage! ({remaining} HP remaining)"
```

### Step 3: Assign to an Actor or Level Table
- **As a starting skill:** Add `"skills": ["strike", "power_slash"]` to the actor in `actors.json`.
- **As a level-up unlock:** Add `"unlocks": ["power_slash"]` to the desired level in `levels.json`.

### Step 4: Validate
Run the content validator:
```bash
cargo run -p cinder-tools -- lint -p layla
```
The linter verifies that:
- Every referenced skill ID exists in `skills.json`.
- All narration keys exist in `messages.json`.
- Target modes and parameters match the schema.

---

## 8. Gaps Identified & Resolved from the Previous Doc

| Topic | Previous Draft (Drifted) | Live Engine Reality | New Unified Specification |
|---|---|---|---|
| **Level Unlocks** | Declared `LevelDefinition.unlocks` retired and deleted. | `unlocks: Vec<String>` exists in `LevelDefinition` and `levels.json`. | **Restored and activated.** Leveling up awards skills from `unlocks`. |
| **Skill Definition** | 7-way meta-grant wrapper (`grants: { action, healing, behavior_rule, stat_bonus, ... }`). | Three disconnected systems (`actions.json`, `behavior.json`, `ActorHealingSpec`). | **Unified `skills.json`** with 5 operational kinds (`attack`, `defend`, `heal`, `spell`, `passive`). |
| **Cast Coverage** | Forced all 60 dungeon mobs into 5 JRPG classes with fake skills (`vanish`, etc.). | Dungeon enemies use stats and neuron behaviors. Only companions and player need skills. | Roles focused on **companions & player** (Guardian, Healer, Skirmisher, Apprentice). |
| **Skill Visibility** | None. Purely speculative status label discussion. | No UI or command exists to view companion or player abilities. | Clear UI specification: **Status Panel, Companion Inspect, Party Sidebar HUD**. |
| **Boss Bugs** | Documented bugs in §3 but left them unpatched in content. | Floor 5 bosses were Lv 1 instead of 12; Lady Sylvan had broken healing spec. | **Patched directly:** `actors.json` and `messages.json` fixed and verified clean. |

---

## 9. Phased Implementation Roadmap

### Phase 1: Core Skill Schema & Content Loading
1. Create `cinder-core/src/content/types/skills.rs` defining `SkillDefinition`, `SkillKind`, `AttackSkillSpec`, `DefendSkillSpec`, `HealSkillSpec`, `SpellSkillSpec`, and `PassiveSkillSpec`.
2. Add `skills: SkillsDefinition` to `ContentPack` and load `content/<pack>/skills.json` via `read_optional_json`.
3. Add `starting_skills: Vec<String>` to `ActorDefinition`.
4. Add `actor_skills: BTreeMap<String, BTreeSet<String>>` to `WorldState`.

### Phase 2: Level-Up Grant Reducer Integration
1. In `cinder-core/src/engine/reducer/combat/defeat.rs`, loop over `definition.unlocks` when an actor levels up.
2. Insert unlocked skills into `state.actor_skills`.
3. Push `combat.skill_unlocked` narrative feedback lines.

### Phase 3: Party Reactions & Autonomous Execution
1. Update `cinder-core/src/engine/reducer/handlers/combat_reactions.rs`:
   - Replace tag-based healing (`actor_has_tag: "healer"`) with checking if the companion knows a `heal` skill.
   - Replace generic intercept with the companion's equipped `defend` skill.
2. Ground Lady Sylvan, Einar, and Astrid in their declared skills.

### Phase 4: UI & Visibility
1. Update `cinder-core/src/ui/` and `StatusPanel.tsx` to surface known skills and upcoming level-up unlocks.
2. Add companion skill summaries to companion inspection (`look <companion>`) and party sidebar cards.
3. Add `skills` command to player command parser.
