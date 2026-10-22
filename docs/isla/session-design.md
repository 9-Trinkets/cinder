# Isla: single-act design (draft)

Terminology: one therapy session is one **act** (an act sits inside a persistent playthrough, which is the session).

Status: proposal for a rebuilt, minimal Isla pack. Nothing here is implemented. Not playtested. Psychology 101, not real therapy: players should learn what therapy feels like without thinking they can do it.

The first patient is **Hal** (`hal-story-spine.md`); pronouns below are she/her for a generic patient. Noa was the earlier candidate (`noa-story-spine.md`, kept for reference).

**The open question is whether this is fun.** The first build is pure authored content (no LLM) so the player experience can be tested as a fixed set of paths before anything else is added.

## 1. Fantasy and goal

The player is Isla, a bibliotherapist. The skill being played is **attunement**: notice what the patient is asking for in each line and answer in a way that fits.

Success is not "the patient gets fixed". Success is "the patient felt seen". The player never sees a score during play. They find out how it went from the patient's review at the end.

## 2. Act flow

| # | Beat | Player does | Output |
|---|------|-------------|--------|
| 1 | Arrival and greeting | short, 1-2 exchanges | first clues (umbrella, seat, phone) |
| 2 | Intake | a few exchanges | first bids |
| 3 | Serve | one beat, one choice | what the patient accepts or refuses is a clue |
| 4 | Pick a book | choose between 2 books | one fits, one is a near miss |
| 5 | Reading and stops | read aloud spread by spread, stop to talk | 3-4 stops, a few bids each |
| 6 | Closing | return one of the patient's own lines | the **final piece** |
| 7 | Leaving, then review | none | the patient's review with rating |

Target length: 20-30 minutes. The next act is an open line at the end of the review, not built yet.

## 3. Bids

Each key line the patient says is a **bid**, written with a hidden type:

| Bid type | Example (Hal) | What he is asking for |
|---|---|---|
| Feeling | "Sensible bear. Follows instructions." | to be noticed, gently |
| Invitation | "Go on. What's the next step?" | you to go on |
| Opening | "I wash both. I don't know why that part matters." | to be received, not interpreted |
| Deflection | "Old habits. Anyway, the bridge." | an exit |

## 4. Fit table

Each bid type has fixed answers:

| Bid | Fits | Hurts | Everything else |
|---|---|---|---|
| Feeling | Reflect, Question | Bridge (too fast) | shallow |
| Invitation | Read on, Question | Wait (leaves her hanging) | shallow |
| Opening | Reflect her words, Wait | Bridge, Link (you start interpreting) | shallow |
| Deflection | Return, Wait | Link, Bridge (you push) | shallow |

Move types: Reflect, Question, Link (to something she said earlier), Bridge (to herself), Wait, Return (to the book), Read on.

## 5. The ladder (the only state)

Her bids are authored in order from surface to vulnerable. Her position on the ladder is the only state:

- You **meet** a bid: she moves up one rung.
- You **miss** it (shallow): she tries once more, in a safer form, then moves on.
- You **hurt** it: she drops one rung.

**The book decides which rungs exist.** The fitting book unlocks the vulnerable rungs. The near-miss book gives only surface rungs, so she never gets there.

## 5b. Secrets (count double)

Each secret is an Opening bid at the top of a rung. Discovering it is fun, and it is also the player's chance to score extra:

- **Held** (Reflect her words, or Wait): the secret scores double, and an authored after-beat plays (a visible shift and a reframe line).
- **Fumbled** (anything else): the player still hears the fact, but it scores nothing and costs nothing (it does not drop her down the ladder).

She only tells a secret if you have climbed the ladder to it, so scoring needs both getting there and receiving it well. The notes page can be called "What she trusted you with"; held entries are filled in and fumbled ones are greyed ("told, not held").

## 6. Menus and content (all authored)

- Every bid is written with its own menu of at most four options. Each option has fixed wording and a fixed type, and the engine looks up its result in the fit table.
- Every menu has at least one option that fits and at least one that does not. Options that do not fit must sound reasonable (too early, too pushy), never rude, so the right answer is not obvious.
- Wait is a separate "stay quiet" button, not a numbered option.
- Slot order is shuffled; nothing in the wording marks which ones fit.
- The patient's lines, reactions, the book text and every ending line are written by hand.
- No LLM in the first build. An LLM for wording can be added later behind the same slots.

## 7. Closing and review

- **Final piece.** Isla returns one of the patient's own lines. The menu offers 3-4 lines she said during the act: one real insight, the rest deflections.
- **Rating.** Every bid met on the first try earns 1 point; a secret is a bid that counts double and earns its points only if held. Stars = points earned divided by points available, times five, rounded to the nearest half star, minimum 1. Bids on rungs she never reaches count as unmet. A bid met only on the retry earns nothing.
- **Review.** Written by the patient at home, first person, from authored lines picked by yes/no facts: which secrets you held, did the book fit, did you return her real line or a deflection. Rating and text may disagree. No score is shown during play.

## 8. First build and the fun test

Keep: Hal (see `hal-story-spine.md`), one room, two books (one fit, one near miss), about 12 bids with fallbacks.

Cut for now: the kitchen and service items, Awa/Jonah/Sophea, the five secrets, the three generic checkpoints, the clock and timeout stages, the Trust meter, Absorption, Depth, flooding.

**Fun hypotheses to test with a prototype script (before any engine work):**
1. The player hesitates between options for a real reason, not because the right one is obvious.
2. A miss feels like a missed chance, not a quiz penalty.
3. When she opens up, it feels like a reward the player earned.
4. The review makes the player want to try again and do something different.

**Likely failure:** it plays as "guess the right move type". Mitigations to try if so: two fits per bid with different consequences, more textured misses (she tries again differently), and bids whose type is ambiguous until you read her reaction.

## 9. Open questions

1. How many bids make a good act, and how many rungs per book?
2. Does Wait need its own consequences?
3. Should the player see her reaction text only, or also a faint hint of the ladder?
4. Is two books enough for the choice to matter?

## 10. Earlier models (superseded)

The first design used three variables (Safety, Absorption, Depth) and then a single Trust meter. Both were replaced by bids because the ladder is easier to explain, author and test. Table-read findings from the three-variable model: a Link from a real clue stepped her in one layer when she was ready; a Wait too early did nothing; reading raised what she could reach; a story can carry her deeper than she can hold, and the right move then is to receive; six or seven options felt heavy (rule: at most four); flagged lines worked as raw material for the closing.
