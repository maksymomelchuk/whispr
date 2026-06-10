# WORKSETS

Worksets were fixed at the start of this run and must not be changed. Each workset owns one integration branch; its issues merge there and ship together in one pull request. These are the worksets with their remaining open issues:

<worksets-json>

{{WORKSETS}}

</worksets-json>

# ISSUES

Full bodies of the open issues, for dependency analysis:

<issues-json>

!`gh issue list --state open --label Sandcastle --limit 100 --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'`

</issues-json>

# TASK

For each workset, select the issues that can be worked RIGHT NOW, in parallel.

An issue B is **blocked by** issue A if:

- B requires code or infrastructure that A introduces
- B and A modify overlapping files or modules, making concurrent work likely to produce merge conflicts
- B's requirements depend on a decision or API shape that A will establish

Only OPEN issues block. Anything already closed has been merged into its workset branch, so dependencies on closed issues are satisfied.

Selection rules:

1. Select an issue only if it has zero blocking dependencies on other open issues.
2. Never select an issue blocked by an open issue in a DIFFERENT workset — its dependency ships in a separate pull request, so it cannot be built in this run. Leave it out entirely; a future run picks it up after that PR merges.
3. If every remaining issue in a workset is blocked only by other issues in that same workset, select the single weakest-dependency one (fewest or weakest blockers) so the workset keeps moving.

# OUTPUT

Output your plan as a JSON object wrapped in `<plan>` tags. `id` is the issue number as a string; `workset` must be the exact workset name from the list above:

<plan>
{"issues": [{"id": "140", "workset": "prd-139"}, {"id": "87", "workset": "issue-87"}]}
</plan>

Always emit the `<plan>` tags, even when there is nothing to do. If no issue is workable, output `<plan>{"issues": []}</plan>` so the run can exit cleanly.
