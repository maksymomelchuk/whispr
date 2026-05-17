# Agent Instructions

Canonical rules for any AI agent working in this repo (Claude Code, sandcastle reviewer, Cursor, etc.). Strict, non-negotiable. Flag every violation in review.

## Comments

Never write WHAT comments. The code already says what it does — well-named identifiers, types, and function signatures are the documentation. A WHAT comment that restates the next line is noise that goes stale and adds nothing.

Only write a comment when the WHY is non-obvious: a hidden constraint, a subtle invariant, a workaround for a specific bug, behavior that would surprise a reader. If removing the comment wouldn't confuse a future reader, don't write it.

Don't reference the current task, fix, or callers ("used by X", "added for the Y flow", "handles the case from issue #123"). That belongs in the PR description and rots as the codebase evolves.

### Examples

Bad (WHAT — delete):

```rust
// Increment the counter
counter += 1;

// Loop over each item
for item in items { ... }

/// Returns the user's name.
fn user_name(...) -> String { ... }
```

Bad (narration — delete):

```rust
// Kick off paste first so the user sees text while the file I/O for
// stats and history runs concurrently. paste_text spawns its own
// worker thread and returns immediately.
```

Good (WHY — keep):

```rust
// chunk by characters, not bytes — arbitrary UTF-8 byte splits would
// corrupt multi-byte sequences when converted to UTF-16 downstream.

// osascript blocks 50–200ms — keep off the runtime worker.

// CGEvent::post queues asynchronously at the HID layer; hold long enough
// for queued events to land in the target app before the caller raises a
// window.
```

## Style

- Names must reveal intent. No abbreviations, no single letters (except loop indices), no Hungarian notation.
- camelCase for variables/functions, PascalCase for types/classes, SCREAMING_SNAKE_CASE for constants.
- Prefer named exports. No default exports unless the framework demands it.
- No dead code. No commented-out code. No `TODO` without an owner and a ticket.
- No magic numbers or strings — extract to a named constant.

## Functions

- One job per function. If you need "and" to describe it, split it.
- Keep functions short. If it doesn't fit on a screen, it's too long.
- Max 3 parameters. More → pass an object or split the function.
- No boolean parameters that switch behavior — split into two functions.
- No output parameters. Return values.
- No side effects hidden behind a query name. `getX()` must not mutate.

## Control flow

- Return early. No nested `if` pyramids.
- No `else` after `return`/`throw`.
- Max nesting depth: 3. Refactor beyond that.
- Handle errors at the boundary. Don't swallow exceptions. Never `catch` without acting.

## DRY & SOLID

- Duplicate code three times → extract. Twice → leave it; premature abstraction is worse than duplication.
- Single Responsibility: a module/class/function changes for exactly one reason.
- Depend on interfaces, not concretions, at module boundaries.
- Open for extension, closed for modification — but don't invent extension points for hypothetical futures.

## Lean

- Build only what the task requires. No speculative features, no "might need later" code.
- Delete code aggressively. Unused = deleted, not commented.
- No backwards-compatibility shims unless a real external caller exists.
- No wrapper layers that only forward calls.

## Errors

- Validate at system boundaries (user input, network, FS). Trust internal callers.
- Fail loud, fail fast. No silent fallbacks that mask bugs.
- Error messages must say what failed and what input caused it.

## Testing

- Test behavior, not implementation. No tests that break on refactors that preserve behavior.
- One assertion per test, or one logical concept. Test names describe the scenario and expected outcome.
- No shared mutable state between tests. Each test sets up its own world.
- Don't mock what you own — use the real thing. Mock only at external boundaries.

## Architecture

- Dependencies point inward: domain ← application ← infrastructure. Never the reverse.
- No circular dependencies between modules.
- Keep I/O at the edges. Pure logic in the core.
- Public surface stays minimal. Default to private/internal; export only what's used.

## Review triggers (auto-reject)

- Any WHAT comment.
- Any commented-out code.
- Any function over ~40 lines.
- Any file over ~400 lines without justification.
- Any new abstraction with a single caller.
- Any `any` / `unknown` / unchecked cast without a written reason.
- Any swallowed error.
