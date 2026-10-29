# Engine Requirements & Architecture: AERA

> *"What cinder-core and the content pack need in order to run a card-driven reality TV producer simulation."*

---

## 1. Overview of Necessary Capabilities

To support *Aera*'s revamped producer gameplay without overcomplicating the engine, we identify five primary technical capabilities:

```
┌────────────────────────────────────────────────────────┐
│                   CINDER CORE ENGINE                   │
├────────────────────────────┬───────────────────────────┤
│ 1. Card Deck & Hand Menu   │ 2. Actor Nudge Injections │
│    • Random card drawing   │    • Dynamic prompt cues  │
│    • Category filtering    │    • Temporary directives │
├────────────────────────────┼───────────────────────────┤
│ 3. Confessional Interview  │ 4. Episode Clip Assembly  │
│    • 1-on-1 Producer room  │    • Tagging day moments  │
│    • Direct Q&A dialogue   │    • Ratings computation  │
├────────────────────────────┴───────────────────────────┤
│ 5. Stat Threshold Triggers (Safety / Breakdown limits) │
└────────────────────────────────────────────────────────┘
```

---

## 2. Detailed Technical Requirements

### 1. Card Deck & Hand Drafting Menu
* **Current State in Cinder**: Menus in [`menus.json`](file:///Users/li-hsuanlung/Projects/cinder/content/aera/locales/en/menus.json) are static trees of choices.
* **Requirement**:
  - Support a **card drafting menu** that can draw $K$ random items from a defined card pool (e.g. draw 3 from the 8 Event Cards).
  - Alternatively, if implemented as static menus, author daily choice matrices or scripted card draws in [`menus.json`](file:///Users/li-hsuanlung/Projects/cinder/content/aera/locales/en/menus.json) that map directly to beat triggers.
  - Playing a card sets game state flags (e.g., `active_event: "grocery-run"`, `assigned_pair: ["ren", "mio"]`).

### 2. Targeted Nudges (Dynamic Prompt Injection for LLM Dialogue)
* **Current State in Cinder**: Autonomous dialogue uses static `prompt_context` in [`actors.json`](file:///Users/li-hsuanlung/Projects/cinder/content/aera/locales/en/actors.json) and room descriptions.
* **Requirement**:
  - When a **Relationship Card** is played (e.g., *"The Vulnerability Push: Ask about their ex"*), this directive needs to be temporarily injected into the actor's conversational context for their next autonomous exchange.
  - *Engine hook*: An `active_nudge` field on the actor instance, which gets appended to the LLM system prompt during autonomous dialogue generation, then cleared once triggered.

### 3. The Confessional Booth Interface
* **Current State in Cinder**: The player has `player_enabled: false` on `speak` across all rooms in Aera.
* **Requirement**:
  - Add a dedicated room: `confession-booth` (or `diary-room`).
  - In this room, the player interacts with the invited actor either through:
    - **A multiple-choice interview menu** (The Empathic Ear, The Shit-Stirrer, The Reality Check).
    - Or **direct player speech** (`player_enabled: true` specifically when in `confession-booth`).
  - The actor answers privately. The resulting answer is tagged as `daily_soundbite` in the game state.

### 4. Episode Clip Tagging & Assembly (The Edit Suite)
* **Requirement**:
  - As beats trigger throughout Day 1 and Day 2, the engine records 1-sentence summaries of key scenes into a `daily_footage_reel`:
    - E.g., `clip_kitchen_burn`: *"Ren coldly corrects Mio's knife work while chopping scallions."* (Tags: `friction`, `ren`, `mio`).
    - E.g., `clip_patio_wine`: *"Daichi admits to Aera that he doesn't know what he's doing with his life."* (Tags: `vulnerability`, `daichi`, `aera`).
  - At the end of each day, in the `studio` (edit bay), the player is presented with a selection menu:
    - *“Select 3 clips for tonight's broadcast.”*
    - *“Select 1 Confessional Soundbite.”*
  - The engine tallies the tags to compute:
    $$\text{Episode Drama} = \sum \text{friction clips}$$
    $$\text{Episode Romance} = \sum \text{attraction clips}$$
    $$\text{Episode Chemistry} = \sum \text{vulnerability clips}$$
  - Ratings output is displayed as a post-broadcast summary report.

### 5. Stat Threshold Triggers
* **Current State in Cinder**: Stats exist in [`stats.json`](file:///Users/li-hsuanlung/Projects/cinder/content/aera/stats.json) with clamp ranges, but don't automatically trigger behavioral emergencies.
* **Requirement**:
  - Add boundary triggers:
    - **Emotional Shutdown**: If any actor's `safety < -5`, they refuse to converse in shared rooms and retreat to their bedroom until consoled.
    - **Walkout Alert**: If an actor's `safety < -8`, a red alert sounds in the control room: the actor is packing their bags. The producer must use an emergency confessional or lose the actor.

---

## 3. Implementation Phasing Strategy

We can build this in two clean stages to avoid breaking existing packs:

### Stage 1: Content Pack Rebuild (Within Existing Cinder Core)
* Build the Day 1 and Day 2 beats using **scripted card menus** (drawing cards via pre-authored conditional choices in `menus.json`).
* Author the 8 Event Cards and 8 Relationship Cards as beat triggers and dialogue states.
* Add the `confession-booth` room and script the interview choice trees.
* Create the End-of-Day Edit menu where player selects clips to see their Ratings scorecard.

### Stage 2: Engine Generalization (Rust `cinder-core`)
* Enhance the autonomous dialogue generator in `cinder-core` to natively support dynamic prompt injection (`actor.add_nudge(...)`).
* Add native deck-shuffling primitives to the Cinder menu script engine so cards can be drawn truly at random from a JSON deck definition.
