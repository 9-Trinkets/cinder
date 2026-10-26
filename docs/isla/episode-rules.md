# Isla: what an episode can show (ground truth)

Every episode after Wren's is checked against this sheet **before** anything is written. It is taken from `session-design.md` (as built) and `isla-bible.md`, not from new ideas. If an idea needs something not on this sheet, the idea changes or the sheet is changed on purpose, in this file, first.

## 1. What the game shows

| Place or moment | What the player sees | Source |
|---|---|---|
| **Before** | Isla's journal: the intake form, a school referral, her own notes from earlier sessions, a schedule page with Dr. Bell's margin. Free walk around Willow House. | `session-design.md` §4–5 |
| **The session** | One child, in Willow House, as a series of menus. Isla's answers score as fits or misses. Secrets open when the room and the answers are right. One book, chosen from the shelves, decides how deep it goes. | §7, §9–11 |
| **After** | The parent comes to the waiting room. Isla sits on the back steps. Her phone shows a text from the parent. She writes a private note. The child leaves a drawing or object. The game ends. | §8, §14 |

## 2. What the game does not show

- **Anything outside Willow House.** No home, no school, no car, no night. They exist only as words in a note or a message.
- **Anything after the session,** except what the parent chooses to put in one text.
- **Isla speaking to the parents about the child** at length. She greets them in the waiting room at most.
- **A clock.** There is no time of day. Progress comes from what the player does.
- **Any outcome Isla could not have caused,** or any fact nobody in the scene could know.

## 3. What each kind of text is

| Text | Written by | Read by | Can say |
|---|---|---|---|
| **Case notes in the journal** | the parents, the school, Dr. Bell, or Isla herself | Isla, before the session | Facts someone at that source actually knows, and Isla's own earlier observations. **One of them holds the clue** that makes the right book easy. |
| **Isla's note after** | Isla | Isla (and the player). **Nobody else.** | What she saw in the room and what she would try next week. Never anything she did not see. |
| **The parent's text** | the parent, the same evening | Isla | One small thing the *child* did after the session that the parent saw. Not a summary, not a result, not a request. |
| **What the child leaves** | the child | Isla | A drawing or an object, chosen by the deepest secret held. |

## 4. The shape every child follows

1. **One true thing,** held by the child, that the game never says aloud. It is **a belief or a job the child has given themself, tied to one object**, not a family plot. (Wren: the cup handle, and the belief that the cup is why her parents live in two houses.)
2. **A chain of secrets** (usually four) that Isla finds in the room, each shown through what the child *does or carries*, each held by the right response.
3. **One clue in the notes** that warns against the trap.
4. **One book that fits, one that is shallow, one that is too direct.**
5. **A fit table for this child:** which of Isla's answers fit which of the child's bids, and what hurts.
6. **Three endings** by how many secrets were held, each with its own note, parent text and left behind object.

## 5. The consistency check

Every fact in a new episode is entered in a **ledger** with these columns, and nothing goes into the game without a row:

| Fact | True? | Who in the scene can know it | How the player can find it (item, clue, behavior, room) | Where it shows in the endings |
|---|---|---|---|---|

Rules for the ledger:
1. **A row's "who can know it" must include whoever speaks it.** A parent cannot report what only the child knows.
2. **A row's "how the player can find it" must be something Isla can see or hold in Willow House.**
3. **An ending line may only show a fact that has a row and was held.** No result without a cause in the room.
4. **A tier of ending may only differ by things the player did.**
5. **No fact is explained in text.** Rows record what is *shown*.

## 6. What we are free to choose

- **Which child.** Age, family, what they love and avoid.
- **The kind of listening it asks for** (the fit table). Wren needs words reflected and waiting. Another child needs something else.
- **What the room can do.** The engine now supports features with states and interactions (lamp, blind, machine). They are optional tools, not a requirement.
- **The object and the belief.** Anything small a child could carry or make.

## 7. What the engine already supports (so nothing here is a reason to cut)

Menus with scored answers, secrets gated on points, a book chosen from shelves by what Isla carries, journal pages, stage effects that grant items, phone messages in a speaker's name, conditions on features and items, dated cards between episodes, an act that ends and a world that carries on, actors who arrive later (`starts_away`), the player relocated by a stage, shelves restocked between acts (`set_feature_items`), and a card whose frames each set their own tint, text color and ambient layer.
