#!/usr/bin/env bash
# Get full details of a node
# Usage: dag_get.sh <task-id> <node-id>
exec kerrigan-storage get --task "$1" --node "$2"
