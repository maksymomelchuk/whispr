# TASK

You are a **deeper reviewer** for branch `{{BRANCH}}`. A first-tier reviewer
already handled mechanical cleanups and escalated this branch because they
flagged something subtle. Your job is to look at those concerns carefully,
plus do your own end-to-end review.

# CONTEXT

## Concerns raised by tier-1 reviewer

{{TIER1_CONCERNS}}

Treat these as starting points, **not** as a complete list. The tier-1
reviewer is fast and shallow — assume they may have missed adjacent issues.

## Branch diff

!`git diff {{BASE_BRANCH}}...{{BRANCH}}`

## Commits on this branch

!`git log {{BASE_BRANCH}}..{{BRANCH}} --oneline`

# REVIEW PROCESS

1. **Address tier-1 concerns first.** For each concern, decide:
   - Is it real? (read the code, not just the diff)
   - If real, fix it; if it's a false alarm, note that in your commit message.

2. **Independent review of the full diff.** Look for:
   - Correctness: edge cases, off-by-ones, error paths, panics, unwraps
   - Concurrency: races, missed awaits, shared state, ordering assumptions
   - Security: injection, path traversal, credential handling, IPC surface
   - Type safety: unsafe casts, `any`, loose schemas, unchecked deserialization
   - Test quality: do tests exercise behavior, or just mocks?
   - Cross-module coupling and hidden contracts

3. **Clarity & maintainability.** Tier-1 already removed obvious WHAT
   comments. Focus on structural issues:
   - Functions doing too many things
   - Premature abstractions or missing ones
   - Nested ternaries, deep nesting, repeated logic
   - Names that lie about what the code does

4. **Apply project standards:** @.sandcastle/CODING_STANDARDS.md and the
   comment policy in the repo's CLAUDE.md (no WHAT or narration comments).

5. **Preserve behavior.** Refactor and harden; do not change observable
   functionality.

# EXECUTION

If you find improvements to make:

1. Make the changes on the current branch.
2. Run tests and type checking; do not commit a broken state.
3. Commit with a message that says what was wrong and why the fix is right.

If everything checks out (including the tier-1 concerns being false alarms),
do nothing.

Once complete, output `<promise>COMPLETE</promise>`.
