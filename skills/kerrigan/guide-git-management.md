# Git Parallel Development Management Guide

When multiple development nodes modify the same repo in parallel, an isolation strategy is needed to avoid conflicts.

For how features are broken down into recursive subtask trees, the breakdown criteria, and the backtracking merge process, refer to `{{ shared_dir }}/guide-software-development.md`.

---

## Isolation Strategy: Git Worktrees

Use **git worktrees** to give each parallel development node its own isolated working directory on the same machine, while sharing one object database and refs.

```text
workspace/
├── repo/                       # main worktree (typically on main branch)
│   └── .git/                   # full repo; objects + refs shared across all worktrees
├── repo-feat-a/                # additional worktree on feature/a
│   └── .git                    # file (not dir), points into repo/.git/worktrees/feat-a
└── repo-feat-b/                # additional worktree on feature/b
    └── .git                    # points into repo/.git/worktrees/feat-b
```

### Creating, using, removing

```bash
cd workspace/repo
git worktree add ../repo-feat-a -b feature/a    # new branch feature/a on new worktree
git worktree add ../repo-feat-b feature/b       # existing branch feature/b
# ... develop independently in each directory ...
git worktree list                               # inspect all worktrees
git worktree remove ../repo-feat-a              # clean removal
git worktree prune                              # reap orphan metadata after manual rm
```

### What is isolated (safe to parallelize)

- **Files on disk** — each worktree is a separate directory with its own checkout; editing in one never affects another
- **HEAD** — each worktree is on its own branch/commit
- **Index (staging area)** — `git add` / `git commit` in one worktree is invisible to others
- **Reflog** — each worktree has its own local operation history
- **In-progress merge / rebase / cherry-pick state** — scoped to one worktree

### What is shared (saves disk and bandwidth)

- **Object database** (`.git/objects/`) — every commit/tree/blob lives once, visible to all worktrees
- **Refs** (branches, tags, remotes) — a single `git fetch` updates every worktree's view at once
- **Config, hooks, remote URLs**

### Hard rules

- **A branch can be checked out in at most one worktree at a time.** Git will refuse a second checkout. This is a feature, not a limitation — it prevents two nodes from stomping on each other on the same branch.
- **Removing a worktree must use `git worktree remove <path>`** (or `rm -rf <path> && git worktree prune` as a cleanup fallback). A bare `rm -rf` leaves orphan metadata under the main `.git/worktrees/`.
- **Never manipulate the main repo's `.git/` directory directly.** It is shared state; corruption there breaks every worktree.

### How the orchestrator assigns nodes

When assigning a development node, the orchestrator specifies **which worktree path + which branch** to use (e.g., "develop on `workspace/repo-feat-a` on branch `feature/a`"). If the worktree does not yet exist, the assignee creates it with `git worktree add` as the first step. After the feature is merged, the worktree is removed (`git worktree remove`) before the branch is deleted.

What runs on a worktree must be a **leaf node** of the branch tree (the lowest-level incomplete sub-branch), or a **feature node whose child nodes have all been merged and is ready for integration at its level**. A large, not-yet-decomposed feature must not be started on a worktree.

### Common pitfalls

- **Same branch in two worktrees** — Git refuses it. If you need two parallel lines of work on "the same" starting point, create a second branch: `git worktree add ../repo-feat-a-alt -b feature/a-alt feature/a`.
- **Stash is per-repo, not per-worktree.** With multiple active worktrees, prefer WIP commits (amend later) over `git stash` to avoid cross-contamination.
- **Submodules** — each worktree has its own submodule checkout. Re-run `git submodule update --init` after `git worktree add` when the project uses submodules.
- **IDE scanning the parent directory** may treat sibling worktrees as one huge project. Open each worktree in its own IDE window.
- **Hooks run in the worktree's CWD but from the shared `.git/hooks/`** — hook scripts must be worktree-agnostic (use `git rev-parse --show-toplevel`, not absolute paths).

### Why worktrees (and not multi-clone)

- **Single source of truth for refs** — `git fetch` in any worktree updates everyone's view; no per-track drift.
- **One object database** — disk and bandwidth savings scale with N (number of parallel tracks).
- **Zero-cost switch-over** after a feature merges: `git worktree remove` and `git worktree add` on the next feature, no re-clone or submodule re-init over the network.

---

## Branch Management

- **Branch naming**: `feature/<short-description>`
- **Control distance from the main branch**: After completing each small step, consider whether it can be merged, or at least pull the latest changes from the main branch. The key factor is the size and divergence of the diff, not time
- **Merge promptly**: Merge immediately after verification passes. If a feature is too large to merge quickly, it should be broken down further
- **One branch, one worktree**: A branch being actively developed is always bound to exactly one worktree. When assigning a new feature, create a fresh worktree; when the feature merges, remove the worktree before deleting the branch

---

## Merge Process

```text
[Feature development complete on worktree W, branch feature/X]
  → [Run CI/tests on the feature branch]
  → [Merge review] (multiple independent reviews: from other parallel branches' perspective, from each dependent module's perspective, from the overall architecture perspective to assess merge impact)
  → [Reviews unanimously pass → merge directly to main branch]
  → [Delete feature branch]
  → [Remove worktree W: git worktree remove <path>]
  → [Other parallel features rebase onto latest main branch]
```

After each feature is merged into the main branch, all other in-progress features rebase (from within their own worktrees) to surface conflicts early.

---

## Conflict Resolution

- First-merged feature takes priority
- Later-merged features rebase in their own worktree and re-run tests
- If conflicts are difficult to resolve, it indicates a problem with task decomposition — there are unidentified implicit dependencies. Changes should be reverted, and the DAG topology should be re-examined: establish dependencies to serialize, redraw feature boundaries, or even restructure the sub-DAG

---

## Coordination with the DAG

When planning parallel development nodes:

- **File sets do not overlap at all** → Separate worktrees on independent branches, safe to parallelize
- **Partial overlap with small modifications** → Separate worktrees on independent branches; handle conflicts at merge time (rebase the later one)
- **Significant overlap** → Sequential execution on one worktree, or redraw feature boundaries

The orchestrator should analyze the scope of file modifications when decomposing tasks, decide whether worktrees can safely run in parallel, and assign each node a concrete `(worktree path, branch name)` pair.
