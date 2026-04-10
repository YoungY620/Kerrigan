# Kerrigan

Communication-less multi-agent orchestration framework. Complex tasks are decomposed into DAG (Directed Acyclic Graph) of work nodes — agents coordinate through shared state instead of messaging each other.

## Design

- **No inter-agent communication**: agents don't message each other. Dependencies are expressed as DAG edges, results flow through `result_summary` fields.
- **Context optimization**: each agent receives exactly the context needed for its specific subtask — no context pollution from irrelevant information.
- **Automatic parallelization**: independent nodes in the DAG execute concurrently. Completed nodes unlock their successors.

## Components

| Component | Description |
|-----------|-------------|
| [kerrigan-storage](kerrigan-storage/) | DAG engine & persistent storage — manages tasks, nodes, and their dependencies |
| [kerrigan-cli](kerrigan-cli/) | Orchestration runner — calls agents in a loop, validates DAG, rotates sessions |

## Installation

```bash
curl -fsSL https://github.com/YoungY620/harness/kerrigan/-/raw/main/install.sh | bash
```

Installs both `kerrigan-storage` and `kerrigan-cli` to `~/.local/bin/`.

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `INSTALL_DIR` | `~/.local/bin` | Binary install location |
| `CLONE_METHOD` | auto-detect | `ssh` or `https` |
| `KERRIGAN_REF` | `main` | Git ref to build from |

## Usage

```bash
# Initialize storage
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

```
~/.kerrigan/
├── registry.json
├── tasks/<task-id>/
│   ├── dag.json
│   ├── board.md          # inject messages into running tasks
│   └── sessions.json
└── skills/
    └── kerrigan-storage/SKILL.md
```
