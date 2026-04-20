# Kerrigan

**Core thesis: the task result IS the communication.** No `sendMessage`, no message queues, no subscription protocols. Every dependency between agents is satisfied when the upstream node writes its `result_summary` — nothing else flows between agents.

Traditional multi-agent frameworks import human-organization complexity (point-to-point messages, broadcasts, interrupts) and inherit its failure modes: context write-amplification, interrupted work, coordination deadlocks. Kerrigan eliminates the category by making structured results the only medium.

DAG is the **means**, not the end: dependencies become edges, topological order replaces protocols, and parallelism falls out for free.

## Design

- **Result = communication**: agents never message each other. A downstream node reads its predecessors' `result_summary` via `kerrigan dag get` — that is the entire coordination surface.
- **Context optimization**: each agent receives exactly the context needed for its subtask — no pollution from irrelevant chatter.
- **Automatic parallelization**: independent nodes execute concurrently; completed nodes unlock successors via cascading `Ready`.
- **Built-in guide system**: guide files deployed to `~/.kerrigan/shared/` provide agents with task decomposition strategy, result-reporting templates, and scenario guidance (software development, research, AB testing, git management).

## Components

| Component | Description |
|-----------|-------------|
| [kerrigan-storage](kerrigan-storage/) | DAG engine & persistent storage — manages tasks, nodes, and their dependencies |
| [kerrigan-cli](kerrigan-cli/) | Orchestration runner — calls agents in a loop, validates DAG, rotates sessions |

## Prerequisites

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) or Kimi CLI installed and in PATH
- Default adapter: `claude`. Use `--adapter kimi` to switch.

## Installation

Download pre-built binaries from the [Releases](https://github.com/YoungY620/harness/kerrigan/-/releases) page, or build from source:

```bash
# Build from source
git clone --recurse-submodules ssh://git@github.com/YoungY620:30022/harness/kerrigan.git
cd kerrigan

# Build both components
cd kerrigan-storage && cargo build --release && cd ..
cd kerrigan-cli && cargo build --release && cd ..

# Copy binaries to PATH
cp kerrigan-storage/target/release/kerrigan-storage ~/.local/bin/
cp kerrigan-cli/target/release/kerrigan-cli ~/.local/bin/

# Initialize
kerrigan-storage init
```

## Usage

```bash
# Initialize storage (first time, or to update guide files)
kerrigan-storage init

# Create a task
kerrigan-storage task create --summary "Refactor auth module" --workdir .
# {"task_id":"abc-123"}

# Build the DAG
kerrigan-storage add --task abc-123 --summary "Analyze current code"
kerrigan-storage add --task abc-123 --summary "Design new architecture" --depends-on <node-1>
kerrigan-storage add --task abc-123 --summary "Implement changes" --depends-on <node-2>

# Run with an agent
kerrigan-cli run abc-123 --adapter claude
kerrigan-cli run abc-123 --adapter kimi
```

## Storage Layout

```text
~/.kerrigan/
├── registry.json
├── shared/                           # guide files (deployed by init)
│   ├── guide.md                      # main index
│   ├── guide-software-development.md
│   ├── guide-research.md
│   ├── guide-decision-ab-testing.md
│   └── guide-git-management.md
├── skills/
│   └── kerrigan-storage/SKILL.md
└── tasks/<task-id>/
    ├── dag.json
    ├── board.md
    └── sessions.json
```

## Documentation

- [Design](docs/design.md) — core design philosophy and architecture
- [Guide Design](docs/guide-design.md) — guide file system design
- [Milestone](docs/milestone.md) — roadmap
- [implement-a-chain](https://github.com/YoungY620/Kerrigan) — head-to-head experiment against Claude Code agent teams; Kerrigan performs on par
