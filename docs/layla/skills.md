# Skills Architecture

Cinder skills are unranked actor capabilities. A skill is defined once, actors
are assigned its id, and every way to execute that capability is authored with
the skill.

## Source of Truth

`content/<pack>/skills.json` owns:

- identity and presentation metadata;
- the player command, when the skill is directly invoked;
- ordered party combat reactions;
- autonomous hostile uses.

`content/<pack>/locales/<locale>/actors.json` only assigns skills to actors.
Most assignments are ids. The object form supplies actor-specific values such
as healing power and narration:

```json
{
  "id": "einar",
  "skills": [
    "strike",
    "hold",
    {
      "id": "heal",
      "power": 5,
      "narration_key": "combat.einar_heal"
    }
  ]
}
```

Runtime ownership is stored in `WorldState.actor_skills`. Starting assignments,
spawned actors, level unlocks, hook grants, and restored saves all feed that
same ownership map.

## Skill Definition

```jsonc
{
  "id": "strike",
  "label": "Strike",
  "kind": "attack",
  "target": "single_enemy",
  "description": "A focused physical attack.",

  "player_action": {
    "id": "attack",
    "command": "ATTACK",
    "effects": ["attack_target"]
  },

  "reaction_priority": 30,
  "reactions": [
    {
      "id": "assist-order",
      "tier": "order",
      "window": "after_hostile_damage",
      "action": "counterattack",
      "conditions": [
        {
          "condition": "order_is",
          "orders": ["follow", "patrol", "guard"]
        }
      ],
      "target": "attacker",
      "cooldown": {
        "mode": "actor_combat_interval"
      },
      "message": "combat.party_counterattack"
    }
  ],

  "autonomous": [
    {
      "id": "hostile-strike",
      "priority": 20,
      "action": "strike",
      "target": "player",
      "rule": {
        "rule": "effect_table"
      }
    }
  ]
}
```

The loader injects the owning skill id into nested player actions and party
reactions, then installs them into the existing generic action and party-policy
indexes. This is a compilation step, not a second authoring surface.

Autonomous uses are read directly from the actor's owned skill definitions.
Lower `priority` values execute first, allowing healing to preempt striking.
`reaction_priority` provides the equivalent ordering across party reactions.

## Layla Skills

| Skill | Player activation | Party reaction | Autonomous use |
|---|---|---|---|
| `strike` | Attack command | Counterattack | Hostile strike |
| `intercept` | None | Guard/follow interception | None |
| `hold` | None | Critical-health hold | None |
| `heal` | None | Heal lowest-health ally | Heal hostile ally |
| `trace` | Trace command | None | None |
| `teleport` | Teleport command | None | None |

The behavior of these skills is not authored in Layla's `actions.json`,
`settings.json`, or `behavior.json`. Those files continue to define non-skill
actions, initial party orders and global settings, and navigation behavior.

## Combat Cadence

Layla uses stat-driven combat cooldowns for autonomous hostile actions and
party reactions:

- physical attacks, counters, guards, and holds use Dexterity;
- healing and other support actions use Intelligence;
- stats 0-2 produce a four-minute cooldown, 3-5 produce three minutes, 6-8
  produce two minutes, and 9-10 produce one minute;
- one game minute is the hard floor, so no actor acts more often than once
  every six real-world seconds while the game is idle.

Dexterity defaults to 6, giving ordinary actors a two-minute cadence (roughly
twelve real-world seconds). Exceptional fast actors can reach the one-minute
floor, while deliberately heavy actors author lower Dexterity values. Packs
without a Dexterity stat retain the legacy `attack_interval_minutes` behavior.

## Generic Runtime Primitives

Centralized definitions use reusable engines rather than skill-specific
dispatch tables:

- `ActionDefinition` handles command parsing, availability, UI, targeting,
  item creation, content events, and command effects.
- `PartyCombatDecisionRule` handles reaction windows, conditions, target
  selection, candidate priority, cooldowns, support effects, and narration.
- `SkillAutonomousUse` handles hostile eligibility, priority, target selection,
  and event selection.
- reducers resolve generic attack, interception, support, hold, trace item
  creation, and teleport events.

Trace and teleport still use specialized generic runtime primitives because
their effects require sigil crafting and anchor navigation. Their complete
player-facing definitions and requirements are nevertheless owned by their
skill records.

## Strict Validation

`"strict": true` makes split authorship invalid. The loader rejects:

- empty or duplicate skill ids;
- unknown or duplicate actor assignments;
- skill-backed entries authored in legacy `actions.json` or party settings;
- hostile strike rules authored in `behavior.json`;
- nested actions or reactions that bind a different skill id;
- incompatible autonomous action and skill kinds;
- combat actors without an autonomous strike skill;
- heal assignments without actor-specific power;
- unresolved level unlocks or hook grants.

This prevents a future skill from being partially implemented across multiple
files.

## Authoring Workflow

To add an actor, assign existing skill ids and provide only actor-specific
parameters.

To add a skill:

1. Add one entry to `skills.json`.
2. Define its player, reaction, and autonomous activation paths there.
3. Assign the skill id to actors that can use it.
4. Use existing generic effects and selectors; add a new runtime primitive only
   when the mechanic itself is new.
5. Add behavior tests against the skill definition and generic execution path.

Do not add a parallel skill action to `actions.json`, a skill reaction to
`settings.json`, or a skill attack rule to `behavior.json`.
