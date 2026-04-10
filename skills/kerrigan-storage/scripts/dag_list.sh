#!/usr/bin/env bash
# List all nodes in a task's DAG
# Usage: dag_list.sh <task-id>
exec kerrigan-storage list --task "$1"
