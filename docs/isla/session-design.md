# Isla: single-session design (draft)

Status: proposal for a rebuilt, minimal Isla pack. Nothing here is implemented. Not playtested.

## 1. Fantasy and goal

The player is Isla, a bibliotherapist. The skill being played is **attunement**: notice what Noa shows, respond at a depth she can take right now, and give back something she actually said.

Success is not "Noa gets fixed". Success is "Noa felt seen". The player never sees a score during play. They find out how it went from Noa's review at the end.

## 2. Session flow

| # | Beat | Player does | Output |
|---|------|-------------|--------|
| 1 | Arrival and greeting | short, 1-2 exchanges | first clues (umbrella, seat, phone) |
| 2 | Intake | ~5 exchanges, listen for handles | 2-3 **handles** in the notes panel |
| 3 | Serve | one beat, one choice | what she accepts or refuses is a clue |
| 4 | Pick a book | choose between 2 books | fit with a handle (one fits, one near miss) |
| 5 | Reading and stops | read aloud spread by spread, stop to talk | the loop below, 3-4 stops |
| 6 | Closing | return one of her own lines | the **final piece** |
| 7 | Leaving, then review | none | Noa's review with rating |

Target length: 20-30 minutes. Next session is an open line at the end of the review, not built yet.

## 3. State model (three variables)

| Variable | RPG analogue | Range | Meaning |
|---|---|---|---|
| **Safety** | HP | 0-10 | how safe Noa feels in the room; changes slowly |
| **Absorption** | MP/SP | 0-3 | how pulled into the story she is; built by reading, spent by stopping |
| **Depth** | stance/position | 0-2 | where the conversation sits now: 0 the book, 1 a pattern, 2 herself |

Possible mapping onto existing Isla stats: Safety = `trust`, Absorption = `focus`, Depth = `openness` (read as thresholds). `resistance` would be dropped.

**Ceiling.** The deepest layer she will go to is set by Safety plus Absorption:

| Safety + Absorption | Deepest layer |
|---|---|
| 0-2 | 0 (book) |
| 3-5 | 1 (pattern) |
| 6+ | 2 (herself) |
| 9+ and Depth 2 | the core line becomes available |

**Absorption.** +1 per spread read without stopping, capped at 3. Spent to 0 when a stop begins. This is the bibliotherapy mechanism: a story temporarily lowers defenses.

**Flooding (optional).** Each spread has intensity 1-3. If Absorption + intensity exceeds Safety + 3, she shuts down: Safety -1, Depth to 0.

## 4. Move types

The menu generator produces **one option per move type**. The type fixes the depth the move asks for. The LLM only writes the wording, so outcomes stay testable.

| Move | Asks for depth | Notes |
|---|---|---|
| **Reflect** | current Depth | mirrors what she just said |
| **Question** | current Depth | open question on the same layer |
| **Link** | Depth + 1 | built from one **clue id**; the clue is real or a decoy |
| **Bridge** | 2 | "does it remind you of anyone?" / of herself |
| **Wait** | none | silence |

Options are never labeled good or bad. A Link built from a decoy clue is a misread.

## 5. Resolution rules

Let `ask` be the move's depth and `reach = Safety + Absorption`.

| Case | Result |
|---|---|
| `ask <= Depth`, accurate | Safety +1 |
| `ask <= Depth`, generic | nothing (polite answer) |
| `ask = Depth + 1`, `reach` clears the ceiling, accurate | Depth +1, Safety +1 (she steps in) |
| `ask = Depth + 1`, `reach` too low | Depth -1, Safety -1 (she retreats) |
| `ask >= Depth + 2` | Depth to 0, Safety -2 |
| Link from a decoy clue | Safety -1, she corrects you (the correction becomes a real clue) |
| **Wait**, `reach` already clears the next ceiling | she steps in on her own: Depth +1 |
| **Wait**, otherwise | nothing; a second Wait in a row costs Safety -1 |

## 6. What the player can read (tells)

| State | What it looks like |
|---|---|
| Retreating | formal, restates her earlier line, polite |
| Open | longer answers, asks back, uses "I" |
| Flooding | short, looks away, goes quiet |

Reaction text after each spread and each reply is where these show.

## 7. Closing and review

- **Final piece.** Isla returns one of Noa's own lines. The menu offers 3-4 lines she said during the session, flagged when she said them. One is the real insight and the others are deflections.
- **Review.** Written by Noa at home, first person, with a star rating. It quotes the page she stopped on, what you served, and the line you returned. Rating and text may disagree. It is assembled from session tags first; the LLM may polish later.

## 8. First build

Keep: Noa, one room, two books (one fit, one near miss), the three variables, the LLM for Noa's wording inside these limits.

Cut for now: the kitchen and service items, Awa/Jonah/Sophea, the five secrets, the three generic checkpoints, the clock and timeout stages.

## 9. Open questions

1. Can a hook read Noa's generated line, to detect when she says something about herself?
2. Can a stage carry per-spread reaction text, or does that need a book content type?
3. Where do flagged lines for the closing menu get stored?
4. Is two books enough for the choice to matter?

## 10. Table-read findings (5 turns, user as Isla, assistant as Noa)

Starting state: Safety 3, Absorption 2, Depth 0. Moves played: Link (real clue), Wait, Read on, Wait, Reflect.

- **Link with a real clue** stepped her in one layer when reach cleared the ceiling.
- **Wait too early did nothing.** At reach 4 it gave her nothing to work with. At reach 6 she stepped in on her own. Silence only pays when she is ready.
- **Reading is the lever for reach.** Two spreads took reach from 4 to 6.
- **Over-ceiling slide (new rule to adopt).** She can go deeper than her ceiling because the story carries her. At the start of the next turn she slides back one layer unless the previous move was an accurate Reflect or Question, which holds her and gives Safety +1. The player's job at that moment is to receive, not push.
- **Too many options.** Six or seven felt heavy. Rule for the first build: at most four options per turn; Bridge hidden once she is at Depth 2; Link shows only the best one or two clues; Wait is a separate "stay quiet" button; **Return** (ask 0) appears only when she has gone deeper than she can hold.
- **Flagged lines worked** as raw material for the closing menu (a surface line, the real insight, a deflection).
- **Untested:** the thresholds (3 and 6), the +1/-1/-2 deltas, flooding, pacing (Safety 3 to 5 in five turns, core needs 9), and whether decoy clues read as misreads or as unfair traps.
