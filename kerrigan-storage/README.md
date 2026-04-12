# kerrigan-storage

DAG engine and persistent storage for the Kerrigan multi-agent framework. Manages tasks, nodes, and their dependencies through a file-based storage layer with concurrent access safety via file locks.

## Design

All state lives under `~/.kerrigan/`. Each task has its own DAG file (`dag.json`) tracking nodes and edges. Nodes flow through statuses: `pending` -> `ready` -> `running` -> `succeeded` / `failed`. When a node succeeds, its successors with all dependencies met automatically become `ready`.

Nodes communicate results through `result_summary` — downstream nodes read predecessor summaries to get context, eliminating the need for inter-agent messaging.

All CLI commands output JSON for machine consumption.

## Installation

```bash
curl -fsSL https://github.com/YoungY620/harness/kerrigan-storage/-/raw/main/install.sh | bash
```

## Commands

```bash
kerrigan-storage init                                    # Initialize ~/.kerrigan/
kerrigan-storage task create --summary "..." --workdir . # Create a task
kerrigan-storage task list                               # List all tasks
kerrigan-storage add --task <id> --summary "..." [--description "..."] [--depends-on a,b]
kerrigan-storage list --task <id>                        # All nodes
kerrigan-storage ready --task <id>                       # Ready nodes only
kerrigan-storage get --task <id> --node <id>             # Full node details
kerrigan-storage update --task <id> --input '<json>'     # Batch update statuses
```
