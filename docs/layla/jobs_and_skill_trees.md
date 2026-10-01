# Unified Skills & Progression — Design Specification

Living design specification for consolidating live combat and tactical capabilities into a **unified skills and progression system** in Cinder (with Layla as the reference implementation).

> **Core Principle:** **Do not invent new skills.** Every entry in the new skills schema is a 1:1 consolidation of an existing live behavior already running in Cinder (`strike`, `intercept`, `hold`, `heal`, `trace`, `teleport`).

---

## 1. Executive Summary & The Migration Goal

Today, Cinder already has a rich set of combat actions and party reactions, but they are scattered across four disconnected subsystems:
1. **Player Actions** (`actions.json`): `ATTACK`, `TRACE`, `TELEPORT`.
2. **Enemy Hostile AI** (`behavior.json` & `hostile_actions.rs`): `strike` events and `ActorHealingSpec`.
3. **Party Defense & Support** (`settings.json` `party.combat_rules` & `combat_reactions.rs`): `intercept`, `hold`, `support` (healer tag), `counterattack`.
4. **Level-Up Unlocks** (`levels.json` & `leveling.rs`): `LevelDefinition.unlocks: Vec<String>` exists in the engine schema, but is left unused.

Because there is no central skill definition:
- Players cannot see what abilities their companions (Astrid, Einar) have.
- Companions cannot learn abilities upon leveling up.
- Adding or adjusting an ability requires touching multiple disparate files and engine match arms.

### The Solution: 1:1 Behavior Consolidation
We do not invent any new RPG skills. We migrate the **six existing live behaviors** into a single declarative file: `skills.json`.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        LIVE BEHAVIORS MIGRATION                        │
├─────────────────┬───────────┬──────────────────────────────────────────┤
│ Live Behavior   │ Skill ID  │ Live Source                              │
├─────────────────┼───────────┼──────────────────────────────────────────┤
│ Basic Attack    │ strike    │ actions.json (ATTACK), behavior.json,    │
│                 │           │ party assist counterattack               │
├─────────────────┼───────────┼──────────────────────────────────────────┤
│ Ally Guard      │ intercept │ settings.json (guard-order, follow-guard)│
├─────────────────┼───────────┼──────────────────────────────────────────┤
│ Survival Hold   │ hold      │ settings.json (survival-hold at < 25% HP)│
├─────────────────┼───────────┼──────────────────────────────────────────┤
│ Ally Healing    │ heal      │ ActorHealingSpec, healer-support-order   │
├─────────────────┼───────────┼──────────────────────────────────────────┤
│ Sigil Tracing   │ trace     │ actions.json (TRACE + magic chalk)       │
├─────────────────┼───────────┼──────────────────────────────────────────┤
│ Fast Travel     │ teleport  │ actions.json (TELEPORT command)          │
└─────────────────┴───────────┴──────────────────────────────────────────┘
```

---

## 2. The Consolidated Skills Schema (`skills.json`)

All abilities are declared in `content/<pack>/skills.json`. Here is the complete declaration for Layla's existing behaviors:

```jsonc
// content/layla/skills.json
{
  "skills": [
    {
      "id": "strike",
      "label": "Strike",
      "kind": "attack",
      "target": "single_enemy",
      "description": "A focused physical attack against a target in the room.",
      "attack": {
        "stat": "strength",
        "power_multiplier": 1.0
      },
      "narration_key": "combat.strike"
    },
    {
      "id": "intercept",
      "label": "Intercept",
      "kind": "defend",
      "target": "single_ally",
      "description": "Steps in front of incoming attacks aimed at a vulnerable ally, absorbing the blow.",
      "defend": {
        "interception": true,
        "trigger_condition": {
          "ally_health_at_most_percent": 50
        }
      },
      "narration_key": "combat.guard_intercepts"
    },
    {
      "id": "hold",
      "label": "Hold",
      "kind": "defend",
      "target": "self",
      "description": "Holds defensive stance when critically wounded, avoiding risky exposure.",
      "defend": {
        "interception": false,
        "trigger_condition": {
          "self_health_at_most_percent": 25
        }
      },
      "narration_key": "combat.party_holds"
    },
    {
      "id": "heal",
      "label": "Heal",
      "kind": "heal",
      "target": "lowest_health_ally",
      "description": "Administers restorative herbs or weaves magic to restore health to a wounded ally.",
      "heal": {
        "amount": 5,
        "trigger_condition": {
          "ally_health_at_most_percent": 60
        }
      },
      "narration_key": "combat.einar_heal"
    },
    {
      "id": "trace",
      "label": "Trace",
      "kind": "spell",
      "target": "room",
      "description": "Inscribes glowing tactical chalk sigils (Charm, Drain, Spawn, Teleport) upon the stone.",
      "spell": {
        "requires_item": "magic-chalk"
      },
      "narration_key": "layla.traced"
    },
    {
      "id": "teleport",
      "label": "Teleport",
      "kind": "spell",
      "target": "anchor",
      "description": "Instantly transports the party back to the armed teleportation anchor.",
      "spell": {
        "requires_story_var": "knows_teleport"
      },
      "narration_key": "teleport.executed"
    }
  ]
}
```

---

## 3. Skill Visibility: Seeing Current Skills

Consolidating behaviors into skills enables players to easily view their own capabilities and inspect what their companions can do.

### 3.1 Player Status Display (`StatusPanel.tsx` / `LOOK STATUS`)

The status panel displays Layla's known skills and next level-up unlock:

```
── Skills ──────────────────────────────────────────
[Attack] Strike        Physical attack against a foe in the room.
[Defend] Hold          Defensive stance when critically wounded.
[Spell]  Trace         Inscribe glowing chalk sigils on the floor.
── Next Level (Lv 2) ──────────────────────────────
Unlocks: Intercept (Protect party members in combat)
```

### 3.2 Companion Profile (`look astrid`, `look einar`)

When inspecting a companion in the room, their role and active skills are rendered clearly:

```
Commander Astrid — Guardian (Level 2)
Stance: Following, Guarding Layla
Health: 26/26 | Defense: 6 | Strength: 6
Active Skills:
• Strike (Attack): Standard attack / counterattack.
• Intercept (Defend): Protects allies when HP < 50%.
• Hold (Defend): Holds stance when HP < 25%.
```

```
Einar — Healer (Level 1)
Stance: Following
Health: 18/18 | Defense: 2 | Wisdom: 5
Active Skills:
• Strike (Attack): Standard physical attack.
• Heal (Heal): Restores 5 HP to wounded allies.
• Hold (Defend): Holds stance when HP < 25%.
```

### 3.3 Party Sidebar HUD

In the HUD sidebar, each companion card displays their active role badge:
- `Astrid [Guardian: Intercept]`
- `Einar [Healer: Heal]`

---

## 4. Tying Skills to Level-Up Rewards (`LevelDefinition.unlocks`)

We activate [`LevelDefinition.unlocks: Vec<String>`](file:///Users/li-hsuanlung/Projects/cinder/cinder-core/src/content/types/leveling.rs) in `levels.json`. When actors level up, skills in `unlocks` are automatically granted.

### 4.1 Progression in `content/layla/levels.json`

```jsonc
// content/layla/levels.json
{
  "default": [
    {
      "exp_required": 50,
      "stat_changes": { "hp": 3, "strength": 1, "defense": 1 },
      "unlocks": ["intercept"]
    },
    {
      "exp_required": 100,
      "stat_changes": { "hp": 3, "strength": 1, "intelligence": 1 },
      "unlocks": []
    }
  ],
  "actors": {
    "astrid": [
      {
        "exp_required": 50,
        "stat_changes": { "hp": 4, "defense": 2 },
        "unlocks": ["intercept"]
      }
    ],
    "einar": [
      {
        "exp_required": 50,
        "stat_changes": { "hp": 2, "wisdom": 2 },
        "unlocks": ["heal"]
      }
    ]
  }
}
```

### 4.2 Level-Up Engine Flow (`reducer/combat/defeat.rs`)

When XP advances an actor's level:
1. Apply stat changes to `WorldState`.
2. For each skill ID in `definition.unlocks`:
   - `state.actor_skills.entry(actor_id).or_default().insert(skill_id);`
   - Push `combat.skill_unlocked`:
     - Player: *"You mastered {skill_label}!"*
     - Companion: *"{actor_name} learned {skill_label}!"*

### 4.3 Diegetic Scroll Unlocks
Reading scrolls (e.g. `teleport-scroll`, `drain-scroll`) continues to work naturally. When a scroll is read:
- It grants the corresponding skill (or sets the story var that makes the sigil available in the `trace` menu).
- Both level-up rewards and scroll items deposit their unlocked abilities into `WorldState.actor_skills`.

---

## 5. Party Roles Grounded in Existing Behaviors

Party roles simply describe which existing behaviors a character focuses on:

```
┌────────────────────────────────────────────────────────────────────────┐
│                              PARTY ROLES                               │
├──────────────────┬─────────────────┬───────────────────────────────────┤
│ Role             │ Key Actors      │ Existing Live Behaviors           │
├──────────────────┼─────────────────┼───────────────────────────────────┤
│ **Commander**    │ Layla (Player)  │ strike, hold, trace, teleport     │
│ **Guardian**     │ Astrid, Sakhra  │ strike, hold, intercept           │
│ **Healer**       │ Einar           │ strike, hold, heal                │
│ **Skirmisher**   │ Malik, Awakened │ strike, hold (counterattacks)     │
└──────────────────┴─────────────────┴───────────────────────────────────┘
```

### 5.1 Layla (Commander)
- **Role:** Directs party movement, issues companion orders (`follow`, `guard`), and shapes the room with chalk sigils.
- **Starting Skills:** `strike`, `hold`, `trace`.
- **Acquired Skills:** `teleport` (unlocked via reading `teleport-scroll`), `intercept` (unlocked at Level 2).

### 5.2 Astrid (Guardian)
- **Role:** Frontline defender shielding the Commander and party.
- **Starting Skills:** `strike`, `hold`.
- **Level-Up Unlock (Level 2):** `intercept` (actively intercepts attacks aimed at Layla or allies below 50% HP).

### 5.3 Einar (Healer)
- **Role:** Combat support keeping the party alive.
- **Starting Skills:** `strike`, `hold`, `heal` (unseals soothing draughts for wounded allies below 60% HP).

### 5.4 Clergy & Boss Hostiles (Lady Sylvan, Elf Bishops, Queen)
- **Role:** Hostile combatants using existing behaviors.
- **Skills:** `strike`, `heal` (heals wounded allies or self).

---

## 6. Migration Map: 1:1 Code Consolidation

| Live Subsystem | Live Location | Consolidated Skill Mapping |
|---|---|---|
| **Attack Command** | `content/layla/actions.json` (`command: "ATTACK"`) | Invokes `strike` skill. |
| **Hostile Strike** | `cinder-core/src/engine/hostile_actions.rs` | Uses `strike` skill. |
| **Party Counter** | `settings.json: assist-order` (`Counterattack`) | Uses `strike` skill during reaction window. |
| **Party Intercept**| `settings.json: guard-order` (`Intercept`) | Uses `intercept` skill during pre-damage window. |
| **Party Hold** | `settings.json: survival-hold` (`Hold`) | Uses `hold` skill when health <= 25%. |
| **Party Healing** | `settings.json: healer-support-order` (`Support`) | Uses `heal` skill when ally health <= 60%. |
| **Hostile Healing**| `ActorDefinition.healing` & `hostile_actions.rs` | Uses `heal` skill when enemy ally health <= 60%. |
| **Chalk Sigils** | `content/layla/actions.json` (`command: "TRACE"`) | Uses `trace` skill with `magic-chalk`. |
| **Fast Travel** | `content/layla/actions.json` (`command: "TELEPORT"`) | Uses `teleport` skill. |

---

## 7. Phased Implementation Plan

1. **Phase 1: Content Type & Loader**
   - Add `cinder-core/src/content/types/skills.rs` defining `SkillDefinition` (`id`, `label`, `kind`, `target`, `attack`, `defend`, `heal`, `spell`).
   - Add `skills.json` loading to `cinder-core/src/content/loader/mod.rs`.
   - Add `skills: Vec<String>` to `ActorDefinition` and `actor_skills: BTreeMap<String, BTreeSet<String>>` to `WorldState`.
2. **Phase 2: Level-Up Wireup**
   - In `cinder-core/src/engine/reducer/combat/defeat.rs`, iterate `definition.unlocks` to grant skills on level advancement.
3. **Phase 3: Reaction & AI Hook Consolidation**
   - Update `combat_reactions.rs` to check whether the companion has `intercept`, `hold`, or `heal` in `actor_skills`.
   - Update `hostile_actions.rs` to check for `heal` skill in enemy `actor_skills`.
4. **Phase 4: Author `content/layla/skills.json`**
   - Create `skills.json` containing the 6 live behaviors.
   - Populate `skills` on `actors.json` (`player`, `astrid`, `einar`).
5. **Phase 5: Visibility & UI**
   - Expose skills in the Status panel and companion inspect/sidebar.
