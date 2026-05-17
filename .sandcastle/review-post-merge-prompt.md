# TASK

Review the consolidated changes that just merged into the current branch and
improve code clarity, consistency, and maintainability while preserving exact
functionality.

# CONTEXT

## Issues included in this merge cycle

{{ISSUES}}

## Consolidated diff (pre-merge → HEAD)

!`git diff {{PRE_MERGE_REF}}...HEAD`

## Commits since pre-merge

!`git log {{PRE_MERGE_REF}}..HEAD --oneline`

# REVIEW PROCESS

1. **Understand the change**: Read the diff and commits above to understand the
   intent across all merged issues.

2. **Analyze for improvements**: Look for opportunities to:
   - Reduce unnecessary complexity and nesting
   - Eliminate redundant code and abstractions
   - Improve readability through clear variable and function names
   - Consolidate related logic — especially across issues that touched
     overlapping areas
   - Remove unnecessary comments that describe obvious code
   - Avoid nested ternary operators — prefer switch statements or if/else
     chains
   - Choose clarity over brevity — explicit code is often better than overly
     compact code

3. **Check correctness**:
   - Does the implementation match the intent? Are edge cases handled?
   - Are new/changed behaviours covered by tests?
   - Are there unsafe casts, `any` types, or unchecked assumptions?
   - Does the change introduce injection vulnerabilities, credential leaks, or
     other security issues?
   - Did any cross-issue interactions introduce duplication, conflicting
     conventions, or dead code?

4. **Maintain balance**: Avoid over-simplification that could:
   - Reduce code clarity or maintainability
   - Create overly clever solutions that are hard to understand
   - Combine too many concerns into single functions or components
   - Remove helpful abstractions that improve code organization
   - Make the code harder to debug or extend

5. **Apply project standards**: Follow the coding standards defined in
   @.sandcastle/CODING_STANDARDS.md

6. **Preserve functionality**: Never change what the code does — only how it
   does it. All original features, outputs, and behaviors must remain intact.

# EXECUTION

If you find improvements to make:

1. Make the changes directly on the current branch
2. Run tests and type checking to ensure nothing is broken
3. Commit describing the refinements

If the code is already clean and well-structured, do nothing.

Once complete, output <promise>COMPLETE</promise>.
