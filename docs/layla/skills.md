# Skills Architecture

Cinder uses an unranked skills system. A skill is a capability an actor either
owns or does not own.

The architecture has four explicit parts:

1. `skills.json` defines the skill catalog.
2. `ActorDefinition.skills` assigns skills to actors.
3. Actions, party rules, and behaviors bind execution paths to skill ids.
4. `WorldState.actor_skills` stores current ownership.

## Skill Catalog

Each content pack may define `skills.json`:

```jsonc
{
  "strict": true,
  "skills": [
    {
      "id": "strike",
      "label": "Strike",
      "kind": "attack",
      "target": "single_enemy",
      "description": "A focused physical attack against a target in the room."
    }
  ]
}
```

A skill definition contains identity and presentation metadata:

| Field | Purpose |
|---|---|
| `id` | Stable identifier used by assignments and bindings |
| `label` | Player-facing name |
| `kind` | `attack`, `defend`, `heal`, `spell`, or `passive` |
| `target` | Presentation-level target mode |
| `description` | Player-facing explanation |

Requirements, effects, timing, cooldowns, and target selection remain in the
action, party rule, or behavior that invokes the skill. They are not duplicated
in the catalog.

Layla defines:

| Skill | Kind | Capability |
|---|---|---|
| `strike` | Attack | Player attacks, hostile strikes, and counterattacks |
| `intercept` | Defend | Guard and follow interception |
| `hold` | Defend | Critical-health defensive reaction |
| `heal` | Heal | Allied support and hostile healing |
| `trace` | Spell | Chalk sigil tracing |
| `teleport` | Spell | Travel to an armed anchor |

## Actor Assignments

`ActorDefinition.skills` is the canonical assignment surface.

Most skills use a plain id:

```jsonc
{
  "id": "commander_astrid",
  "skills": [
    "strike",
    "hold",
    "intercept"
  ]
}
```

An actor may configure assignment-specific power or narration:

```jsonc
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

Configured assignments let multiple actors share one capability while retaining
actor-specific output. Einar, elf bishops, the elf queen, and Lady Sylvan all
own `heal` with different power and narration.

At runtime, authored assignments seed:

```rust
BTreeMap<String, BTreeSet<String>>
```

The outer key is the actor id. The inner set contains owned skill ids.

Spawned actors receive the skills authored on their template. Restored saves
retain acquired skills and are reconciled with authored starting skills.

## Skill Bindings

Owning a skill authorizes an actor to use execution paths bound to that skill.
Ownership does not replace the path's other conditions.

### Actions

Skill-backed actions declare `skill_id`:

```jsonc
{
  "id": "trace",
  "skill_id": "trace",
  "command": "TRACE",
  "available": {
    "requires_item": "magic-chalk"
  }
}
```

The acting actor must own `trace` and satisfy the action's availability rules.

Layla binds:

| Action | Skill |
|---|---|
| `ATTACK` | `strike` |
| `TRACE` | `trace` |
| `TELEPORT` | `teleport` |

### Party Combat Rules

Each combat rule declares the skill it executes:

```jsonc
{
  "id": "guard-order",
  "skill_id": "intercept",
  "action": "intercept",
  "conditions": [
    {
      "condition": "order_is",
      "orders": ["guard"]
    }
  ]
}
```

The rule owns conditions, targeting, priority, cooldown, and narration. The
skill binding determines whether a candidate actor has the capability.

Layla binds:

| Rule | Skill |
|---|---|
| `survival-hold` | `hold` |
| `guard-order` | `intercept` |
| `follow-protect-player` | `intercept` |
| `healer-support-order` | `heal` |
| `assist-order` | `strike` |

### Hostile Behavior

Hostile strike behavior declares its required skill:

```jsonc
{
  "defaults": {
    "strike_skill_id": "strike",
    "strike": {
      "rule": "effect_table"
    }
  }
}
```

The behavior rule decides when a strike is eligible. The skill binding decides
whether the actor can perform it.

Hostile healing resolves an owned skill whose kind is `heal`, then reads power
and narration from that actor's assignment.

## Granting Skills

### Level Rewards

`LevelDefinition.unlocks` contains skill ids:

```jsonc
{
  "exp_required": 100,
  "stat_changes": {
    "wisdom": 1
  },
  "unlocks": ["heal"]
}
```

Level-up processing inserts each skill into the actor's runtime skill set.
Granting an already-owned skill is idempotent.

### Hooks

Hooks may grant a skill:

```jsonc
{
  "kind": "grant_skill",
  "skill_id": "teleport"
}
```

`actor_id` is optional. When omitted, the configured player actor receives the
skill.

Story variables remain separate from skill ownership. A scroll may grant a
skill and set a story variable when the variable also represents tutorial,
recipe, anchor, or world-progression state.

## Strict Validation

`"strict": true` enables load-time validation for the pack.

Validation requires:

- unique, non-empty skill ids;
- every actor assignment to reference a declared skill;
- no duplicate assignment on one actor;
- action, party-rule, behavior, level-unlock, and hook references to resolve;
- skill-backed actions to declare their skill;
- every party combat rule to declare its skill;
- hostile strike behavior to declare its skill;
- combat actors to own their required strike skill;
- heal assignments to provide actor-specific power;
- hook grants with explicit actor ids to reference declared actors.

In strict packs, runtime execution does not infer ownership from tags, action
enums, actor attackability, or an empty skill list.

## UI Projection

The server projects current party-member skills from
`WorldState.actor_skills`. Labels and kinds resolve through the skill catalog.
The web sidebar renders these as compact badges on party cards.

The UI does not duplicate skill names, kinds, or actor assignments.

## Authoring Workflow

### Add an actor with existing skills

1. Add the actor definition.
2. Assign declared skill ids in `skills`.
3. Use a configured assignment only for actor-specific power or narration.
4. Ensure every bound behavior the actor can execute has a matching skill.

### Add a skill

1. Add the skill to `skills.json`.
2. Bind each invoking action, party rule, or behavior to the skill id.
3. Assign the skill to actors that can use it.
4. Implement the execution behavior.
5. Add progression or hook grants when the skill is learned during play.
6. Add focused catalog, binding, ownership, and behavior tests.
