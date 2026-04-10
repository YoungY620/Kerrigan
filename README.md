# kerrigan-cli

Orchestration runner for the Kerrigan multi-agent framework. Reads ready nodes from a DAG, dispatches them to agents, validates results, and loops until the task is complete.

## Design

The runner operates in a loop:

1. Query `kerrigan-storage ready` for executable nodes
2. Build a prompt from templates (init / continue / resume / fix)
3. Call an agent via adapter (Claude Code or Kimi CLI)
4. Validate DAG consistency — if issues found, send fix prompt
5. Repeat until no ready nodes remain

Sessions rotate after `--max-turns` rounds to keep agent context fresh. The `board.md` file allows injecting human messages into running tasks.

### Adapters

| Adapter | CLI | Session | Skills |
|---------|-----|---------|--------|
| Claude | `claude --print --dangerously-skip-permissions` | UUID | `--add-dir` |
| Kimi | `kimi --print --yolo` | Any string | `--skills-dir` |

## Installation

```bash
# Installs both kerrigan-cli and kerrigan-storage (build + runtime dependency)
curl -fsSL https://github.com/YoungY620/harness/kerrigan-cli/-/raw/main/install.sh | bash
```

## Usage

```bash
kerrigan-cli run <task-id> --adapter claude
kerrigan-cli run <task-id> --adapter kimi
kerrigan-cli run <task-id> --adapter claude --max-turns 20
kerrigan-cli run <task-id> --adapter claude --skills-dir /path/to/skills
```
