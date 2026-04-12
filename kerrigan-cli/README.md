# kerrigan-cli

Orchestration runner for the Kerrigan multi-agent framework. Reads ready nodes from a DAG, dispatches them to agents, validates results, and loops until the task is complete.

## Design

The runner operates in a loop:

1. Query `kerrigan-storage ready` for executable nodes
2. Build a prompt from templates (init / continue / resume / fix), injecting guide file path
3. Call an agent via adapter (Claude Code or Kimi CLI)
4. Validate DAG consistency — if issues found, send fix prompt
5. Repeat until no ready nodes remain

Sessions rotate after `--max-turns` rounds to keep agent context fresh. The `board.md` file allows injecting human messages into running tasks. Guide files at `~/.kerrigan/shared/` are referenced in every prompt.

### Adapters

| Adapter | CLI | Session | Skills |
|---------|-----|---------|--------|
| Claude (default) | `claude --print --dangerously-skip-permissions` | UUID | `--add-dir` |
| Kimi | `kimi --print --yolo` | Any string | `--skills-dir` |

## Usage

```bash
kerrigan-cli run <task-id>                          # default: --adapter claude
kerrigan-cli run <task-id> --adapter kimi
kerrigan-cli run <task-id> --max-turns 20
kerrigan-cli run <task-id> --skills-dir /path/to/skills
```
