# Kerrigan

Communication-less multi-agent orchestration framework. Complex tasks are decomposed into DAG (Directed Acyclic Graph) of work nodes — agents coordinate through shared state instead of messaging each other.

## Design

- **No inter-agent communication**: agents don't message each other. Dependencies are expressed as DAG edges, results flow through `result_summary` fields.
- **Context optimization**: each agent receives exactly the context needed for its specific subtask — no context pollution from irrelevant information.
- **Automatic parallelization**: independent nodes in the DAG execute concurrently. Completed nodes unlock their successors.
- **Built-in guide system**: a set of guide files deployed to `~/.kerrigan/shared/` provides agents with task decomposition strategy, result reporting templates, and scenario-specific guidance (software development, research, AB testing, git management).

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
