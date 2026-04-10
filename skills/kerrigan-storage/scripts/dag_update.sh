#!/usr/bin/env bash
# Update nodes in a task's DAG
# Usage: dag_update.sh <task-id> <json>
exec kerrigan-storage update --task "$1" --input "$2"
