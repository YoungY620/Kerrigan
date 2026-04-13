---
name: kerrigan-swarm
description: >
  Create and manage Kerrigan multi-agent swarm tasks. Use when the user wants
  to run a complex task with parallel sub-tasks, deploy a background DAG-based
  workflow, or check status of running swarm jobs.
compatibility: Requires kerrigan and kerrigan-storage binaries in PATH
---

# Kerrigan Swarm

Launch and manage background multi-agent tasks powered by the Kerrigan DAG framework. These tasks are long-running — use an async/background task to execute `kerrigan-cli run` and avoid blocking your session.

## Installation

```bash
# Install all components (kerrigan-storage + kerrigan-cli)
curl -fsSL https://github.com/YoungY620/harness/kerrigan/-/raw/main/install.sh | bash

# Or install individually
curl -fsSL https://github.com/YoungY620/harness/kerrigan-storage/-/raw/main/install.sh | bash
curl -fsSL https://github.com/YoungY620/harness/kerrigan-cli/-/raw/main/install.sh | bash
```

Verify: `kerrigan-storage --version && kerrigan-cli --version`

## Quick Start

```bash
# Initialize (first time only)
kerrigan-storage init

# Create a task
kerrigan-storage task create --summary "Refactor auth module" --workdir /path/to/project

# Set up the DAG nodes
kerrigan-storage add --task <task-id> --summary "Analyze current code" --description "..."
kerrigan-storage add --task <task-id> --summary "Design new architecture" --depends-on <node-1-id>
kerrigan-storage add --task <task-id> --summary "Implement changes" --depends-on <node-2-id>

# Run the task (background, using Claude Code as the agent)
kerrigan run <task-id> --adapter claude

# Check progress
kerrigan-storage list --task <task-id>
kerrigan-storage ready --task <task-id>
```

## Examples

### Deploy a code review pipeline

```bash
# Create the task
kerrigan-storage task create --summary "Review PR #42: add caching layer" --workdir ~/project
# Output: {"task_id":"abc-123"}

# Build the review DAG
kerrigan-storage add --task abc-123 --summary "Read the PR diff and understand changes" --description "Use git diff to read all changes in the PR. Summarize what was changed and why."
# Output: {"node_id":"n1"}

kerrigan-storage add --task abc-123 --summary "Check for security issues" --description "Review the caching implementation for potential security vulnerabilities: cache poisoning, unauthorized access, data leaks." --depends-on n1
# Output: {"node_id":"n2"}

kerrigan-storage add --task abc-123 --summary "Check for performance issues" --description "Analyze cache hit/miss patterns, TTL settings, memory usage, and eviction policy." --depends-on n1
# Output: {"node_id":"n3"}

kerrigan-storage add --task abc-123 --summary "Write review summary" --description "Combine findings from security and performance reviews into a final review comment." --depends-on n2,n3
# Output: {"node_id":"n4"}

# Launch
kerrigan run abc-123 --adapter claude
```

### Monitor a running task

```bash
# List all tasks
kerrigan-storage task list

# See DAG overview for a specific task
kerrigan-storage list --task abc-123

# Check what's currently ready to run
kerrigan-storage ready --task abc-123

# Read details of a specific node (including execution results)
kerrigan-storage get --task abc-123 --node n2
```

### Leave a message for a running task

Write to the task's board file to inject instructions into the next round:

```bash
echo "Please also check the database migration in db/migrate/" > ~/.kerrigan/tasks/abc-123/board.md
```

The orchestrator agent will see this message in its next round.

## Command Reference

| Command | Description |
|---------|-------------|
| `kerrigan-storage init` | Initialize ~/.kerrigan/ (first time) |
| `kerrigan-storage task create --summary "..." --workdir .` | Create a new task |
| `kerrigan-storage task list` | List all tasks |
| `kerrigan-storage add --task <id> --summary "..." [--depends-on a,b]` | Add a DAG node |
| `kerrigan-storage list --task <id>` | List all nodes |
| `kerrigan-storage ready --task <id>` | List ready nodes |
| `kerrigan-storage get --task <id> --node <id>` | Get node details |
| `kerrigan-storage update --task <id> --input '<json>'` | Update node status |
| `kerrigan run <task-id> --adapter claude\|kimi` | Run a task to completion |

## How it works

1. You define a task and its DAG (nodes + dependencies)
2. `kerrigan run` launches a main agent with DAG skills injected
3. Each round, the agent reads ready nodes, dispatches subagents, and records results
4. Completed nodes unlock dependent nodes automatically
5. The loop continues until no ready nodes remain
6. Failed nodes don't block siblings — spawn retry nodes instead
