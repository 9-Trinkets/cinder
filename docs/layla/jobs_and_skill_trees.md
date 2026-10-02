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
        "power_multiplier": 1
      }
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
          "ally_health_at_most_percent": 50,
          "self_health_at_least_percent": 25
        }
      },
      "narration_key": "combat.guard_intercepts"
    },
    {
      "id": "hold",
      "label": "Hold",
      "kind": "defend",
      "target": "self_actor",
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
      "description": "Unseals soothing draughts for any wounded ally, restoring health.",
      "heal": {}
    },
    {
      "id": "trace",
      "label": "Trace",
      "kind": "spell",
      "target": "room",
      "description": "Inscribes glowing tactical chalk sigils upon the stone.",
      "spell": {
        "requires_item": "magic-chalk"
      }
    },
    {
      "id": "teleport",
      "label": "Teleport",
      "kind": "spell",
      "target": "anchor",
      "description": "Instantly transports the party back to the armed teleportation anchor.",
      "spell": {
        "requires_story_var": "knows_teleport"
      }
    }
  ]
}
```

### 2.1 Corrections From Earlier Drafts of This Document

Earlier revisions of this spec drifted from the code it described. The
differences below are now the source of truth, and
`cinder-core/tests/layla_content_loads.rs` enforces them so the drift cannot
silently return.

| Earlier claim | Shipped reality |
|---|---|
| `intercept` triggers at `ally_health_at_most_percent: 50` | The 50% applies to the **player**, and only under the `follow` order (`player_health_at_most_percent`). Under the `guard` order there is no ally threshold at all. |
| `heal` triggers at `ally_health_at_most_percent: 60` | **No 60% threshold exists anywhere in the pack.** The live rule is tag-based (`actor_has_tag: healer`) plus `any_ally_wounded`. |
| `intercept` has one trigger condition | Live behavior is **two** rules — `guard-order` and `follow-protect-player` — with different gating. Both additionally require the defender at ≥25% health. |
| `heal` declares `amount: 5` and `narration_key: combat.einar_heal` | Healing potency and narration are **per-actor**: bishops heal for 4 (`combat.bishop_heal`), Einar for 5, the queen for 6, Lady Sylvan for 10. A shared amount or narration would silently rewrite four of the five healers. Both now live on `ActorDefinition.healing`. |
| `hold` targets `"self"` | `SkillTargetMode` renames to `self_actor`; `"self"` was never loadable. |

Two schema notes worth keeping in mind when authoring:

- `power_multiplier` is a `u32`, so fractional multipliers cannot be expressed
  without a schema change.
- `defend.trigger_condition` describes the *stricter* of the backing rules. The
  per-rule conditions in `settings.json` remain authoritative; the drift test
  only guarantees that every percentage a skill names is one some live rule
  actually gates on.

---


## 3. Skill Visibility: Seeing Current Skills

Consolidating behaviors into skills enables players to easily view their own capabilities and inspect what their companions can do.

> **Status: design target, not yet implemented.** Nothing in this section
> renders today — see Phase 5 in §7. The layouts below are the intended shape,
> with values corrected against `actors.json` and `skills.json`.

### 3.1 Player Status Display (`StatusPanel.tsx` / `LOOK STATUS`)

The status panel displays Layla's known skills:

```
── Skills ──────────────────────────────────────────
[Attack] Strike        Physical attack against a foe in the room.
[Defend] Hold          Defensive stance when critically wounded.
[Spell]  Trace         Inscribe glowing chalk sigils on the floor.
```

There is no "Next Level" block today: `levels.json` grants no unlocks (see
§5.1). The panel should render one only once an unlock exists to show.

### 3.2 Companion Profile (`look astrid`, `look einar`)

When inspecting a companion in the room, their role and active skills are rendered clearly:

```
Commander Astrid — Guardian (Level 7)
Stance: Following, Guarding Layla
Health: 34/34 | Defense: 6 | Strength: 6
Active Skills:
• Strike (Attack): Standard attack / counterattack.
• Intercept (Defend): Steps in when the player drops to 50% health, or on a guard order.
• Hold (Defend): Holds stance when HP < 25%.
```

```
Einar — Healer (Level 6)
Stance: Following
Health: 20/20 | Defense: 3 | Wisdom: 6
Active Skills:
• Strike (Attack): Standard physical attack.
• Heal (Heal): Restores 5 HP to the most wounded ally.
• Hold (Defend): Holds stance when HP < 25%.
```

Levels and stats above match `actors.json`; the skill lists match
`skills.json` plus each actor's declared skills.

### 3.3 Party Sidebar HUD

In the HUD sidebar, each companion card displays their active role badge:
- `Astrid [Guardian: Intercept]`
- `Einar [Healer: Heal]`

---

## 4. Tying Skills to Level-Up Rewards (`LevelDefinition.unlocks`)

We activate [`LevelDefinition.unlocks: Vec<String>`](file:///Users/li-hsuanlung/Projects/cinder/cinder-core/src/content/types/leveling.rs) in `levels.json`. When actors level up, skills in `unlocks` are automatically granted.

### 4.1 Progression in `content/layla/levels.json`

`levels.json` currently ships **no** unlocks. The mechanism is live and tested
(`leveling_up_grants_declared_unlocks_and_narrates_them_per_actor`); the content
simply has nothing to award yet, for the reason given in §5.1. The shape an
author would use:

```jsonc
// content/layla/levels.json
{
  "default": [
    {
      "exp_required": 50,
      "stat_changes": { "hp": 3, "strength": 1, "defense": 1 },
      "unlocks": ["some_skill_id"]
    }
  ],
  "actors": {
    "commander_astrid": [
      {
        "exp_required": 50,
        "stat_changes": { "hp": 4, "defense": 2 },
        "unlocks": ["some_skill_id"]
      }
    ]
  }
}
```

Per-actor tables are keyed by **actor id**, so Astrid's key is
`commander_astrid`, not `astrid`. Remember that an entry at index `n` governs
the `n+1 → n+2` transition, and that actors start above level 1 (Astrid at 7,
Einar at 6) — a short table will never fire for them.

### 4.2 Level-Up Engine Flow (`reducer/combat/defeat.rs`)

When XP advances an actor's level:
1. Apply stat changes to `WorldState`.
2. For each skill ID in `definition.unlocks`:
   - `state.grant_actor_skill(actor_id, skill_id)` — returns `true` only on
     first acquisition, so a re-granted skill never narrates twice.
   - Push one of:
     - `combat.player_skill_unlocked` — *"You unlocked {skill_label}!"*
     - `combat.skill_unlocked` — *"{actor_name} unlocked {skill_label}!"*

Both messages are keyed off `skill.label`, falling back to the raw skill id when
a skill declares no label.

### 4.3 Scroll Gating

Scrolls continue to work as before, but they gate on **story vars rather than
skill ownership** — `teleport` is unlocked by `knows_teleport`, which
`teleport-scroll` sets, and the `trace` sigils are gated by `teleport-sigil`.
Scrolls do not currently deposit into `WorldState.actor_skills`; teaching them
to grant skills is future work.

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
- **Acquired Skills:** `teleport` (available once `knows_teleport` is set by
  reading `teleport-scroll`; it is gated by story var rather than by skill
  ownership).
- **No level-up unlocks yet.** Every reactive party behavior filters the player
  out (`party_policy.rs` rejects the player actor from intercept candidates), so
  a player-side unlock would be a cosmetic badge with no mechanic behind it
  until that exclusion is deliberately revisited.

### 5.2 Astrid (Guardian)
- **Role:** Frontline defender shielding the Commander and party.
- **Skills:** `strike`, `hold`, `intercept`.
- **Note:** `intercept` is a starting skill, not a level-2 unlock. Astrid ships
  at level 7 in `actors.json`, so a level-2 unlock would never fire for her —
  and interception is her defining live behavior, gated behind `guard`/`follow`
  orders.

### 5.3 Einar (Healer)
- **Role:** Combat support keeping the party alive.
- **Skills:** `strike`, `hold`, `heal`. Heals for 5 under the tag-based
  `healer-support-order` rule, which fires whenever any ally is wounded.

### 5.4 Clergy & Boss Hostiles (Lady Sylvan, Elf Bishops, Queen)
- **Role:** Hostile combatants using existing behaviors.
- **Skills:** `strike`, `heal`. Healing potency and narration are per-actor:
  bishops 4 (`combat.bishop_heal`), queen 6 (`combat.queen_heal`), Lady Sylvan
  10 (`combat.sylvan_heal`).

### 5.5 Actors Outside the Skills System
Golems and sprites are converted to allies at runtime by `set_stance_by_tag`
hooks and have `follow` orders, but they declare no `skills` list. The gate in
`party_policy.rs` is opt-in per actor, so they keep their original rule-driven
reactions. Any actor that joins the party later without a `skills` list behaves
the same way.


---

## 6. Migration Map: 1:1 Code Consolidation

| Live Subsystem | Live Location | Consolidated Skill Mapping |
|---|---|---|
| **Attack Command** | `content/layla/actions.json` (`command: "ATTACK"`) | Invokes `strike`. |
| **Hostile Strike** | `cinder-core/src/engine/hostile_actions.rs` | Uses `strike`. |
| **Party Counter** | `settings.json: assist-order` (`Counterattack`) | Uses `strike` during the reaction window. Not skill-gated: companions own `strike` implicitly. |
| **Party Intercept (guard)** | `settings.json: guard-order` (`Intercept`) | Gated on `intercept`. Requires `order_is: guard` + defender ≥25% health. |
| **Party Intercept (follow)** | `settings.json: follow-protect-player` (`Intercept`) | Gated on `intercept`. Requires `order_is: follow` + player ≤50% + defender ≥25%. |
| **Party Hold** | `settings.json: survival-hold` (`Hold`) | Gated on `hold` when self health ≤25%. |
| **Party Healing** | `settings.json: healer-support-order` (`Support`) | Gated on `heal`. Fires on `actor_has_tag: healer` + `any_ally_wounded` — **no health threshold**. |
| **Hostile Healing** | `ActorDefinition.healing` & `hostile_actions.rs` | `heal` declares the capability; amount and message stay per-actor. |
| **Chalk Sigils** | `content/layla/actions.json` (`command: "TRACE"`) | `trace` requires `magic-chalk`. |
| **Fast Travel** | `content/layla/actions.json` (`command: "TELEPORT"`) | `teleport` requires story var `knows_teleport`. |

The gate in `party_policy.rs` maps `Intercept → intercept`, `Hold → hold`,
`Support → heal`, and leaves `Counterattack` ungated.

---

## 7. Implementation Status

| Phase | Status |
|---|---|
| 1. Content type & loader | Done — `content/types/skills.rs`, `skills.json` loading, `ActorDefinition.skills`, `WorldState.actor_skills`. |
| 2. Level-up wireup | Done — `defeat.rs` grants `definition.unlocks` and narrates per actor, no re-narration on re-grant. |
| 3. Reaction & AI hook consolidation | Done — `party_policy.rs` gates intercept/hold/support; `combat_reactions.rs` and `hostile_actions.rs` share `actor_skill_of_kind`. |
| 4. Author `content/layla/skills.json` | Done — 6 skills authored; `player`, `commander_astrid`, `einar`, and the four clergy/boss healers declare skills. |
| 5. Visibility & UI | **Not started** — status panel, companion inspect, and sidebar badges do not render skills yet. |

### 7.1 Guard Tests

`cinder-core/tests/layla_content_loads.rs` pins the content invariants:

- `layla_skills_declare_only_live_behaviors` — the six ids, in order.
- `layla_actor_skills_resolve_against_skills_json` — no actor names a skill
  that does not exist.
- `layla_party_actors_keep_their_live_reactive_behaviors` — Astrid and Einar
  retain exactly their live reactive skills.
- `actors_outside_the_skills_system_are_ungated` — golems/handler stay outside.
- `level_up_unlocks_reference_declared_skills` — any unlock must resolve.
- `skill_trigger_percentages_come_from_the_live_party_rules` — **the drift
  guard**: every percentage a skill declares must be one the backing party rule
  actually gates on.
- `every_reactive_party_rule_has_a_declared_skill` — a new rule cannot be added
  without a corresponding skill.
- `the_heal_skill_defers_potency_and_narration_to_the_actor` — the shared heal
  skill claims no amount or narration, and the 4/5/6/10 spread is intact.

`cinder-core/tests/reducer/party_defense.rs` pins the gate itself, including a
mutation-verified pair: the same follower does not intercept without the skill,
does once it learns it, and still intercepts if it declares no skills at all.

