# Kerrigan: 一个无通信的 Agent 间协作协议

## 核心主张:任务结果即通信

**本框架的设计核心只有一句话:agent 之间不通过消息通信,它们通过「把自己那份任务的结果写下来」来协同。** 没有 `sendMessage`、没有消息队列、没有订阅、没有广播。下游 agent 唯一的输入来自前驱节点的 `result_summary`。

这个主张一旦确立,传统 multi-agent 框架里 80% 的复杂度就被消除了 —— 没有通信协议、没有中断语义、没有上下文写放大、没有"A 给 B 发了消息但 B 错过了"之类的问题。

**DAG 是这个主张的实现手段,不是设计目标本身**:只要存在任务依赖,就可以表达为一张有向无环图;只要图存在,就可以按拓扑序执行;执行时每个节点读前驱 `result_summary`、写自己的 `result_summary`,这就是全部协议。并行性、上下文最小化、级联 Ready 都是这个结构的自然产物,而不是额外设计的 feature。

后续各节将展开这一主张为什么成立、DAG 怎么落地、以及存储/执行/事件流层面的具体形状。

---

## 为什么需要 Multi-Agent

**关键假设:agent 运行的最佳状态是刚好看到所有它需要的上下文。**

模型的能力体现在它能解决的任务粒度上。当 agent 刚好拿到足够完成一个任务的上下文时效果最好;面对更大规模的问题,单个 agent 的上下文不得不被分片——受限于自身能力,它只能关注问题的一部分,这可能是大规模问题下 agent 失效的根本原因。多个 agent 协作可以视为对单个 agent 上下文窗口的扩展,表现形式包括交叉验证、分治聚合等。即使仅考虑并行优化,多智能体也是有意义的。

后面的所有设计都基于这个假设(但这个假设也可能被推翻,需要验证)。

## 事决定人

在确立这个方案之前，我们商讨和尝试过多种其他方案，最终都被一一否决。

最常见的思路是照搬人类的即时通信工具和组织架构，原因有二：

- **人机协同**：人类需要一个熟悉的界面来参与。
- **经验迁移**：很多人认为人工智能就是"虚拟人"，于是给 agent 接上 agent 版的 Facebook、WeChat、Twitter，或者套用人类的组织架构来解决问题。但这个类比的前提本身就有问题。

核心问题是：到底是通信工具定义了我们的沟通方式、组织架构定义了每个人的分工，还是我们共同面临的**事**定义了通信工具和组织架构？

答案是事决定人。所有人与人之间的沟通和依赖，归根结底都由事的依赖决定。

人的生理躯壳和上下文是绑定的，上下文切换需要巨大的心智和时间代价。因此人类组织面临一个两难：架构要稳定（降低切换成本），又要能应对灵活多变的事的变化。人工智能不是人——它的"生命"完全由输入的字符串决定。agent 的分工完全可以根据事的演化动态变化，不需要绑定到一个稳定的上下文和组织结构中。

因此，我们真正要解决的不是 agent 之间应该怎样沟通，而是：**一件复杂的任务如何进行结构化的拆分和维护，以及如何在一个持久化、可读写的存储中承载这些结构。**

## 为什么"无通信"是对的

很多 multi-agent 框架的论文都绕不开一个核心问题:agent 之间的通信复杂度。点对点通信、广播会导致上下文写放大,主动沟通还会打断另一个 agent 的工作。对人类来说这无法避免 ——《人月神话》的核心论点即此。但**对 agent 完全没有必要**,因为:

- **agent 间需要通信的唯一原因是任务依赖**。任何依赖都可以拆为"依赖前"和"依赖后"两个节点,写进 DAG 即可。
- **DAG 可以做拓扑排序**:所有没有未完成前驱的节点可以并行执行。每个 agent 拿到的是刚好足够的上下文,不多不少。
- **执行中发现需要新的协作?** agent 停止当前任务、`dag add` 新节点、建立依赖,下一轮按拓扑序继续。DAG 是可演化的,不是静态的。

换句话说,**`sendMessage` 能做的每一件事,都可以用"拆节点 + 加依赖"表达出来,而且更清楚、更可追溯、更可重放**。既然新模型下不需要 `sendMessage` 也能做,那就砍掉它。

## 架构

### 三层结构

```
主 Agent（用户的 agent session）
│  编排决策：分解任务、重构 DAG、启动/中断 worker、响应事件
│
├── kerrigan CLI ─── DAG 管理 + worker 生命周期 + 日志 + 事件流
│
└── Worker Agents（claude/kimi 后台进程）
     各自独立运行，通过 kerrigan skill 回写状态
```

**主 Agent** 是使用 kerrigan 的 agent。它负责所有编排决策——任务分解、DAG 重构、启动和中断 worker。主 agent 是事件驱动的：每次被唤醒（用户输入、worker 完成/失败事件），做一轮决策后返回，保持对用户的响应能力。

**kerrigan CLI** 是无状态的命令行工具，管理三件事：
1. DAG 状态（节点、依赖、状态流转、级联 Ready）
2. Worker 生命周期（启动、消息注入、终止、崩溃兜底）
3. 事件流（节点状态变化 → stdout JSON，供 Monitor 消费）

**Worker Agents** 是后台运行的 agent 进程（claude、kimi 等），各自独立执行一个 DAG 节点。每个 worker 拥有 kerrigan skill，能够读取前驱结果、回写自身状态、衍生新节点。Worker 不知道彼此的存在——它们通过 DAG 中的 `result_summary` 间接传递信息。

### 关键设计原则

- **结果即通信(第一性原则)**:`result_summary` 是**唯一**的 agent 间协调机制。下游 agent 通过 `kerrigan dag get` 读前驱的结构化摘要,没有消息队列、没有共享文件、没有事件订阅。所有其他设计都是这个原则的推论。
- **编排在主 agent,不在框架**:kerrigan 不决定先做什么后做什么,只提供 DAG 原语和进程管理。所有"该怎么做"的决策由主 agent 的 prompt/skill 驱动 —— 框架不侵入内容决策。
- **一次执行**:节点只执行一次,永不原地重试。失败后通过 `dag add` 衍生新的修复/重试节点并建立依赖,让失败也变成结构化产物。
- **崩溃安全**:每个 worker 都有 exec wrapper 兜底 —— 不管 agent 怎么死,状态注册表都会被正确更新。

## Storage（DAG 引擎 + 持久化）

### 数据模型

一个任务（task）包含一个 DAG，DAG 由节点（node）和边（edge）组成。

**节点字段**：
- **id**：唯一标识
- **summary**：一行简述
- **description**：详细描述（markdown），自包含到足够让一个 agent 独立执行
- **predecessors / successors**：依赖关系（双向维护）
- **status**：Pending → Ready → Running → Succeeded / Failed
- **metadata**：
  - `result_summary`：成功时的结果摘要，后续节点依赖此信息
  - `failure_reason`：失败时的原因和建议修复方向
  - `session_ids`：执行过此节点的 agent session 列表

**状态流转**：
- Pending：有未完成的前驱
- Ready：所有前驱已 Succeeded（由引擎自动级联）
- Running：正在被 worker 执行
- Succeeded / Failed：终态

### 存储后端

两级 SQLite（均 WAL 模式）：

```text
~/.kerrigan/
├── registry.db              # 全局注册表
├── skills/kerrigan/         # skill 文件（init 时部署）
└── tasks/
    └── <task-id>/
        ├── task.db          # 任务级 DB
        └── logs/
            └── <node-id>.jsonl
```

**registry.db**（全局，极轻量）：

- `tasks` — 任务索引（id, summary, workdir, created_at）

**task.db**（每任务独立，worker 并发写不跨任务竞争）：

- `nodes` — 节点（id, summary, description, status, result_summary, failure_reason）
- `node_edges` — DAG 边（predecessor_id, successor_id）
- `processes` — 进程注册表（node_id, pid, session_id, adapter, log_path, status）
- `node_messages` — 消息队列（node_id, content, consumed）
- `node_sessions` — 历史 session 记录

任务隔离：删任务 = 删目录。全局 DB 只做索引。

### 引擎接口

所有接口基于 task_id 和 node_id，不暴露文件路径：

- `task_create(summary, workdir) → task_id`
- `task_list() → Vec<TaskEntry>`
- `add_node(task_id, node, depends_on) → node_id`：自动判定初始状态（Ready / Pending）
- `list_all(task_id) → Vec<NodeSummary>`
- `list_ready(task_id) → Vec<NodeSummary>`
- `get_node(task_id, node_id) → Node`：包含完整 description 和前驱 result_summary
- `update_nodes(task_id, Vec<NodeUpdate>)`：批量更新，自动级联 Ready

**级联逻辑**：当某节点变为 Succeeded 时，引擎检查其所有后继——若后继的全部前驱已 Succeeded，则将后继从 Pending 升为 Ready。循环执行直到无新变化。

## Worker 生命周期

### exec

`kerrigan exec` 是 worker 的统一入口，支持两种模式：

**`--detach`（后台）**：主 agent 常用。验证节点 Ready → 生成 session_id → fork 自身为后台进程 → 记录进程注册表 → 标记节点 Running → 立即返回 JSON。

**前台**：实际执行 agent。流程：

1. 打开 log 文件（追加），写入 exec_start 事件
2. 根据 adapter 名称直接 match 构建 agent 命令：
   - `claude --print --verbose --dangerously-skip-permissions --output-format stream-json --session-id <uuid> -p "<prompt>"`
   - `kimi --print --yolo --output-format stream-json --session <id> --prompt "<prompt>"`
3. 启动 agent 子进程，逐行读取 stdout 写入 log 文件
4. `wait()` 等待 agent 退出
5. 退出判定：
   - 状态已 terminal → agent 自行收尾，正常退出
   - exit code = 0 + 仍 Running → turn 正常完成，检查消息队列：有消息则 `--resume` 继续，无消息则退出
   - exit code ≠ 0 + 仍 Running → 崩溃，标记节点 Failed
6. SIGTERM 处理：kill agent → 检查队列 → 有消息则 resume，无则退出

**日志**：同一节点的所有 turn 追加到同一个 `.jsonl` 文件。包含 kerrigan wrapper 事件和 agent 的完整 stream-json 输出。

### send

`kerrigan send` 向运行中（或已暂停）的 worker 注入消息：

1. 写消息到 `node_messages` 表
2. 如果 exec 没在运行 → fork 新 exec（消费队列后 `--resume`）
3. 如果 exec 在运行 + `--interrupt` → 发 SIGTERM（exec 中断后消费队列）
4. 如果 exec 在运行 + 无 interrupt → 什么都不做（当前 turn 结束后 exec 自动消费）

### stop

`kerrigan stop` 终止 worker：查进程注册表 → kill PID → 更新状态。

### 崩溃保障

两级保障：
1. **exec wrapper 兜底**：agent 进程崩溃后，exec wrapper 检测到非零 exit code + 状态仍 Running → 标记 Failed。
2. **watch PID 检查**：极端情况下 exec wrapper 本身被 kill → watch 检测到 PID 死亡 + 节点仍 Running → 60s 超时后标记 Failed。

## 事件流

`kerrigan watch <task-id>` 是长运行进程，每 N 秒轮询 DB，输出状态变化事件到 stdout（一行一个 JSON），设计用于对接 Claude Code 的 Monitor 机制：

```jsonl
{"ts":"...","node":"abc","event":"succeeded","summary":"Analyze auth","result":"Found 3 files"}
{"ts":"...","node":"def","event":"ready","summary":"Implement changes"}
{"ts":"...","event":"process_died","node":"ghi","pid":1234}
{"ts":"...","event":"task_complete","succeeded":5,"failed":0}
```

主 agent 通过 Monitor 消费这些事件，驱动下一轮编排决策。

## Skills

kerrigan 的 skill 目录（`~/.kerrigan/skills/kerrigan/`）包含：

1. **SKILL.md**：CLI 命令参考和使用指导，供 worker agent 学习如何操作 DAG
2. **guide.md** + 场景指导：任务执行原则、分解策略、结果格式模板。基于学术研究（MASS, ADaPT, MetaGPT 等）设计，核心原则包括：
   - prompt 优化 > agent 数量扩展
   - 先执行再分解（ADaPT），规划深度限制为 2 层
   - 结构化产物 > 自由文本
   - 执行反馈 > 多 agent 讨论
   - 透明披露所有已知问题

`kerrigan init` 时部署到 `~/.kerrigan/skills/`。Worker agent 通过 `--add-dir`（claude）或 `--skills-dir`（kimi）加载。

## CLI 命令总览

```bash
kerrigan init                                              # 初始化
kerrigan task create --summary "..." [--workdir .]         # 创建任务
kerrigan task list                                         # 列出任务

kerrigan dag add --task <id> --summary "..." [...]         # 添加节点
kerrigan dag list --task <id>                              # 所有节点
kerrigan dag ready --task <id>                             # 可执行节点
kerrigan dag get --task <id> --node <id>                   # 节点详情
kerrigan dag update --task <id> --input '<json>'           # 批量更新

kerrigan exec --task <id> --node <id> --adapter claude [--detach]
kerrigan send --node <id> --message "..." [--interrupt]
kerrigan stop --node <id>

kerrigan watch <task-id> [--interval 2]
kerrigan status [task-id]
```

所有命令输出 JSON。
