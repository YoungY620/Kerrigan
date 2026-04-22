# Kerrigan

**Core thesis: the task result IS the communication.** No `sendMessage`, no message queues, no subscription protocols. Every dependency between agents is satisfied when the upstream node writes its `result_summary` — nothing else flows between agents.

Traditional multi-agent frameworks import human-organization complexity (point-to-point messages, broadcasts, interrupts) and inherit its failure modes: context write-amplification, interrupted work, coordination deadlocks. Kerrigan eliminates the category by making structured results the only medium.

DAG is the **means**, not the end: dependencies become edges, topological order replaces protocols, and parallelism falls out for free.

## Design

- **Result = communication**: agents never message each other. A downstream node reads its predecessors' `result_summary` via `kerrigan dag get` — that is the entire coordination surface.
- **Context optimization**: each agent receives exactly the context needed for its subtask — no pollution from irrelevant chatter.
- **Automatic parallelization**: independent nodes execute concurrently; completed nodes unlock successors via cascading `Ready`.
- **Ships as a Claude Code skill**: task decomposition methodology, result-reporting templates, and scenario guides (software dev, research, AB testing, git management) are bundled into the binary and deployed by `kerrigan init`.

## Prerequisites

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) or Kimi CLI installed and in PATH
- Default adapter: `claude`. Use `--adapter kimi` to switch.

## Install

One-liner (clones repo, deploys agent config + skills, optionally builds CLI):

```bash
# clone from GitHub (public)
curl -fsSL \
  "https://raw.githubusercontent.com/YoungY620/Kerrigan/main/scripts/install.sh" | sh
```

This will:

1. Clone the repo to `~/.kerrigan/.source/`
2. Deploy `default-agent` config to `~/.kitty/agents/kerrigan-agent/`
3. Deploy skills to `~/.kerrigan/skills/`
4. Build the `kerrigan` CLI (if `cargo` is available)

Re-run to update. Then make the skill discoverable by your agent:

- **Kitty Worker**: pass `--skills ~/.kerrigan/skills` when launching the worker
- **Claude Code**: symlink into the user skills directory
  ```bash
  ln -sfn ~/.kerrigan/skills/kerrigan ~/.claude/skills/kerrigan
  ```
- **Kimi CLI**: `kerrigan exec` auto-injects `--skills-dir ~/.kerrigan/skills`, no extra config needed

### Manual install

If you prefer not to pipe to shell:

```bash
git clone https://github.com/YoungY620/Kerrigan.git ~/.kerrigan/.source
cd ~/.kerrigan/.source
sh scripts/install.sh
```

## Usage

```bash
# Create a task
kerrigan task create --summary "Refactor auth module" --workdir .
# → {"task_id":"abc-123"}

# Build the DAG
kerrigan dag add --task abc-123 --summary "Analyze current code"
kerrigan dag add --task abc-123 --summary "Design new architecture" --depends-on <node-1>
kerrigan dag add --task abc-123 --summary "Implement changes" --depends-on <node-2>

# Execute ready nodes
kerrigan dag ready --task abc-123
kerrigan exec --task abc-123 --node <node-id> --adapter claude
```

Full command reference: [`skills/kerrigan/SKILL.md`](skills/kerrigan/SKILL.md).

## Storage Layout

```text
~/.kerrigan/
├── registry.db                  # global task registry (SQLite)
├── skills/kerrigan/             # deployed by `kerrigan init`
│   ├── SKILL.md
│   ├── guide.md
│   ├── guide-software-development.md
│   ├── guide-research.md
│   ├── guide-decision-ab-testing.md
│   └── guide-git-management.md
└── tasks/<task-id>/
    ├── task.db                  # per-task DAG state
    └── logs/
```

## Documentation

- [Design](docs/design.md) — core design philosophy and architecture
- [Guide Design](docs/guide-design.md) — guide file system design
- [Milestone](docs/milestone.md) — roadmap
- [default-agent/](default-agent/README.md) — Kitty agent bundle + full Feishu deployment walkthrough
- [implement-a-chain](https://github.com/YoungY620/Kerrigan) — head-to-head experiment against Claude Code agent teams; Kerrigan performs on par
