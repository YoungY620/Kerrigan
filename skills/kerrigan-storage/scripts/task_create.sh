#!/usr/bin/env bash
# Create a new task
# Usage: task_create.sh <summary> [workdir]
exec kerrigan-storage task create --summary "$1" --workdir "${2:-.}"
