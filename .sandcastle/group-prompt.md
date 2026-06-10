# ISSUES

Here are the open issues in the repo:

<issues-json>

!`gh issue list --state open --label Sandcastle --limit 100 --json number,title,body --jq '[.[] | {number, title, body}]'`

</issues-json>

# TASK

Cluster ONLY these issues (they have no parent PRD): {{PARENTLESS_IDS}}

Ignore every other issue in the list above.

Each cluster becomes one integration branch and ships as one pull request, so a cluster must contain exactly the issues that belong together in a single reviewable change.

Two issues belong to the same cluster when:

- one is **blocked by** the other — it needs code, infrastructure, or an API decision the other introduces
- they modify overlapping files or modules
- they are parts of one feature that should ship and be reviewed together

Clusters are **connected components** of these relations: if issue 2 is blocked by issue 1, issue 3 by issue 2, and issue 4 by issue 3, all four form ONE cluster — even though issue 4 has no direct relation to issue 1. Chains and trees of dependencies always collapse into a single cluster.

An issue related to nothing else forms a cluster of one. When unsure whether two issues are related, keep them separate — two small pull requests are cheaper to review than one mixed one.

# OUTPUT

Output the clusters as JSON wrapped in `<clusters>` tags. Issue ids are the issue numbers as strings. Every issue listed in the task must appear in exactly one cluster:

<clusters>
{"clusters": [["12", "15", "18"], ["87"]]}
</clusters>

Always emit the `<clusters>` tags.
