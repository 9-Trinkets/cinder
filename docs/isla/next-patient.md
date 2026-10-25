# The next patient: a plan (draft)

A plan, not a script. The second child at Willow House, chosen to be as different from Wren as possible, so we find out what the framework in `game-design.md` really needs. This follows the patient sheet in section 3 of that document. Items marked **(decision)** can change.

**Working name: Tam.** The full story is in `tam-bible.md`: a Vietnamese American family in South Philadelphia. The names and details there are placeholders until a reader from the community has checked them.

## 1. Why this child

Wren talks, tells her story through Rabbit, and is read by reflecting her words. A child who barely speaks breaks every one of those habits, so what the player knew about listening has to change.

He is also close to Isla. Her brother Danny did not speak until he was four, and the only thing that reached him was one picture book. Isla learned to read him through what he did, which is where her whole method comes from. This episode is where that method is tested without words.

**Danny is not Tam, and Tam is not Danny.** Danny's life belongs to Danny. Tam is a different child, and Isla does not tell him about her brother, in the game or anywhere else. The link is only in how she sits next to him and what she has learned to look for.

## 2. Who he is **(decision)**

- **Six years old,** first grade, in a public school classroom with an aide for part of the day.
- **He speaks very little.** He has a handful of words, which he uses at home and almost never elsewhere. He understands far more than he says. He does not mind silence.
- **Autistic,** diagnosed at four. The game never uses the word, and it never explains him. The notes mention "an evaluation at four", and the text shows what he does.
- **His family:** the Trans, in South Philadelphia. His mother, **Linh**, is a night-shift nurse, tired and fiercely on his side. His father, **Dũng**, works long days and is home late. His grandfather, **Ông**, lives with them and walks him to school. There is a baby sister, **Mai**, who is the reason the house has become louder. They speak Vietnamese and English at home, and the school's first guess was that his quiet was about language.
- **Why he is here:** the school asks whether he is "responding", because he has stopped joining the one group activity he used to like. His mother is not sure anything is wrong. She is worried that everyone wants him to be a different child.
- **What he loves:** trains, and especially the sound they make going over a particular bridge. It is his own interest, not his father's. Lining up objects by length. The same few books.
- **When it happens:** a Tuesday in late winter, a few weeks before the Lunar New Year, when the house is cleaned, relatives come and go, and the street fills with firecrackers and drums. The season adds real noise and real love.
- **What he avoids:** sudden noise, bright overhead light, being touched without warning, the word "no" spoken sharply.

**What is really happening (never said aloud):** the baby has made the house louder, and his own room, the one quiet place, is now next to the nursery. He has stopped doing the things in the world that cost him the most, because he has no energy left over from coping with the noise. He is not withdrawing from people. He is saving himself. Everyone has been reading it as the first, and it is the second.

## 3. How he bids

His offers are actions, and each is a request.

| Bid | Example | What he is asking for |
|---|---|---|
| **Line up** | Sets animals in a row by length | For it to be seen, not interrupted |
| **Repeat** | Turns back to the same page | To be given the same thing again |
| **Retreat** | Goes under the low table | Less noise, less light, more room |
| **Hand over** | Puts an object in Isla's hand | To be asked nothing in return |
| **A word** | Says one word, once | To be received with no fuss |

## 4. The fit table, which is the point **(decision)**

The whole episode turns on a table that is nearly the opposite of Wren's in one way: **the more Isla says, the less well it fits.**

| Bid | Fits | Hurts | Everything else |
|---|---|---|---|
| Line up | Mirror (add a piece in the same pattern), Wait | Moving his pieces, a comment about them | Shallow |
| Repeat | Read it again, the same way | Reading it differently, a new book | Shallow |
| Retreat | Change the room (lower the light, silence the hall), Wait | Following him under the table, asking him out | Shallow |
| Hand over | Take it and hold it, saying nothing | Praise, a question | Shallow |
| A word | Do not answer in words. Use the word back once, softly, or wait | Making a fuss | Shallow |

New moves Isla gets for him, besides the ones from Wren's table:
- **Mirror:** do what he does, in the same way, without speed or comment.
- **Change the room:** dim the lamp, close the blind, turn the machine in the hall off or on.
- **Read it again:** the same page, the same voice.

What does not fit, on purpose: reflect (speaking a lot), wonder, reassure. They are normal good moves with Wren and mostly noise here.

## 5. The secrets, as things Isla discovers

None are told. Each is a change Isla makes or a pattern she notices.

1. **The noise.** The radiator knock and the hall machine are too loud for him. Isla notices him stiffen at the knock and finds she can turn the machine down.
2. **The light.** The overhead light is the problem and the lamp is not. Once the room is dimmer, he comes out from under the table.
3. **The book he brings.** He carries the same book every time and wants it read the same way. The right thing is to read it, again and again, and not to offer a better one.
4. **The word.** After enough of all this, he says one word. It is the name of the book, or "again", or his sister's name. What it is must be decided while writing the novel (see `game-design.md`, section 8).

The secrets are discovered through steady, quiet behavior. A secret is held if Isla does less, and lost if she tries to hurry it.

## 6. The book problem

- **The fit:** the book he brings, which is his own. It is not on Isla's shelf, and she must notice it in his bag.
- **The shallow choice:** a train book from the shelf. He likes it a little, and it goes nowhere.
- **The trap:** a book about a baby sibling. It names the thing that hurts, and he leaves.

The player who read the note about "the same few books" has the clue, and the trap is the same kind as *Two Houses for Fox*.

## 7. What the room does

Unlike Wren's episode, where the room stays the same, here Isla changes it:
- **The lamp** and **the overhead light** are controls.
- **The white-noise machine** in the hall can be turned down, and Isla has to go out to the hall to do it.
- **The blind** on the window can be lowered.
- **The low table,** where he sits, is a place he goes to and comes out from.

Each change is a small action, with a short line, and a state the later menus can read.

## 8. What this needs from the engine

| Need | Notes |
|---|---|
| **Room state that changes** | Light level and noise level as story variables, set by Isla's actions. Menus and gates read them. |
| **Actions on room features** | Lowering the blind or turning the machine down is an action on a feature. We have features and stage effects, and need a clean way to attach actions to them. |
| **A "wait" that counts** | Waiting is an option today. Here the player may wait through several quiet beats, and the pace has to allow it. |
| **A "read it again" loop** | The same page, the same menu, with small variation each time. |
| **Fit rules that read room state** | A secret opens only if the light is low. This works with conditions on story variables. |
| **A second pack** | A separate folder, in the same house, with the same notes folder and Gloria. |

## 9. How the episode feels

- **Before:** the notes are short and slightly mismatched. His mother's words say one thing, and the school's say another. Dr. Bell's margin note says something like "Go slowly. Fewer words."
- **The waiting room:** he is not in the chair. He is under it, or standing by the fish tank, looking at the bubbles.
- **The session:** very little is said. The reader should feel the pleasure of a small change working.
- **The ending:** nothing explained. He does or says one small thing, and his mother's message afterward is short.

## 10. What to find out by writing it

This plan is guesswork until we try to write him. Questions the novel and the first lines should answer:
1. **What is his one word,** and who is it for?
2. **How much of his inner life can Isla's chapters show** without the narration naming anything?
3. **Is "the more Isla says, the less it fits" fun to play,** or is it only a different kind of waiting?
4. **What does his mother say in the notes** that the player should disagree with?
5. **How does a session with no spoken answers feel in the transcript?** It might need a different presentation.

## 11. Later children, in a line each

So the next ones can be chosen with the contrast in mind:
- **An angry child (eight)** who tests the limits. The fit is a warm limit, held without anger.
- **A child with a new baby sibling who has stopped doing things she could do.** The fit is not letting her shrink and not pushing her.
- **A very funny child** who makes every hard thing a joke. The fit is laughing, and then not leaving.
- **A child who does not want to be there.** The fit is giving the choice back, again and again.
