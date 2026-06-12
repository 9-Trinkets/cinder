# Isla V2 Draft

## Premise
Isla is a mysterious therapist with an unorthodox method. She hosts one patient at a time in a bookstore cafe office, listens to their story, recommends one book from a curated set, and supports them while they read the full book in-session.

The player is Isla.

## Design Pillars
1. Emotional care through story, not clinical diagnosis.
2. Cozy and intimate surface with slightly uncanny undertones.
3. Player agency through meaningful choices:
   1. Book recommendation quality.
   2. Moment-to-moment care responses to patient requests.
4. Endings determined by accumulated care, not one final choice.

## Core Session Loop
1. Intake conversation.
2. Book recommendation menu (pick 1 of 3).
3. Reading phase with intermittent patient requests.
4. Reflection checkpoints at reading milestones.
5. Ending at 100% reading completion.

## Book Recommendation System
At recommendation time, Isla chooses one of three books.

Each session has hidden book fit tiers:
1. Best fit.
2. Middle fit.
3. Worst fit.

### Fit Model
Each patient has hidden need tags and defense tags.
Each book has theme tags and tone tags.
Fit score is computed from overlap and conflict.

### Fit Effect
Selected book fit modifies baseline trajectory:
1. Best fit: easier growth in trust and openness.
2. Middle fit: neutral progression, recoverable with good care.
3. Worst fit: higher resistance, still recoverable with consistently strong care choices.

## Patient Request Gameplay
During reading, the patient can initiate requests that create tactical choices.

### Example Request Types
1. Coffee.
2. Pastry.
3. Water.
4. Quiet/no talking.
5. Brief conversation break.
6. Ambient adjustment (music/light/seating).

### Player Response
When a request appears, present Isla with 2-4 response options.
Each option applies stat deltas and sometimes flags narrative consequences.

## Core Stats
Suggested minimum stats for V1 of Isla V2:
1. `trust` (patient trust in Isla).
2. `openness` (willingness to reflect honestly).
3. `focus` (ability to continue reading).
4. `resistance` (defensiveness, avoidance, pushback).
5. `book_fit` (hidden scalar from recommendation quality).
6. `care_streak` (running measure of good response consistency, optional).

## Reading Progress Beats
Replace clock-driven progression with reading-progress stages:
1. `reading_start` (0%).
2. `first_turn` (25%).
3. `deepening` (50%).
4. `threshold` (75%).
5. `completion` (100%).

Each stage can:
1. Trigger reflective dialogue.
2. Trigger one or more patient requests.
3. Gate which menu options are available.

## Ending Framework
Endings are evaluated from:
1. Book fit quality.
2. Final stat thresholds.
3. Optional failure flags (for repeated poor care).

### Proposed Ending Categories
1. Transformative: patient reaches integration and clear emotional movement.
2. Partial: insight gained, but unresolved core tension remains.
3. Misaligned: session remains intellectually or emotionally mismatched.
4. Rupture: trust breaks down before full integration despite completion.

## Cinder Content Mapping
This design can be implemented using existing Cinder data patterns.

### `settings.json`
1. Keep `channel_surfing_only = true` (player as observer-host role).
2. Keep autonomous NPC dialogue enabled.
3. Tune tick timing slower than Aera if needed for reflective pacing.

### `commands.json`
Add/repurpose authored commands for Isla loop:
1. `listen` (intake depth trigger).
2. `recommend_book` (opens recommendation menu).
3. `serve_request` (menu-driven response action).
4. `check_in` (reflection prompt at milestones).
5. `observe_reading` (non-advancing flavor action, optional).

### `menus.json`
Primary menu families:
1. Recommendation menu (3 books, one choice).
2. Request response menu (options vary by request type).
3. Optional ambiance menu.

### `beats.json`
Model stage transitions via content events + stat/flag conditions:
1. Intake.
2. Recommendation.
3. Reading 25/50/75.
4. Completion + ending resolution stage.

### `hooks.json`
Use hook rules for:
1. Book fit initialization on recommendation choice.
2. Request outcome stat deltas.
3. Reading progress advancement gates.
4. Ending routing based on final state.

### `actors.json`
Initial cast:
1. Isla (player-facing therapist host role).
2. One patient actor per session.

Patient records should include:
1. Hidden needs/defense tags.
2. Request tendencies.
3. Reaction profiles for each progress stage.

### `rooms.json`
Single-room MVP:
1. Isla's bookstore cafe office.

Room features can support service flavor:
1. Espresso station.
2. Pastry case.
3. Reading nook.
4. Ambient controls.

## MVP Scope
1. One room.
2. Three patients.
3. Nine books (three recommendation trios).
4. Five reading progress stages.
5. Four ending types.

## First Implementation Milestones
1. Build base Isla content pack skeleton.
2. Implement recommendation menu and fit assignment.
3. Implement request generation + response menus.
4. Wire stage progression to reading progress events.
5. Add ending resolver and end-session outcomes.

## Open Design Questions
1. Should recommendation happen once only, or allow one mid-session swap with penalties?
2. Should patient requests be fully event-driven, or partially player-triggered?
3. Should some patient archetypes prefer fewer interventions and reward restraint?
4. Should endings include explicit "book mismatch but care success" variant text?
