# Producer Cards Specification: AERA

> *"A producer does not write the script. A producer adjusts the temperature of the room until something boils over."*

---

## 1. Card Architecture & Data Schema

Each card in *Aera* is represented as a structured data object that can be authored in JSON and executed by the Cinder menu/event engine.

### Card Types
1. **`event`**: Macro-level interventions played during Phase 1 (Morning Call Sheet). Changes house rules, logistical constraints, or group activities.
2. **`relationship`**: Micro-level interventions played during Phase 2 (Afternoon Surveillance). Targeted nudges delivered to an individual housemate to provoke or comfort them.

### Data Schema Example
```json
{
  "id": "grocery-run",
  "category": "event",
  "name": "Grocery Run on a Budget",
  "flavor": "Slash the weekly house grocery allowance in half. Send two housemates to negotiate dinner ingredients on a shoestring budget.",
  "assignment": {
    "type": "pair",
    "prompt": "Select two housemates to send on the grocery run:"
  },
  "mechanics": {
    "hunger_tick_rate": 1.2,
    "mutual_stat_delta": {
      "connection": 2,
      "safety": -1
    },
    "trigger_beat": "day-event-grocery-run"
  }
}
```

> [!NOTE]
> For the complete vocabulary of autonomous actions (`move`, `act`, `inspect_feature`, `speak`, `confess`) that housemates execute in response to cards, see [Actor Actions & Reality-Show Autonomy](file:///Users/li-hsuanlung/Projects/cinder/docs/aera/actions-and-autonomy.md).

---

## 2. Starter Deck: 8 Event Cards (Macro / Environmental)

These cards are drawn during **Phase 1: Morning Call Sheet** (Draw 3, Pick 1).

### 1. `grocery-run`: "Grocery Run on a Budget"
* **Flavor**: *“Slash the weekly food budget by 50%. Force two housemates to negotiate the pantry on a shoestring.”*
* **Assignment**: Pick any 2 housemates.
* **Dramatic Effect**: Isolates the pair away from the house for 2 hours. If paired with Ren (the chef), his perfectionism will clash with budget constraints.
* **Stat Shifts**: Pair `connection` +2; if budget causes an argument, `safety` -2.

### 2. `dinner-from-scratch`: "Three-Course Collaboration"
* **Flavor**: *“Ban takeout and instant food for tonight. Dinner must be prepared from raw ingredients by a designated pair.”*
* **Assignment**: Pick any 2 housemates (one must be Ren unless Ren is on strike).
* **Dramatic Effect**: High kitchen intimacy. Cooking shoulder-to-shoulder under time pressure.
* **Stat Shifts**: Pair `attraction` +2, `confidence` +1. If Ren is paired with someone who can't cook (like Mio), friction rises.

### 3. `maintenance-blackout`: "Wi-Fi & AC Failure"
* **Flavor**: *“Kill the house Wi-Fi and air conditioning for three hours. No phones, no Netflix, no escaping to private bedrooms.”*
* **Assignment**: Affects all 4 housemates.
* **Dramatic Effect**: Forces everyone out of their rooms onto the breezy patio or lounge floor. Eliminates solo distractions.
* **Stat Shifts**: All actors `stamina` -1, group `connection` +1, lounge tension +2.

### 4. `the-living-room-game`: "Two Truths & A Brutal Lie"
* **Flavor**: *“Leave a sealed envelope with a provocative parlor game on the coffee table alongside two bottles of chilled Pinot Noir.”*
* **Assignment**: Affects all 4 housemates.
* **Dramatic Effect**: Everyone gathers after dinner. Alcohol and structured personal revelations break down barriers.
* **Stat Shifts**: Group `attraction` +2, group `safety` -2 (high risk of someone taking offense).

### 5. `cleaning-audit`: "The Chore Rota Crackdown"
* **Flavor**: *“A notice from the landlord: the kitchen counter and shared bathroom failed inspection. A deep clean is mandatory.”*
* **Assignment**: Pick 2 housemates for cleaning duty.
* **Dramatic Effect**: Tests domestic compatibility. Daichi (disciplined) vs. anyone messier creates instant passive-aggressive friction.
* **Stat Shifts**: Pair `safety` -3, `confidence` -1, Drama Score +3.

### 6. `patio-workout`: "Sunrise Cross-Training"
* **Flavor**: *“Schedule an early 7:00 AM fitness session on the patio before breakfast.”*
* **Assignment**: Pick 2 housemates (Daichi + 1 partner).
* **Dramatic Effect**: Physical vulnerability, sweat, competitive teasing, and early morning endorphins.
* **Stat Shifts**: Pair `stamina` -2, pair `attraction` +2, `confidence` +2.

### 7. `mystery-gift`: "The Unaddressed Bouquet"
* **Flavor**: *“Deliver a bouquet of fresh ranunculus and a boutique pastry box to the front door with no recipient card.”*
* **Assignment**: Drops at `entrance`; first actor to find it brings it to the lounge.
* **Dramatic Effect**: Sparks widespread speculation. Who sent it? Who was it meant for? Mio assumes it's for her; Aera looks away.
* **Stat Shifts**: House-wide `attraction` suspicion +3; Mio `confidence` +2.

### 8. `studio-screening`: "Raw Footage Review"
* **Flavor**: *“Invite two housemates into the editing studio to review B-roll footage from Day 1.”*
* **Assignment**: Aera (the editor) + 1 partner of player's choice.
* **Dramatic Effect**: Puts Aera in her comfort zone and lets her partner see her analytical, passionate side.
* **Stat Shifts**: Aera `confidence` +3, pair `connection` +3.

---

## 3. Starter Deck: 8 Relationship Cards (Micro / Targeted Nudges)

These cards are drawn during **Phase 2: Afternoon Surveillance** (Draw 3, Pick 1). They represent whisper cues or producer prompts fed to individual actors.

### 1. `the-third-wheel`: "The Well-Timed Interruption"
* **Flavor**: *“A private two-person moment is getting too comfortable. Send a third person in with fresh tea.”*
* **Targeting**: Target Actor A; send them into Room where Actors B and C are talking.
* **Dramatic Effect**: Snaps romantic momentum; triggers awkward pauses and hidden jealousy.
* **Stat Shifts**: Actor B/C `attraction` pauses; Actor A `safety` -1; Episode Drama Score +2.

### 2. `vulnerability-push`: "Ask About the Ex"
* **Flavor**: *“Prompt a housemate via floor PA: 'Ask them why their last serious relationship ended.'”*
* **Targeting**: Target Actor A (to ask Actor B).
* **Dramatic Effect**: High risk/reward. Breaks polite surface conversation; delves into personal trauma.
* **Stat Shifts**: If target `safety` > 0: `connection` +3. If target `safety` < 0: `safety` -3 (defensive shutdown).

### 3. `flirt-with-intent`: "The Specific Compliment"
* **Flavor**: *“Whisper cue: 'Don't just say thank you—tell them specifically what you like about the way they carry themselves.'”*
* **Targeting**: Target Actor A (to compliment Actor B).
* **Dramatic Effect**: Deepens romantic tension and spikes reciprocal interest.
* **Stat Shifts**: Actor B `confidence` +2; Pair `attraction` +3.

### 4. `the-cold-shoulder`: "The Sudden Retreat"
* **Flavor**: *“When the conversation gets sweet, cue Actor A to make an excuse and leave the room immediately.”*
* **Targeting**: Target Actor A (who is currently with Actor B).
* **Dramatic Effect**: Leaves Actor B hanging and overthinking. Classic unscripted dating tension.
* **Stat Shifts**: Actor B `attraction` +2 (mystery), Actor B `confidence` -2.

### 5. `air-the-grievance`: "Don't Let It Slide"
* **Flavor**: *“Floor nudge: 'You looked really annoyed when they said that. Why didn't you speak up?'”*
* **Targeting**: Target Actor A (toward Actor B).
* **Dramatic Effect**: Provokes an honest confrontation about a domestic slight or passive-aggressive comment.
* **Stat Shifts**: Pair `safety` -4; Drama Score +4; Confessional quote unlocked.

### 6. `acts-of-service`: "Make Them a Coffee"
* **Flavor**: *“Suggest to an actor that they quietly prepare a pour-over coffee or tea and bring it to someone working alone.”*
* **Targeting**: Target Actor A (to serve Actor B).
* **Dramatic Effect**: Tender, quiet intimacy. Very *Terrace House*.
* **Stat Shifts**: Pair `safety` +2, Pair `connection` +2.

### 7. `the-confidante`: "The Bedroom Whisper"
* **Flavor**: *“During evening downtime, encourage Actor A to ask their roommate what they really think of the others.”*
* **Targeting**: Target Girls' Room (Aera & Mio) or Guys' Room (Ren & Daichi).
* **Dramatic Effect**: Unlocks the late-night roommate debrief where true romantic targets are revealed.
* **Stat Shifts**: Roommate `connection` +3; reveals hidden attraction vectors to the producer.

### 8. `the-hot-seat`: "Put Them on the Spot"
* **Flavor**: *“Prompt during dinner: 'Ask Daichi what he thinks of Mio's career, point-blank.'”*
* **Targeting**: Target Actor A (to challenge Actor B in front of everyone).
* **Dramatic Effect**: Public exposure; everyone at the dining table stops chewing to listen.
* **Stat Shifts**: Target `confidence` -2; Audience Buzz +3; Golden Soundbite unlocked.

---

## 4. Confessional Prompts & Soundbite Generation

When the player summons an actor with `confessional <actor>`, the player chooses 1 of 3 interview postures:

| Confessional Posture | Focus | Player Goal | Typical Soundbite Extracted |
| :--- | :--- | :--- | :--- |
| **The Empathic Ear** | Emotional reassurance | Restore `safety`, gain honest confession | *"I came here hoping people would look past my photos... but maybe that's all I am to them."* (Mio) |
| **The Shit-Stirrer** | Probing other cast members | Generate friction, extract spicy soundbite | *"Ren acts nice, but he's measuring every single thing you do. It feels like an audition, not a home."* (Aera) |
| **The Reality Check** | Confronting bad behavior | Shift `confidence`, push self-reflection | *"I know I was rigid about the schedule. I just don't know how to exist without one."* (Daichi) |
