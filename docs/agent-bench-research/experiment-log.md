# Experiment Log

## Environment

- Host: `user@remote-host.example`
- OS: Linux 5.10.134-18 x86_64
- RAM: 61GB
- CPUs: 8
- Python: 3.12.12 (conda)
- Network: GitHub/PyPI accessible, HuggingFace blocked (use `hf-mirror.com`)

## Installations

| Package | Version | Install Command | Time |
|---------|---------|----------------|------|
| evalplus | 0.3.1 | `pip install evalplus --upgrade` | ~30s |
| lm-eval | 0.4.11 | `pip install lm-eval` | ~30s |
| dspy | 3.1.3 | `pip install dspy` | ~30s |
| datasets | 4.8.4 | `pip install datasets` | (already installed) |

**Total setup time**: <2 minutes for all packages.

## Benchmark Loading Tests

| Benchmark | Load Method | Problems | Load Time | Status |
|-----------|------------|----------|-----------|--------|
| HumanEval+ | `evalplus.data.get_human_eval_plus()` | 164 | 0.13s | OK |
| MBPP+ | `evalplus.data.get_mbpp_plus()` | 378 | ~0.1s | OK |
| GSM8K | `load_dataset('openai/gsm8k')` via hf-mirror | 1,319 test | ~2s | OK (needs HF_ENDPOINT env) |
| MATH | `load_dataset('hendrycks/competition_math')` | -- | -- | FAILED (not on mirror) |

### HuggingFace Mirror Workaround

```bash
export HF_ENDPOINT=https://hf-mirror.com
python3 -c "from datasets import load_dataset; ds = load_dataset('openai/gsm8k', 'main', split='test')"
```

## EvalPlus Full Evaluation

**Test**: Run all 164 HumanEval+ canonical solutions through evaluator.

```bash
python3 -m evalplus.evaluate --dataset humaneval --samples /tmp/evalplus_test.jsonl
```

**Results**:
- humaneval (base tests): pass@1 = 0.994
- humaneval+ (base + extra tests): pass@1 = 0.994
- Total evaluation time: 21.5s (0.13s per sample)
- 1 problem out of 164 has a known edge case in canonical solution

**Conclusion**: Evaluation is fast, deterministic, and self-contained.

## Per-Sample Execution Test

**Test**: Execute individual solutions and verify deterministic pass/fail.

| Input | Expected | Result |
|-------|----------|--------|
| HumanEval/0 correct solution | PASS | PASS |
| HumanEval/1 correct solution | PASS | PASS |
| HumanEval/2 correct solution | PASS | PASS |
| HumanEval/3 correct solution | PASS | PASS |
| HumanEval/4 correct solution | PASS | PASS |
| HumanEval/0 `return 42` | FAIL | FAIL |
| HumanEval/0 `pass` | FAIL | FAIL |

**Conclusion**: Per-sample evaluation is 100% deterministic and suitable as DSPy metric.

## DSPy DAG Proof-of-Concept

**Created**: `dspy_dag_poc.py` on remote server

**Components verified**:
1. GSM8K → DSPy Example conversion (50 train + 50 dev)
2. 3-module DAG pipeline (PlanSolution → ExecutePlan → ReviewAnswer)
3. `gsm8k_exact_match` metric (numeric comparison, handles $, commas)
4. HumanEval+ → DSPy Example conversion (20 examples)
5. Optimization config for MIPROv2 and GEPA

**Not yet tested** (requires LLM API key):
- Actual LLM inference through the pipeline
- MIPROv2 optimization loop
- Before/after accuracy comparison

## Data Format Summary

### HumanEval+ Sample

```json
{
  "task_id": "HumanEval/0",
  "prompt": "from typing import List\n\ndef has_close_elements(numbers: List[float], threshold: float) -> bool:\n    \"\"\" Check if in given list of numbers, are any two numbers closer...",
  "entry_point": "has_close_elements",
  "canonical_solution": "\n    sorted_numbers = sorted(numbers)\n    for i in range(len(sorted_numbers) - 1):\n        if sorted_numbers[i + 1] - sorted_numbers[i] < threshold:\n            return True\n    return False\n",
  "base_input": [7 test cases],
  "plus_input": [999 test cases]
}
```

### GSM8K Sample

```json
{
  "question": "Janet's ducks lay 16 eggs per day. She eats three for breakfast every morning and bakes muffins for her friends every day with four. She sells the remainder at the farmers' market daily for $2 per fresh duck egg. How much in dollars does she make every day at the farmers' market?",
  "answer": "Janet sells 16 - 3 - 4 = <<16-3-4=9>>9 duck eggs a day.\nShe makes 9 * 2 = $<<9*2=18>>18 every day at the farmer's market.\n#### 18"
}
```

Final numeric answer extracted via: `answer.split('####')[-1].strip()`

## Phase 5: Additional Validation (2026-04-15)

### BFCL Benchmark

- Installed `bfcl-eval 2026.3.23` via pip
- Import issue: `import bfcl` fails (module discovery issue, likely `bfcl_eval` or similar)
- Data format documented: question + function schemas → function call JSON, AST-based eval
- **Verdict**: Data format is good for tool-use agents, but package needs further investigation for programmatic use

### GSM-Symbolic (Apple, 2024) -- Contamination-Free Alternative

- **Available** on hf-mirror: `load_dataset('apple/GSM-Symbolic', split='test')`
- **5,000 problems** (vs GSM8K's 1,319)
- Columns: `id, instance, question, answer, original_id, original_question, original_answer, canary`
- Contains canary GUID for contamination detection
- Same `#### <number>` answer format as GSM8K -- drop-in replacement
- **GSM-Plus** also available: 10,552 problems with perturbation types

### DSPy Pipeline Mechanics Verification

Tested without LLM (structural validation):

| Test | Result |
|------|--------|
| DSPy Example format with GSM8K | OK |
| Single-module instantiation | OK |
| Multi-module DAG (2 predictors: planner + solver) | OK |
| exact_match metric | OK |
| MIPROv2 optimizer instantiation | Requires LM config (expected) |
| Evaluator instantiation | OK |
| Per-node signature inspection | OK (inputs/outputs correct) |

Key finding: DSPy `ChainOfThought` automatically adds a `reasoning` field, which maps naturally to the "show your work" step in DAG pipelines. Each predictor in a multi-module DAG has independently optimizable instruction text and few-shot demonstrations.

## Reproducibility

All scripts are saved in `~/bench-research/` on the remote server:
- `explore_benchmarks.py` -- Data format exploration
- `dspy_dag_poc.py` -- DSPy DAG proof-of-concept
- `evalplus_sanity_v2.py` -- EvalPlus evaluation verification
- `dspy_mock_test.py` -- DSPy pipeline mechanics verification
