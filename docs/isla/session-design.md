# Isla: the Wren act, session design (draft)

How the game plays, for the first patient, in Willow House. Read with `wren-bible.md`, `isla-bible.md`, `willow-house.md` and `voice-guide.md`. Items marked **(decision)** can change.

Terminology: one therapy session is one **act**. An act sits inside a persistent playthrough, which the engine calls a session.

**No clock.** The game has no time of day. Progress comes from quests, hooks and what the player does. Thursday at 4:00 and a thirty-minute session are backstory only.

Status: implemented in `content/isla` (see section 14). Psychology 101, not real therapy: the player should learn what attunement feels like without coming away thinking they can do therapy.

## 1. What the player is doing

The player is Isla. The skill being played is **attunement**: notice what a child offers, answer it in a way that fits, and let the child lead. Success is not "Wren gets fixed". Success is "Wren felt met", and the player finds out how it went from what Wren and her mom leave behind at the end. No score is shown during play.

## 2. The shape of the game **(decision)**

Three phases, with no clock:

| Phase | What it is |
|---|---|
| **1. Before you greet Wren** | Isla gets ready. Free exploration of Willow House, with a short, optional list. **Wren is already in the waiting room.** |
| **2. The session** | The act begins when Isla greets Wren. Mostly choices, in the playroom. |
| **3. After** | Mom collects Wren. Isla writes her note on the back steps, and her phone buzzes with what came in. |

## 3. Where Isla is at the start **(decision)**

**On the front porch, on a gray afternoon in November.** She has just chained her bike and unclipped her tote. Through the glass in the green door she can see Wren sitting alone on a low chair in the waiting room with Biscuit in her lap and her shoes lined up in a perfect row.

Dad dropped her off early, on his way to a job across town, and left. That is why she is already there. Gloria is at the desk. Mom will collect her afterward, because her meeting ran late.

The game starts here because it is the last moment Isla can choose what to do before she greets a child.

## 4. The first quest: "Get ready for Wren" **(decision)**

The quest log shows one quest with three small, **optional** steps. None is required, and none is scored. Together they decide how well Isla knows Wren, what she holds, and what the room contains.

| Step | How it is done | What it changes |
|---|---|---|
| **Read your notes** | Look at any of the case notes Isla carries (section 5). | The player learns what Isla knows. Without it the player goes in knowing only what the opening said, and is more likely to pick the wrong book. |
| **Choose a book** | Visit the book room and take one from the shelves. | Decides which book Isla reads. It is the first real decision (section 9). |
| **Bring a snack** | Take a clementine and some crackers from the kitchen to the playroom. | Wren will ask for a snack. If there is none ready, Isla has to leave the room to get one. |

**Reading the notes is strongly encouraged but never required.** The ways the game does it, all driven by events and not by a clock:
- **The notes are on the Look panel** from the start, listed under Tote with what Isla is carrying.
- **The quest log lists it first.**
- **It matters.** The notes hold the one clue that makes the right book choice easy, and a player who skips them is likelier to make the mistake.
- **Only one picture book is carried at a time.** Taking a different book from the third-floor shelves returns the previously chosen one to the shelf.
- **Greeting Wren closes any unfinished preparation steps** so they do not linger in the sidebar during the session.

**Pacing.** There is no deadline. Wren waits, and the player can explore the whole house. Finishing a step can prompt a short line, for example Gloria saying "She's been very patient" once the notes are read.

**If she does nothing.** She walks in and greets Wren with only the tote, and she has read nothing. There is no default book: when Stories first comes and she is not carrying one, she tells Wren she left the books upstairs, and the story waits until she has fetched one from the book room. The opening text says where the books are.

## 5. Isla's notes and her phone **(decision)**

Two things, with different jobs. **What she carries is read when she chooses.** **What arrives on its own comes on her phone.**

### Her case notes (things she holds)

Isla keeps a **folder of notes** on Wren. Each page is its own item in her tote, so the whole folder is a collection she can read, in any order, as often as she likes:

1. **The intake form,** in the parents' own words. Mom: "She's been so good. Honestly, no trouble at all." Dad: "She's a good little trooper."
2. **The school counselor's referral,** with Ms. Okafor's observations: she stopped sharing at morning meeting, and she sorts the class library into piles.
3. **Isla's notes from session one,** a brief meeting.
4. **Isla's notes from session two.** Wren built a tower of blocks in silence, knocked it down, and tidied every block. **Isla also wrote: "Did not name the move. W. left the table when I said 'two houses.'"** This is the clue that warns against a book about exactly that.
5. **The family schedule,** a typed page with Mom's days and Dad's days, and Dr. Bell's margin note: "Both parents attentive. Child very tidy. Go slowly."

None of it is about the cup.

**How it works in the game:**
- The player opens the **Look** panel, where the pages are listed with whatever else Isla is carrying, and chooses one. Its text prints into the transcript.
- No new screen and no new action. It uses what the game already has for looking at things Isla holds.
- **A new page is added after the session:** Isla's own **session three note**, written on the back steps. It is in her own voice, and what it says depends on what the player did. The folder keeps growing with each patient and each session.
- The player can read the folder any time they like, with no time cost.

### Her phone (things that arrive)

Isla's phone is in her tote, and **off while she is with a child**, so nothing arrives during the session. Messages appear in the transcript as a styled block with the sender's name, in the same style as a handler's voice.

- **After the session,** on the back steps, her phone has buzzed twice:
  - **A message from Mom** about the session. It is the feedback, and in the fullest case it says Wren hummed the Friday song in the car.
  - **A text from Dr. Bell:** "How did it go? Come and find me tomorrow."
- Wren's drawing, which she left for Isla, is described in plain narration.

## 6. When the session begins **(decision)**

Wren is **already in the waiting room.** There is no arrival scene: no truck, no goodbye from Dad, no walk through the door, and nothing waits on a clock.

- **The pickup card** shows Dad's photo, not Mom's. That is the plant. Isla can look at it, and Gloria mentions it.
- **From Isla's first step inside** Wren is visible and can be looked at. She is sorting the fish tank's smooth stones into two piles, in silence.
- **The act begins** when the player chooses **Greet Wren**, a bar action available in the waiting room while Wren is there. Until then the player can walk anywhere and do the optional steps, for as long as they like.
- If the player greets her immediately, the preparation is skipped.

## 7. The session **(decision)**

All authored, with no LLM. The session has these beats, each a short screen of text and a small choice:

1. **Meeting her.** Wren is wary and a little off-balance (Dad dropped her early, and it is a Mom day). Isla greets her. **Bid:** she checks her notebook and says it is the wrong day.
2. **Walking in.** Down the hall, past the white-noise machine, into the playroom. The rules card is read aloud.
3. **The bins.** Wren starts sorting toys into the two lidded bins and counting. Isla can join, wait, or comment.
4. **The snack.** She asks for a clementine and half a cracker, cut in squares, or notices that there is none.
5. **Rabbit's bag.** She sets Biscuit's bag beside her and packs it again. **First secret:** what is in it.
6. **Stories first.** Isla draws the cloth over the toy shelf and turns on the lamp. The reading begins.
7. **The book, in stops.** Isla reads a few pages, Wren reacts, and there is a choice. The book opens the next secrets, one at a time.
8. **The cup.** The last secret, and the hardest to reach.
9. **Closing.** The timer on the shelf gives its soft warning, and then its chime. This is a story beat that arrives when the session reaches its closing, not a clock running out. Isla can offer one of Wren's own lines back, or nothing.

## 8. After the session **(decision)**

- **Mom arrives** and waits in the waiting room. Isla speaks with her briefly. Mom is relieved and a little worried.
- **Isla sits on the back steps for five minutes** with her bike beside her. Her phone buzzes (section 5), and she writes her note. That note is the new page added to the folder.
- **The game ends** when she has read her note and Mom's message, or chooses to stop. There is no star rating. The note, Mom's message and Wren's drawing are the feedback, and the lines in each depend on which secrets were held and which book was chosen.

## 9. The choice of book **(decision)**

The shelves in the book room are sorted loosely by subject. There are many books, and three matter. Each is something a careful player can reason about from the file.

| Book | What it is | Effect |
|---|---|---|
| ***The Bird Who Carried Her Nest*** | A bird who will not set down her nest because she is afraid either tree will be sad. | **The fit.** It opens the later secrets. |
| ***Bear Counts to Ten*** | A cheerful counting book. Wren will love it. | **Safe and shallow.** It reaches the first secret or two, and no more. |
| ***Two Houses for Fox*** | A book about a fox whose parents live apart. It is on the low DIVORCE shelf. | **Too direct.** Wren goes quiet and the session retreats. |

The third is the instructive one. The file says her parents are separating, and a book about exactly that looks like the obvious choice. A player who read Isla's note from session two ("W. left the table when I said 'two houses'") has the clue to avoid it. A player who skipped the file is more likely to fall in, and learns, through play, that a child may not be ready to meet her own story head on.

## 10. Bids, moves and fit, for a child **(decision)**

A **bid** is something Wren offers, in words or in action. Each has a hidden type:

| Bid type | Example | What she is asking for |
|---|---|---|
| **Offer** | Sorting toys into the bins. "These ones live here." | for it to be noticed |
| **Ask** | "Is Rabbit allowed?" | a plain, short answer |
| **Step back** | Switching to a cartwheel when the topic gets big | to be let off the hook |
| **Tell** | "It came off all in one bit." (a secret) | to be received, not fixed |

Isla's **moves:** Reflect ("Those ones live here."), Wonder ("I wonder what Biscuit thinks."), Join (puts a toy in a bin too), Offer (hands over the red crayon), Wait (sits and watches), Follow (goes along with the change of subject), Probe (a direct question), and Reassure ("It's okay.").

**Fit table:**

| Bid | Fits | Hurts | Everything else |
|---|---|---|---|
| Offer | Reflect, Join, Wait | Probe | shallow |
| Ask | a short plain answer (Reflect or Offer) | Probe | shallow |
| Step back | Follow, Wait | Probe, Reassure | shallow |
| Tell | Reflect, Wait | Probe, Reassure | shallow |

With a child, **Wait and Follow often fit,** which is a change from the adult version. **Probe always hurts,** and **Reassure hurts at a Tell** because it answers a guilt that has not been said yet.

## 11. Secrets and scoring **(decision)**

Kept from the earlier design, and patient-neutral:
- **Every bid met on the first try earns 1 point.** A **secret** is a bid that counts double, and only counts if it is **held** (Reflect or Wait). A fumbled secret costs nothing, and the player still hears it.
- A secret appears only if the player has earned at least **60%** of the points available so far. The last one also needs the one before it held.
- The book decides how far the chain goes (section 9).
- The number of bids and the point total will be set once the bids are written.

The four secrets, as things Isla finds before Wren says them (see `wren-bible.md`): 1. The bag. 2. The notebook. 3. The quiet. 4. The cup.

## 12. What the engine can already do

| Need | Support |
|---|---|
| Walk the house, look at things | Built (Willow House) |
| Wren already in the waiting room | An actor placed there at the start |
| **Greet Wren** starts the act | An action available only in the waiting room while Wren is present, opening the first menu |
| Quests and nudges, with no clock | Stages, quests, and hooks on events such as entering a room, reading something, or taking an item |
| **Case notes Isla can read** | Items carried in the tote, listed on the Look panel, whose description prints when looked at. Built. |
| **A new note added after the session** | The `GrantItem` effect on a stage, which already exists. Conditional variants use a hook. |
| The book is chosen | `item_acquired:<book>` signal, with books as items |
| A snack is brought | `item_acquired` plus a room check |
| Bids as menus, points and the 60% gate | Built and tested earlier (menus, hooks, conditional signals) |
| **Messages on the phone** | The handler pattern exists, but it has **one** voice. **Needs** a small generic change so a pack message can name its speaker (Dr. Bell, Mom), and a text setting for the block's header ("Message" instead of "Dispatch"). |
| **A "read your notes" quest step** | **Needs** one line: looking at an item should fire an `item_observed:<id>` signal, as looking at a feature already does. |
| Closing and feedback that depend on what was held | **Needs** options and lines shown only when a condition holds |

So the notes need almost nothing. The phone needs a small generic change, and the closing needs the conditional options and lines we already knew about.

## 13. Open questions

1. **Wren waiting alone** with Gloria, with no limit. Fair to her, or too sad before the session starts?
2. **Without a deadline, does the preparation matter enough?** The three steps are optional. The pull is the notes' clue about the books, Dr. Bell's text, and Gloria's nudge.
3. **Reading notes with Wren in the room.** The engine allows it. Should the story put the folder away when she greets Wren, so it cannot be read mid-session?
4. **One new page, or one per outcome?** The session three note has several versions depending on what happened. Is that the right size?
5. **Would Mom really message her psychologist?** Clinics often have secure messaging for this. The alternative is a note left at the desk.
6. **Three books enough?** Should there be a fourth, and what would it be?

## 14. As built: the session so far

Implemented in `content/isla` and tested in `cinder-core/tests/isla_wren_session.rs`. Everything from the greeting to the end is playable, including the after.

**Flow.** Greet Wren (Talk) → meeting bid → Wren follows Isla to the playroom → bins → snack → bag (first secret) → Stories first → book read in stops → closing → farewell and drawing.

**Bids and points.** Every bid has four answers. Fitting answers score and the others do not.

| Bid | Fitting answers | Points |
|---|---|---|
| Meeting, bins, snack, each book stop, closing | reflect, wait, join or follow, plain answer | 1 each |
| Secrets (bag, notebook, quiet, cup) | reflect, wait | 2 each |

**Answer order is mixed.** Each menu lists its answers in a fixed but scrambled order, so the first answer is a fit only about a quarter of the time. The tests choose answers by id, not by position.

**Secrets are gated by fixed point thresholds,** a simplification of "60% of the points so far". The thresholds are 2 for the bag, 4 for the notebook, 6 for the quiet, and 8 for the cup. The cup also needs the quiet to have been held. A player who has not fit the first answers never hears the deeper ones.

**The book decides the path.** The branch is read from what Isla carries when she opens the book: *The Bird Who Carried Her Nest* reaches every secret, *Bear Counts to Ten* reaches the notebook, and *Two Houses for Fox* makes Wren retreat to the bins.

**Snack.** If Isla is carrying it she hands it over, and if not she goes to the kitchen and back.

**Farewell.** Wren leaves a drawing, chosen by the deepest secret Isla held: the cup gives the bird on the ground and a girl with a stop sign, the notebook or the quiet gives the bird with her nest, and anything less gives a row of crayons.

**After.** Isla goes out to the back steps. Her phone has buzzed twice. Reading it plays Mom's message and Dr. Bell's check-in, one per turn, as `Mom (to Isla)` and `Dr. Bell (to Isla)` under a `Message` header. Mom's text matches the same three tiers: the car ride, the hall light, the humming. Writing the note adds Isla's session three note to her Tote, in the matching tier, and reading it ends the game.

**Engine pieces added for this:** `item_observed`, room items, actor topics, stage-set following, `inventory.<id>` in stage conditions, menus that wait for their actor, and a stage effect that queues a scripted sequence.
