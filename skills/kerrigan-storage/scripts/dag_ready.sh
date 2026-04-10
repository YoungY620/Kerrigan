#!/usr/bin/env bash
# List all ready nodes in a task's DAG
# Usage: dag_ready.sh <task-id>
exec kerrigan-storage ready --task "$1"
