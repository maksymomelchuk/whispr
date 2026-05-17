---
name: Whispr
description: macOS push-to-talk dictation, with a settings shell that fades when it's not needed.
colors:
  background: "#f6f7f9"
  foreground: "#101419"
  card: "#ffffff"
  muted: "#eceef1"
  muted-foreground: "#626871"
  border: "#dce0e6"
  destructive: "#b91e1e"
  primary-indigo: "#3460e9"
  primary-violet: "#7748e1"
  primary-coral: "#e0612d"
  primary-emerald: "#2a9268"
  primary-graphite: "#272d36"
  background-dark: "#15171a"
  foreground-dark: "#f1f2f5"
  card-dark: "#1c1f24"
  muted-dark: "#272a31"
  muted-foreground-dark: "#9aa0a8"
  border-dark: "#2e323a"
typography:
  page-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', system-ui, sans-serif"
    fontSize: "22px"
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: "-0.012em"
  section-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "-0.005em"
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "normal"
  form-label:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 500
    lineHeight: 1.3
    letterSpacing: "0.2px"
  eyebrow-mono:
    fontFamily: "'SF Mono', ui-monospace, Menlo, 'Cascadia Mono', Consolas, monospace"
    fontSize: "10.5px"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "0.14em"
  kbd-mono:
    fontFamily: "'SF Mono', ui-monospace, Menlo, 'Cascadia Mono', Consolas, monospace"
    fontSize: "10.5px"
    fontWeight: 500
    lineHeight: 1
    letterSpacing: "0"
rounded:
  sm: "0.25rem"
  md: "0.4rem"
  lg: "0.625rem"
  xl: "0.875rem"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "14px"
  lg: "24px"
  xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.primary-indigo}"
    textColor: "#ffffff"
    rounded: "{rounded.md}"
    padding: "8px 16px"
    height: "36px"
  button-primary-hover:
    backgroundColor: "#2b53d4"
    textColor: "#ffffff"
  button-outline:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
    height: "36px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.foreground}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
    height: "36px"
  input-default:
    backgroundColor: "transparent"
    textColor: "{colors.foreground}"
    rounded: "{rounded.md}"
    padding: "4px 12px"
    height: "36px"
  row-card:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    rounded: "10px"
    padding: "10px 8px 10px 12px"
  section-header:
    textColor: "{colors.foreground}"
    padding: "0 0 6px 0"
  toggle-row:
    backgroundColor: "transparent"
    textColor: "{colors.foreground}"
    padding: "2px 0"
---

# Design System: Whispr

## 1. Overview

**Creative North Star: "The Native Console"**

Whispr is a macOS dictation tool whose settings surface is mostly absent — used once, then forgotten until something specific needs tuning. The visual language has to behave the same way: native enough to vanish into the OS chrome, terminal-adjacent enough to reward the power user who lives on the keyboard, and disciplined enough that nothing decorative ever asks for attention. This is Linear's restraint and Raycast's density crossed with Apple's own Settings — a console that happens to be drawn with macOS materials.

Color is allowed in exactly three situations: a single accent for the current action, a destructive red for irreversible operations, and the user's chosen accent for state. Everything else is a cool-tinted neutral at 220° hue and chroma so low it reads as grey. There is no marketing energy here. There are no celebratory toasts, no green "Saved" confetti, no gradient hero metrics. The chrome recedes; the active control speaks.

The interface explicitly rejects the neon-on-black "hacker" theme called out in PRODUCT.md. Whispr is terminal-adjacent because it shares the audience's values — keyboard parity, semantic color, density without noise — not because it cosplays a TTY.

**Key Characteristics:**
- 220° cool-tinted neutrals; chroma never exceeds the cool-grey threshold.
- Hairline 1px borders at 40–60% alpha; no thick strokes.
- Density earned through hierarchy, not airy padding.
- Mono (`SF Mono`) reserved for shortcuts, identifiers, numerics — never decoration.
- One accent at a time. The user picks indigo, violet, coral, emerald, or graphite, and that hue is the only chromatic voice on the screen.
- Light and dark are equal citizens; both ship.

## 2. Colors

A cool-tinted greyscale (220° hue, 8–20% saturation) plus one user-selected accent. The neutrals are doing 90% of the work; the accent earns its keep by being rare.

### Primary
- **Indigo Signal** (#3460e9, canonical `hsl(224 76% 56%)`): default accent. Used for primary buttons, the current selection, focus rings, and active state in the sidebar. In dark mode, lifts to `hsl(224 88% 66%)` so contrast against the cool-dark surface stays above AA.
- **Violet Signal** (#7748e1, canonical `hsl(262 70% 58%)`): user-selectable accent.
- **Coral Signal** (#e0612d, canonical `hsl(15 80% 55%)`): user-selectable accent.
- **Emerald Signal** (#2a9268, canonical `hsl(155 55% 38%)`): user-selectable accent.
- **Graphite Signal** (#272d36, canonical `hsl(220 16% 18%)`): "the disciplined choice" — the accent becomes a near-neutral and the focus ring carries the chromatic load instead.

### Destructive
- **Crimson Stop** (#b91e1e, canonical `hsl(0 72% 42%)`): destructive actions, irreversible operations, validation errors. Never used for general emphasis.

### Neutral (light)
- **Cool Ash 97** (#f6f7f9, `hsl(220 14% 97%)`): page background.
- **Cool Ash 95** (#f1f2f4, `hsl(220 14% 95.5%)`): sidebar background — one tonal step cooler than the canvas to mark navigation territory.
- **Cool Ash 94** (#eceef1, `hsl(220 12% 94%)`): muted surface, secondary buttons, toggle group rest state.
- **Cool Ash 89** (#dce0e6, `hsl(220 14% 89%)`): hairline borders and input strokes.
- **Cool Slate 42** (#626871, `hsl(220 8% 42%)`): secondary copy, form labels, muted icons.
- **Near Black 8** (#101419, `hsl(220 20% 8%)`): body foreground.
- **Pure White** (#ffffff): card surface — the only true white in the palette, used to lift a card out of the cool-ash field.

### Neutral (dark)
- **Cool Char 9** (#15171a, `hsl(220 14% 9%)`): canvas.
- **Cool Char 10.5** (#191c20, `hsl(220 14% 10.5%)`): sidebar — again, one step cooler than the canvas to separate territory.
- **Cool Char 12.5** (#1c1f24, `hsl(220 14% 12.5%)`): card surface.
- **Cool Char 17** (#272a31, `hsl(220 12% 17%)`): muted surface.
- **Cool Char 20** (#2e323a, `hsl(220 12% 20%)`): borders.
- **Cool Mist 64** (#9aa0a8, `hsl(220 8% 64%)`): muted copy.
- **Cool Mist 96** (#f1f2f5, `hsl(220 12% 96%)`): body foreground.

### Named Rules

**The One Voice Rule.** A single accent is active per screen. Choosing violet doesn't make indigo a secondary option — it replaces it. Multi-accent compositions are forbidden.

**The Semantic Color Rule.** Color must carry meaning: state (selected, focus, capture-armed), action (primary, destructive), or diff. Decorative color is forbidden. If a surface is "blue because settings UI tends to be blue", the surface is wrong.

**The No Confetti Rule.** No green success toasts, no transient "Saved" alerts, no confirmation banners that pop in and fade out. When a value is saved, the control's new state IS the confirmation: input collapses, dirty indicator disappears, dropdown closes. Save *failures* go to a sonner toast at bottom-right. Successes stay silent.

## 3. Typography

**Body & UI Font:** `-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif`. Native macOS face, falls through to the platform's system stack on Windows/Linux dev. No custom display font — this is product UI, not a marketing page.

**Mono Font:** `"SF Mono", ui-monospace, "Menlo", "Cascadia Mono", "Consolas", monospace`. Reserved scope (see The Mono Discipline Rule).

**Character:** quiet, native, technical. The type pairing is one family doing all the work, with mono playing a strict supporting role for things that need to look like data, not prose.

Base size is 14px / line-height 1.4, set on `<body>`. Scale ratio is tight — 1.1× between most steps — because product UI with too much typographic contrast becomes noise.

### Hierarchy

- **Page Title** (`22px / 600 / -0.012em / 1.15`): the `<h1>` in `PageHeader`. One per page, top of the canvas, optional eyebrow above.
- **Section Title** (`14px / 600 / -0.005em / 1.4`): `<h3>` in `SectionHeader` / `CollapsibleCard`. Bordered by a 1px bottom hairline at 40% alpha — this border, not size, is what carries the section break.
- **Body / Row** (`13px / 400 / normal / 1.45`): the workhorse. Toggle row labels, table rows, descriptive copy, control labels in `OptionRow`.
- **Form Label** (`11px / 500 / 0.2px / 1.3`): muted-foreground color, sits above a control. The default form-field label; quieter than the body so the control reads first.
- **Eyebrow Mono** (`10.5px / 600 / 0.14em uppercase`): the eyebrow above page titles and the sidebar section labels ("WORKSPACE", "PIPELINE", "INSIGHTS", "SYSTEM"). Tracking is intentionally wide because this is meant to feel like a printed terminal banner.
- **Kbd Mono** (`10.5px / 500 / tabular-nums`): keyboard shortcuts in the sidebar, the `<Keycap>` component on the Hotkeys page, threshold value displays.

### Named Rules

**The Mono Discipline Rule.** `SF Mono` is reserved for keyboard shortcuts, file/identifier strings, masked credentials, and numerics that need to align in columns. It is forbidden for headings, body copy, button labels, or anything that wants to "look technical". Mono is a tool, not a vibe.

**The Tight Scale Rule.** Steps between sizes never exceed 1.6×, and most are closer to 1.1×. The page hierarchy is achieved through *weight* and *color* contrast as much as *size*. A 22px title and 14px section header look different because the title is 600/foreground and the section is 600/foreground-with-a-rule-under-it — not because of dramatic size contrast.

## 4. Elevation

The system is **flat by default**. Depth is conveyed through three mechanisms, in order of priority:

1. **Tonal layering** — the sidebar is one neutral step cooler than the canvas; cards are one step warmer/lighter; muted surfaces sit one step inward. The eye reads the strata before any shadow.
2. **Hairline borders** at 40–60% alpha — used to mark sections, card edges, and input strokes. Borders are always 1px and always slightly transparent so they read as a drawn line, not a hard divider.
3. **Shadows respond to state, never decorate.** A `shadow-xs` lives under interactive `RowCard` elements at rest. `shadow-lg` is reserved for the Toaster (the one thing that overlays everything). Hover lifts to `shadow-sm`. There is no "card shadow" scale otherwise.

### Shadow Vocabulary

- **`shadow-xs`** (`box-shadow: 0 1px 1px rgba(0,0,0,0.04)`): RowCard rest state. Barely there; reads as a thickening of the border on macOS retina screens.
- **`shadow-sm`** (`box-shadow: 0 2px 4px rgba(0,0,0,0.06)`): RowCard hover. The lift is the only animation; padding stays put.
- **`shadow-lg`** (Tailwind default, `0 10px 15px -3px rgba(0,0,0,0.1)`): Toaster only. Toasts are the one element that legitimately overlay everything, so they get the most assertive shadow in the system.

### Named Rules

**The Flat-By-Default Rule.** Surfaces are flat at rest. If a card has a shadow and it isn't hovered, focused, or being dragged, the shadow is wrong. Static elevation is forbidden — depth is a response, not a state.

**The Hairline Rule.** Borders are 1px and live at 40–60% alpha against their neighbor. A 2px solid border on a card is wrong; thickening is how it announces itself, which is the opposite of what this system is for.

## 5. Components

### Buttons
- **Shape:** rounded-md (6.4px / `0.4rem`). Never pill (only the Switch uses pill). Never sharp-cornered.
- **Primary:** background = current accent (default Indigo Signal #3460e9), white text. `h-9 px-4`, 36px tall. Hover darkens to 90% lightness in the same hue. Disabled = 50% opacity, no color change.
- **Outline:** background = card, 1px border = input stroke (#dce0e6), foreground text. Used for `Replace` in CredentialField, dialog secondary actions.
- **Ghost:** transparent rest, `accent` background on hover. Used for icon-only actions, `Cancel`, `Remove` in CredentialField, sidebar trigger.
- **Destructive:** Crimson Stop (#b91e1e), white text. Reserved for irreversible operations (delete a binding, clear a credential after an explicit Remove confirmation).
- **Focus:** 3px ring at 50% accent alpha, slight border shift to ring color. Visible always — never `outline: none`.

### Inputs / Fields
- **Shape:** rounded-md, `h-9 px-3 py-1`. 1px border at `input` stroke color.
- **Focus:** 3px ring at 50% accent alpha. The border itself shifts to the ring color so the affordance reads at any zoom.
- **Error:** border becomes destructive, ring shifts to destructive @ 20% alpha. `FormMessage` renders below the input at 11px in destructive color.
- **Disabled:** 50% opacity, no pointer events.
- **Numeric:** native HTML number input is permitted; spinners are not styled away. `inputMode="numeric"` / `"decimal"` set so iOS-style keyboards behave (Tauri doesn't need this but the discipline is correct).

### CredentialField (signature component)

The credential UX is documented because it is the most repeated pattern in the settings surface and was previously the source of the most confusion. It governs API keys, OAuth tokens, and any secret-string input.

- **Configured rest state:** the input is *not visible*. In its place sits a 36px-tall masked field showing twelve `•` characters in mono at 55% alpha, flanked by an outline `Replace` button and a ghost `Remove` button. The mask is the affordance — nothing announces "Configured" in words or color.
- **Editing state:** `Replace` swaps the mask for a real password input + `Save` + ghost `Cancel`. Esc cancels. Input autofocuses.
- **Not-configured state:** input is shown directly, with `Save` and no Cancel.
- **Validation:** on blur, the persisted `validate` function is called. While checking, an 11px muted caption reads "Checking key…". Invalid results render under the input as a destructive FormMessage.
- **Save success:** the field collapses back to the masked rest state. No alert, no toast, no green text. The collapse is the confirmation.

### Toggle Row
- **Layout:** label on the left (13px, foreground), Switch on the right.
- **Switch:** Radix Switch; pill-shaped track. Unchecked = `input` neutral; checked = current accent. 4.6mm tall on the default size — small but tappable on macOS.
- **Stacked rows** receive a 1px top border at the `border` color when they follow another `[data-slot="toggle-row"]` or a `form-item`, via a CSS adjacent-sibling rule in `globals.css`. This is the only place the system uses a sibling rule for layout — it's how toggle-only sections get their rhythm without each row carrying a border.

### Section Card
- **No frame.** A section is a `<section>` with a 14px/600 title, an optional info tooltip, and an optional trailing badge — separated from its content by a 1px bottom border at 40% alpha. No card, no shadow, no panel. The section is defined by the rule under the title.

### RowCard
- **Used for:** repeating list rows that need to be tappable as a unit (hotkey bindings, modes, snippets).
- **Shape:** rounded-[10px], 1px border, `shadow-xs`. Hover lifts to ring-tinted border + `shadow-sm`. Padding `pl-3 pr-2 py-2.5`.
- **Tones:** `neutral` (default), `destructive` (red-tinted border + bg), `accent` (ring-tinted border + bg, used for "currently held" hotkey), `dashed` (border-dashed, used for "+ Add" affordances).

### Sidebar Navigation
- **Background:** one tonal step cooler than the canvas.
- **Item:** 32px tall, left-aligned icon (15px) + 13px label + right-aligned ⌘N keycap.
- **Keycap:** mono 10.5px, 0% opacity at rest, 100% on row hover or row active. Active items show their keycap permanently so the user is reminded which shortcut they're already using.
- **Section labels:** mono 10px, uppercase, 0.16em tracking, 60% alpha. Hidden when the sidebar collapses to icon-only.

### Toaster (error surface)
- **Position:** bottom-right.
- **Width:** content-sized.
- **Style:** background card, 1px border, `shadow-lg`. Title in foreground, description in muted-foreground at 12px. Errors get destructive-tinted background and border at 10%/30% alpha.
- **Used for:** save failures, validation API errors. Never for success.

### Named Rules

**The Configured-Collapse Rule.** Any field that stores a secret defaults to a masked-rest state when configured. The input is removed from the page, not just hidden via `type="password"`. A user looking at a settings screen with three credentials should see *no input boxes at all* — three masked rests, three `Replace` buttons.

**The Inline Affordance Rule.** Action buttons live next to the control they act on — `Save` is to the right of the input, not at the bottom of a section. Section-level Save buttons are forbidden because they ask the user to scan back and forth to figure out what they're committing.

## 6. Do's and Don'ts

### Do:
- **Do** use one accent at a time. Indigo by default; user can pick violet, coral, emerald, or graphite. The accent is used for primary actions, focus rings, and current-selection — nothing else.
- **Do** keep neutrals tinted to 220° hue at 8–14% saturation. A "pure" grey looks alien against the rest of the surface.
- **Do** render section breaks with a 1px bottom border at 40% alpha under the title. Hierarchy comes from the rule, not from a frame.
- **Do** collapse credential fields to a masked rest state once configured. The mask is the confirmation.
- **Do** route save *failures* to the sonner toast at bottom-right. Errors are loud; successes are silent.
- **Do** keep the focus ring visible at all times — 3px at 50% accent alpha. Power users navigate by keyboard.
- **Do** use `SF Mono` only for keyboard shortcuts, identifier strings, masked credentials, and column-aligned numerics.
- **Do** earn density. Toggle rows are 2px padding tall. Section gaps are 32px. The contrast between row-density and section-spacing is the rhythm.

### Don't:
- **Don't** ship the neon-on-black "hacker" theme PRODUCT.md explicitly rejects. Terminal-green on pitch-black, scanline overlays, cyberpunk affectation — forbidden. We share the audience's values; we do not cosplay a TTY.
- **Don't** use green "Saved" alerts, transient success toasts, or any confirmation that pops in and fades out. The control's state change IS the confirmation. If you find yourself adding `<Alert variant="success">`, delete it.
- **Don't** stack Save buttons section by section. Use the inline pattern (Save next to the field) or autosave with a 450ms debounce.
- **Don't** use `border-left` greater than 1px as a colored stripe to mark cards, callouts, or list rows. Use a tinted background or a leading icon instead.
- **Don't** apply `background-clip: text` to a gradient. Use solid colors. Emphasis comes from weight, not gradient.
- **Don't** add static elevation. No card has a shadow at rest unless it is the Toaster. Shadows respond to hover, focus, or drag.
- **Don't** introduce a second sans-serif. There is one body face and one mono face. A display font has no role in product UI.
- **Don't** wrap individual settings in cards. Sections are framed by their rule, not by a panel. Card grids of icon-plus-heading-plus-text are forbidden as a default layout.
- **Don't** invent affordances for standard tasks. A dropdown is a Radix `Select`. A toggle is a Radix `Switch`. A modal is a Radix `Dialog`. The interface trusts platform conventions.
- **Don't** use `outline: none`. Ever. Replace the ring if you must restyle, but never remove it.
- **Don't** clamp typography sizes fluidly. Product UI is viewed at a known DPI. A 22px page title that becomes 18px in a narrow window is worse, not better.
