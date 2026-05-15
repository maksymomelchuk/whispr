# Whispr — Implementation Plan

## Stack

- **Frontend**: React 19 + TypeScript, Vite 7
- **Desktop**: Tauri v2 (macOS target)
- **UI library**: Tailwind v4 + shadcn/ui + lucide-react (adopted in Phase 1)
- **Routing**: react-router-dom (adopted in Phase 1)
- **Toast**: sonner (adopted in Phase 1)

## Architecture Decision: Tailwind v4 + shadcn/ui Adoption (Phase 1)

The original plan was to build the settings UI with hand-rolled CSS only (no Tailwind, no component library). This decision has been reversed in Phase 1.

**Rationale**: The sidebar shell redesign requires:
- Left sidebar (240px) + detail panel layout with collapse/expand animation
- Focus rings, keyboard nav, active-state highlighting across 6 destinations
- A consistent dark/light theme token system consumed by new shell components

Hand-rolling these primitives would require ~2000 more lines of CSS and rebuild accessibility primitives (focus rings, keyboard nav, sidebar collapse) from scratch. Tailwind v4's CSS-first config (`@theme` directive + Vite plugin) keeps the dependency footprint smaller than the v3-era setup: no `tailwind.config.js`, no `postcss.config.js`.

**What changed**:
- `tailwindcss` + `@tailwindcss/vite` added as devDependencies
- `lucide-react`, `react-router-dom`, `sonner` added as dependencies
- `src/globals.css` defines shadcn CSS-variable tokens (light + dark) and maps them to Tailwind color utilities via `@theme inline`
- `dark:` variant is class-based (`@custom-variant dark (&:is(.dark, .dark *))`), toggled by `useTheme` via the `.dark` class on `<html>`
- shadcn/ui components copy-pasted into `src/components/ui/` (button, card, separator, sonner) — only what Phase 1 needs
- `App.css` stays in place for Phase 1; existing component styling is untouched

## Phase Roadmap

### Phase 1 (current) — Shell redesign
- [x] Sidebar shell: 240px sidebar + detail panel, 950×600 window, overlay titlebar
- [x] 6-destination flat nav with Lucide icons, routing via react-router-dom
- [x] Home page: welcome card with formatted shortcut + setup hints
- [x] Tailwind v4 + shadcn/ui adoption
- [x] `useTheme` rewritten to toggle `.dark` on `<html>` (shadcn token system)
- [x] Transcription errors → Sonner bottom-right toast
- [x] UpdateBanner inline at top of detail panel
- [x] Sidebar toggle button (session-scoped, not persisted)

### Phase 1b — Component rewrites (one PR each)
- [ ] MicrophoneField → Tailwind/shadcn
- [ ] AppearanceField → Tailwind/shadcn
- [ ] ShortcutField → Tailwind/shadcn
- [ ] TranscriptionProviderField → Tailwind/shadcn
- [ ] AiCleanupField → Tailwind/shadcn
- [ ] ReplacementsField → Tailwind/shadcn
- [ ] HistoryTab → Tailwind/shadcn
- [ ] StatsTab → Tailwind/shadcn

### Phase 2 — IA reorganization (~8-item sidebar)
- [ ] Split Transcription into Engine / AI Cleanup / Dictionary
- [ ] Add Recording section (separate from General)
- [ ] Rename Shortcut → Hotkeys
- [ ] Sidebar section headers

### Phase 3+ — Pipeline & dashboard
- [ ] Real Home dashboard (stats, recent transcriptions, activity chart)
- [ ] Presets concept (provider-agnostic model settings)
- [ ] Snippets / workflows / post-processing pipeline
