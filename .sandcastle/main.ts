// Parallel Planner with Worksets and Tiered Review — four-phase orchestration
//
//   Phase 0 (Group):   Open Sandcastle issues are grouped into worksets. Issues
//                      sharing a parent PRD group mechanically via the
//                      sub-issues relationship. Parentless issues are clustered
//                      by an Opus agent into connected components of
//                      relatedness (a blocked-by chain 1←2←3←4 is one
//                      component); an unrelated issue becomes a workset of one.
//                      Each workset owns one integration branch forked from
//                      SOURCE_BRANCH and ships as one pull request.
//   Phase 1 (Plan):    Per iteration, an Opus agent selects the issues in each
//                      workset that are unblocked right now. Issues blocked by
//                      an open issue in a different workset are deferred to a
//                      future run — their dependency ships in a separate PR.
//   Phase 2 (Execute): Per issue: branch sandcastle/issue-{id} forked from the
//                      workset branch, implementer (Sonnet, 100 iters), host
//                      diff triage, Haiku tier-1 review, Sonnet tier-2 on
//                      ESCALATE. All pipelines run concurrently.
//   Phase 3 (Merge):   Per workset, one agent merges that workset's completed
//                      issue branches into the workset branch and closes the
//                      issues. Once every issue in a workset is closed, the
//                      branch is pushed and a PR is opened against
//                      SOURCE_BRANCH ("Closes #<prd>" when it has a parent).
//
// SOURCE_BRANCH itself is never committed to — integration happens on workset
// branches and lands via PRs, so running from main is safe.
//
// Usage:
//   npx tsx .sandcastle/main.ts

import { execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import * as sandcastle from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { z } from "zod";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

// Maximum number of plan→execute→merge cycles before stopping.
const MAX_ITERATIONS = 10;

// cargo test linking and vitest runs in the resource-limited container can
// stay silent well past sandcastle's 600s default without being stuck.
const AGENT_IDLE_TIMEOUT_SECONDS = 1800;

// Each issue pipeline runs cargo build/test in the Docker VM, so this bounds
// how many native-dep compiles run at once. The ceiling is the VM's memory
// allocation (Docker Desktop), not host RAM — too high OOMs/swaps the VM
// regardless of host size. 4 assumes the VM is sized well above the 8 GB the
// mold / VITEST_MAX_FORKS workarounds were tuned for.
const MAX_PARALLEL_ISSUES = 4;

function sh(command: string): string {
  return execSync(command, { encoding: "utf-8" }).trim();
}

// Promise.allSettled with a worker-pool ceiling: preserves the settled-result
// shape callers depend on while bounding how many tasks run at once.
async function mapWithConcurrency<T, R>(
  items: T[],
  limit: number,
  task: (item: T) => Promise<R>,
): Promise<PromiseSettledResult<R>[]> {
  const results = new Array<PromiseSettledResult<R>>(items.length);
  let next = 0;
  async function worker(): Promise<void> {
    while (next < items.length) {
      const index = next++;
      try {
        results[index] = {
          status: "fulfilled",
          value: await task(items[index]!),
        };
      } catch (reason) {
        results[index] = { status: "rejected", reason };
      }
    }
  }
  await Promise.all(
    Array.from({ length: Math.min(limit, items.length) }, worker),
  );
  return results;
}

// Resolved once at startup so an external checkout switch mid-run can't shift
// the fork point of workset branches or the base of their PRs.
const SOURCE_BRANCH = sh("git rev-parse --abbrev-ref HEAD");

// Implementer/reviewer/merger sandboxes build the project, so they need the
// workspace's node_modules; planner/grouper only touch git + gh and skip this
// entirely. The image bakes a warm pnpm store (see .sandcastle/Dockerfile), so
// --prefer-offline links from the store instead of resolving the registry cold.
//
// The bind-mounted worktree shadows the image's baked dist/, and dist/ is
// gitignored so fresh worktrees lack it — recreate the empty placeholder Tauri's
// generate_context! (frontendDist: "../dist") needs to compile, byte-identical
// to the image build so the app crate's fingerprint doesn't churn.
//
// 180s is a ceiling, not a wait: the store sits on overlayfs while node_modules
// lands on a VirtioFS bind mount, so install full-copies (no cross-fs hardlink)
// and can run slow on Docker Desktop. The slack costs nothing on warm runs.
const hooksWithNode = {
  sandbox: {
    onSandboxReady: [
      { command: "mkdir -p dist && touch dist/index.html", timeoutMs: 10_000 },
      {
        command: "pnpm install --frozen-lockfile --prefer-offline",
        timeoutMs: 180_000,
      },
    ],
  },
};

const copyToWorktree = ["pnpm-lock.yaml"];

// ---------------------------------------------------------------------------
// Sandbox image preflight
// ---------------------------------------------------------------------------

// Pinned so the preflight can find the image by name and so manual
// `pnpm sandcastle:build` runs agree with these runs. Must match the
// --image-name in package.json's sandcastle:build script.
const IMAGE_NAME = "wispr-sandcastle";

// Records the lockfile hash the current image was built from. Local +
// gitignored; paired with an image-existence check so a deleted image forces a
// rebuild even when the stamp is stale.
const IMAGE_STAMP_FILE = ".sandcastle/.image-lockhash";

// Bumping a dependency invalidates the baked pnpm store / cargo registry /
// compiled target; a pure source edit does not — the warm target is a starting
// point, not a source of truth, and cargo recompiles changed crates at runtime.
// So rebuild only when these move, not on every run.
const IMAGE_LOCKFILES = ["pnpm-lock.yaml", "src-tauri/Cargo.lock"];

function dockerSandbox() {
  return docker({ imageName: IMAGE_NAME });
}

function lockfileHash(): string {
  const hash = createHash("sha256");
  for (const file of IMAGE_LOCKFILES) hash.update(readFileSync(file));
  return hash.digest("hex");
}

function imageExists(): boolean {
  try {
    execSync(`docker image inspect ${IMAGE_NAME}`, { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

// Build the image when it's missing or a dependency lockfile changed since it
// was built. Source-only changes skip the rebuild. A rebuild after a manual
// `pnpm sandcastle:build` is Docker-layer-cached and near-instant, which also
// self-heals the stamp.
function ensureImage(): void {
  const hash = lockfileHash();
  const stamp = existsSync(IMAGE_STAMP_FILE)
    ? readFileSync(IMAGE_STAMP_FILE, "utf-8").trim()
    : "";
  const exists = imageExists();
  if (exists && stamp === hash) return;

  const reason = exists ? "lockfiles changed" : "image missing";
  console.log(`Building sandbox image '${IMAGE_NAME}' (${reason})...`);
  execSync("pnpm sandcastle:build", { stdio: "inherit" });
  writeFileSync(IMAGE_STAMP_FILE, hash);
}

// ---------------------------------------------------------------------------
// Backlog discovery
// ---------------------------------------------------------------------------

type IssueRef = { id: string; title: string };

type Workset = {
  name: string;
  branch: string;
  prd?: IssueRef;
  issues: IssueRef[];
};

const BACKLOG_RESPONSE = z.object({
  data: z.object({
    repository: z.object({
      issues: z.object({
        nodes: z.array(
          z.object({
            number: z.number(),
            title: z.string(),
            body: z.string(),
            parent: z
              .object({ number: z.number(), title: z.string() })
              .nullable(),
          }),
        ),
      }),
    }),
  }),
});

// Issues created by the to-issues skill record their PRD in a "## Parent"
// body section rather than the sub-issues relationship — honor both.
const PARENT_SECTION = /## Parent\s+#(\d+)/;

function fetchBacklog(): { issue: IssueRef; parent?: IssueRef }[] {
  const [owner, repo] = sh(
    "gh repo view --json nameWithOwner -q .nameWithOwner",
  ).split("/");
  const query = `query($owner:String!,$repo:String!){repository(owner:$owner,name:$repo){issues(states:OPEN,labels:["Sandcastle"],first:100){nodes{number title body parent{number title}}}}}`;
  const response = BACKLOG_RESPONSE.parse(
    JSON.parse(
      sh(
        `gh api graphql -F owner=${owner} -F repo=${repo} -f query='${query}'`,
      ),
    ),
  );

  const parentTitles = new Map<string, string>();
  const parentFromBody = (body: string): IssueRef | undefined => {
    const id = body.match(PARENT_SECTION)?.[1];
    if (id === undefined) return undefined;
    const title =
      parentTitles.get(id) ??
      sh(`gh issue view ${id} --json title --jq .title`);
    parentTitles.set(id, title);
    return { id, title };
  };

  return response.data.repository.issues.nodes.map((node) => ({
    issue: { id: String(node.number), title: node.title },
    parent: node.parent
      ? { id: String(node.parent.number), title: node.parent.title }
      : parentFromBody(node.body),
  }));
}

function openIssueIds(): Set<string> {
  const numbers = z
    .array(z.number())
    .parse(
      JSON.parse(
        sh(
          `gh issue list --state open --label Sandcastle --limit 100 --json number --jq '[.[].number]'`,
        ),
      ),
    );
  return new Set(numbers.map(String));
}

// ---------------------------------------------------------------------------
// Phase 0: Group into worksets
// ---------------------------------------------------------------------------

const CLUSTERS_OUTPUT = sandcastle.Output.object({
  tag: "clusters",
  schema: z.object({ clusters: z.array(z.array(z.string())) }),
});

// One resume retry: malformed output costs only a re-emit of the tag, not a
// re-run of the agent's whole analysis.
async function runGrouper(parentless: IssueRef[]) {
  const options = {
    sandbox: dockerSandbox(),
    name: "grouper",
    maxIterations: 1,
    agent: sandcastle.claudeCode("claude-opus-4-8"),
    output: CLUSTERS_OUTPUT,
  };
  try {
    return await sandcastle.run({
      ...options,
      promptFile: "./.sandcastle/group-prompt.md",
      promptArgs: {
        PARENTLESS_IDS: parentless
          .map((i) => `#${i.id} (${i.title})`)
          .join(", "),
      },
    });
  } catch (error) {
    if (
      !(error instanceof sandcastle.StructuredOutputError) ||
      error.sessionId === undefined
    ) {
      throw error;
    }
    return await sandcastle.run({
      ...options,
      resumeSession: error.sessionId,
      prompt: `Your previous output failed validation: ${error.message}. Re-emit the complete clusters JSON inside <clusters> tags.`,
    });
  }
}

async function clusterParentless(
  parentless: IssueRef[],
): Promise<IssueRef[][]> {
  if (parentless.length === 0) return [];
  if (parentless.length === 1) return [parentless];

  const result = await runGrouper(parentless);
  const byId = new Map(parentless.map((issue) => [issue.id, issue]));
  const assigned = new Set<string>();
  const clusters: IssueRef[][] = [];
  for (const ids of result.output.clusters) {
    const cluster = ids
      .filter((id) => byId.has(id) && !assigned.has(id))
      .map((id) => byId.get(id)!);
    for (const issue of cluster) assigned.add(issue.id);
    if (cluster.length > 0) clusters.push(cluster);
  }
  // Omissions become standalone worksets so one bad cluster output can't
  // silently drop an issue from the run.
  for (const issue of parentless) {
    if (assigned.has(issue.id)) continue;
    console.warn(`  grouper omitted #${issue.id}; treating as standalone`);
    clusters.push([issue]);
  }
  return clusters;
}

function worksetForCluster(cluster: IssueRef[]): Workset {
  const ids = cluster.map((issue) => Number(issue.id)).sort((a, b) => a - b);
  const name = ids.length === 1 ? `issue-${ids[0]}` : `issues-${ids.join("-")}`;
  return { name, branch: `sandcastle/${name}`, issues: cluster };
}

async function buildWorksets(): Promise<Workset[]> {
  const byPrd = new Map<string, { prd: IssueRef; issues: IssueRef[] }>();
  const parentless: IssueRef[] = [];
  for (const { issue, parent } of fetchBacklog()) {
    if (!parent) {
      parentless.push(issue);
      continue;
    }
    const group = byPrd.get(parent.id) ?? { prd: parent, issues: [] };
    group.issues.push(issue);
    byPrd.set(parent.id, group);
  }
  const prdWorksets = [...byPrd.values()].map(({ prd, issues }) => ({
    name: `prd-${prd.id}`,
    branch: `sandcastle/prd-${prd.id}`,
    prd,
    issues,
  }));
  // A labeled umbrella issue is tracking overhead, not implementable work —
  // its children carry the actual tasks, so it must never reach an implementer.
  const workable = parentless.filter((issue) => {
    if (!byPrd.has(issue.id)) return true;
    console.warn(
      `  skipping #${issue.id}: it is the parent of other backlog issues`,
    );
    return false;
  });
  const clusters = await clusterParentless(workable);
  return [...prdWorksets, ...clusters.map(worksetForCluster)];
}

// ---------------------------------------------------------------------------
// Branch plumbing
// ---------------------------------------------------------------------------

function branchExists(branch: string): boolean {
  try {
    execSync(`git rev-parse --verify --quiet refs/heads/${branch}`);
    return true;
  } catch {
    return false;
  }
}

function ensureBranch(branch: string, base: string): void {
  if (!branchExists(branch)) sh(`git branch ${branch} ${base}`);
}

function commitsAhead(branch: string, base: string): number {
  return Number(sh(`git rev-list --count ${base}..${branch}`));
}

// An issue branch left from a previous iteration without unique commits is
// moved to the workset tip so new work builds on already-merged waves. A
// branch with unmerged commits keeps its progress; the merger reconciles.
function syncIssueBranch(branch: string, base: string): void {
  if (!branchExists(branch)) {
    sh(`git branch ${branch} ${base}`);
    return;
  }
  if (commitsAhead(branch, base) === 0) sh(`git branch -f ${branch} ${base}`);
}

// ---------------------------------------------------------------------------
// Phase 1: Plan
// ---------------------------------------------------------------------------

const PLAN_OUTPUT = sandcastle.Output.object({
  tag: "plan",
  schema: z.object({
    issues: z.array(z.object({ id: z.string(), workset: z.string() })),
  }),
});

async function runPlanner(pending: Workset[], open: Set<string>) {
  const options = {
    sandbox: dockerSandbox(),
    name: "planner",
    maxIterations: 1,
    // Opus for planning: dependency analysis benefits from deeper reasoning.
    agent: sandcastle.claudeCode("claude-opus-4-8"),
    output: PLAN_OUTPUT,
  };
  const worksetsJson = JSON.stringify(
    pending.map((workset) => ({
      name: workset.name,
      issues: workset.issues.filter((issue) => open.has(issue.id)),
    })),
    null,
    2,
  );
  try {
    return await sandcastle.run({
      ...options,
      promptFile: "./.sandcastle/plan-prompt.md",
      promptArgs: { WORKSETS: worksetsJson },
    });
  } catch (error) {
    if (
      !(error instanceof sandcastle.StructuredOutputError) ||
      error.sessionId === undefined
    ) {
      throw error;
    }
    return await sandcastle.run({
      ...options,
      resumeSession: error.sessionId,
      prompt: `Your previous output failed validation: ${error.message}. Re-emit the complete plan JSON inside <plan> tags.`,
    });
  }
}

// ---------------------------------------------------------------------------
// Phase 2: Execute + tiered review
// ---------------------------------------------------------------------------

// Skip the reviewer entirely when the implementer's diff is too small or
// scoped to files where LLM review wouldn't add value over the implementer's
// own typecheck + tests. Calibrate against `.sandcastle/logs/` history.
const TRIAGE_MIN_LINES = 30;
const TRIAGE_TRIVIAL_PATHS = [
  /^pnpm-lock\.yaml$/,
  /\.snap$/,
  /\.test\.(ts|tsx|rs)$/,
  /^docs\//,
  /\.md$/,
];

function triageDiff(
  base: string,
  branch: string,
): { trivial: boolean; reason: string } {
  const shortstat = sh(`git diff --shortstat ${base}...${branch}`);
  const files = sh(`git diff --name-only ${base}...${branch}`)
    .split("\n")
    .filter(Boolean);

  const linesChanged = [
    ...shortstat.matchAll(/(\d+) (insertion|deletion)/g),
  ].reduce((sum, match) => sum + Number(match[1]!), 0);

  if (linesChanged > 0 && linesChanged < TRIAGE_MIN_LINES) {
    return { trivial: true, reason: `${linesChanged} lines changed` };
  }

  const allTrivialPaths =
    files.length > 0 &&
    files.every((file) => TRIAGE_TRIVIAL_PATHS.some((p) => p.test(file)));
  if (allTrivialPaths) {
    return { trivial: true, reason: "only tests/docs/lockfile touched" };
  }

  return { trivial: false, reason: "" };
}

async function runIssuePipeline(issue: IssueRef, workset: Workset) {
  const branch = `sandcastle/issue-${issue.id}`;
  syncIssueBranch(branch, workset.branch);

  const sandbox = await sandcastle.createSandbox({
    branch,
    sandbox: dockerSandbox(),
    hooks: hooksWithNode,
    copyToWorktree,
  });

  try {
    const implement = await sandbox.run({
      name: "implementer",
      maxIterations: 100,
      idleTimeoutSeconds: AGENT_IDLE_TIMEOUT_SECONDS,
      agent: sandcastle.claudeCode("claude-sonnet-4-6"),
      promptFile: "./.sandcastle/implement-prompt.md",
      promptArgs: {
        TASK_ID: issue.id,
        ISSUE_TITLE: issue.title,
        BRANCH: branch,
      },
    });

    if (implement.commits.length === 0) {
      return implement;
    }

    const triage = triageDiff(workset.branch, branch);
    if (triage.trivial) {
      console.log(`  ${issue.id}: skipping reviewer (${triage.reason})`);
      return implement;
    }

    // Tier 1: Haiku — fixes mechanical issues itself, escalates the rest.
    const tier1 = await sandbox.run({
      name: "reviewer-tier1",
      maxIterations: 1,
      idleTimeoutSeconds: AGENT_IDLE_TIMEOUT_SECONDS,
      agent: sandcastle.claudeCode("claude-haiku-4-5"),
      promptFile: "./.sandcastle/review-prompt-tier1.md",
      // BASE_BRANCH is explicit because 0.7's built-in TARGET_BRANCH is the
      // host's checked-out branch, not the workset branch reviews diff against.
      promptArgs: { BRANCH: branch, BASE_BRANCH: workset.branch },
    });

    // sandbox.run() has no structured-output option, so verdicts are parsed
    // from stdout. Since 0.6.0, stdout is a bounded rolling tail (64KiB
    // default) — parseable only because the tier-1 prompt requires the
    // verdict tags at the very end of the response.
    const escalate = /<verdict>\s*ESCALATE\s*<\/verdict>/.test(tier1.stdout);
    const concerns =
      tier1.stdout.match(/<concerns>([\s\S]*?)<\/concerns>/)?.[1]?.trim() ?? "";

    let tier2Commits: typeof tier1.commits = [];
    if (escalate) {
      const preview = concerns.replace(/\s+/g, " ").slice(0, 80);
      console.log(
        `  ${issue.id}: escalating to Sonnet${preview ? ` — ${preview}` : ""}`,
      );
      const tier2 = await sandbox.run({
        name: "reviewer-tier2",
        maxIterations: 1,
        idleTimeoutSeconds: AGENT_IDLE_TIMEOUT_SECONDS,
        agent: sandcastle.claudeCode("claude-sonnet-4-6"),
        promptFile: "./.sandcastle/review-prompt-tier2.md",
        promptArgs: {
          BRANCH: branch,
          BASE_BRANCH: workset.branch,
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
}

// ---------------------------------------------------------------------------
// Phase 3: Merge + pull requests
// ---------------------------------------------------------------------------

async function mergeWorkset(workset: Workset, completed: IssueRef[]) {
  const sandbox = await sandcastle.createSandbox({
    branch: workset.branch,
    sandbox: dockerSandbox(),
    hooks: hooksWithNode,
    copyToWorktree,
  });
  try {
    await sandbox.run({
      name: `merger-${workset.name}`,
      maxIterations: 5,
      idleTimeoutSeconds: AGENT_IDLE_TIMEOUT_SECONDS,
      agent: sandcastle.claudeCode("claude-opus-4-8"),
      promptFile: "./.sandcastle/merge-prompt.md",
      promptArgs: {
        BRANCHES: completed.map((i) => `- sandcastle/issue-${i.id}`).join("\n"),
        ISSUES: completed.map((i) => `- ${i.id}: ${i.title}`).join("\n"),
      },
    });
  } finally {
    await sandbox.close();
  }
}

const PR_BODY_DIR = mkdtempSync(join(tmpdir(), "sandcastle-pr-"));

function shellQuote(value: string): string {
  return `'${value.replaceAll("'", `'\\''`)}'`;
}

function openPullRequest(workset: Workset): void {
  if (commitsAhead(workset.branch, SOURCE_BRANCH) === 0) {
    console.log(
      `  ${workset.name}: issues closed but no commits over ${SOURCE_BRANCH}; skipping PR`,
    );
    return;
  }
  const title = workset.prd
    ? workset.prd.title
    : `Sandcastle: ${workset.issues.map((i) => `#${i.id}`).join(", ")}`;
  const bodyLines = [
    ...(workset.prd ? [`Closes #${workset.prd.id}`, ""] : []),
    "Implemented by Sandcastle:",
    ...workset.issues.map((i) => `- #${i.id} ${i.title}`),
  ];
  const bodyFile = join(PR_BODY_DIR, `${workset.name}.md`);
  writeFileSync(bodyFile, bodyLines.join("\n"));

  sh(`git push -u origin ${workset.branch}`);
  sh(
    `gh pr create --base ${SOURCE_BRANCH} --head ${workset.branch} --title ${shellQuote(title)} --body-file ${shellQuote(bodyFile)}`,
  );
  console.log(`  ${workset.name}: pull request opened`);
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

console.log(`Sandcastle starting from ${SOURCE_BRANCH}`);

ensureImage();

const worksets = await buildWorksets();
if (worksets.length === 0) {
  console.log("No open Sandcastle issues. Exiting.");
  process.exit(0);
}

console.log(`${worksets.length} workset(s):`);
for (const workset of worksets) {
  ensureBranch(workset.branch, SOURCE_BRANCH);
  console.log(
    `  ${workset.name}: ${workset.issues.map((i) => `#${i.id}`).join(", ")} → ${workset.branch}`,
  );
}

const worksetsByName = new Map(worksets.map((ws) => [ws.name, ws]));
const finalized = new Set<string>();

function finalizeCompletedWorksets(open: Set<string>): void {
  for (const workset of worksets) {
    if (finalized.has(workset.name)) continue;
    if (workset.issues.some((issue) => open.has(issue.id))) continue;
    try {
      openPullRequest(workset);
      finalized.add(workset.name);
    } catch (error) {
      // Push/PR failures are usually transient (auth, network); the workset
      // stays unfinalized so the next iteration retries.
      console.error(`  ${workset.name}: pull request failed: ${error}`);
    }
  }
}

// A worktree left behind by an interrupted run keeps its branch checked out,
// so the per-issue `git branch -f` reset fails and the scheduler skips that
// issue on every subsequent run.
function removeStaleWorktrees() {
  const stalePaths = sh("git worktree list --porcelain")
    .split("\n")
    .filter((line) => line.startsWith("worktree "))
    .map((line) => line.slice("worktree ".length))
    .filter((path) => path.includes("/.sandcastle/worktrees/"));
  for (const path of stalePaths) {
    try {
      sh(`git worktree remove --force ${shellQuote(path)}`);
      console.log(`Removed stale worktree: ${path}`);
    } catch (error) {
      console.warn(`Could not remove worktree ${path}: ${error}`);
    }
  }
}

removeStaleWorktrees();

for (let iteration = 1; iteration <= MAX_ITERATIONS; iteration++) {
  console.log(`\n=== Iteration ${iteration}/${MAX_ITERATIONS} ===\n`);

  const open = openIssueIds();
  finalizeCompletedWorksets(open);

  const pending = worksets.filter(
    (ws) => !finalized.has(ws.name) && ws.issues.some((i) => open.has(i.id)),
  );
  if (pending.length === 0) {
    console.log("All worksets complete.");
    break;
  }

  const plan = await runPlanner(pending, open);
  const selected = plan.output.issues.flatMap(({ id, workset }) => {
    const ws = worksetsByName.get(workset);
    const issue = ws?.issues.find((i) => i.id === id);
    return ws && issue && open.has(id) ? [{ issue, ws }] : [];
  });

  if (selected.length === 0) {
    console.log(
      "Remaining issues are blocked across worksets. Exiting; rerun after open PRs merge.",
    );
    break;
  }

  console.log(
    `Planning complete. ${selected.length} issue(s) to work in parallel:`,
  );
  for (const { issue, ws } of selected) {
    console.log(`  ${issue.id} [${ws.name}]: ${issue.title}`);
  }

  const settled = await mapWithConcurrency(
    selected,
    MAX_PARALLEL_ISSUES,
    ({ issue, ws }) => runIssuePipeline(issue, ws),
  );

  const completedByWorkset = new Map<string, IssueRef[]>();
  for (const [i, outcome] of settled.entries()) {
    const { issue, ws } = selected[i]!;
    if (outcome.status === "rejected") {
      console.error(`  ✗ ${issue.id} (${ws.name}) failed: ${outcome.reason}`);
      continue;
    }
    const issueBranch = `sandcastle/issue-${issue.id}`;
    const needsMerge =
      outcome.value.commits.length > 0 ||
      commitsAhead(issueBranch, ws.branch) > 0;
    // Branch already merged into workset but GitHub issue still open —
    // no merge needed, but merger must still close the issue.
    const mergedNeedsClose =
      commitsAhead(issueBranch, ws.branch) === 0 &&
      commitsAhead(issueBranch, SOURCE_BRANCH) > 0;
    if (!needsMerge && !mergedNeedsClose) continue;
    completedByWorkset.set(ws.name, [
      ...(completedByWorkset.get(ws.name) ?? []),
      issue,
    ]);
  }

  if (completedByWorkset.size === 0) {
    console.log("No commits produced. Nothing to merge.");
    continue;
  }

  const mergeEntries = [...completedByWorkset.entries()];
  const merges = await Promise.allSettled(
    mergeEntries.map(([name, issues]) =>
      mergeWorkset(worksetsByName.get(name)!, issues),
    ),
  );
  for (const [i, outcome] of merges.entries()) {
    if (outcome.status === "rejected") {
      console.error(
        `  ✗ merge for ${mergeEntries[i]![0]} failed: ${outcome.reason}`,
      );
    }
  }

  console.log("\nMerge phase complete.");
}

finalizeCompletedWorksets(openIssueIds());

const unfinished = worksets.filter((ws) => !finalized.has(ws.name));
if (unfinished.length > 0) {
  console.log(
    `Worksets without a PR (unfinished or deferred): ${unfinished
      .map((ws) => ws.name)
      .join(", ")}`,
  );
}

console.log("\nAll done.");
