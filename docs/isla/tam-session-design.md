# Isla: the Tam episode, session design (draft)

How Tam's episode plays. Read with `tam-bible.md` (who he is), `next-patient.md` (why him), `game-design.md` (the diner) and `session-design.md` (how Wren's session was built, which this one reuses). Items marked **(decision)** are proposals that need a yes. Nothing here is written as lines yet.

## 1. What the player is doing, and how it differs from Wren

With Wren the player *answers*: each turn offers things to say, and the fit table scores them. Tam does not speak and must not be asked anything. The player *changes the room and waits*. The skill is attunement without words: notice a sign, make one small change, then do less.

| | Wren | Tam |
|---|---|---|
| Player's main verb | Say | **Do**, then **wait** |
| What the child offers | Words, sorted into piles | Behaviors: covers an ear, goes under the table, turns back a page |
| What hurts | Probing, early reassurance | Hurrying, asking, a second sound, a book that names the baby |
| What holds a secret | A fitting answer | **A room state** (and enough quiet beats) |
| The book | Isla picks one of three | **He brings it.** Isla must read it, again |

## 2. How the episode plugs into the game **(decision)**

**One long game, and a world that is never thrown away.** An episode ends with `end_act`. The world carries on into the next act: what Isla holds, where things are, every feature's state and every story variable. Only variables declared with `"scope": "act"` are cleared. (The old act-series rotation, which rebuilt the whole world for each patient, was removed. Nothing used it, and `engine_act_boundary.rs` pins the new behavior.)

What a new episode changes is therefore **authored, not automatic:**

| Thing | How the Tam episode sets it |
|---|---|
| **The date** | `elapsed_minutes` on the time-jump stage moves the clock (days are 1,440 minutes). The card shows the date in Isla's hand. |
| **Where Isla is** | The stage puts her in a different room, such as the staff room or the landing, and not at the front door. *Needs a check:* stages relocate actors with `actor_relocations`, and it is not yet confirmed that this moves the player. If not, a small `move_player` stage effect is needed. |
| **What persists** | Her journal pages and case notes (items), what she learned (story variables meant to carry, declared `game`), and Wren's drawing on the hall wall. |
| **What resets** | Anything declared `act` scope: quiet-beat counters, `asked_him`, feature states Tam's session depends on (set explicitly by the stage, for example lamp back to `off`). |
| **Who is around** | Wren and her family are simply not there. Gloria is. |

Choosing who comes next stays a story trigger for now.

## 3. The room is the instrument **(decision)**

All of these are **features with states and interactions**, the new engine pieces. Most already exist as inspect-only features in the house.

| Feature (room) | States | What Isla can do | What it changes |
|---|---|---|---|
| **Overhead light** (playroom) | on, off | Turn it off | The room is dimmer. Required with the lamp. |
| **Lamp** (playroom, exists) | off, dim, bright | Turn on, dim | Warm light. He does not like bright white. |
| **Blind** (playroom window) | open, shut | Lower it | Less of the street. A small relief. |
| **White-noise machine** (hall, exists) | loud, quiet, off | Turn down, turn off | The hiss. Isla has to leave the room to do it. |
| **Radiator** (hall or playroom) | knocking | None | The knock is a thing that happens. Tam reacts to it. |
| **Low table** (playroom) | empty, Tam under it | None directly | A place he goes to and comes out from. |

Two of them (overhead light, blind) are new. The radiator is an event, not a control: the knock comes at fixed beats and is what makes the ear-cover sign visible.

**One rule makes the room fair:** every state change is quiet, takes one turn, and has one short line. The game never says "that was the right choice."

## 4. His signs and what each asks for **(decision)**

These are the bids. Each is shown as behavior, never as a feeling.

| Sign (what the text shows) | Reading | What fits | What hurts |
|---|---|---|---|
| He stops at the door and looks at the light | The room is not right yet | Lower the overhead light, turn on the lamp | Telling him to come in |
| Covers one ear at the radiator knock | Too loud | Go to the hall, turn the machine down | Asking if he is okay |
| Goes under the table and taps twice | I need a place | Sit on the floor nearby, say nothing, wait | Reaching under, coaxing |
| Lines up the animals, shortest to longest | This is how I settle | Do not touch them. Hand him one that fits, if one is missing | Moving them, praising |
| Puts a book on the floor in front of Isla | Read this | Read it, from the first page | Offering another book |
| Turns back to the same page | Again | Read it again, the same way | Skipping, summarizing |
| Puts the train car in her hand | A little trust | Hold it. Give it back when he reaches | Putting it away, saying thank you too much |

## 5. The fit table **(decision)**

Wren's table asks *what do you say?* Tam's asks *what do you do, and how much?* The scored dimensions:

| Dimension | Fits | Hurts | Why it is honest |
|---|---|---|---|
| **Words** | Fewer. Plain. One sentence at most. | Questions. Any question. | A child who cannot filter sound is hurt by more of it |
| **Pace** | Wait one or two beats after each change | Doing the next thing at once | He cannot be hurried, and a stacked change is a second sound |
| **Changes** | One at a time, in order of the sign | Fixing everything before he has shown anything | Following before leading |
| **The book** | His, read the same way each time | A better one | Sameness is the comfort |
| **Touch and reach** | None, until he reaches | Anything | He decides when |

Every beat has a hidden *attunement* value, as with Wren, but the engine tracks it as story variables read by conditions (`quiet_beats`, `changes_made`, `asked_him`), not as a score on screen.

## 6. The secrets, held by room state **(decision)**

Each opens only if the room is right *and* enough quiet beats have passed. Nothing is told.

1. **The noise.** Isla sees the ear-cover at the knock. She leaves, finds the machine in the hall and turns it down. *Held if:* machine is `quiet` or `off` before the second knock.
2. **The light.** He comes out from under the table. *Held if:* overhead light `off`, lamp `dim`, and two quiet beats.
3. **The book.** He places his book in front of her. *Held if:* he is out and the room is settled. *Lost if:* she has already offered one from the shelf.
4. **The word.** After the second reading he says it, once, to the book. *Held if:* she read it the same way twice and did not ask him anything.

If she asks him a question at any point, `asked_him` is set and he goes back under the table. It is recoverable, and costs time.

## 7. The word **(decision)**

The bible left open whether the word is *Mai* or *again*. Proposal: **both, in order.** *Again* is the first word, when he turns back the page (it is a request and the player can hear it). *Mai* comes only at the top tier, once, after the second reading, to the book. Most players will hear *again*. A few will hear *Mai*, and for them the ending changes. The crib scene stays in the novel.

## 8. The book problem

| Book | Where | What it does |
|---|---|---|
| ***The Little Train Over the Bridge*** | In his backpack | The fit. Read the same way each time. |
| A train book from the shelf | Book room or picture shelf | Shallow. He looks at it and sets it down. |
| ***A New Baby at Our House*** | Book room | The trap. He goes under the table and stays. |

This reuses the book-room shelves, held items and the "pick a book" stage already built for Wren. The difference is that the right book is *his*, and the player must not reach for the shelf first.

## 9. The shape of the episode

A first pass at the stages, mirroring Wren's phases.

1. **Time-jump card.** "Tuesday, February 4." One line. The house has changed a little.
2. **Before.** Isla reads her notes on Tam (short, mismatched: his mother's, the school's, Dr. Bell's margin). Small preparation: she may set the room *before* he arrives, but any change she makes early is a guess.
3. **The waiting room.** Ông brings him. Tam is not in a chair. A short exchange with Ông, who calls her *Cô*.
4. **At the door.** He stops. This is the first sign. Nothing happens until Isla acts on it or waits.
5. **The room** (the middle): signs and answers, secrets one to four, in the order the room allows.
6. **The book.** Reading, twice.
7. **The closing.** He leaves, or does not, depending on what she held.
8. **After.** Ông on the steps. Linh's message later. Isla's note in her journal. A card.

## 10. The ending and its feedback **(decision)**

Three tiers, by how many secrets she held and whether she asked.

| Tier | What happens | Ông on the steps | Linh's message |
|---|---|---|---|
| **Low** | Tam leaves with his backpack on. He does not look at the table. | Nods. Takes his hand. | Polite and short. |
| **Mid** | He leaves the orange car on the table, then takes it back. | Says one thing in Vietnamese she does not catch. | Says he slept. |
| **High** | He leaves the car and does not take it back. He says *Mai* at the door. | Stays a moment with his hand on Isla's sleeve. | Two sentences about the crib, and a question about headphones. |

The practical outcome is small: a quieter corner, headphones at school, a door closed at night. It is not a cure, and the note says so.

## 11. What the engine already does, and what it still needs

| Need | Status |
|---|---|
| Features with a state, per-state descriptions, interactions, the Do panel | **Built** (Phases 0 to 3) |
| Conditions reading feature state, items and story variables | **Built** |
| Dated card between episodes | **Built** (card movies) |
| Held items on shelves, a book to pick | **Built** |
| Feedback tiers from stage `completion_effects` | **Built** (Wren) |
| **A beat that waits and counts quiet turns** | **Gap.** Waiting exists. Counting quiet beats as a gate needs a small effect (`adjust` a story variable on each wait) or a stage signal. |
| **Tam acting on room state** | **Gap.** A child who reacts to the lamp or the knock needs stages or hooks that read feature state. Conditional signals can do it. A scripted sequence for the knock. |
| **A silent child in the transcript** | **Open.** Tam's lines are all behavior. Probably no engine change; the voice guide should say how it reads. |
| **Time-jump stage that carries state** | **Built.** `end_act` keeps the world; only `act`-scoped variables clear. |
| **Moving Isla at the start of an episode** | **Needs a check** (section 2). |

## 12. Build order

1. **This design, agreed.**
2. **Feature states and interactions for the playroom and hall** (lamp, overhead light, blind, machine, radiator), with tests that the states change and read back.
3. **The time-jump card and the Tam stage skeleton:** waiting room, door, room, closing, end, with placeholder lines.
4. **Signs and fit rules,** one at a time, each with a test that the right move holds and the wrong move costs.
5. **The book, the word, the three endings.**
6. **Lines,** reviewed against the voice guide and, for the family, by a reader from the community.
7. **The novel,** written alongside to find what the lines cannot yet say.

## 13. Open questions

1. **Can a stage move the player?** Section 2 depends on it. Check `actor_relocations` with the player, or add `move_player`.
2. **Is *again* then *Mai* right,** or does the second word make the ending too neat?
3. **Does Isla need an early chance to set the room,** or should every change wait for a sign? Early changes reward preparation and risk making her feel she has solved him before he arrives.
4. **How long is a quiet beat?** If a session is a handful of turns and each is mostly waiting, it might feel empty, and might need small things to notice in each.
5. **How do we show a session with no spoken answers** so it reads as full and not as nothing? This is the biggest risk, and the first stage skeleton should test it.
