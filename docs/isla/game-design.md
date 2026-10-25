# Isla: the overarching game design (draft)

How the whole game is shaped, beyond the first session with Wren. `session-design.md` describes how one session plays. This document describes what the game is. Items marked **(decision)** are settled, and the rest is proposal.

## 1. The decision: a diner, not a case **(decision)**

Isla is built like *Tokyo Midnight Diner*. The place stays, the regulars come and go, and each episode is one visitor's story. The player does the listening.

- **The place is Willow House.** It is the same rooms, the same people at the desk, and the same back steps every episode.
- **One child comes per episode.** The episode is one session, the way Wren's act is.
- **Each child is told in their own way,** and each needs a different kind of listening (section 3).
- **A picture book is the way in** for every child, but which book fits changes with the child.
- **There is no villain and no case.** Nobody is solving the family, and nobody is failing the child. The game is about being present for someone, and what the player carries away is a better ear.

### What we are not doing: a mystery or an intervention

We considered making Isla investigate what is going on at a child's home and step in. We decided against it, for these reasons:
- **It turns the core skill upside down.** The skill is attunement: noticing what a child offers and following it. An investigator needs answers, and a probing question is exactly the move that always hurts in this game.
- **It needs a wrongdoer.** The best thing about Wren's story is that the cup was a trigger and not a cause, and nobody is at fault.
- **It is not what a psychologist does,** and it would use real children in danger as plot. If a child were ever in danger, the right thing is to bring in the people whose job that is, and that is outside this game.

What we do keep from that direction is the pull of a question the player wants answered across episodes. That pull comes from Isla herself (section 5), and not from the children.

## 2. The shape of an episode

Every episode follows the same three phases, so the player learns the rhythm and the writer knows what to deliver.

| Phase | What happens | What varies |
|---|---|---|
| **Before** | Isla arrives, reads the notes on today's child, and chooses to greet them. Small preparations come as the session needs them. | The notes and what they hint at. The child's state when Isla arrives. |
| **The session** | Bids, answers, a book read in stops, secrets found, a closing. | Almost everything: see section 3. |
| **After** | The back steps. A message from the child's parent, a note from the director, Isla's own note added to her folder. | The tone of the feedback, picked by what the player held. |

The engine already supports this shape. An act ends with `end_act` and the world carries on, so one session is one act inside a persistent playthrough. Only variables declared with act scope are cleared.

## 3. One child per episode, with a different fit table **(decision)**

**The central rule:** every child has their own *fit table*, the rules for which of Isla's answers fit which of the child's bids. This is what makes each episode a new skill, and not the same game with new text.

Wren's table (see `session-design.md`, section 10) rewards reflecting her words and waiting, and it punishes probing and early reassurance. A different child needs a different table.

### What can differ between children

| Dimension | Wren | What another child might change |
|---|---|---|
| **Bids** | Words and sorting, in small offers | Actions only, or jokes, or silence, or anger |
| **Fits** | Reflect, wait, join, follow | Mirroring an action, holding a rule, matching a pace, saying less |
| **What hurts** | Probing, early reassurance | Too many words, a surprise, an unwelcome touch, a broken promise |
| **Secrets** | Four things she tells, in a chain | Things Isla discovers by changing the room, or by being consistent over time |
| **The book** | One fits, one is shallow, one is a trap | The book is one the child brings, or is the same one each time |
| **The pace** | A thirty-minute session | A session that must not be rushed, or one that needs a fixed shape |
| **The ending** | A drawing and a message | Something the child does, or says, or does not |

### The patient sheet

Every child gets one sheet before any line is written, in the style of `wren-bible.md`. It answers:

1. **Who they are:** age, family, what they love, what they avoid. A person first, not a case.
2. **What really happened:** the true story, which the game never says out loud.
3. **How they bid:** the kinds of offers, and what each one is asking for.
4. **The fit table:** which answers fit, which hurt, and why.
5. **The secrets:** what the player can find, in order, and what holds each one.
6. **The book problem:** which book fits, which is shallow, and which is a trap.
7. **What changes in the room:** anything Isla can adjust (light, sound, seating).
8. **The feedback:** what the parent's message and Isla's note say at each depth.

### Keeping it psychology 101

The player should learn what attunement feels like, and should not come away believing they can do therapy. Each child's table teaches one honest, ordinary idea: follow before you lead, say less, hold a limit warmly, let a story do the work. No episode teaches a technique.

## 4. The rhythm between episodes

- **The regulars.** Gloria at the desk, Dr. Bell upstairs, and Isla on the back steps give each episode a familiar opening and closing. They hold the diner feeling.
- **The folder grows.** Isla's notes on each child stay in her tote, and the player can reread any of them.
- **The shelf grows.** Every book Isla has read with a child is something she knows. Over time the player chooses books out of experience, and not by reading the cover.
- **Children return.** A child can come back in a later episode and be different because of what happened. This rewards memory without needing a plot.
- **The order is not fixed.** A day at Willow House could bring one child, chosen by the player or by the schedule. Which child comes next is a framework question (section 7).

## 4b. Between episodes: time passes, but not by the day **(decision)**

As in *Midnight Diner*, the game does not step from one day to the next. An episode ends, and the next one begins whenever it begins: three weeks later, two months later, the following spring.

- **A card marks the jump.** When an episode ends and when the next one opens, a card in Isla's journal look covers the transcript. A date is written in, letter by letter, in her hand. A short line follows. Behind it the paper tints with the season and a slow layer drifts: bare branches, snow, rain, leaves. It lasts ten to fifteen seconds and can be skipped.
- **The gaps are uneven on purpose,** so it feels like a life and not a calendar.
- **The house shows the time.** The willow, the snack shelf, the drawings on the hall wall and Gloria's stickers change between episodes, and nothing says so.
- **It is a reusable engine feature, not an Isla one.** Any pack can play a *card movie*, which covers the transcript panel and leaves the rest of the screen in view, when a stage begins, by naming it in `movie_id`, and set how it looks in `movies.json`: the font (prose, mono or the theme's handwriting), the colors, the alignment, an ambient layer (none, drift, snow, rain or leaves), how text appears (fade or typed) and how fast frames dissolve. Each frame can carry a heading and its own tint, which is how a season or a time of day changes from frame to frame. A stage's lines can be shown before the card or after it.

Isla uses it now to end the first episode with a journal page: "Thursday, November 21. Wren." The next episode will open on a dated page of its own.

## 5. Isla's own thread **(decision, to be developed)**

The pull across episodes is Isla's life. She does not share it with a child, but the player learns it a little at a time, in small places:
- **Danny,** her brother, and the picture book he made her read two hundred times. Everything about how she listens begins there (`isla-bible.md`, section 2).
- **The back steps,** and why she sits there for five minutes after the hard sessions.
- **The notebook of sentences children have said,** which she cannot stop rereading.
- **The cloth over the toy shelf,** which was a towel over Danny's.

This thread is told in her notes, her phone, and things she carries. A novel based on Isla (section 8) is a way to find what it is.

## 6. What the game never does

- Shows a score. The note, the message and the drawing are the feedback.
- Names a feeling or diagnoses a child. The text shows behavior.
- Makes a child a puzzle to solve or a problem to fix.
- Treats a child's disability or family structure as a flaw.

## 7. What the framework needs

Wren's session was built as one act. Several episodes need a little more:

| Need | Notes |
|---|---|
| **A patient pack** | One folder of content per child, built from the patient sheet, that plugs into the same house. |
| **Choosing who comes** | A schedule, a menu, or a story trigger that decides which child is next. |
| **Memory between episodes** | A child's earlier result changes their next session. Story variables can carry this. |
| **Different fit tables** | The scoring is already content, and each child brings their own menus and hooks. |
| **Acts that change the room** | A child who needs the light lowered means Isla's actions change room state, and not only the dialogue. |
| **A shared notes folder** | Notes for every child in one tote, ordered by episode. |

## 8. Where the novel comes in

We want to write a novel based on Isla, the way we did with Layla. The novel is a place to find things the game cannot yet say: Isla's inner voice, what it costs her to sit still, Danny, the other children, the house at night. What the writing turns up goes back into the patient sheets and Isla's thread.

## 9. Open questions

1. **How many episodes?** Four or five children is enough to show the shape, and a dozen would be a lot of content.
2. **Is the order chosen or given?** A day-by-day schedule feels like a diner, and a free choice feels like a library.
3. **Do children return?** It adds depth and it costs content. We could start with one return.
4. **How much of Isla's thread belongs in the game,** and how much in the novel?
5. **Does a parent ever come into the room?** For Wren, no. Another child might need it.
