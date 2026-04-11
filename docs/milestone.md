# Kerrigan Milestone: 自演化指导系统

## 目标

让 Kerrigan 能够通过 benchmark 执行 → 责任追溯 → 自动修订 → AB 验证的闭环，持续优化自己的指导文件，而非依赖人类手写 prompt。

---

## M1: 执行可追溯性

**目标**：让每次执行的完整轨迹可持久化、可回溯。

### 1.1 Session 持久化

当前 `run_loop` 收集了 `sessions_used` 但没有写入 storage。需要：

- `run_loop` 结束时将 session 列表写入 `sessions.json`
- 记录 round→session 的映射关系（哪一轮用了哪个 session）

### 1.2 节点级 session 记录

agent 通过 subagent 执行节点时使用的 session ID 需要回写到 `metadata.session_ids`。在 prompt 模板和 SKILL.md 中引导 agent 在 update 时带上 session ID。

### 1.3 结构化 result_summary / failure_reason

定义模板规范，写入指导文件：

**result_summary**：执行摘要 + 关键产出 + 对后续的建议 + 已知限制

**failure_reason**：根因分类（上游输入不足 / 描述不清 / 执行能力不足 / 外部依赖）+ 已尝试的方法 + 建议修复方向

---

## M2: 指导文件 v0

**目标**：在 `~/.kerrigan/` 公共区域放置初始指导文件，所有 agent 执行时读取。

### 2.1 内容

- 任务拆分指导：粒度判断、依赖建立原则、第一轮粗粒度策略
- 节点描述规范：summary/description 的写法、不同任务类型的模板
- 结果反馈规范：结构化 result_summary/failure_reason 模板
- 执行策略：何时停止、何时衍生新节点、如何判断前驱输入是否充足

### 2.2 部署

- `kerrigan-storage init` 时部署到 `~/.kerrigan/guide.md`
- prompt 模板中注入指导文件路径，引导 agent 阅读

### 2.3 Sprint 式分解

借鉴 SOEN-101 Scrum 模式和 ADaPT：指导文件中明确要求第一轮粗粒度分解（2-3 个高层节点），后续轮次根据执行结果决定是否细化。

---

## M3: 责任追溯

**目标**：沿 DAG 依赖链反向追溯失败根因。

### 3.1 追溯逻辑

```text
失败节点 N
  ├── 前驱的 result_summary 是否提供了足够信息？
  │     └── 不够 → 前驱的执行或描述有问题
  ├── N 自身的 description 是否清晰？
  │     └── 不清晰 → 分解阶段的问题
  └── N 的执行过程是否合理？
        └── 不合理 → agent 能力或指导文件的问题
```

### 3.2 追溯报告格式

结构化输出：失败节点信息 → 责任链分析（前驱输入/描述/执行三个维度）→ 根因归类 → 改进建议。

### 3.3 手动验证

先在几个 benchmark 任务上手动做责任追溯，验证机制是否能有效定位根因。

---

## M4: 自动化调查（用 Kerrigan 调查 Kerrigan）

**目标**：将责任追溯过程本身建模为 Kerrigan 任务。

### 4.1 调查 DAG 结构

```text
[收集失败案例]
  → [分类失败模式]（分解/描述/结果传递/执行）
    → [逐类深入分析]（并行）
      → [交叉验证]（多个分析节点独立判断，比对一致性）
        → [汇总调查报告]
```

交叉验证：不同分析节点独立判断同一批失败案例，结论一致则可信；不一致则继续调查。

### 4.2 输出修订意见

调查报告 → 结构化修订意见：当前说法 + 问题 + 建议修改 + 支撑证据（具体失败节点 ID）。

---

## M5: AB 测试验证

**目标**：修订意见不直接应用，必须通过统计验证。

- A 组：原版指导文件，B 组：修订版，跑同一批 benchmark
- 对比：任务成功率、平均节点数、执行轮数、失败模式分布
- 只有统计显著的改进才纳入正式版本

---

## M6: 多智能体 Benchmark 评测

**目标**：在专门为多智能体设计的 benchmark 上评测 Kerrigan，验证 DAG+无通信架构的有效性。

### 6.1 候选 Benchmark

协作类（验证 DAG 分解和并行执行）：
- MARBLE/MultiAgentBench — 研究、编码、数据库诊断等多场景，支持多种拓扑对比
- REALM-Bench — 动态规划调度，测试 disruption 下的 replan 能力

代码生成类（验证实际产出质量）：
- SWE-bench Verified — 标准代码 agent 评测
- HumanEval / MBPP — 基线对比

长周期类（验证 DAG 在持续任务上的优势）：
- SWE-EVO — 多文件软件演化
- FeatureBench — 跨 commit 的 feature 开发

### 6.2 对比基线

- 单 agent（相同模型，无 DAG）
- ChatDev / MetaGPT（固定角色 SOP）
- 单 agent + self-consistency

### 6.3 评测指标

- 任务成功率
- Token 总消耗 / 成本
- 执行时间（含并行加速比）
- DAG 相关指标：节点数、深度、并行度、失败节点比例

---

## M7: 难度感知与模型路由

**目标**：不同节点使用不同模型/策略。

- 简单节点（信息收集、格式转换）→ 轻量模型
- 复杂节点（架构设计、多文件重构）→ 强模型
- 需要 adapter 层支持指定模型的能力

---

## 依赖关系

```text
M1（可追溯性）→ M2（指导文件 v0）→ M3（责任追溯）→ M4（自动化调查）→ M5（AB 测试）
                                                                              ↓
                                                                    指导文件 v1, v2, ...
M6（Benchmark 评测）可在 M2 之后开始，用于验证指导文件效果
M7（难度感知）独立于上述链条，可并行推进
```

M1-M3 是手动 bootstrap 阶段，M4-M5 是自动化闭环。

---

## 参考文献

调研文档：

- [Benchmark 综述](multi-agent-eval-benchmarks.md)
- [相关工作](survey-related-work.md)
- [任务分解与 SOP 研究](survey-task-decomposition.md)

关键论文：

- EvoMAC (ICLR 2025) — DAG 拓扑的测试时演化，文本反向传播
- SOEN-101 (ICSE 2025) — Scrum > Waterfall > TDD，迭代 sprint
- ADaPT (NAACL 2024) — 失败时再分解 +28%
- MetaGPT (ICLR 2024) — 结构化文档 > 自由对话
- MASS (ICLR 2026) — prompt 优化 84% > agent 扩展 76%
- AgentPrune (ICLR 2025) — 28-73% agent 间通信 token 冗余
- Know the Ropes (2025) — 领域先验编码 3%→95%
- AFlow (ICLR 2025) — workflow 是可学习参数，小模型在 4.55% 成本下胜过大模型
- Pressure Fields (2026) — stigmergy 协调 30x 优于层级控制
