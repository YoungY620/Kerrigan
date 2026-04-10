#!/usr/bin/env bash
# Add a node to a task's DAG
# Usage: dag_add.sh <task-id> <summary> [description] [depends-on]
TASK="$1"
SUMMARY="$2"
DESC="${3:-}"
DEPS="${4:-}"

CMD=(kerrigan-storage add --task "$TASK" --summary "$SUMMARY")
[ -n "$DESC" ] && CMD+=(--description "$DESC")
[ -n "$DEPS" ] && CMD+=(--depends-on "$DEPS")

exec "${CMD[@]}"
