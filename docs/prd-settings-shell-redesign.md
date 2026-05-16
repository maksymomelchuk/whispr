# PRD — Settings shell redesign (post-Hotkeys)

## Problem Statement

The settings shell has eight pages (Home, General, Hotkeys, Transcription, Modes, Snippets, History, Stats). They have drifted into different visual languages — the Modes page is calm and content-led, while older pages mix heavy headers, divider lists, modal editors, and ad-hoc card recipes. The Hotkeys page was just rebuilt to use Modes as a baseline and push it further (keycap chips, inline recording, live armed-dot, recently-edited flash), and the result is the standard the rest of the shell should rhyme with.

The risk going forward is symmetrical:

- If the remaining pages stay as they are, the app reads as seven different tools stitched into one sidebar.
- If every page is blanket-cardified, the app collapses into monotony — same chassis, same cards, same hero.

The user has explicitly opted to keep iterating. They want the shell to feel cohesive without flattening pages whose shape isn't a list.

## Solution

A staged redesign anchored by three extracted UI primitives. Each page receives a treatment that **matches its shape**, not a uniform card pass.

- **List pages** (Snippets, History) → adopt the Modes/Hotkeys recipe: row cards + section header + inline editor + save-flash. Each list page differentiates by hero element so they don't visually merge.
- **Form pages** (General, Transcription) → keep field-based layout, adopt only the section header so the chrome rhymes with list pages without faking item-ness.
- **Surface pages** (Home, Stats) → keep their distinct identity; borrow primitives only where semantically justified.

Done in this order, with the primitives extracted before any per-page work begins.

## User Stories

1. As a power user, I want every settings page to feel like one app, so that I don't have to relearn the visual vocabulary as I navigate.
2. As a power user, I want list-shaped pages (Modes, Hotkeys, Snippets, History) to share a row chassis, so that the same gestures (hover, edit, remove, save-flash) work the same way everywhere.
3. As a power user, I want each list page to have its own focal element (mode name, keycap, trigger word, timestamp), so that pages remain distinguishable at a glance.
4. As a Snippets user, I want to edit a snippet in place on its row, so that I don't lose context to a side-sheet modal every time I tweak a trigger.
5. As a Snippets user, I want a clear empty state on Snippets that telegraphs what the page is for, parallel to the empty-mode placeholder in Hotkeys.
6. As a Snippets user, I want a brief visual confirmation when I save a snippet, so that I know my edit was persisted without watching the toast area.
7. As a History reader, I want entries grouped by day with a quiet section header, so that I can scan when things happened.
8. As a History reader, I want a brand-new entry to flash in when a dictation lands, so that I trust the page is live and not stale.
9. As a History reader, I want each entry to lead with timestamp + duration, so that the chronology is the hero, not the transcript text.
10. As someone tuning General settings, I want section headers between groups of fields, so that the page rhymes with Hotkeys without pretending fields are list items.
11. As someone tuning Transcription settings, I want the same section-header treatment, so that the two form pages stay consistent.
12. As someone on the Home surface, I want the current PTT shortcut shown using the same keycap chips as the Hotkeys page, so that the canonical answer is visible without a click.
13. As any user, I want `prefers-reduced-motion` honored, so that the flash and armed-dot animations don't fight my OS preference.
14. As any user, I want WCAG AA contrast preserved across the new accents, so that the new state colors stay readable.
15. As a developer adding a future settings page, I want a `RowCard` / `SectionHeader` / `useFlash` primitive set, so that I don't drift a new card recipe.
16. As a developer maintaining shortcut display, I want a single `Keycap` + `ShortcutKeycaps` component, so that any future page (Home, command palette) renders chords the same way.
17. As a developer wiring live-state visualization, I want a consistent pattern (page subscribes to a Tauri event, derives a "live id," lights matching rows), so that the Hotkeys precedent is repeatable.
18. As a reviewer, I want the changes staged page by page in order, so that each PR is independently reviewable and reversible.

## Implementation Decisions

### Already shipped (context — do not re-do)

These changes are already in the working tree on `main`-track work:

- **HotkeysPage redesigned** to use the Modes card-row chassis with three pushed-further moves: keycap chips for shortcuts, inline `RecordingRow` replacing the modal recorder, `ArmedDot` driven by PTT events, recently-edited flash.
- **Rust `ptt-pressed` event** now emits the matched `Shortcut` as payload (was `()`). Consumers that ignore the payload (`OverlayApp`, the legacy `usePtt` callers) are unaffected.
- **`usePtt` hook** returns `{ isHeld, activeShortcut: Shortcut | null }`.
- **`src/components/ShortcutRecorder.tsx`** deleted — the modal is replaced by `RecordingRow` inline.

### Module plan

**New primitives (no UX change on their own — extract first):**

1. **`SectionHeader`** — leading mono index (`01`, `02`, …) + title + optional `Default` / status badge + count drift on the right + faint bottom border. Used today by Hotkeys; will be used by Snippets and History; lightly adopted by General and Transcription.
2. **`RowCard`** — wraps the row chassis: `rounded-[10px] border bg-card pl-3 pr-2 py-2.5 shadow-xs` + hover ring + tone variants. Tones: `neutral`, `destructive` (for conflict), `accent` (for armed/active), `dashed` (for empty placeholders). Encapsulates the recipe so future pages can't drift it.
3. **`useFlash(ms = 700)`** — `{ flash(id: string), isFlashing(id: string) }`. Triggers a row's outline ring for the duration, then fades via the existing `motion-safe:duration-[600ms]` recipe.
4. **`Keycap` + `ShortcutKeycaps`** — promote from `HotkeysPage.tsx` into the shared component set so Home (and any future surface) can render shortcuts the same way.
5. **`lib/shortcut.ts`** — extract the pure helpers (`shortcutKey`, `shortcutsEqual`, `displayKey`, `MOD_LABEL`, `KEY_LABEL`, `isModifierCode`, `collectModifiers`) out of `HotkeyaPage.tsx`. Pure functions are easier to test and these will be used wherever a shortcut is rendered or compared.

**Page modifications:**

- **Snippets** — replace the `Sheet`-based editor with an inline editor row. Each snippet is a `RowCard` showing trigger word in mono on the left and a dimmed truncated replacement preview on the right. Editing is in-place (mirroring `RecordingRow`'s pattern). Adopt `useFlash` on save. Differentiating hero: the **trigger word** in mono.
- **History** — `RowCard` per entry, grouped by day with `SectionHeader` per group. Subscribe to `history-updated` and flash the newly-added entry's row. Differentiating hero: **timestamp + duration** on the left.
- **General** — keep field-based shape. Add `SectionHeader` between logical field groups (Audio · Appearance · etc.). Do **not** cardify fields.
- **Transcription** — same as General. `SectionHeader` between groups (Provider · Cleanup · Dictionary · etc.).
- **Home** — keep `PageHeader`. Optionally display the current default PTT shortcut using `Keycap`. This is the lowest-priority change and only if Home benefits from the live chord display.
- **Stats** — explicitly untouched.

### Pattern: live-state visualization

Hotkeys established the precedent. The pattern is reusable wherever a Tauri event identifies a row:

1. Page subscribes to the relevant event (e.g., `history-updated`).
2. Page derives an identity (a stable id matching one rendered row).
3. Matching row gets the accent treatment (border + tone) and optionally an external indicator dot.

Re-applied for History as the "just-arrived" flash. **Do not** add this pattern to pages where the live signal isn't meaningful (Snippets has no live state worth showing; General is configuration, not telemetry).

### Anti-decisions (commit-by-omission)

- **Stats stays as-is.** Charts are not cards. No `RowCard` pass on Stats.
- **Home keeps its `PageHeader`.** That heavy block was removed from Hotkeys because Modes had none; Home is a landing surface where identity beats density.
- **Form rows do not get the hover ring.** The hover ring is a "this is an actionable item" affordance. Form fields are not items.
- **No numeric indices on History.** Indices are for ordered / addressable collections (modes, snippets); a feed of past events is neither.
- **`PageHeader.tsx` is not deleted yet.** Home still uses it. Re-evaluate only if Home is also redesigned away from it.

### Order of work

1. Extract `SectionHeader`, `RowCard`, `useFlash`, and `lib/shortcut.ts` (with `Keycap` / `ShortcutKeycaps` promoted). Re-wire HotkeysPage to consume the extracted primitives. No UX change.
2. Snippets — port to row cards + inline editor.
3. History — row cards + day grouping + arrival flash.
4. General + Transcription — `SectionHeader` adoption only.
5. Home — `Keycap` of current PTT shortcut (optional).

## Testing Decisions

The frontend has no Vitest / RTL setup today (no `*.test.*` / `*.spec.*` under `src/`). Two paths:

- **Minimum viable** (keep current practice): manual reload + reviewer's eyes. Pure-function correctness is checked at the typecheck level only.
- **Recommended** (small upfront cost, durable): introduce Vitest. Test the pure-function surface from `lib/shortcut.ts` (`shortcutsEqual`, `hasConflict`, `displayKey`) and `useFlash` (timer behavior, motion-reduced fallback). These have known inputs and known outputs and no DOM concerns.

Component snapshot tests for `SectionHeader` / `RowCard` are **not recommended** — they encode visual decisions that are expected to evolve, so snapshots produce churn without protecting behavior.

Prior art: Rust side already has unit tests in `src-tauri/src/ptt.rs` and `groq_session_state.rs`. The bar is "test external behavior, not implementation" — the same bar applies to whatever frontend tests are added.

Decision deferred to the implementer: whether to introduce Vitest now or wait until a real bug demands it.

## Out of Scope

- Stats page redesign.
- Mobile / responsive layout. Whispr is macOS-desktop only.
- New keyboard navigation features beyond what's already wired.
- Accent palette changes (`useAccent` stays as-is).
- Modes page redesign — Modes is the reference and stays put.
- New Tauri events. The only event-payload change (`ptt-pressed`) has already shipped.
- A new `PageHeader` variant. The existing one stays on Home and is removed elsewhere; if a future page needs a heavier header, that's its own PRD.

## Further Notes

- The recipe `outline-ring/45` + `motion-safe:duration-[600ms]` on flashing rows already respects `prefers-reduced-motion`. Reuse it from `useFlash`'s consumers verbatim.
- The bg-tinted "accent" tone on a card uses `bg-ring/[0.04]` and `border-ring/60`. These tokens scale with the per-page `--ring` accent set by `useAccent`, so the live-state tint inherits the user's preferred color automatically. Don't hardcode hues.
- The `Default` badge style from Hotkeys (`text-[9.5px] font-semibold uppercase tracking-[0.08em] px-1.5 py-0`) should ride along on the extracted `SectionHeader` as the canonical "this group is the default" badge.
- Snippets and History both have prior data layers (`getSnippets`/`setSnippets`, history events). No backend changes are required for either page.
- Keep the user's CLAUDE.md comment policy: no WHAT comments, no narration of the current task, no "added for X flow". Comments stay only for non-obvious WHY.
