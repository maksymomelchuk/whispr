# Product

## Register

product

## Users

Power users and developers tuning their own setup. They picked a
self-hosted dictation tool over Wispr Flow / Superwhisper because they
want control: hotkeys, prompts, providers, models. They live on the
keyboard, run a terminal nearby, and tolerate density. They open
Settings deliberately — not to be onboarded, but to change something
specific. They want the app to get out of the way the rest of the time.

## Product Purpose

Whispr is a macOS push-to-talk dictation tool: hold a shortcut, speak,
release, and the transcription lands in the focused app. The settings
shell is where users shape that pipeline — capture device, hotkey,
provider, modes, snippets, post-processing. Success looks like a user
configuring it once, forgetting the UI exists, and only returning to
tune a specific behavior.

## Brand Personality

Technical, confident, quiet. Linear and Raycast adjacent — opinionated
when it speaks, restrained the rest of the time. Three-word personality:
**precise, native, opinionated**. The interface should feel like a
well-built tool, not a product launch. No marketing energy in the
settings surface.

## Anti-references

- **Neon-on-black "hacker" theme.** Terminal-LARP green on black,
  scanlines, cyberpunk affectation. Whispr is terminal-adjacent because
  it shares the audience's values (keyboard, density, semantic color),
  not because it cosplays a TTY.

## Design Principles

1. **Earn density.** Power users want to see everything at once. Use
   real information density, not airy padding, but every row must have
   clear hierarchy so density never becomes noise.
2. **Keyboard is a first-class surface.** Every interaction has a focus
   state, every destination a shortcut. The mouse is optional.
3. **Color is semantic, not decorative.** One restrained accent.
   Additional color only when it carries meaning (status, capture state,
   diff, destructive).
4. **Type does the heavy lifting.** Hierarchy through weight and scale
   contrast, not boxes and dividers. Mono is reserved for shortcuts,
   keys, identifiers, and numerics — not decoration.
5. **Quiet by default, sharp on demand.** The chrome recedes; the
   active surface speaks. Bolder moments are reserved for the one thing
   that matters on each screen.

## Accessibility & Inclusion

- WCAG AA contrast across text, controls, and chrome (4.5:1 body, 3:1
  large text and UI components).
- Visible, generous focus rings at all times — never `outline: none`.
- Honor `prefers-reduced-motion` for every transition.
- Keyboard parity with mouse for every action on every page (the
  audience expects this even though it's not formally required by AA).
