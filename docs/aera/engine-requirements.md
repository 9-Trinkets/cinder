# Engine Requirements & Execution: AERA

> *"How the personified crew structure makes Aera realistic to author and execute within Cinder without nuance overload."*

---

## 1. Why the Crew Structure Solves the Execution Challenge

The primary risk with simulation games is **nuance overload**: trying to simulate too many continuous variables, freeform video clipping, or complex UI subsystems that bog down both the engine and the player.

By personifying the producer levers into **Maya (Network)**, **Chika (Floor PA)**, and **Toby (Editor)**, the technical implementation in `cinder` becomes remarkably clean:
1. **No complex timeline UI**: Toby presents the cuts as clear, high-level choices within Cinder's existing choice menus.
2. **No arbitrary scoring math**: Ratings are evaluated against Maya's explicit morning mandate (Conflict vs. Romance vs. Buzz).
3. **No parser guessing**: The player interacts through dialogue choices and card options with their crew, not cryptic console commands.

---

## 2. Technical Architecture in Cinder

```
┌────────────────────────────────────────────────────────┐
│                   CINDER CORE SYSTEMS                  │
├────────────────────────────┬───────────────────────────┤
│ 1. Crew Dialogue Beats     │ 2. State & Flag Tracking  │
│    • Maya (09:00 Briefing) │    • active_mandate       │
│    • Chika (15:00 Radio)   │    • morning_event        │
│    • Toby (22:00 Edit Bay) │    • afternoon_nudge      │
├────────────────────────────┼───────────────────────────┤
│ 3. Autonomous House Beats  │ 4. Stat & Safety System   │
│    • Housemate interactions│    • confidence / stamina │
│    • Prop affordances      │    • connection / safety  │
└────────────────────────────┴───────────────────────────┘
```

---

## 3. Data & Content Specification

### A. The Control Room & Crew (`rooms.json` & `actors.json`)
* Add the **Control Room / Video Village** as the player's primary hub:
  * `control-room`: The air-conditioned truck outside the house. Features: `switcher-board`, `multi-monitors`, `comms-headset`.
  * `edit-bay`: The studio room where Toby cuts footage.
* Add the **3 Crew Members** as actors:
  * `maya`: Network liaison.
  * `chika`: Floor PA (walks between backstage and house).
  * `toby`: Senior editor.

### B. Daily Menu Choices (`menus.json`)
Each day needs only three clean choice menus:
1. **`menu-day1-morning-mandate`**:
   - Choice 1: Event Card A (e.g., *Grocery Budget Squeeze*). Sets `morning_event = "grocery-run"`.
   - Choice 2: Event Card B (e.g., *AC & Wi-Fi Blackout*). Sets `morning_event = "blackout"`.
2. **`menu-day1-afternoon-nudge`**:
   - Choice 1: Relationship Card A (e.g., *Stir Conflict*). Sets `afternoon_nudge = "confrontation"`.
   - Choice 2: Relationship Card B (e.g., *Foster Romance*). Sets `afternoon_nudge = "comfort"`.
3. **`menu-day1-night-cut`**:
   - Choice 1: *Cut A: "Recipe for Disaster" (Conflict framing)*. Evaluates against `active_mandate`.
   - Choice 2: *Cut B: "Afternoon Tea" (Romance framing)*. Evaluates against `active_mandate`.

### C. Stat Updates & Night Resolution Hook
When a cut is selected in `menu-day1-night-cut`, a simple hook or beat effect:
1. Compares `selected_cut.type` to `active_mandate`.
2. Computes the morning rating string and tweets.
3. Applies delta to housemates:
   - If Conflict cut chosen $\rightarrow$ Ren `safety` -4.
   - If Romance cut chosen $\rightarrow$ Daichi/Mio `connection` +3.

---

## 4. Why This Keeps the Project Scope Grounded

* **Authorable**: You don't need to write hundreds of branching edge cases. Each day has:
  - 1 Morning briefing conversation.
  - 1 Midday autonomous scene + PA radio exchange.
  - 1 Night editing conversation + Confessional.
  - 1 Morning resolution report.
* **Tested & Verified**: Because choices use Cinder's native menu and beat mechanisms, playthroughs can be fully covered by automated regression tests in `cinder-core/tests/aera_*.rs`.
* **Action & Autonomy Reference**: See [Actor Actions & Reality-Show Autonomy](file:///Users/li-hsuanlung/Projects/cinder/docs/aera/actions-and-autonomy.md) for the complete housemate action taxonomy, speech intent mechanics, and verified integration test scenarios.
