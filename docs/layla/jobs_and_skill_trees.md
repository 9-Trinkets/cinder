# Jobs & Skill Trees — Design Proposal

Living design doc for a **job + skill progression system** in Layla. Companion to
the floor plans: the floors decide *when* things unlock, this doc decides *what
they are* and *how they are modeled*.

> Status: **design only — no code landed.** Decisions recorded below are the
> starting contract for a future implementation pass.

---

## 1. Motivation

Layla's actors can currently do three unrelated things — **attack**, **defend
(hold)**, and **heal** — and Layla herself has a set of unique actions (tracing
sigils, teleporting, reading scrolls). None of it is organized. There is no
concept of a **job**, no prerequisites, and no tree.

Today every ability is gated by a flat **story-var boolean**:

| Gate | Where | Set by |
|---|---|---|
| `knows_teleport` | `teleport` action availability | `item.teleport_scroll_read` |
| `knows_drain` / `knows_spawn` | `trace` `craftable_item_gates` | `item.scroll_read` / `item.spawn_scroll_read` |
| `first_mob_defeated` | `vitals_sidebar_story_var` | `actor.defeated` |
| `shaman_defeated` | `minimap_requires_story_var` | `actor.defeated` |
| `has_sensory_enhancer` | map entity tracking | `item.sensory_enhancer_used` |

Story vars are excellent for **diegetic unlocks** — the floor plan's "Feature
Surface Order" is built entirely on them, and that design is working. They are
poor at expressing **progression**: there is no way to say *"requires two other
skills"*, *"scales with level"*, or *"belongs to the Handler track"*.

The proposal keeps story vars for what they are good at and adds a **skill layer**
for progression.

---

## 2. What Already Exists (the parts we build on)

Two pieces of the target architecture are already present, one of them dormant.

### `LevelDefinition.unlocks` — declared but never read

`content/layla/levels.json` has a flat 5-step table; index `n` is the XP to
advance from level `n+1` → `n+2`. Each entry carries `stat_changes` and an
`unlocks: Vec<String>` list.

`LevelDefinition.unlocks` (`cinder-core/src/content/types/leveling.rs:18`) has a
doc comment that reads:

> Identifiers of skills/spells granted by reaching this level. **Not yet consumed
> by the engine**; declared so the schema is stable for when abilities land.

Every `unlocks` array in Layla is currently `[]`. The level-up loop
(`reducer/combat/defeat.rs:83-100`) applies `stat_changes` and narrates, but never
reads `unlocks`. **This is the intended hook and it is already reserved.**

`LevelingDefinition.actors` also already provides **per-actor level tables**,
which is the job-differentiation primitive we need.

### `healing` — a first-class actor capability

`ActorDefinition.healing: Option<ActorHealingSpec>` gates the hostile-heal
priority in `hostile_actions.rs:33-46`: an actor with `healing` heals a wounded
ally (or itself below 50%) instead of striking. Five actors have it
(`einar`, `elf-bishop-3`, `elf-queen-4`, `elf-bishop-6`, `lady_sylvan`).

`behavior.json` supplies the other two capabilities — `strike` and `hold` — as
neuron `effect_table` rules per actor, with pack-wide `defaults` and per-actor
overrides.

So the **three existing capabilities already live in three unrelated places**:
`behavior.json` (strike/hold), `ActorDefinition.healing` (heal), `actions.json`
(player-only). That inconsistency is the core problem this refactor addresses.

---

## 3. Bugs Found While Surveying (fix these regardless)

All three are the same class of defect: **a struct that doesn't
`deny_unknown_fields` silently discards content instead of erroring**, and the
content linter (`cinder-tools/src/lint/checker.rs`) never validates actor
capability fields. Each was found by probing the loaded pack, not by reading it.

### 3.1 `lady_sylvan` heals for 0 HP

Her `healing` block uses `{heal_amount, cooldown_turns, priority_threshold_percent}`
but `ActorHealingSpec` expects `{amount, message}`. Verified by loading the pack:

```
lady_sylvan healing = amount:0 message:
einar      healing = amount:5 message:combat.einar_heal
```

She is a floor-5 boss who heals her allies for nothing.

### 3.2 All three floor-5 bosses are level 1, not level 12

`lady_sylvan`, `lord_vane`, and `warmaster_torin` declare **`initial_level: 12`**.
`ActorDefinition` has no such field — the real field is `level`. Verified:

```
lady_sylvan    def.level=1
lord_vane      def.level=1
warmaster_torin def.level=1
elf-king-5     def.level=8
```

The three floor-5 bosses are seeded at level 1 while their JSON claims 12. This
matters doubly for this design: job/skill eligibility is level-gated, so the bug
would also suppress their skills.

### 3.3 `unlocks` absorbs typos

Because `unlocks` is unconsumed, a misspelled skill id fails silently. Any skill
system must add lint coverage for exactly this.

### Recommendation

Add `#[serde(deny_unknown_fields)]` to `ActorDefinition`,
`ActorHealingSpec`, and `LevelDefinition`, then fix the content. That converts
all three silent failures into loud load errors. (Note `ActionItemCreation`
already does this — precedent exists.)

---

## 4. Decisions (agreed)

| Question | Decision |
|---|---|
| Where do skills live? | **First-class `skills.json`** + engine support. Not story vars, not items. |
| How do actors get jobs? | **Fixed on actor definitions.** Including the player. |
| Scope of first pass? | **This doc only.** Code in a later pass, once the shape is settled. |

---

## 5. Proposed Model

### 5.1 Jobs live on the actor

A new optional `job` field on `ActorDefinition`:

```jsonc
// actors.json
{
  "id": "player",
  "job": "handler",          // ← new
  "level": 1,
  "initial_stats": { ... }
}
```

Jobs are **content-authored** in a new `jobs.json`, so a pack defines its own
vocabulary. Nothing in the engine hardcodes "handler" or "healer".

**Job identity is fixed per actor.** Layla is a handler; `einar` is a warden;
`elf-bishop-3` is a cleric. Changing jobs is not a runtime mechanic.

### 5.2 `skills.json`

```jsonc
// skills.json
{
  "jobs": {
    "handler":   { "label": "Handler",        "description": "..." },
    "warden":    { "label": "Warden",         "description": "..." },
    "cleric":    { "label": "Cleric",         "description": "..." },
    "brawler":   { "label": "Brawler",        "description": "..." },
    "adjutant":  { "label": "Chalk Adjutant", "description": "..." }
  },

  "skills": [
    {
      "id": "trace_charm_sigil",
      "job": "handler",
      "tier": 0,                          // tier 0 = innate, granted at spawn
      "label": "Chalk Ring",
      "description": "Close a ring of chalk around an enemy to convert it.",
      "grants": { "action": "trace" },    // what the skill actually unlocks
      "requires": [],                     // prerequisite skill ids
    },
    {
      "id": "drain_sigil",
      "job": "handler",
      "tier": 1,
      "label": "Drain Sigil",
      "requires": ["trace_charm_sigil"],
      "requires_level": 3,
      "requires_story_var": "knows_drain", // diegetic gate still respected
      "grants": { "craftable_item": "drain-sigil" }
    },
    {
      "id": "teleport_anchor",
      "job": "handler",
      "tier": 2,
      "label": "Teleport Anchor",
      "requires": ["trace_charm_sigil"],
      "requires_level": 5,
      "grants": { "action": "teleport" }
    }
  ]
}
```

**Key design point: `grants` describes the *effect* of a skill, and every effect
kind maps onto a mechanism that already exists.**

| `grants` key | Maps to | Already exists? |
|---|---|---|
| `action` | enables an `actions.json` entry | ✅ `player_enabled` / `available` |
| `craftable_item` | adds to `craftable_items` | ✅ `item_creation.craftable_items` |
| `healing` | sets `ActorHealingSpec` | ✅ `ActorDefinition.healing` |
| `behavior_rule` | sets `behavior.json` `strike`/`hold` | ✅ `BehaviorActorDefinition` |
| `stat_bonus` | adds a flat `stat_changes` delta | ✅ `LevelDefinition.stat_changes` |
| `story_var` | sets a var on grant | ✅ `set_story_var` hook effect |
| `passive` | a named modifier read by Rust | 🆕 *the one genuinely new mechanism* |

`passive` is deliberately last and deliberately narrow. Resist the urge to make
skills a general scripting language — the floor plans already work, and every
new effect kind is engine surface we have to maintain.

### 5.3 Precedence rules

A skill is **granted** when *all* of these hold:

1. The actor's `job` matches the skill's `job`.
2. The actor's level ≥ `requires_level` (default 1).
3. Every id in `requires` is already granted.
4. `requires_story_var` is truthy, if present.
5. The tier is reachable — see below.

**Story vars remain a valid gate, not a replacement.** `drain_sigil` above needs
both a level and `knows_drain`, because the fiction is that Layla *reads the
scroll* — she doesn't simply level into understanding spiral sigils. This is the
"diegetic unlock" discipline from the floor plans, preserved.

### 5.4 Tiers vs. auto-grant

Two options, and this is the main open question for the implementation pass:

- **Auto-grant on level-up.** Reaching the level grants every skill whose
  requirements are met. Simple, zero player agency, matches how `levels.json`
  already works.
- **Explicit spend.** Skills land as *available* and the player commits XP or a
  token. Real tree feel, but needs a spend mechanic, an XP sink, and a UI.

Recommendation: **auto-grant for tier 0–1, explicit spend for tier 2+**. Innate
and basic skills just happen; capstone abilities are earned deliberately. Keep
the tree feeling like progression without building a full character sheet.

### 5.5 How the three capabilities unify

The scattered systems converge on jobs:

- `strike` / `hold` — become `behavior_rule` grants. A `brawler`'s `strike` rule
  differs from a `cleric`'s, and both come from the job's skill list.
- `healing` — becomes a skill grant. `einar`'s heal is the `cleric` job's
  `field_mending`; `elf-bishop-3` gets it too via the same job.
- player actions — become `action` / `craftable_item` grants.

Nothing is deleted in this refactor. `ActorDefinition.healing` and
`behavior.json` stay as the **defaults**, and skills become the override layer.
A pack that ships no `skills.json` behaves exactly as it does today. That
backwards-compatibility property is the main reason to attempt this at all.

---

## 6. Jobs for Layla's Cast (proposed)

Derived from existing capabilities and fiction, not invented wholesale.

| Job | Members (existing) | Tier 1 | Tier 2 |
|---|---|---|---|
| `handler` | `player` | chalk ring, drain sigil | teleport, spawn |
| `warden` | `einar` | guard intercept, steady aim | field mending (heal) |
| `cleric` | `elf-bishop-3`, `elf-bishop-6`, `elf-queen-4` | field mending (heal) | consecrate (ally defense) |
| `brawler` | `goblin-1..4`, `elf-knight-2`, `elf-knight-7`, `elf-rook-1`, `elf-rook-8` | cleave | flanking strike |
| `adjutant` | golems, `elf-pawn-1..8`, `zayd` | shield wall | intercept |
| `sovereign` | `elf-king-5`, `goblin-shaman`, `lord_vane`, `warmaster_torin`, `lady_sylvan` | — | boss-tier passives |

Notes:

- **`zayd` as an `adjutant`** is speculative — he's a rescued child, not a
  fighter. Worth a design conversation before committing.
- **`lady_sylvan` / `lord_vane` / `warmaster_torin`** currently have no working
  combat behavior configured (they're `house_head` bosses with
  `initial_level: 12`). Their job rows are placeholders pending a real pass.
- The `brawler` / `adjutant` split is the sharpest open question: the floor plans
  describe elf pawns as "the forward military screen" with fast cadence, which
  reads as `adjutant`, while knights and rooks read as `brawler`. Confirm before
  encoding.

---

## 7. Implementation Sketch (for the next pass)

Ordered smallest-blast-radius-first.

### Phase 1 — schema, no behavior change

1. Add `JobDefinition` and `SkillDefinition` to
   `cinder-core/src/content/types/leveling.rs` (or a sibling `skills.rs`).
2. Add `SkillsDefinition { jobs, skills }` and a `skills` field on `ContentPack`.
3. Load `skills.json` in `content/loader/mod.rs` alongside `levels.json`
   (`read_optional_json`, so absence = empty, not an error).
4. Add `job: String` to `ActorDefinition`.
5. Add a `skill_index: HashMap<String, usize>` to `ContentPack`.
6. Scaffold `skills.json` in `cinder-tools/src/scaffold/pack.rs`.

Nothing consumes any of it yet. Packs without `skills.json` are unaffected.

### Phase 2 — validation

1. Extend `cinder-tools/src/lint/checker.rs`:
   - every actor's `job` resolves to a declared job (error if not)
   - every skill's `grants.action` / `craftable_item` resolves (warning)
   - every id in `requires` resolves (error — kills the typo class from §3.3)
   - no `requires` cycles
   - every skill's `requires_level` is reachable in `levels.json` (warning)
2. Add `#[serde(deny_unknown_fields)]` to `ActorDefinition`,
   `ActorHealingSpec`, `LevelDefinition` and fix the resulting content errors
   from §3.

### Phase 3 — granting

1. `WorldState.skills: BTreeSet<String>` (or `BTreeMap<String, u32>` if tiers
   track investment).
2. `ContentPack::actor_skills(&self, state, actor_id) -> Vec<&SkillDefinition>` —
   the single resolution function.
3. In the level-up loop (`reducer/combat/defeat.rs`), after applying
   `stat_changes`, grant newly-satisfied skills and narrate each.
4. Seed tier-0 skills at world creation (`state/seeding.rs`).

### Phase 4 — effect application

1. `action` grants → filter `actions.json` entries in the availability check.
2. `craftable_item` grants → merge into `craftable_item_gates` resolution.
3. `behavior_rule` grants → overlay onto `resolved_behavior`
   (`engine/behavior.rs`).
4. `healing` grants → overlay onto `ActorDefinition.healing`.
5. `stat_bonus` grants → merge into `effective_actor_stat`.

### Phase 5 — UI

1. New `PanelDataSource::Skills` for a "Skills" panel, listing granted and
   next-tier-locked skills.
2. `UiSnapshot` gains `skills: Vec<SkillDisplay>`, `job: String`.
3. `StatusPanel.tsx` gains a Job section (and folds in the existing Level block).

---

## 8. Open Questions

1. **Auto-grant vs. explicit spend** (§5.4). Recommendation: hybrid by tier.
2. **Does Layla's job stay `handler` for the whole game?** Her memory loss is
   central to the premise — does the job reflect what she *is* or what she's
   *recovering*? A mid-game job change would be a strong story beat and needs
   engine support the "fixed" decision currently excludes.
3. **Is `zayd` really an `adjutant`** (§6)? He's a child.
4. **Do jobs gate *content* or only *capabilities*?** E.g. should the frost-citadel
   dialogue refuse a non-adjutant? Probably not, but worth deciding explicitly
   rather than by accident.
5. **What happens to story-var gates when both exist?** Recommended: keep both,
   requiring all conditions (§5.3). Confirm no existing gate should be *replaced*
   rather than supplemented.
6. **`passive` scope.** The one new mechanism. Try hard to express the first
   several tiers without it.

---

## 9. Non-Goals

- **Not** a replacement for the floor plans' diegetic unlock beats. Story vars
  stay; the Handler's reward framing is orthogonal and unaffected.
- **Not** a respec/spend system in phase 1. See §5.4.
- **Not** multiplayer-safe skill trading or shared XP pools.
- **Not** per-turn or per-sigil charge systems. Sigil charges already live on
  items (`chalk` capacity, drain-sigil limits) and stay there.