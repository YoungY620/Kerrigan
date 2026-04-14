# Git Parallel Development Management Guide

When multiple development nodes modify the same repo in parallel, an isolation strategy is needed to avoid conflicts.

For how features are broken down into recursive subtask trees, the breakdown criteria, and the backtracking merge process, refer to `{{ shared_dir }}/guide-software-development.md`.

---

## Isolation Strategy: Parallel Tracks

Avoid using git worktree (complex state, error-prone). Use an **independent clone** strategy — providing N equivalent parallel tracks:

```text
workspace/
├── track-1/
│   └── repo/
├── track-2/
│   └── repo/
└── track-3/
    └── repo/
```

Each track is an independent repo clone, equivalent to one another. Create as many as needed for parallel work. Tracks are reused long-term: after a feature is merged, pull the latest main and proceed to the next feature.

When the orchestrator assigns development nodes, it specifies which track to use in the node description (e.g., "develop on track-2"), ensuring only one task is running on each track at any given time.

What runs on a track must be a **leaf node** of the branch tree (the lowest-level incomplete sub-branch), or a **feature node whose child nodes have all been merged and is ready for integration at its level**. Running a large feature that has not been fully broken down on a track is not allowed.

---

## Branch Management

- **Branch naming**: `feature/<short-description>`
- **Control distance from the main branch**: After completing each small step, consider whether it can be merged, or at least pull the latest changes from the main branch. The key factor is the size and divergence of the diff, not time
- **Merge promptly**: Merge immediately after verification passes. If a feature is too large to merge quickly, it should be broken down further

---

## Merge Process

```text
[Feature development complete]
  → [Run CI/tests on the feature branch]
  → [Merge review] (multiple independent reviews: from other parallel branches' perspective, from each dependent module's perspective, from the overall architecture perspective to assess merge impact)
  → [Reviews unanimously pass → merge directly to main branch]
  → [Delete feature branch]
  → [Other parallel features rebase onto latest main branch]
```

After each feature is merged into the main branch, all other in-progress features should rebase to surface conflicts early.

---

## Conflict Resolution

- First-merged feature takes priority
- Later-merged features need to rebase and resolve conflicts, then re-run tests
- If conflicts are difficult to resolve, it indicates a problem with task decomposition — there are unidentified implicit dependencies. Changes should be reverted, and the DAG topology should be re-examined: establish dependencies to serialize, redraw feature boundaries, or even restructure the sub-DAG

---

## Coordination with the DAG

When planning parallel development nodes:
- File sets **do not overlap at all** → Independent clones, safe to parallelize
- **Partial overlap with small modifications** → Different branches in the same clone, handle conflicts at merge time
- **Significant overlap** → Sequential execution is safer, or redraw feature boundaries

The orchestrator should analyze the scope of file modifications when decomposing tasks, and select the isolation strategy accordingly.
