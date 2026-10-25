# Interactive objects in rooms: design (draft)

How the engine should model things in a room that the player or an NPC can operate and that keep a state afterwards: a lamp, a valve, a gate, a desk, a projector. Written for every pack, not for one. Isla's second episode (a lamp, a blind and a hall machine that Tam reacts to) is the first user, and the survey below shows Layla, Aera and Ella need the same thing. Items marked **(decision)** need a call.

## 1. What we found

There is no first-class interactive object today. Every case in the four packs is built from the same three pieces:

1. **A room-bound action** gated by `allowed_rooms`, because actions cannot target a feature.
2. **A hook that sets a story variable.**
3. **A read-side check that reads that variable:** an exit gate, a description override, a topic gate, a teleport arming rule.

| Case | Where | How it is done now |
|---|---|---|
| Steam valve opens the fortress gate | Layla `turn_steam_valve`, hook `valve.diverted` | Action in `village_west_1`, sets `fortress_gate_open`. The valve feature is inspect-only. The action has no `blocked_by_story_var`, so it can be repeated. |
| Gate, cage, courtyard cages, safe unlocks | Layla `unlock_*` (4 actions, 4 hooks) | The same block copied four times: key item, room, content event, hook, story var. `fortress_gate_open` is written by two different actions. |
| Boss-gated exits and rooms | Layla `descriptions[]`, exit `requires_story_var` | Generic engine features, but each reads a different kind of condition. |
| Cooking, brewing, plating | Aera (9 actions) | `allowed_rooms: ["kitchen"]` stands in for the kitchen island feature. State is objective-progress keys plus a loose room item. |
| Editing desk, watching clips | Aera `edit`, `watch` | Items created at a feature's stock. |
| Projector | Ella | A menu sets a story variable, and a stage reads it. The projector feature is inspect-only. |
| Sofa, beds | `allow_rest` | Stateless stat bump, for NPCs only. |

### Redundancy: the same idea in several places

- **Story-variable gates in at least eight independent places:** action availability, exits, room descriptions, topics, teleport arming, craftable gates, skill gates and stage conditions. They share one truthiness helper, and each has its own loader and validator.
- **Three stores for "has this happened":** a story variable, an objective-progress key, and an item's presence in a room.
- **Four ways to set a variable:** a hook effect, a stage effect, an item-creation field and a menu selection field.
- **Two effect lists** that overlap: stage effects (`AdvanceEffect`) and hook effects (`WorldHookEffect`).
- **Five hook entry points** (`use_hook`, `placement_hook`, `equip_hook`, an action's `hook_id` and a content event's hook) that all end in the same rule table.
- **Topics (just built) and typed `unlock_*` actions** are both "an authored thing the player chooses to do", built twice.

### Gaps: what cannot be said without engine code

- A feature has **no state** and no per-state description.
- Actions **cannot target a feature.**
- No effect **places or removes** a room item or toggles an exit directly. Everything goes through a story variable.
- No **toggle or counted state** (a lamp, a three-position lever), and no equality or numeric test in availability.
- NPCs **cannot target a feature.**

## 2. The proposal in one idea

> **A feature is an object.** It can have a state. It can offer **interactions**, authored choices the player makes about it. A single **condition** language gates everything that depends on state.

Three pieces, each replacing several legacy ones.

### 2.1 One condition language

The stage system already has one: `AdvanceCondition { path, operator, value }`, with `equal`, `not_equal`, `gte`, `lte`, `greater_than`, `less_than`, `contains`, `array_contains` and more, evaluated over a JSON view of the world (story variables, actor stats, inventory, active stages).

Make that the **only** gate. Anything that is gated today gets a `when` list of conditions:
- action availability, exits, room descriptions, topics, menu options, craftable gates, skill gates, teleport arming.

The world view gains `features.<room>.<feature>.state` and a few helpers for "item is in this room" and "item is on this feature". The old fields (`requires_story_var`, `blocked_by_story_var`, `requires_room_item`, `armed_by`, and so on) become sugar that the loader rewrites into conditions, and then are removed from the content.

### 2.2 A feature with a state

```json
{ "id": "lamp", "label": "reading lamp",
  "states": ["off", "dim", "bright"], "state": "bright",
  "descriptions": [
    { "when": [{"path": "self.state", "operator": "equal", "value": "off"}],  "inspect_text": "The lamp is off." },
    { "when": [{"path": "self.state", "operator": "equal", "value": "dim"}],  "inspect_text": "The bulb glows low and amber." }
  ],
  "inspect_text": "A tall lamp with a paper shade." }
```

- **State is a named value from a declared list,** stored per feature (keyed `room::feature`, like feature items) and saved with the game. Declaring the list lets the loader catch a typo.
- **A feature can have several states over time but one at once.** A second axis (the lamp and the blind) is a second feature.
- **A feature's description can change with state,** like room descriptions already do.
- **The loader checks** every `when` that reads a feature's state names a state that exists.

### 2.3 Interactions: one thing for topics, unlocks and operating objects

An **interaction** is an authored choice with a label, optional conditions, lines it plays, effects it applies, a signal it fires, and whether it can be repeated. It belongs to an owner, either an **actor** (it appears in the Talk panel) or a **feature** (it appears in a new panel for doing things).

```json
{ "id": "dim", "label": "Dim the lamp",
  "when": [{"path": "self.state", "operator": "equal", "value": "bright"}],
  "lines": ["You turn the dial until the glow is low and amber."],
  "effects": [{"kind": "set_feature_state", "state": "dim"}],
  "repeatable": true }
```

- **Topics are interactions on an actor.** `ActorTopicDefinition` becomes `Interaction`, and the `topic_selected:<actor>:<topic>` signal becomes one signal for both. This replaces what I built two weeks ago, and it is better to do now, before more content is written against it.
- **Layla's unlocks become interactions on features:** a cage, a gate and a safe each offer "Unlock", gated by the key item (`when`) and by not already being open. The four copied blocks become four short entries.
- **The valve becomes an interaction on the valve feature,** and can no longer be repeated by accident.
- **Aera's kitchen actions** can later become interactions on the kitchen island, which lets NPCs act on a feature too (Phase 4).

Interactions apply **effects.** Instead of a second effect type, they use the same effect list as hooks (`set_story_var`, `acquire_item`, `adjust_actor_stat`, `narrate_message`, `move_actors`, ...), plus a few new ones that remove today's workaround through a story variable:
- `set_feature_state`
- `place_item` and `remove_item` (in a room or on a feature)
- `set_exit_open` is not needed: an exit just reads a condition.

Stage effects (`AdvanceEffect`) and hook effects (`WorldHookEffect`) become **one `Effect`** list. This is the largest replacement and it can wait (Phase 5).

## 3. How each legacy piece is replaced

| Today | Becomes | Phase |
|---|---|---|
| `ActionAvailability.requires_story_var`, `blocked_by_story_var` | `when` conditions | 1 |
| Exit `requires_story_var` | exit `when` | 1 |
| Room `descriptions[].requires_story_var` and `actor_defeated` | `when` (a condition on the actor's defeat) | 1 |
| Topic `requires_story_var` | interaction `when` | 2 |
| `armed_by` on teleport anchors, `craftable_item_gates`, skill `requires_story_var` | `when` | 1 |
| Topics | interactions on an actor | 2 |
| Layla `unlock_*`, `turn_steam_valve` | interactions on features | 3 |
| Per-feature inspect text | `descriptions` by state | 2 |
| `requires_room_item`, `requires_room_without_item` | a `when` over the room's items | 1 |
| Story variable as the only state | feature state, with story variables kept for global flags | 2 |
| Aera kitchen and studio actions, `allowed_rooms` as a stand-in | interactions on the kitchen island and desk | 4 |
| Two effect enums | one `Effect` | 5 |
| Five hook entry points | one | not planned |

Each step keeps the pack working: the loader accepts the old field and the new one together for one release, content is migrated in the same commit that removes the old field, and each migrated pack gets a before-and-after test.

## 4. The player's side

- **A "Do" panel** on the action bar lists the interactions available on the features in the room, grouped by feature, the way the Talk panel lists topics by actor. Its label comes from the pack's UI text ("Do", "Use", "Adjust"), like Talk and Take.
- **Typing works too:** `use lamp`, or the interaction's label.
- **Look reflects state:** inspecting the lamp reads the description for its current state.

## 5. Tam's playroom, as the first user

| Feature | States | Interactions | Read by |
|---|---|---|---|
| Reading lamp | off, dim, bright | Dim it, brighten it, turn it off | Tam's scenes (he comes out when the room is dim), the description |
| Overhead light | on, off | Turn it off | Same |
| White-noise machine in the hall | loud, quiet, off | Turn it down, turn it off (Isla must go to the hall) | Tam's reaction to the radiator, and a secret |
| Window blind | open, shut | Lower it | The description, and the light |
| Radiator | knocking | None. It is something Tam reacts to. | Tam's covering of an ear |

Everything Tam does reads these states through `when` conditions on stages, menu options and interactions, with no new engine concept beyond the three in section 2.

## 6. Phases

| Phase | Adds | Replaces | Proves it on |
|---|---|---|---|
| 0 | One condition evaluator, and one truthiness helper | The two copies of `story_var_is_truthy` | Existing tests |
| 1 | `when` on exits, descriptions, actions, topics and the rest, with the old fields rewritten as sugar | Eight separate gates | Layla and Aera, with identical behavior |
| 2 | Feature state, per-state descriptions, interactions on features, the Do panel. Topics become interactions on actors. | Topics, inspect-only features | **Isla's Tam playroom, and Wren's topics** |
| 3 | Layla's unlocks and the valve as interactions | Four copied unlock blocks | Layla gate, cages, safe, valve |
| 4 | NPCs target features | Aera's `allowed_rooms` stand-in | Aera kitchen |
| 5 | One effect list | Stage effects and hook effects | Everything |

**For Tam, Phases 0 to 2 are enough.** Phase 3 comes next because it removes the most copy-paste, and Phases 4 and 5 are larger and can wait.

## 7. Risks and open questions

1. **Naming.** "Interaction" is neutral. Alternatives are "affordance" (precise, awkward) and "action" (taken). **(decision)**
2. **Replacing topics now** means rewriting Isla's three authored topics and their tests, and one signal name. It is cheap today and gets more expensive with each new episode.
3. **Layla's behavior must not change.** Phase 1's sugar and Phase 3's migration each need before-and-after tests that replay the existing gate and unlock flows.
4. **State as a declared enum or free text?** Declared is safer and can be validated. It costs one line per feature. **(decision)**
5. **Localization.** Interaction labels and per-state descriptions are player text and live in each locale's files, like everything else.
6. **Saved games.** Feature state is new data with a default, so old saves load. Story variables stay as they are.
7. **What NPCs can do with features** (Phase 4) is the least settled part, because Aera's NPC turns pick commands from a prompted list.
