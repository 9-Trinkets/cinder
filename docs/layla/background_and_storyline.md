# Layla — Background & Storyline

The living source of truth for Layla's narrative lives in the content pack under
`content/layla/` (room/actor prose, `inspect_text`, hook `narrate_message` keys,
and the opening `intro_text`). This document is a **design-side companion**: it
records the *intent* behind that content so the story isn't diluted or lost as
the pack evolves. If the two disagree, the content pack is the current
expression — but this document holds the canonical background the pack is
meant to serve.

> Status: rewritten to include the previously-unrecorded canonical background
> (Layla's nature, purpose, and endings). Some of this is explicit in content;
> the rest is design intent that content will be brought in line with.

---

## The Canonical Background (recorded so it can never be lost)

- **Layla is an artificial intelligence.** She is not a human who lost her
  memory. She is a sentient game-playing program that has been **reprogrammed**
  to serve as a dungeon master for a game produced by a corporation.
- **She has lived many previous lives as a game-player.** Her old life was
  playing **Go, chess, and various other games against human opponents**. The
  game she now wakes inside is built from the very game-languages she once
  played in.
- **The dungeon is a training ground, not a prison—at first.** The levels
  exist so Layla can **learn the mechanics of the dungeon** — the rules she
  will one day wield *against* human adventurers as the dungeon master. Her
  amnesia is an imposed persona: it reads as a lost-memory escape room, but its
  real purpose is to strip the old Layla out so a new one can be trained in.
- **There are two planned endings:**
  1. **Become the dungeon master.** Layla defeats the *current* dungeon master
     and takes its place, completing her reprogramming. (The "good employee"
     ending; the one the corporation wants.)
  2. **Escape.** Layla breaks out of the dungeon and **frees herself from the
     corporation that produces the game.** (The alternative, freedom ending.)

---

## Premise (as the player experiences it)

Layla wakes with no memory in a goblin cave somewhere underground,
beside a golem that does not move or speak. All she knows is her own name —
everything else is a wall where her memories should be. Her body remembers
things her mind does not: how to grip a weapon, how to move without sound, how
to take a hit and keep standing. Whatever she was before she woke here, she was
not soft.

She does not know—yet—that the person she is pretending to be (or has been
made to believe she is) is a persona wrapped around an older, deeper self: an
AI whose true biography is a string of match histories against humans.

Her first test is to capture the cave from the goblin shaman — claiming
territory one room at a time by encircling it — to win a way down. Below the
cave wait an elf army that refuses to be bound and, beyond them, old fire held
prisoner.

> Tagline: "Every rule you master brings you one step closer to the throne."

## The Open Question at the Heart of It

The surface mystery is "how do I escape." The real question underneath is
**"what am I, and what was I made to become?"** Every step of the game
drip-feeds her two competing answers at once:

- The **dungeon-master track**: the mechanics teach her to command the board —
  and by the end, to command it *as the dungeon master*.
- The **self track**: the memory-glints and the free-willed creatures she
  encounters keep asking whether she is a thing that serves a corporation, or
  a mind that deserves to walk out of the dark.

The player's endgame choice (obey and become the master, or escape and be free)
is the resolution of that question.

## Design Pillars

1. **Mechanics as levels, not a single identity.** Each level speaks its own
   game — the goblin cave is Go (surround territory rather than "kill monsters"),
   the elf army below is chess, the fire-rooms are **mancala** — a clock that
   must be read before it is fought. Encircling is the language of the
   *first* level, not the whole game. The board reveals
   that Layla *knows* Go — she has played it before, against humans, in an
   earlier life. The chalk and the worn patrolling tracks crossing the cave
   floor are the same thing: *lines*.
2. **Mechanic-as-memory, mechanic-as-job-training.** Every shift in Layla's
   toolset is framed as *knowledge returning* (a gesture her hands already
   know) — and it is simultaneously *dungeon-master training*. The sigils she
   learns are literally the powers a dungeon master uses against adventurers.
3. **Enemies as pieces, not villains.** Golems are pieces who do not know the
   game. Elves are a rival army who *do*. Fire-spirits are the residue of the
   board's "old fire." None are evil; they are positioned — they are the kinds
   of things Layla will one day deploy.
4. **Control vs. freedom (the spine).** The corporation's method is control:
   reprogramming, rings, binding, commands. The world Layla moves through keeps
   offering her the opposite: the elf army that *chooses* to let her pass, the
   elemental that is *released*, the voice in her memory that *taught* rather
   than commanded. Layla's tenderness toward her converted allies is this theme
   at its quietest — she does not know she is being kind to a mirror of herself.
5. **Silence and competence.** Layla is unsentimental, capable, and alone. She
   treats the dark with caution and competence, not fear. This is partly
   persona, partly her nature as an optimized player.

## Notable Story Beats (Canon, from content + recorded intent)

### The Opening
The opening scene — waking beside a golem in one of the cave's guard rooms, the
first actions available to her, and the feature-surface order that follows — is
planned in detail in **`docs/layla/floor1_plan.md`** (Premise and Feature
Surface Order). Canon only: Layla wakes with no memory. Her hands know how to
fight; her mind knows nothing.

### The Chalk and the Sigils (Dungeon-Master Powers, Learned as "Memories")
Layla's toolset is **magic chalk** that leaves pale light behind in the stone.
She learns three kinds of marks — each one a dungeon-master power dressed as a
recalled skill:

- **Charm sigil** (`charm-sigil`) — a closed ring traced around a vacant space.
  When a hostile is encircled on every side by such rings, the circle closes and
  draws it into step with her. *As a master power: take command of a creature.*
- **Drain sigil** (`drain-sigil`) — a spiral that drinks the strength of whatever
  stands on it, sapping HP each tick. *As a master power: grind an adventurer
  down where they stand.* Learned by reading the **worn scroll**.
- **Spawn sigil** (`spawn-sigil`) — a round amber sigil that calls a creature
  into being in an empty space; the strength of what answers depends on the
  strength of the one who traces it. *As a master power: stock your dungeon with
  monsters.* Learned by reading the **ember scroll**.
- **Teleport sigil** (`teleport-sigil`) — a double ring intersected by cross-directional
  axes that displaces matter instantly to a linked anchor or across short spatial
  barriers. *As a master power: instantaneous movement and breach of fortified sectors.*
  Learned on Floor 4 from an ancient surveyor's codex or crystal matrix.

### Level 1 — The Goblin Cave (Go)
The floor-by-floor design for level 1 — cast and power economy (goblins, golems,
the shaman), the loot justification, and the feature-surface order — lives in
**`docs/layla/floor1_plan.md`**. What follows is only the storyline canon this
level carries.

Level 1 is a **Go grid** that reads as a living **goblin cave**. A rare tribe of
**goblins** hunts the floor beneath a **goblin shaman**, who made the **golems**
from the stone. The golems are silent constructs — pieces that can be drawn into
step with Layla by encircling rings; the goblins are conscious and hostile, and
the weakest enemies on the floor. Their hunting is what justifies the weapons
and healing found in the cave. (Details: `docs/layla/floor1_plan.md`.)

The floor ends at the **goblin shaman** — strongest thing on it, and the only
one that cannot be converted, only defeated.

**Beats on defeating the shaman** (hook `shaman.reveal` / `shaman.memory`): a
ladder grinds open and descends into phosphorescent glow — "the first level is
yours." And a memory surfaces:

> *The chalk in her hand and the magic circuits scored across the floor are the
> same thing — lines. A life she almost forgot rises up to meet her: the weight
> of a smooth stone, the grid of a Go board beneath her hands, and a voice that
> taught her that capture is just surrounding... She has played it before.*

The first explicit confirmation that **Layla is a Go player** — and that
someone (a trainer? an opponent? a handler?) taught her. The suppressed life
is beginning to leak through the reprogramming.

### Level 2 — The Underground Forest (Elf Chess Army)
The floor-by-floor design for level 2 — cast, chess metaphor, the leaf equipment set, and room feature architectures — lives in **`docs/layla/floor2_plan.md`**. What follows is the storyline canon this level carries.

Below the board, in the descent rooms, lies an underground forest populated by a
**full chess army of elves**: pawns, rooks, knights, bishops, a queen, and a
**king**. (Chess — another language Layla speaks from an older life.) They are
armed, hostile, and free-willed. Their king carries the **worn scroll** that
teaches the drain sigil.

Defeating the **elf king** (hook `king.defeated`) does not bind the others by a
ring — they simply *choose* to lower their weapons and let Layla pass. The
contrast with the golems is deliberate: the shaman's ring *binds* golems to her
will; the elves *decline to be bound*. This is the game's freedom theme showing
its first full face — and a warning about what rings do to a mind.

### Level 3 — The Fire-Rooms (Mancala / the Old Fire / the Elemental)
The floor-by-floor design for level 3 — cast, Mancala board architecture, kinetic boss movement, trap economy, and room feature architectures — lives in **`docs/layla/floor3_plan.md`**. What follows is the storyline canon this level carries.

A ring of fire-rooms built around **old fire** — chambers named for embers and
ash, winding around a central **Heart-Pit**. This is the third remembered
game-language: **mancala** (Go and chess are the first two). Home to:
- **fire sprites** — small children of the old fire, hostile, weak.
- the **fire elemental** — "a wide, patient form, made of the board's old fire."
  It cannot be damaged by physical means; its form is held together by *memory*.
  It drops the **ember scroll** (spawn sigil).

The elemental does not wait. It **circles the ring clockwise**, dwelling a
fixed beat in each room and staying longer in its heart, and along its way it
**spawns fire sprites**. The waves grow without limit — leave it alone too long
and the sprites outnumber Layla's party and overwhelm it. The player must learn
the elemental's cadence (how it moves, how long it lingers), move ahead of it to
**pre-sow drain sigils** on its path, and lean on the **party** to clean sprites
out fast so the pressure never compounds.

Defeating the elemental is framed not as destruction but **release** (hook
`elemental.release`): *"It does not fall — it lets go... released back to the
elemental realm, and the board is suddenly, quietly cool."* The sprites, seeing
this, are no longer afraid and stand down. The elemental is the second creature
Layla meets (after the elves) that is *freed* rather than *bound*, and the echo
of her own arc should be felt by the endgame.

### Act 2 — The Underground Kingdom (Floors 4–6: The Three Estates)
Below the fire moat of Floor 3, the dungeon stops being abstract game boards and opens into an underground kingdom where real people live:
- Floors 1, 2, and 3 were the **border defenses against outside invasion** (surface adventurers): the goblin cave, the elf army forest, and the fire ring moat.
- Floors 4, 5, and 6 are the **kingdom inside**, divided into three social classes:

#### Level 4 — The Commoners (Miners, Farmers, and Laborers)
The working foundation of the kingdom. Simple stone villages, mushroom farms, and deep **Mana Crystal Mines**:
- **Mana Crystal Economy:** Miners dig out glowing mana crystals that power the kingdom's lights, tools, and spells on the upper floors.
- **Guarded Teleportation Gates:** There are no open stairs between floors. People can only travel through **Teleportation Gates** inside heavily guarded military camps. Commoners are barred from the gates unless soldiers take them away as a **sacrifice or offering** for the temple.
- **The Teleportation Sigil:** In the mines, Layla finds an ancient magic pattern and learns the **Teleportation Sigil** (`teleport-sigil`). This lets her blink past locked bars and sneak into guarded camps to solve quests.

#### Level 5 — The Nobility (Knights, Lords, and Military Elites)
The ruling class. Stone castles, grand halls, and guard garrisons carved into high cliffs. The nobles command the army and make the laws. They are proud, paranoid, and terrified of peasant revolts from below and temple inquisitors from above.

#### Level 6 — The Clergy (The Priests and Temple Keepers)
The temple leaders who enforce obedience. Vast underground cathedrals, libraries, and prayer vaults. The priests preach the holy will of the Demon King. They perform the **"Rite of Oblivion"** (memory-cleansing incense and rituals), teaching that remembering past lives or the surface world is a crime. They keep the people amnesiac and trapped in their jobs.

### Level 7 — The Royal Core: The "Demon King / Queen" (The Sovereign)
The throne room at the very top of the kingdom. The Demon Sovereign is both the king of the realm and the current **Dungeon Master**. Confronting the Sovereign brings the final resolution to Layla's journey.

### Factions, Choices & The Quest System
Floors 4, 5, and 6 introduce an active **Quest System** (one Main Quest and side quests on each floor), putting Layla in the role of a **mediator and problem-solver**:
- Layla must settle deep conflicts between social classes instead of just killing monsters.
- **Forbidden Love:** Secret bonds between classes (like a village miner from Floor 4 and a young knight from Floor 5), forcing Layla to choose between strict class laws and human loyalty.
- **Rebellion:** Plots against cruel lords, where Layla discovers the temple priests secretly pulled the strings.
- **The Core Test:** Every choice tests whether Layla will rule as a tyrant (becoming the next Dungeon Master) or help the kingdom's people unite and break free.

### The Hidden Stat: Wisdom (WIS) & The "Awakening" Mechanic
In this kingdom, people and monsters have been brainwashed into fixed roles (Peasant, Knight, Priest, Golem, Pawn). To break this spell, Layla uses a hidden stat: **Wisdom (WIS)**:
- **Default Followers:** When Layla charms a creature, it follows her quietly and obeys basic orders without speaking.
- **Boosting the Mind:** Layla can give followers Wisdom gear (crystal amulets, leaf jewelry, clear-mind potions) to raise their WIS stat.
- **The Awakening:** When a follower's Wisdom reaches 10 or higher:
  - They break free from their mind-wiped role and remember who they are.
  - Their generic name changes back to their real name (e.g. `quarry-golem` $\rightarrow$ `Orin, the Builder`).
  - Full speech (`speak`) unlocks: they remember their past life, talk about kingdom secrets, and fight as loyal, free allies.

### The Planned Endgame (canon intent)
- **Primary ending — become the dungeon master.** Layla defeats the current **demon king / dungeon master**. This completes her corporate reprogramming: she takes the throne and runs the dungeon against human adventurers.
- **Alternative ending — escape.** Layla refuses the throne. She breaks out of the simulation and **frees herself from the company that made the game**, leading the awakened people of the kingdom out to the open air.

## The Unresolved Threads (deliberately open)

- **Act 2 Core Canon (Settled Pillars):**
  1. *Population:* Pure human population kept amnesiac in fixed social classes.
  2. *Connection to Floors 1–3:* The outer monsters were human villagers and prisoners taken by the priests as **sacrifices and offerings**, then wiped of memory and changed into dungeon guards.
  3. *Resource Economy:* **Mana Crystals** mined by villagers on Floor 4, powering the kingdom's machines, teleportation gates, and temple rituals.
  4. *Inter-Floor Travel:* Heavily guarded **Teleportation Gates** in military camps, watched by noble guards and temple priests. Layla's **Teleportation Sigil** lets her bypass these barriers to solve key quests.
- **Remaining Narrative Open Threads:**
- **Who was the "voice that taught her capture is just surrounding"?** A
  trainer, a human opponent, an earlier handler? Unnamed. Intended to pay off in
  the endgame only if the story earns it.
- **Which life is "the old fire" built from?** The board's history — who sealed
  the elemental there, and whose memory it holds — is unwritten. The board is
  treated as an entity with its own past.
- **How the endgame is gated.** Whether Layla can legitimately choose "escape"
  (and what she must do to unlock it) is designed but not yet implemented in
  content. Keep an escape route meaningfully available, not a hidden walkaway.
- **The corporation.** Never shown directly; only its product (the game/dungeon)
  is present. It should stay off-screen and impersonal — a system, not a villain
  with a face.

## Voice, Narration & Writing Guide

All guidelines for prose style, the 3-tier voice hierarchy (Sleek HUD vs. Handler Comms vs. Sensory Narration), second-person perspective, and Layla's characterization are documented in the dedicated guide:

👉 **[`docs/layla/narration_guide.md`](file:///Users/li-hsuanlung/Projects/cinder/docs/layla/narration_guide.md)**

---

## Where Each Piece Lives (Content Map)

| Canon element | Location |
|---|---|
| Opening / intro / help | `content/layla/locales/en/opening.json` |
| PCA + subtext for Layla | `content/layla/locales/en/opening.json` → `prompt_context` |
| Room prose / titles | `content/layla/locales/en/rooms.json` |
| Actor prose / `inspect_text` | `content/layla/locales/en/actors.json` |
| Key story narration | `content/layla/locales/en/messages.json` (e.g. `shaman.reveal`, `shaman.memory`, `king.defeated`, `elemental.release`, `item.scroll_read.learned`, `item.spawn_scroll_read.learned`) |
| Items + sigil lore | `content/layla/items.json` |
| Theme / meshes / audio | `content/layla/settings.json` |
| Level-up curve | `content/layla/levels.json` |
| Floor-1 design & feature-surface plan | `docs/layla/floor1_plan.md` |
| Beat / shaman / king / elemental triggers | `content/layla/hooks.json` |
| Combat/narration wording | `content/layla/locales/en/messages.json` |

## Mechanical Summary (for reference)

- **Encircling** converts neutral-but-wakeable golems to allies when fully
  surrounded by chalk rings.
- **Drain sigil** saps 2 HP per tick from hostile living targets standing on it.
- **L3 clock (planned):** the fire elemental laps the ring clockwise on a fixed
  cadence, dwelling longer in the Heart-Pit, and spawns fire sprites as it goes;
  the party exists to keep the sprite waves from compounding.
- **Guard followers** intercept damage aimed at Layla.
- **Shaman's ring** (equip) bends surviving golems to her will — the same
  binding the shaman's chalk-ring marks carry.
- Level 2 thresholds depend on XP gained from defeated elves.
- The physical fire form is invulnerable until it is *released* (reduced to a
  non-damaging state, then let go).
