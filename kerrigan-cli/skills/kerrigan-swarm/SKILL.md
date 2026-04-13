---
name: kerrigan-swarm
description: >
  Create and manage Kerrigan multi-agent swarm tasks. Use when the user wants
  to run a complex task with parallel sub-tasks, deploy a background DAG-based
  workflow, or check status of running swarm jobs.
compatibility: Requires kerrigan-cli and kerrigan-storage binaries in PATH
---

# Kerrigan Swarm

Launch and manage background multi-agent tasks powered by the Kerrigan DAG framework. These tasks are long-running — use an async/background task to execute `kerrigan-cli run` and avoid blocking your session.

## Quick Start

```bash
# Run a new task (creates task + builds DAG + executes automatically)
kerrigan-cli run --summary "Refactor auth module" --workdir /path/to/project

# Resume an existing task
kerrigan-cli resume <task-id>

# Check status of all tasks
kerrigan-cli status

# Check status of a specific task's DAG
kerrigan-cli status <task-id>

# Send a message to a running task
kerrigan-cli tell <task-id> --message "Please also check the database migration"
```

## Options

```bash
kerrigan-cli run --summary "..." --workdir .    # Required: task description and directory
                 --adapter claude               # Agent backend: claude (default) or kimi
                 --max-turns 10                 # Session rotation interval (default: 10)
                 -v                             # Info logging
                 -vv                            # Debug logging with agent output streaming

kerrigan-cli resume <task-id>                   # Same options as run (--adapter, --max-turns, -v)
```

## Examples

### Deploy a code review pipeline

```bash
kerrigan-cli run --summary "Review PR #42: add caching layer — check security, performance, and write review summary" --workdir ~/project --adapter claude
```

The agent will automatically decompose this into DAG nodes (analyze diff, security review, performance review, write summary), execute them with subagents, and produce the final output.

### Monitor and interact

```bash
# List all tasks
kerrigan-cli status

# Check DAG progress for a specific task
kerrigan-cli status abc-123

# Inject instructions into a running task
kerrigan-cli tell abc-123 --message "Please also check the database migration in db/migrate/"
```

### Resume after interruption

```bash
# The task ID was printed when you ran the task
kerrigan-cli resume abc-123
```

## How it works

1. `kerrigan-cli run` creates a task, initializes the DAG, and launches the orchestration loop
2. Each round, the orchestrator agent reads ready nodes, dispatches subagents, and records results
3. Subagents use `kerrigan-storage` CLI (the internal DAG tool) to read/write node state
4. Completed nodes unlock dependent nodes automatically
5. The loop continues until no ready nodes remain
6. Failed nodes don't block siblings — the agent spawns retry nodes instead

## Internal tools (used by agents, not users)

Agents interact with the DAG via `kerrigan-storage` during execution:

| Command | Description |
|---------|-------------|
| `kerrigan-storage ready --task <id>` | List ready nodes |
| `kerrigan-storage get --task <id> --node <id>` | Get node details |
| `kerrigan-storage add --task <id> --summary "..." [--depends-on a,b]` | Add a DAG node |
| `kerrigan-storage update --task <id> --input '<json>'` | Update node status |
