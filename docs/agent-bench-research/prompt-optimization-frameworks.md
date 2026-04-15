# Prompt Optimization Frameworks for Multi-Agent DAG Systems

## Key Question

**Can any framework optimize prompts for individual nodes in a DAG pipeline, where the optimization signal comes from end-to-end benchmark performance?**

**Answer: Yes. DSPy does this natively. TextGrad and SAMMO also support DAG structures.**

---

## Framework Comparison

| Framework | Stars | Version | Last Active | Multi-step DAG? | Integration Ease |
|-----------|-------|---------|-------------|-----------------|------------------|
| **DSPy** | 33.7K | 3.1.3 | Apr 2026 | **Yes** (native) | High |
| **TextGrad** | 3.5K | -- | Jul 2025 | **Yes** (computation graph) | Medium |
| **SAMMO** | 751 | -- | Jun 2025 | **Yes** (symbolic DAG) | Medium |
| OPRO | 731 | -- | Dec 2024 | No | Low |
| APE | 1.3K | -- | Apr 2024 | No | Low |
| EvoPrompt | 228 | -- | Sep 2025 | No | Low |
| PromptBreeder | -- | -- | N/A | No | Low (no official code) |

## DSPy (Recommended)

**Repository**: https://github.com/stanfordnlp/dspy

### Why DSPy

1. Module composition = DAG nodes. Each `dspy.Predict(Signature)` is a node.
2. MIPROv2 optimizer jointly optimizes ALL module prompts using end-to-end metric
3. GEPA optimizer: specialized for multi-module pipeline optimization
4. Only requires: training examples + metric function. No per-node ground truth needed.
5. Actively maintained, large community, extensive documentation

### Data Format

```python
# DSPy expects Example objects with input/output fields
trainset = [
    dspy.Example(question="Janet's ducks...", answer="18").with_inputs("question"),
    ...
]

# Metric function: takes (example, prediction) -> bool/float
def metric(example, prediction, trace=None):
    return prediction.answer.strip() == example.answer.strip()
```

### Multi-Module Pipeline (DAG mapping)

```python
class Signature1(dspy.Signature):
    """Plan a solution approach."""
    question: str = dspy.InputField()
    plan: str = dspy.OutputField()

class Signature2(dspy.Signature):
    """Execute the plan."""
    question: str = dspy.InputField()
    plan: str = dspy.InputField()
    answer: str = dspy.OutputField()

class DAGPipeline(dspy.Module):
    def __init__(self):
        self.node1 = dspy.Predict(Signature1)  # Kerrigan DAG node 1
        self.node2 = dspy.Predict(Signature2)  # Kerrigan DAG node 2
    
    def forward(self, question):
        plan = self.node1(question=question)
        result = self.node2(question=question, plan=plan.plan)
        return dspy.Prediction(answer=result.answer)
```

### Optimization

```python
# MIPROv2: instruction + few-shot optimization
optimizer = dspy.MIPROv2(metric=metric, auto='medium')
optimized = optimizer.compile(DAGPipeline(), trainset=trainset)

# Each module now has optimized instruction + demos
for name, module in optimized.named_predictors():
    print(f"Node: {name}, Instruction: {module.signature.instructions}")
```

### Benchmarks Tested With DSPy

GSM8K, HotPotQA, MATH, BBH, SQuAD, many RAG tasks

---

## TextGrad

**Repository**: https://github.com/zou-group/textgrad

### How It Works

Models the pipeline as a computation graph. "Text gradients" (natural language feedback) propagate backward from loss to each node. Each node's prompt is updated based on the textual gradient.

### Strengths
- Per-node feedback is explicit (you can see what the optimizer thinks each node should change)
- Published in Nature for molecule optimization

### Weaknesses
- Gradient signal degrades exponentially with graph depth
- Less mature than DSPy for prompt optimization specifically

---

## SAMMO (Microsoft)

**Repository**: https://github.com/microsoft/sammo

### How It Works

Represents prompts as **symbolic programs** (DAGs of operations). Mutations are structure-aware: reorder sections, compress, paraphrase, add/remove components. Searches over structural mutations using validation set performance.

### Strengths
- Explicit DAG structure in prompt representation
- Structure-aware mutations (not just text changes)
- Good for prompt compression and restructuring

### Weaknesses
- Smaller community than DSPy
- Less documentation

---

## Single-Prompt Optimizers (Not Recommended for DAGs)

| Framework | Method | Limitation |
|-----------|--------|-----------|
| **OPRO** | Meta-prompt generates+evaluates prompts | Single prompt loop, no pipeline |
| **APE** | Generate instructions from demos | Single instruction search |
| **EvoPrompt** | GA/DE on prompt populations | One prompt at a time |
| **PromptBreeder** | Self-referential mutation-prompt evolution | No official code |

These would require building a custom outer loop to handle multi-node optimization -- feasible but significant engineering vs. using DSPy natively.

---

## Related Research

- [MASS: Multi-Agent System Design (2025)](https://arxiv.org/abs/2502.02533) -- Jointly optimizes prompts AND DAG topology
- [HiveMind: Contribution-Guided Prompt Optimization (2024)](https://arxiv.org/pdf/2512.06432) -- Shapley values for per-agent credit
- [Self-Optimizing Multi-Agent Systems (2025)](https://arxiv.org/abs/2604.02988) -- Self-improvement loops
- [Trace is the Next AutoDiff (NeurIPS 2024)](https://proceedings.neurips.cc/paper_files/paper/2024/file/83ba7056bce2c3c3c27e17397cf3e1f0-Paper-Conference.pdf) -- Theoretical foundation for text gradient
