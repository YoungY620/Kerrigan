# Bench Environment Setup & Usage Guide

## Location

Remote server: `ssh user@remote-host.example`
Directory: `~/bench-env/`

## Quick Start

```bash
ssh user@remote-host.example
cd ~/bench-env
source config.env  # auto-sourced via .bashrc

# See available benchmarks
python3 run.py list

# Get info about a benchmark
python3 run.py info gsm-symbolic

# Sanity check (gold answers, should be 100%)
python3 run.py eval humaneval --gold --limit 10

# Run optimization (needs API key)
export ANTHROPIC_API_KEY=sk-ant-...
python3 optimize.py run --bench gsm8k --pipeline plan-solve --limit 50
```

## Available Benchmarks

| Name | Problems | Type | Eval Speed | Reliability |
|------|----------|------|-----------|-------------|
| `humaneval` | 164 | Code (pass/fail) | 0.1s/sample | B |
| `mbpp` | 378 | Code (pass/fail) | 0.1s/sample | B |
| `gsm8k` | 1,319 | Math (exact match) | <1ms/sample | C (contaminated) |
| `gsm-symbolic` | 5,000 | Math (exact match) | <1ms/sample | A- (contamination-free) |
| `gsm-plus` | 10,552 | Math (exact match) | <1ms/sample | B+ |
| `math` | 5,000 | Math (symbolic match) | <1ms/sample | B |

All datasets pre-downloaded and cached.

## Runner CLI (`run.py`)

```bash
# List benchmarks
python3 run.py list

# Detailed info with samples
python3 run.py info <bench>

# Evaluate with gold/canonical answers (sanity check)
python3 run.py eval <bench> --gold [--limit N]

# Evaluate model predictions from JSONL file
python3 run.py eval <bench> --input predictions.jsonl [--limit N]

# Export data
python3 run.py export <bench> --format jsonl --output data.jsonl [--limit N]
python3 run.py export <bench> --format dspy [--limit N]
```

### Prediction File Format (JSONL)

```json
{"task_id": "gsm8k_0", "prediction": "18"}
{"task_id": "gsm8k_1", "prediction": "3"}
```

### Results Output Format

```json
{
  "benchmark": "gsm8k",
  "total": 100,
  "passed": 85,
  "accuracy": 0.85,
  "avg_time_per_sample": 0.001,
  "results": [
    {"task_id": "gsm8k_0", "passed": true, "predicted": "18", "expected": "18", "time_sec": 0.0}
  ]
}
```

## DSPy Optimization (`optimize.py`)

### Prerequisites

```bash
# Anthropic API (recommended, reachable from this server)
export ANTHROPIC_API_KEY=sk-ant-...

# Or OpenAI (currently blocked from this server, need proxy)
export OPENAI_API_KEY=sk-...
```

### Available Pipelines

```bash
python3 optimize.py pipelines
```

| Pipeline | Nodes | Type | Description |
|----------|-------|------|-------------|
| `single` | 1 | math | Baseline single-step CoT solver |
| `plan-solve` | 2 | math | Planner → Executor |
| `plan-solve-review` | 3 | math | Planner → Executor → Reviewer |
| `single-coder` | 1 | code | Baseline single-step code generation |
| `plan-code` | 2 | code | Plan implementation → Code generation |

### Running Optimization

```bash
# Light optimization (~25 LLM calls)
python3 optimize.py run --bench gsm8k --pipeline plan-solve --limit 50 --auto light

# Medium optimization (~75 LLM calls)
python3 optimize.py run --bench gsm-symbolic --pipeline plan-solve-review --limit 100 --auto medium

# Save optimized pipeline
python3 optimize.py run --bench gsm8k --pipeline plan-solve --limit 50 --output results/gsm8k_optimized

# Evaluate optimized pipeline
python3 optimize.py eval --bench gsm8k --pipeline plan-solve --load results/gsm8k_optimized --limit 100
```

### Recommended Optimization Runs

```bash
# 1. Math: Compare 1-node vs 2-node vs 3-node DAGs
python3 optimize.py run --bench gsm-symbolic --pipeline single --limit 50
python3 optimize.py run --bench gsm-symbolic --pipeline plan-solve --limit 50
python3 optimize.py run --bench gsm-symbolic --pipeline plan-solve-review --limit 50

# 2. Code: Compare 1-node vs 2-node
python3 optimize.py run --bench humaneval --pipeline single-coder --limit 30
python3 optimize.py run --bench humaneval --pipeline plan-code --limit 30

# 3. Cross-benchmark validation
python3 optimize.py run --bench gsm8k --pipeline plan-solve --limit 50 --output results/gsm8k_opt
python3 optimize.py eval --bench gsm-symbolic --pipeline plan-solve --load results/gsm8k_opt --limit 100
# ^ Does optimization on GSM8K transfer to GSM-Symbolic?
```

## Using Benchmarks in Custom Code

```python
import os, sys
os.environ['HF_ENDPOINT'] = 'https://hf-mirror.com'
sys.path.insert(0, os.path.expanduser('~/bench-env'))

from benchmarks import get_benchmark

# Load benchmark
bench = get_benchmark('gsm-symbolic', limit=100)

# Get raw data
data = bench.load_data()
print(f"{len(data)} problems loaded")
print(f"Sample: {data[0]['input'][:100]}...")

# Convert to DSPy examples
import dspy
examples = bench.to_dspy_examples()
trainset = examples[:60]
devset = examples[60:]

# Evaluate custom predictions
predictions = {item['task_id']: 'my_answer' for item in data}
result = bench.evaluate_batch(predictions)
print(result.summary())
result.save('results/my_eval.json')
```

## Network Constraints

| Service | Status | Notes |
|---------|--------|-------|
| GitHub | OK | |
| PyPI | OK | |
| HuggingFace | Blocked | Use `HF_ENDPOINT=https://hf-mirror.com` (auto-configured) |
| OpenAI API | Blocked | Need proxy if using OpenAI models |
| Anthropic API | Reachable | Returns 403 without key (auth works) |

## Directory Structure

```
~/bench-env/
├── config.env              # Environment config (auto-sourced)
├── run.py                  # Benchmark runner CLI
├── optimize.py             # DSPy optimization CLI
├── benchmarks/
│   ├── __init__.py         # Registry
│   ├── base.py             # BenchmarkAdapter base class
│   ├── evalplus_bench.py   # HumanEval+ / MBPP+
│   ├── gsm_bench.py        # GSM8K / GSM-Symbolic / GSM-Plus
│   └── math_bench.py       # MATH
└── results/                # Evaluation and optimization results
```

## Adding a New Benchmark

1. Create `benchmarks/my_bench.py`:

```python
from .base import BenchmarkAdapter

class MyBench(BenchmarkAdapter):
    name = "my-bench"

    def load_data(self):
        # Return list of dicts with: task_id, input, expected, dspy_fields, dspy_input_keys
        return [...]

    def evaluate_single(self, task_id, prediction, expected):
        # Return True if prediction matches expected
        return prediction.strip() == expected.strip()
```

2. Register in `benchmarks/__init__.py`:

```python
from .my_bench import MyBench
REGISTRY["my-bench"] = MyBench
```

3. Add a DSPy pipeline in `optimize.py` if needed.
