# 任务分解与协调策略研究

研究什么样的任务分解策略、开发流程（SOP）和 prompt 设计能让 agent 更有效地工作——直接关系到 DAG 节点描述的质量和编排提示词的设计。

---

## 1. 开发流程模型对比

### 1.1 SOEN-101 / FlowGen（ICSE 2025）

**核心实验**：用 GPT-3.5 实现了三种软件开发流程，每种流程包含 5 个角色（需求工程师、架构师、开发者、测试者、Scrum Master），对比它们在代码生成上的效果。

**三种流程的实现方式**：

- **FlowGenWaterfall**：严格线性——需求→设计→编码→测试，每阶段完成后才进入下一阶段
- **FlowGenTDD**：测试先行——先写测试用例，再写代码使测试通过，最后重构
- **FlowGenScrum**：Sprint 迭代——所有 agent 参与讨论，Scrum Master 将讨论结果总结为共享上下文，作为下一轮迭代的输入

**结果（Pass@1）**：

| 流程 | HumanEval | HumanEval-ET | MBPP | MBPP-ET |
|------|-----------|-------------|------|---------|
| 单 agent 基线 | ~64-70 | ~50-55 | ~70-77 | ~43-50 |
| FlowGenWaterfall | 中等提升 | 中等 | 中等 | 中等 |
| FlowGenTDD | 中等提升 | 中等 | 中等 | 中等 |
| **FlowGenScrum** | **75.2** | **65.5** | **82.5** | **56.7** |

FlowGenScrum 平均比单 agent 提升 **15%**，标准差仅 **1.3%**（最稳定）。

**Scrum 为什么赢**：
1. 迭代 sprint 允许增量改进和早期纠错
2. Scrum Master 的**总结机制**创建了所有 agent 可以利用的共享上下文
3. 协作讨论而非线性交接
4. 但注意：**这些流程对应的是固定角色绑定**——每个 agent 始终扮演同一角色

**消融实验**：
- 移除测试环节：Pass@1 下降 **17.0%-56.1%**——测试是影响最大的活动
- 移除设计和代码审查：减少异常处理、增加代码异味

[arxiv](https://arxiv.org/abs/2403.15852)

### 1.2 类级别代码的流程对比（2025）

SOEN-101 的结论在更复杂的类级别代码（ClassEval）上**不成立**：

| 模型 | 相对单 agent 的 Pass@1 变化 |
|------|----------------------------|
| GPT-4o-mini | **-37.8%** |
| DeepSeek-Chat | **-39.8%** |
| Claude-3.5-Haiku | **+9.5%** |

**关键发现**：流程模型的有效性是**模型依赖的**。对于能力较强的模型，多 agent 协调的开销可能引入比它预防的更多的错误。函数级别有效的流程不一定适用于类级别。

[arxiv](https://arxiv.org/abs/2511.09794)

### 1.3 AgileCoder（FORGE 2025）

在 FlowGen 基础上引入**动态代码依赖图**：随着代码修改实时更新依赖关系图，让 agent 理解当前代码库的结构。

| 系统 | HumanEval | MBPP |
|------|-----------|------|
| MetaGPT | ~62.8 | ~74.7 |
| **AgileCoder** | **70.53** | **80.92** |

[arxiv](https://arxiv.org/abs/2406.11912)

### 1.4 EvoMAC（ICLR 2025）

**自演化多 agent 协作网络**：用"文本反向传播"在测试时动态调整 agent 和 agent 之间的连接关系。

- Website 任务 **+26.48%**，Game 任务 **+34.78%**，HumanEval **+6.10%**
- 这是目前效果最好的方法——固定拓扑不如让拓扑自己演化
- [arxiv](https://arxiv.org/abs/2410.16946)

---

## 2. 结构化产物 vs 自由对话

### MetaGPT（ICLR 2024 Oral）

核心发现：agent 之间传递**结构化文档**（PRD、系统设计、任务列表）远优于传递对话消息。

| 指标 | MetaGPT | ChatDev |
|------|---------|---------|
| 可执行性（1-4 分） | **3.75** | 2.1 |
| 每行代码的 token 开销 | **126.5** | 248.9 |
| 生成代码量 | **2.24x** | 基线 |

SOP 的价值在于：强制每个阶段产出结构化中间产物，作为下游 agent 的**约束**，减少幻觉传播。

[arxiv](https://arxiv.org/abs/2308.00352)

### ChatDev 经验共学（2024）

通过 Co-Learning 让 agent 从历史轨迹中积累"捷径经验"：

- 质量指标：0.4267 → **0.7304**（+71%）
- 自主性指标：0.3340 → **0.7100**（+113%）

机制：co-tracking（追踪历史）→ co-memorizing（记忆模式）→ co-reasoning（应用捷径）。

[arxiv](https://arxiv.org/abs/2312.17025)

---

## 3. 拓扑结构对代码生成的影响

### MASS（Google，ICLR 2026）

三阶段优化：局部 prompt→拓扑→全局 prompt。对**代码任务**的拓扑发现：

| 拓扑 | LiveCodeBench 效果 |
|------|-------------------|
| **Executor（执行者模式）** | **+6%** |
| Debate（辩论） | 微弱正面 |
| Reflect（反思） | **-15%（有害）** |
| Summarize（总结） | **有害** |

**最重要的发现**：prompt 优化（84%）比 agent 数量扩展（76%）更有效。

[arxiv](https://arxiv.org/abs/2502.02533)

### 运行时调试 vs 多 agent 讨论（2025）

19 个 LLM 在 HumanEval/HumanEval+ 上的系统对比：

| 策略 | 平均准确率 |
|------|-----------|
| 多 agent 协作 | 57.16% |
| **运行时调试** | **63.86%** |
| 两者结合 | 64.82% |

**结论**：对于代码正确性，**实际执行反馈 > agent 之间的讨论**。调试单独就比多 agent 好。

[arxiv](https://arxiv.org/abs/2505.02133)

---

## 4. 任务分解策略

### 4.1 失败时再分解（ADaPT，NAACL 2024）

不要预先把所有任务分解为原子步骤。让 agent 先尝试，失败了再递归细分。分解粒度应该匹配模型能力而非人类直觉。

- ALFWorld **+28.3%**，WebShop **+27%**，TextCraft **+33%**
- [arxiv](https://arxiv.org/abs/2311.05772)

### 4.2 约束引导分解（ACONIC，2025）

用形式化的复杂度度量（treewidth）决定在哪里切分任务。

- Claude **+8.8pp**，LLaMA **+15pp**
- [arxiv](https://arxiv.org/abs/2510.07772)

### 4.3 领域先验编码（Know the Ropes，2025）

当你知道问题的算法/流程时，直接把它编码为分解蓝图（而非让 LLM 自己发现）。

- Knapsack 问题：**3% → 95%**（3 个 GPT-4o-mini agent）
- 任务分配（6-15 jobs）：6 个 o3-mini agent 达到 **100%**
- [arxiv](https://arxiv.org/abs/2505.16979)

### 4.4 难度感知路由（DAAO，2025）

简单查询走简单 workflow，复杂查询走复杂 workflow。用 VAE 预测难度。

- 成本 **-36%**，准确率 **+2.83-11.21%**
- [arxiv](https://arxiv.org/abs/2509.11079)

---

## 5. 自动 Workflow 发现

| 系统 | 年份 | 方法 | 核心结果 | 链接 |
|------|------|------|----------|------|
| **ADAS** | 2024（ICLR 2025） | Meta Agent Search：在代码空间中迭代发明 agent 设计 | 自动发现的 agent 大幅优于人工设计，且**跨领域迁移** | [arxiv](https://arxiv.org/abs/2408.08435) |
| **AFlow** | 2024（ICLR 2025 Oral） | MCTS 搜索 workflow | **GPT-4o-mini 在优化 workflow 后胜过 GPT-4o，仅 4.55% 成本** | [arxiv](https://arxiv.org/abs/2410.10762) |
| **SOAN** | 2025 | 自组织 agent 网络，识别可复用结构单元 | +10% over AFlow（高复杂度任务），89.3% pass@1 | [arxiv](https://arxiv.org/abs/2508.13732) |
| **EvoAgent** | 2024 | 进化算法从一个 agent 繁殖出多样化团队 | QA、推理、规划任务上持续提升 | [arxiv](https://arxiv.org/abs/2406.14228) |
| **ScoreFlow** | 2025 | 梯度优化 + Score-DPO | +8.2% | [arxiv](https://arxiv.org/abs/2502.04306) |

**关键结论**：workflow 本身是一个可学习参数。人工设计的 workflow 远不是最优的。

---

## 6. Prompt 优化

| 系统 | 方法 | 核心发现 | 链接 |
|------|------|----------|------|
| **MASS** | 局部 prompt→拓扑→全局 prompt 三阶段 | **Prompt 优化 84% > agent 扩展 76%** | [arxiv](https://arxiv.org/abs/2502.02533) |
| **DSPy/GEPA** | 遗传-Pareto 进化优化，100-500 次评估 | 可端到端优化多 agent 管线 | [dspy.ai](https://dspy.ai/api/optimizers/GEPA/overview/) |
| **TextGrad** | LLM 反馈作为"文本梯度"反向传播 | 78% → 92%，几轮迭代 | [arxiv](https://arxiv.org/abs/2406.07496) |
| **Trace/OPTO** | 执行轨迹传播，prompt+代码联合优化 | 比仅优化 prompt 更有效 | [github](https://github.com/microsoft/Trace) |

---

## 7. 任务描述质量

### Prompt 详细程度与效果的关系（IJCNLP 2025）

引入 PartialOrderEval，从最简到最详细测试 prompt 详细程度的影响：

- 详细程度**持续提升** Pass@1，但有**边际递减**
- 三段式曲线：**上升**（更多信息有帮助）→**平台**（继续加细节不再帮助）→**下降**（信息过载）
- **三个关键驱动力**：明确的输入/输出规格、边界情况处理、分步骤分解

[arxiv](https://arxiv.org/abs/2508.03678)

### 系统 Prompt 研究（2026）

360 种配置（4 模型 × 5 system prompt × 3 策略 × 2 语言）：增加约束详细程度**不是单调提升**，效果取决于具体配置。

[arxiv](https://arxiv.org/abs/2602.15228)

---

## 8. 角色分配

| 发现 | 来源 |
|------|------|
| 强模型：固定**顺序**，让 agent 自选**角色** | Drop the Hierarchy（2026）：+44% |
| 弱模型：需要同时固定结构和角色 | 同上：存在能力阈值 |
| 动态选择参与的 agent（不是所有 agent 都参与每个任务） | DyLAN（COLM 2024）：+13%，[arxiv](https://arxiv.org/abs/2310.02170) |
| 不规则 DAG 拓扑优于规则拓扑 | MacNet（ICLR 2025） |

---

## 9. 对 Kerrigan 的启示

Kerrigan 的设计（无固定角色、DAG 驱动、每节点独立上下文）在以下研究中得到支持，但也需要注意：

### 直接支持

1. **Prompt 优化 > agent 数量**（MASS）——优化节点描述比增加并行更重要
2. **失败时再分解**（ADaPT）——不要在第一轮就把整个 DAG 分解到原子粒度
3. **结构化产物 > 自由对话**（MetaGPT）——result_summary 应该是结构化文档
4. **执行反馈 > 讨论**（运行时调试研究）——Kerrigan 的 validate→fix 循环比 agent 之间讨论更有效
5. **Workflow 是可学习参数**（AFlow）——DAG 结构本身可以被自动优化

### 需要注意

1. **流程有效性是模型依赖的**（ClassEval 研究）——对弱模型，更多结构有帮助；对强模型，结构可能成为负担
2. **Reflect/Summarize 拓扑对代码有害**（MASS）——Kerrigan 的 continue 模板不应该让 agent 做过多反思
3. **任务描述有最优详细程度**——节点 description 不是越详细越好，存在信息过载拐点
4. **领域先验极其有效**（KtR: 3%→95%）——当知道问题的解法结构时，应该直接编码到 DAG 拓扑中
