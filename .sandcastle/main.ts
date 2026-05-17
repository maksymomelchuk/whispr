// Parallel Planner with Tiered Review — three-phase orchestration loop
//
// This template drives a multi-phase workflow:
//   Phase 1 (Plan):             An opus agent analyzes open issues, builds a
//                               dependency graph, and outputs a <plan> JSON
//                               listing unblocked issues with branch names.
//   Phase 2 (Execute + Review): For each issue, a sandbox is created via
//                               createSandbox(). The implementer runs first
//                               (100 iterations). If it produces commits, the
//                               diff is triaged on the host: trivial diffs
//                               (tiny, or only tests/docs/lockfile) skip
//                               review entirely. Otherwise a Haiku tier-1
//                               reviewer runs in the sandbox; it commits
//                               mechanical fixes and emits a <verdict> tag.
//                               If the verdict is ESCALATE, a Sonnet tier-2
//                               reviewer runs against the same branch with
//                               the tier-1 concerns as a seed. All issue
//                               pipelines run concurrently via
//                               Promise.allSettled().
//   Phase 3 (Merge):            A single agent merges all completed branches
//                               into the current branch.
//
// The outer loop repeats up to MAX_ITERATIONS times so that newly unblocked
// issues are picked up after each round of merges.
//
// Usage:
//   npx tsx .sandcastle/main.ts
// Or add to package.json:
//   "scripts": { "sandcastle": "npx tsx .sandcastle/main.ts" }

import { execSync } from "node:child_process";
import * as sandcastle from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

// Maximum number of plan→execute→merge cycles before stopping.
// Raise this if your backlog is large; lower it for a quick smoke-test run.
const MAX_ITERATIONS = 10;

// Source branch the orchestrator merges into and diffs against. Resolved
// once at startup so an external checkout switch mid-run can't shift the
// baseline used by the triage filter.
const SOURCE_BRANCH = execSync("git rev-parse --abbrev-ref HEAD", {
  encoding: "utf-8",
}).trim();

// Hooks run inside the sandbox before the agent starts each iteration.
// pnpm install ensures the sandbox always has fresh dependencies.
//
// timeoutMs override: sandcastle's per-hook default is 60s. A cold install
// of this Tauri tree in a fresh container has no warm store to draw from
// and routinely exceeds that. Warm runs finish in seconds; the ceiling is
// just slack for the first boot.
const hooks = {
  sandbox: {
    onSandboxReady: [{ command: "pnpm install", timeoutMs: 180_000 }],
  },
};

const copyToWorktree = ["pnpm-lock.yaml"];

// ---------------------------------------------------------------------------
// Triage
// ---------------------------------------------------------------------------

// Skip the reviewer entirely when the implementer's diff is too small or
// scoped to files where LLM review wouldn't add value over the implementer's
// own typecheck + tests. Calibrate against `.sandcastle/logs/` history:
// scan past reviewer runs that produced zero commits and widen these until
// you stop skipping useful reviews.
const TRIAGE_MIN_LINES = 30;
const TRIAGE_TRIVIAL_PATHS = [
  /^pnpm-lock\.yaml$/,
  /\.snap$/,
  /\.test\.(ts|tsx|rs)$/,
  /^docs\//,
  /\.md$/,
];

function triageDiff(branch: string): { trivial: boolean; reason: string } {
  const shortstat = execSync(
    `git diff --shortstat ${SOURCE_BRANCH}...${branch}`,
    { encoding: "utf-8" },
  );
  const files = execSync(`git diff --name-only ${SOURCE_BRANCH}...${branch}`, {
    encoding: "utf-8",
  })
    .split("\n")
    .filter(Boolean);

  const linesChanged = [
    ...shortstat.matchAll(/(\d+) (insertion|deletion)/g),
  ].reduce((sum, m) => sum + Number(m[1]!), 0);

  if (linesChanged > 0 && linesChanged < TRIAGE_MIN_LINES) {
    return { trivial: true, reason: `${linesChanged} lines changed` };
  }

  const allTrivialPaths =
    files.length > 0 &&
    files.every((f) => TRIAGE_TRIVIAL_PATHS.some((p) => p.test(f)));
  if (allTrivialPaths) {
    return { trivial: true, reason: "only tests/docs/lockfile touched" };
  }

  return { trivial: false, reason: "" };
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

for (let iteration = 1; iteration <= MAX_ITERATIONS; iteration++) {
  console.log(`\n=== Iteration ${iteration}/${MAX_ITERATIONS} ===\n`);

  // -------------------------------------------------------------------------
  // Phase 1: Plan
  //
  // The planning agent (opus, for deeper reasoning) reads the open issue list,
  // builds a dependency graph, and selects the issues that can be worked in
  // parallel right now (i.e., no blocking dependencies on other open issues).
  //
  // It outputs a <plan> JSON block — we parse that to drive Phase 2.
  // -------------------------------------------------------------------------
  const plan = await sandcastle.run({
    hooks,
    sandbox: docker(),
    name: "planner",
    // One iteration is enough: the planner just needs to read and reason,
    // not write code.
    maxIterations: 1,
    // Opus for planning: dependency analysis benefits from deeper reasoning.
    agent: sandcastle.claudeCode("claude-opus-4-7"),
    promptFile: "./.sandcastle/plan-prompt.md",
  });

  // Extract the <plan>…</plan> block from the agent's stdout.
  const planMatch = plan.stdout.match(/<plan>([\s\S]*?)<\/plan>/);
  if (!planMatch) {
    throw new Error(
      "Planning agent did not produce a <plan> tag.\n\n" + plan.stdout,
    );
  }

  // The plan JSON contains an array of issues, each with id, title, branch.
  const { issues } = JSON.parse(planMatch[1]!) as {
    issues: { id: string; title: string; branch: string }[];
  };

  if (issues.length === 0) {
    // No unblocked work — either everything is done or everything is blocked.
    console.log("No unblocked issues to work on. Exiting.");
    break;
  }

  console.log(
    `Planning complete. ${issues.length} issue(s) to work in parallel:`,
  );
  for (const issue of issues) {
    console.log(`  ${issue.id}: ${issue.title} → ${issue.branch}`);
  }

  // -------------------------------------------------------------------------
  // Phase 2: Execute + Tiered Review
  //
  // For each issue, create a sandbox via createSandbox() so the implementer
  // and reviewers share the same sandbox instance per branch. Flow:
  //
  //   implementer (Sonnet, 100 iters)
  //       │
  //       ├─ no commits → done
  //       │
  //       └─ commits → triageDiff() on host
  //                      ├─ trivial → done
  //                      └─ non-trivial → tier-1 reviewer (Haiku)
  //                                          ├─ CLEAN/FIXED → done
  //                                          └─ ESCALATE → tier-2 (Sonnet)
  //                                                          with tier-1
  //                                                          concerns as seed
  //
  // Promise.allSettled means one failing pipeline doesn't cancel the others.
  // -------------------------------------------------------------------------

  const settled = await Promise.allSettled(
    issues.map(async (issue) => {
      const sandbox = await sandcastle.createSandbox({
        branch: issue.branch,
        sandbox: docker(),
        hooks,
        copyToWorktree,
      });

      try {
        const implement = await sandbox.run({
          name: "implementer",
          maxIterations: 100,
          agent: sandcastle.claudeCode("claude-sonnet-4-6"),
          promptFile: "./.sandcastle/implement-prompt.md",
          promptArgs: {
            TASK_ID: issue.id,
            ISSUE_TITLE: issue.title,
            BRANCH: issue.branch,
          },
        });

        if (implement.commits.length === 0) {
          return implement;
        }

        const triage = triageDiff(issue.branch);
        if (triage.trivial) {
          console.log(`  ${issue.id}: skipping reviewer (${triage.reason})`);
          return implement;
        }

        // Tier 1: Haiku — fixes mechanical issues itself, escalates the rest.
        const tier1 = await sandbox.run({
          name: "reviewer-tier1",
          maxIterations: 1,
          agent: sandcastle.claudeCode("claude-haiku-4-5"),
          promptFile: "./.sandcastle/review-prompt-tier1.md",
          promptArgs: {
            BRANCH: issue.branch,
            SOURCE_BRANCH,
          },
        });

        const escalate = /<verdict>\s*ESCALATE\s*<\/verdict>/.test(
          tier1.stdout,
        );
        const concerns =
          tier1.stdout.match(/<concerns>([\s\S]*?)<\/concerns>/)?.[1]?.trim() ??
          "";

        let tier2Commits: typeof tier1.commits = [];
        if (escalate) {
          const preview = concerns.replace(/\s+/g, " ").slice(0, 80);
          console.log(
            `  ${issue.id}: escalating to Sonnet${preview ? ` — ${preview}` : ""}`,
          );
          const tier2 = await sandbox.run({
            name: "reviewer-tier2",
            maxIterations: 1,
            agent: sandcastle.claudeCode("claude-sonnet-4-6"),
            promptFile: "./.sandcastle/review-prompt-tier2.md",
            promptArgs: {
              BRANCH: issue.branch,
              SOURCE_BRANCH,
              TIER1_CONCERNS: concerns || "(no specific concerns provided)",
            },
          });
          tier2Commits = tier2.commits;
        }

        // Merge commits from all stages so the merge phase sees all of them.
        // Each sandbox.run() only returns commits from its own run.
        return {
          ...implement,
          commits: [...implement.commits, ...tier1.commits, ...tier2Commits],
        };
      } finally {
        await sandbox.close();
      }
    }),
  );

  // Log any agents that threw (network error, sandbox crash, etc.).
  for (const [i, outcome] of settled.entries()) {
    if (outcome.status === "rejected") {
      console.error(
        `  ✗ ${issues[i]!.id} (${issues[i]!.branch}) failed: ${outcome.reason}`,
      );
    }
  }

  // Only pass branches that actually produced commits to the merge phase.
  // An agent that ran successfully but made no commits has nothing to merge.
  const completedIssues = settled
    .map((outcome, i) => ({ outcome, issue: issues[i]! }))
    .filter(
      (entry) =>
        entry.outcome.status === "fulfilled" &&
        entry.outcome.value.commits.length > 0,
    )
    .map((entry) => entry.issue);

  const completedBranches = completedIssues.map((i) => i.branch);

  console.log(
    `\nExecution complete. ${completedBranches.length} branch(es) with commits:`,
  );
  for (const branch of completedBranches) {
    console.log(`  ${branch}`);
  }

  if (completedBranches.length === 0) {
    // All agents ran but none made commits — nothing to merge this cycle.
    console.log("No commits produced. Nothing to merge.");
    continue;
  }

  // -------------------------------------------------------------------------
  // Phase 3: Merge
  //
  // One agent merges all completed branches into the current branch,
  // resolving any conflicts and running tests to confirm everything works.
  //
  // The {{BRANCHES}} and {{ISSUES}} prompt arguments are lists that the agent
  // uses to know which branches to merge and which issues to close.
  // -------------------------------------------------------------------------
  await sandcastle.run({
    hooks,
    sandbox: docker(),
    name: "merger",
    maxIterations: 1,
    agent: sandcastle.claudeCode("claude-opus-4-7"),
    promptFile: "./.sandcastle/merge-prompt.md",
    promptArgs: {
      // A markdown list of branch names, one per line.
      BRANCHES: completedBranches.map((b) => `- ${b}`).join("\n"),
      // A markdown list of issue IDs and titles, one per line.
      ISSUES: completedIssues.map((i) => `- ${i.id}: ${i.title}`).join("\n"),
    },
  });

  console.log("\nBranches merged.");
}

console.log("\nAll done.");
