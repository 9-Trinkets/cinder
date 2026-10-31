# zh-TW localization audit (Layla)

Method: static scans of `cinder-web-ui/src`, `cinder-core`, `cinder-srv` and `content/layla`; the `cinder-tools locale check` report; and a live run of the server with a throwaway account set to zh-TW. The run created a Layla play and issued about 20 commands (help, look, attack, trace, teleport, follow, equip, bad input, and so on), then scanned every string in the API responses for English. The browser extension was not connected, so there was no visual pass of the web UI. Typography findings below come from reading the CSS and components.

`locale check` reports 4096/4096 strings translated. That number only covers the files in `locales/<code>/` and only the keys present in the English files. Most of the leaks below sit outside that coverage.

## 1. Root causes

1. **Several pack files are never localized.** The loader reads `items.json`, `skills.json`, `actions.json`, `settings.json` and `movement.json` straight from the pack root (`content/loader/mod.rs`). Only the files listed in `locales/<code>/` are localized.
2. **UI text falls back to English defaults.** `UiTextDefinition` has about 70 fields with English defaults. `locales/en/ui.json` and `zh-TW/ui.json` define 55 of them, so about 25 keys always render in English in zh-TW. `messages_defaults` has a zh-TW bundle, but `ui` and `system` defaults do not.
3. **User-facing English is hard-coded in Rust.** See section 3.
4. **Pack metadata is not locale-aware.** `GET /api/packs` takes no locale, so title, tagline, description and tags are always English.
5. **The translation tooling cannot see the gaps.** It only walks the keys of the English locale files, and it treats aliases and keywords as structural.

## 2. English visible in the zh-TW UI (from the live run)

| Area | Where it shows | Source |
|---|---|---|
| Action bar labels and panel titles/prompts (`Look`, `Move`, `Trace`, `Attack`, `Look Around`, `Where To?`, `What catches your eye?`, `Choose a direction`, `Who do you want to attack?`, `Select a target`, `Who do you want to talk to?`, `Drop an item`) | action bar, panels | `actions.json`, `skills.json` |
| Item names and descriptions (`magic chalk`, `Charm Sigil`, `charm sigil`, ...) | inventory, look, equip/erase/trace messages (e.g. 「你無法裝備magic chalk。」「抹去Charm Sigil」) | `items.json` |
| Combat and trace narration (`蕾拉 strikes 白石魔像.`, `Layla crouches and draws a line of chalk...`) | transcript | `skills.json`, `items.json` |
| Skill scalar summaries (`Charm bonus: +1`, `Bonus damage: +0`, `Base MP cost`) | skill panel | `skills.json`, `engine/skills.rs` |
| Panel and transcript chrome (`Switch rooms`, `switched to {title}`, `Follow someone`, `following {title}`, `stopped following anyone`, `Choose someone to follow.`, `Switch channels from {}.`, `No additional commands available.`, `On the ground`, `Current focus`, `Relationship status`, `A quiet stretch...`, `No clear relationship shifts yet.`, `Session Ended`) | menus, panels, recap | UI text defaults not overridden in `ui.json` |
| Teleport errors (`You have not learned how to teleport yet.` and 7 more) | transcript | `turn_runner/planning/teleport.rs` |
| Help text (section headings `— Observation —`, `— Conversation —`, command syntax lines) | `help` output | `actions.json` (usage/example/group) |
| Bad-input hint (`試試：look around, go north, talk to companion, attack crawler, ...`) | transcript | `actions.json` examples |
| Direction word in an error (「你從這裡看不到往north的路。」) | transcript | engine message |
| Pack cards and pack page (title, tagline, description, tags) | Library, pack page | `settings.json`, `GET /api/packs` |
| Play list `current_room_name` on a fresh play | Library card | not localized until first load |
| Auth and server errors (`invalid credentials`, `username taken`, `username must be 3–32 characters`, `A running chronicle already exists...`, `invalid or expired token`, `play not found`) | login form, toasts | `routes/auth.rs`, `routes/game.rs`, `auth.rs` |

## 3. Hard-coded English in Rust

Player-visible:
- `cinder-core/src/engine/turn_runner/planning/teleport.rs` (8 messages).
- `cinder-core/src/engine/skills.rs` (scalar summary strings).
- `cinder-core/src/engine/reducer/combat/offscreen_comms.rs` (15 ally radio milestone lines).
- `cinder-srv/src/game_manager/ui/panels/options.rs` (`Erase Sigils`, `Wipe chalk marks from this room`, `Erase Sigil`, `No sigils in this room`, `Max summons active`).
- `cinder-srv/src/game_manager/ui/sidebar.rs` (`Current room`, `No destinations available`, `Explore the floor to unlock deployment positions`).
- `cinder-core/src/engine/runtime/menus/active_menu.rs` (`Confirm selection`).
- `cinder-srv/src/routes/auth.rs`, `routes/game.rs`, `auth.rs` (API error strings).

Not user-facing (log and internal errors such as `failed to lock runtime state ...`): left as is.

## 4. Gameplay-affecting gaps

- **Chinese cannot be typed as commands.** The parser is a deterministic phrase matcher, English only, and it requires a space after the verb (`parse.rs`). All phrases and aliases are English, and the zh-TW actor and room aliases are English-only, so typing 「魔像」 never resolves a target.
- **Beat keywords are English-only.** Quests that advance on spoken `keywords` and `reply_keywords` (for example `teleport`, `scroll`, `floor 5`) will not advance when the player speaks Chinese. 26 + 23 entries in `beats.json`.
- **Teleport anchor aliases** are English-only.

## 5. Web UI

- `<html lang>` stays `en` for every locale. In Chromium on macOS and Windows, CJK text under `lang="en"` can pick a Japanese or Simplified font face.
- No CJK font stack in `index.css` or the theme defaults.
- 48 uses of `uppercase`, 66 of `tracking-*`, 15 of `italic` and 57 of `font-mono` on text that is Chinese in zh-TW. Letter-spacing widens CJK text unevenly, synthetic italic distorts CJK glyphs, and the mono stack pairs poorly with CJK.
- `utils/grouping.ts` still matches the English subtitles `Permanent Anchor`, `Temporary Anchor`, `Ephemeral Anchor`.
- `<title>` and the login footer product name are not locale-aware.

## 6. zh-TW translation quality

`cinder-web-ui/src/i18n/strings.ts` (241 keys) is complete but literal:
- 「密語」/「通關密語」 for Password (should be 密碼); 「讀者名」, 「檔案館通行」 and 「卷首 • 通行」 for the login chrome read as invented jargon.
- 「編年史」 for a playthrough (「開始編年史」, 「{count} 卷編年史」, 「離開編年史？」) is stiff for a game save; 「逐字稿」 for the transcript is a transcription term, not a play log.
- 「生命徵象」, `Lv`, `XP`, `SP` and `OK` left in Latin; 「閱讀」 for a play session.
- Several strings repeat across `game.*`, `hud.*` and `pack.*` with identical text.

`content/layla/locales/zh-TW`: sampled files read acceptably but carry English-structure phrasing in places (for example 「你不確定要怎麼「X」」). No untranslated prose was found in the locale files beyond aliases, keywords, ids and signals.

## 7. Plan

1. Loader: add a locale overlay for `items`, `skills`, `actions`, `settings`, `movement`; make `GET /api/packs` locale-aware.
2. Bundle zh-TW UI-text and system defaults; fill the missing keys.
3. Move the hard-coded Rust strings into message tables with zh-TW defaults.
4. Parser: allow CJK phrases without a following space; add zh-TW phrases, aliases and beat keywords.
5. Web: `lang` sync, CJK typography, grouping fix, rewrite `strings.ts` zh-TW wording, error-code mapping for auth/server errors.
6. Re-run the live audit and the tests.

## 8. Resolution

Done in this pass:
- **Loader overlays** (`content/loader/overlay.rs`): `items`, `skills`, `actions`, `settings` can be localized by a partial file at `locales/<code>/<file>.json`, merged by `id`; `phrases` and `aliases` are unioned so English commands keep working. `GET /api/packs` takes `?locale=` (or the account preference) and returns localized pack metadata.
- **Bundled zh-TW defaults** for UI text (`ui_defaults.zh-TW.json`, all 76 fields) and the 11 prompt/label keys that were English-only (`system_defaults.zh-TW.json`).
- **Hard-coded Rust text moved to message tables** (`teleport.*`, `skill.summary.*`, `comms.*`, `help.group.*`, `ui.menu_*`), with zh-TW entries. `Session Ended`, follow/switch transcript lines and the other panel chrome now come from the zh-TW UI defaults.
- **Chinese command input**: CJK verb phrases can be followed directly by their argument (`查看魔像`), a bare verb resolves to its action (the planner then asks for the missing argument), and zh-TW ships phrases for every action, zh aliases for rooms, exits, actors and teleport anchors, and zh beat keywords.
- **Layla zh-TW content**: item, skill, action and settings overlays (labels, descriptions, panel titles and prompts, event narration, defeat text, help and usage text), terminology fixes (傳送信物, 故事紀錄) and number/unit spacing in combat messages.
- **Web UI**: `<html lang>` follows the UI locale, CJK-aware tracking/italic/line-height rules, pack list refetches on locale change, server error strings mapped to localized messages, and `strings.ts` zh-TW rewritten away from the library/chronicle jargon.
- **Tooling and tests**: `cinder-tools locale check` now also reports overlay-file coverage; new tests assert that zh-TW UI text, items, skills, actions and settings contain no English and that Chinese phrases load beside the English ones.

Visual pass (headless Chromium driven by Playwright, since the browser extension was unavailable; desktop 1280px and mobile 390px; login, library, pack page, game, the look/move/trace/attack/talk panels, menu, language picker, quests, folio): after the fixes below no Latin-letter text remains on any screen except the `English` language label and the other packs' own English metadata.
- Language badge showed the raw code (`尚無ZH-TW版本`); it now uses the language name.
- Pull-quote taglines use 「」 in Chinese; 10-11px eyebrow labels are bumped to 12px for CJK.
- The `12-hour` clock (`1:00 AM`) is shown as 凌晨 1:00 / 下午 3:00.
- Handler/comms headers read `通訊 • 通訊` because the speaker split only recognized an ASCII colon; it now accepts `：`.
- Clicking a panel option echoed the raw command (`attack golem-pale-se`) in the transcript, and the server stored that text; the UI now sends a readable echo (`攻擊 白石魔像`) that is also what the transcript saves.
- Lists (exits, items, people, party, drops) joined with `, ` and `and`; they now use `、` and `和` in Chinese through `list.*` messages.

Still open:
- Isla still shows English pack metadata in zh-TW (it ships no zh-TW locale; Ella now has a settings overlay).
- The book-recommender flow (Isla) still contains English strings.
- Layla's Orva is now 奧爾娃 in the game zh-TW, matching the novel.
- Typed give commands still need the English word `to` (`給予 藥水 to 小宰德`).
