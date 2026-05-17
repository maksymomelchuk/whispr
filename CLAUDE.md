# Comments

Never write WHAT comments. The code already says what it does — well-named identifiers, types, and function signatures are the documentation. A WHAT comment that restates the next line is noise that goes stale and adds nothing.

Only write a comment when the WHY is non-obvious: a hidden constraint, a subtle invariant, a workaround for a specific bug, behavior that would surprise a reader. If removing the comment wouldn't confuse a future reader, don't write it.

Don't reference the current task, fix, or callers ("used by X", "added for the Y flow", "handles the case from issue #123"). That belongs in the PR description and rots as the codebase evolves.

## Examples

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
