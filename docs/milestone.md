# Kerrigan Milestone

## 当前优先级

**首要目标：让产品以可用状态发布。** 核心瓶颈是指导文件——agent 拿到任务后怎么拆分、怎么描述节点、怎么反馈结果，这些都依赖一个足够优秀的默认指导文件。

---

## M1: 指导文件 v0（最高优先级）

**目标**：写出一个足够好的默认指导文件，让 Kerrigan 开箱即用。

### 1.1 内容

**任务拆分指导**：
- 第一轮粗粒度分解（2-3 个高层节点），不要试图一次性把所有工作都规划好
- 后续轮次根据执行结果决定是否继续细化（借鉴 ADaPT: 失败时再分解 +28%，Scrum 迭代 sprint +15%）
- 粒度判断原则：一个节点应该是"一个 agent 在一次 session 中能完成的工作"
- 依赖建立原则：只有当 B 确实需要 A 的输出才能开始时，才建立 A→B 依赖；能并行就并行

**节点描述规范**：
- summary：一句话，动词开头，说清要做什么（如"分析 src/auth/ 下的认证流程"）
- description：包含足够的上下文让任意 agent 都能执行——引用哪些前驱结果、操作哪些文件/目录、产出什么、约束是什么
- 不要假设执行者知道项目背景，每个节点的 description 应该是自包含的

**结果反馈规范**（结构化 result_summary / failure_reason）：

result_summary 模板：
```text
## 执行摘要
[一句话结论]

## 关键产出
[具体改了什么、发现了什么、生成了什么]

## 对后续节点的建议
[下游 agent 需要知道的信息]

## 已知限制
[未覆盖的场景、遗留问题]
```

failure_reason 模板：
```text
## 失败原因
[根因：上游输入不足 / 任务描述不清 / 执行能力不足 / 外部依赖问题]

## 已尝试的方法
[做了什么、走到哪一步失败了]

## 建议的修复方向
[应该增加什么节点、修改什么依赖]
```

**执行策略**：
- 拿到节点后先读前驱的 result_summary，判断信息是否充足
- 如果前驱信息不足，通过 failure_reason 说明缺什么，而非硬做
- 完成后如果发现需要进一步的工作，通过 add 衍生新节点，而非在当前节点继续
- 每个节点只执行一次，不原地重试

### 1.2 部署

- `kerrigan-storage init` 时部署到 `~/.kerrigan/guide.md`
- prompt 模板（init.md、continue.md、resume.md）中注入指导文件路径

### 1.3 验证

用几个实际任务手动跑通，确认 agent 能正确遵循指导文件。

---

## M2: 执行可追溯性

**目标**：让每次执行的完整轨迹可持久化、可回溯。为后续的责任追溯和自演化提供数据基础。

### 2.1 Session 持久化

- `run_loop` 结束时将 session 列表写入 `sessions.json`
- 记录 round→session 的映射关系

### 2.2 节点级 session 记录

- prompt 模板和 SKILL.md 中引导 agent 在 update 时带上 session ID
- 回写到 `metadata.session_ids`

### 2.3 结构化 result_summary / failure_reason 的代码支持

M1 定义了格式规范（在指导文件中），M2 确保 storage 层和 validate 逻辑能理解和校验这些格式。

---

## M3: Benchmark 评测

**目标**：在标准 benchmark 上评测 Kerrigan，验证架构有效性，同时为指导文件优化提供数据。

### 3.1 候选 Benchmark

代码生成类：
- SWE-bench Verified — 标准代码 agent 评测
- HumanEval / MBPP — 基线对比

协作类：
- MARBLE/MultiAgentBench — 多场景，支持拓扑对比
- REALM-Bench — 动态规划调度

长周期类：
- SWE-EVO — 多文件软件演化
- FeatureBench — 跨 commit 的 feature 开发

### 3.2 对比基线

- 单 agent（相同模型，无 DAG）
- ChatDev / MetaGPT（固定角色 SOP）

### 3.3 评测指标

- 任务成功率
- Token 总消耗 / 成本
- 执行时间（含并行加速比）
- DAG 相关：节点数、深度、并行度、失败节点比例

---

## M4: 责任追溯

**目标**：沿 DAG 依赖链反向追溯失败根因，为指导文件改进提供依据。

### 4.1 追溯逻辑

```text
失败节点 N
  ├── 前驱的 result_summary 是否提供了足够信息？
  │     └── 不够 → 前驱的执行或描述有问题
  ├── N 自身的 description 是否清晰？
  │     └── 不清晰 → 分解阶段的问题
  └── N 的执行过程是否合理？
        └── 不合理 → agent 能力或指导文件的问题
```

### 4.2 手动验证

先在 M3 的 benchmark 结果上手动做几轮追溯，验证机制有效。

---

## M5: 自动化调查与 AB 验证

**目标**：用 Kerrigan 调查 Kerrigan，形成自演化闭环。

### 5.1 调查 DAG

```text
[收集失败案例]
  → [分类失败模式]（分解/描述/结果传递/执行）
    → [逐类深入分析]（并行）
      → [交叉验证]（多个分析节点独立判断，比对一致性）
        → [汇总调查报告 → 指导文件修订意见]
```

### 5.2 AB 测试

- A 组：原版指导文件，B 组：修订版，跑同一批 benchmark
- 对比：成功率、节点数、轮数、失败模式分布
- 只有统计显著的改进才纳入

### 5.3 迭代

```text
指导文件 v0 → benchmark → 追溯 → 调查 → 修订 → AB 验证 → 指导文件 v1 → ...
```

---

## M6: 难度感知与模型路由

**目标**：不同节点使用不同模型/策略。

- 简单节点 → 轻量模型
- 复杂节点 → 强模型
- 需要 adapter 层支持指定模型的能力

---

## 依赖关系

```text
M1（指导文件 v0）→ M2（可追溯性）→ M3（Benchmark 评测）→ M4（责任追溯）→ M5（自演化闭环）
                                        ↑                                       ↓
                                        └──────── 指导文件 v1, v2, ... ─────────┘
M6（难度感知）独立，可并行推进
```

**发布节点**：M1 完成后即可发布可用版本。M2-M5 是持续优化。

---

## 参考文献

调研文档：
- [Benchmark 综述](multi-agent-eval-benchmarks.md)
- [相关工作](survey-related-work.md)
- [任务分解与 SOP 研究](survey-task-decomposition.md)

关键论文：
- EvoMAC (ICLR 2025) — DAG 拓扑测试时演化，文本反向传播
- SOEN-101 (ICSE 2025) — Scrum > Waterfall > TDD，迭代 sprint +15%
- ADaPT (NAACL 2024) — 失败时再分解 +28%
- MetaGPT (ICLR 2024) — 结构化文档 > 自由对话
- MASS (ICLR 2026) — prompt 优化 84% > agent 扩展 76%
- AgentPrune (ICLR 2025) — 28-73% agent 间通信冗余
- Know the Ropes (2025) — 领域先验编码 3%→95%
- AFlow (ICLR 2025) — workflow 可学习，小模型 4.55% 成本胜大模型
- Pressure Fields (2026) — stigmergy 协调 30x 优于层级控制
