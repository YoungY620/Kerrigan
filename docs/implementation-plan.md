# Kerrigan 实现计划

## Context

将 thinking-space/kerrigan 中与 kitty 耦合的多智能体 swarm 机制，用 Rust 重新实现。按设计文档重新设计，不是直接移植。

**Repo 结构**：

- `kerrigan-storage`（独立 repo/submodule）：Storage 核心库 + CLI，包含 `~/.kerrigan/` 存储管理、任务注册表、DAG 图维护，所有接口基于 id
- `kerrigan-cli`（独立 repo/submodule）：编排层，依赖 kerrigan-storage，包含 agent adapter、主循环、skills

核心原则：每一步高内聚、充分测试、对外接口承诺绝对有效。

## 步骤

### Step 1: Storage 核心库 + 存储管理（kerrigan-storage）

**目标**：实现 Storage的纯库层，包含 `~/.kerrigan/` 目录管理、任务注册表、DAG 图操作，所有接口基于 id，带完整单元测试。

**文件**：

```text
kerrigan-storage/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── engine.rs      # DagEngine struct：init, task_create, task_list + DAG 操作
│   ├── model.rs       # Task, Node, NodeSummary, NodeUpdate, TaskStatus, NodeMetadata, TaskEntry
│   ├── persist.rs     # JSON 序列化/反序列化 + 原子写入 + 文件锁
│   └── error.rs       # DagError 枚举
├── tests/
│   └── engine_test.rs
```

**实现内容**：

存储管理：
- `init()`：初始化 `~/.kerrigan/` 目录（幂等）
- `task_create(summary, workdir) -> task_id`：生成 task-id、创建目录结构、注册到 registry.json
- `task_list() -> Vec<TaskEntry>`：列出所有注册的任务
- 文件锁：`fs2` crate，保证并发安全
- 原子写入：write tmp + rename

DAG 操作（所有接口通过 task_id 定位 dag.json）：
- `TaskStatus` 枚举：Pending / Ready / Running / Succeeded / Failed
- `Node` 结构体：id, summary, description, predecessors, successors, status, metadata
- `NodeMetadata`：result_summary, failure_reason, spawned_tasks, session_ids
- `add_node(task_id, NewNode, depends_on) -> Result<NodeId>`
- `list_all(task_id) -> Vec<NodeSummary>`
- `list_ready(task_id) -> Vec<NodeSummary>`
- `update_nodes(task_id, Vec<NodeUpdate>) -> Result<()>`（含级联 Ready 逻辑）
- `get_node(task_id, node_id) -> Option<Node>`

**依赖**：serde, serde_json, uuid, thiserror, fs2

**测试要点**：
- init 幂等：多次调用不破坏已有数据
- task_create → task_list 一致性
- 并发 task_create：两个进程同时创建不冲突
- 目录结构完整性：create 后 dag.json / config.json / sessions.json / board.md / shared/ 都存在
- 基础 CRUD：add_node → get_node → list_all 一致性
- 依赖解析：add A, add B(depends_on A) → B 是 Pending；complete A → B 变 Ready
- 多前驱：C depends_on [A, B]，A 完成后 C 仍 Pending，B 也完成后 C 变 Ready
- 级联深度：A→B→C→D 链式级联
- 失败不传播：A Failed → B 永远 Pending
- update_nodes 批量：一次更新多个节点
- 环检测：add_node 不能引入环
- 持久化往返：engine → save → load → engine，状态完全一致
- 并发文件锁：多个 swarm job 同时操作注册表和公共区域文件时的互斥

**验证方式**：`cargo test`，100% 通过

---

### Step 2: DAG CLI（kerrigan-storage）

**目标**：在同一 repo 中为 Storage提供 CLI 入口，所有接口基于 id，JSON 输出，可被 agent skills 直接调用。

**文件**：

```text
kerrigan-storage/src/
├── main.rs
├── cli.rs
```

**CLI 接口**：

```bash
kerrigan-storage init                                                    # 初始化 ~/.kerrigan/
kerrigan-storage task create --summary "..." --workdir .                 # 创建任务
kerrigan-storage task list                                               # 列出所有任务

kerrigan-storage add --task <task-id> --summary "..." --description "..." --depends-on n1,n2
kerrigan-storage list --task <task-id>                                   # list_all, JSON output
kerrigan-storage ready --task <task-id>                                  # list_ready, JSON output
kerrigan-storage update --task <task-id> --input <json>                  # update_nodes, JSON input
kerrigan-storage get --task <task-id> --node <node-id>                   # get_node, JSON output
```

- 所有输出为 JSON（方便 agent 解析）
- 所有操作通过 task-id 定位，不暴露文件路径
- `update` 从 stdin 或 `--input` 读取 JSON

**依赖**：clap (derive), assert_cmd, predicates (dev)

**测试要点**：
- CLI 端到端：通过 `assert_cmd` 测试每个子命令的 stdin/stdout/exit code
- 错误场景：不存在的 task-id、不存在的 node-id、非法 JSON 输入
- 与 Step 1 的库层一致性：CLI 输出 == 直接调用引擎的结果

**验证方式**：`cargo test` + 手动 CLI 调用验证

---

### Step 3: Storage Skills 封装（kerrigan-storage）

**目标**：将 Storage CLI 封装为 skills，供 loop 内 agent 通过 DAG skills 操作 DAG。这一步必须在 kerrigan-cli 主循环之前完成，因为 agent 依赖这些 skills 来编排任务。

**文件**：

```text
kerrigan-storage/
└── skills/
    └── kerrigan-storage/
        ├── SKILL.md           # DAG 操作 skill 说明：何时触发、可用操作、使用示例
        └── scripts/           # 调用 kerrigan-storage CLI 子命令的 wrapper
```

**SKILL.md 内容**：
- 描述所有可用操作：task create/list、add/list/ready/update/get
- 每个操作的参数说明和输出格式
- 典型使用流程示例（查看 ready 节点 → get 详情 → subagent 执行 → update 回写结果）
- 何时应该 add 新节点（失败重试、任务拆分）

**测试要点**：
- SKILL.md 内容完整性：覆盖所有 CLI 子命令
- 每个 script 独立运行不报错
- script 调用 CLI 后输出格式正确

**验证方式**：手动验证每个 script + 对照 Step 2 的 CLI 输出

---

### Step 4: Agent Adapter（kerrigan-cli）

**目标**：在 kerrigan-cli 中实现 agent CLI 的封装层，首批支持 Claude Code 和 Kimi CLI 两个 adapter，默认 YOLO 模式。

**文件**：

```text
kerrigan-cli/
├── Cargo.toml              # 依赖 kerrigan-storage
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── adapter/
│   │   ├── mod.rs
│   │   ├── traits.rs      # AgentAdapter trait
│   │   ├── claude.rs      # Claude Code adapter
│   │   ├── kimi.rs        # Kimi CLI adapter
│   │   └── session.rs     # session id 生成与管理
│   └── cli/
│       └── mod.rs
```

**AgentAdapter trait**：

```rust
pub trait AgentAdapter {
    async fn run(&self, prompt: &str, session_id: &str, skills_dir: Option<&Path>) -> Result<AgentOutput>;
    async fn resume(&self, session_id: &str, prompt: &str) -> Result<AgentOutput>;
}
```

**Claude Code adapter**：
- 调用 `claude --print --session-id <id> --prompt <prompt> --dangerously-skip-permissions` 非交互式 YOLO 模式
- 支持 `--allowedTools` 和 skills 注入
- 解析 stdout 为 AgentOutput

**Kimi CLI adapter**：
- 调用 `kimi` 的非交互式模式，YOLO 默认开启
- 支持 session id 指定和 resume
- 支持 skills 目录注入

**Session 记录**：每个 node 执行时由 agent CLI 自行创建和管理 session（claude/kimi 各自的 session 机制），kerrigan 只在 `~/.kerrigan/tasks/<task-id>/sessions.json` 中记录 node_id 与 agent session id 的映射关系，用于 debug 追溯，不做 session 归一化存储

**测试要点**：
- trait 可 mock：用 MockAdapter 测试上层逻辑
- session id 唯一性
- Claude adapter 命令行构造正确性（验证 YOLO 标志、session id 等参数）
- Kimi adapter 命令行构造正确性
- 可选集成测试（需要 claude/kimi 可用）

**验证方式**：`cargo test`

---

### Step 5: 提示词模板 + 主循环（kerrigan-cli）

**目标**：实现 Kerrigan 的核心编排循环。

**文件**：

```text
kerrigan-cli/src/
├── runner/
│   ├── mod.rs
│   ├── loop.rs        # 主循环逻辑
│   ├── templates.rs   # 提示词模板渲染
│   └── board.rs       # 留言板监控
├── cli/
│   └── run.rs         # kerrigan run <task-id>
```

**三个提示词模板**（内置，可覆盖）：
1. **init.md**：首轮，注入 DAG skills 说明 + ready 节点 + 任务描述
2. **continue.md**：后续轮次，注入最新 DAG 状态
3. **resume.md**：新 session 恢复，引导读取所有信息后继续

**主循环**：

kerrigan 不自行遍历节点分发执行，而是每轮调用一个主 agent，由 agent 通过 DAG skills 自行读取 ready 节点、用 subagent 并行执行、回写结果。kerrigan 的 loop 只负责反复"踢"主 agent。

```text
loop {
    ready = kerrigan-storage ready(task_id)  // 仅用于检查终止条件
    if ready.is_empty() { break }

    // 构造本轮提示词：
    //   - 注入所有 ready 节点的 summary 信息（kerrigan 程序侧预先查询并拼入）
    //   - 要求 agent 使用 DAG engine skill 获取节点详情和前置结果
    //   - 要求 agent 用 subagent 并行执行 ready 节点
    //   - 要求 agent 通过 DAG engine skill 回写结果、衍生新节点
    prompt = render_template(ready_nodes_summary)
    adapter.run(prompt, session_id, skills_dir)
}
```

**CLI 接口**：

```bash
kerrigan run <task-id> --adapter claude
```

**测试要点**：
- 模板渲染正确性
- 循环终止：mock adapter，DAG 全部完成后退出
- 失败不传播，循环继续
- 衍生任务正确插入
- 留言板注入

**验证方式**：`cargo test` + mock adapter 端到端跑完 3 节点 DAG

---

### Step 6: Kerrigan Swarm Skills 封装

**目标**：将 Kerrigan CLI 封装为 skills，供外部 agent 创建/管理后台任务。

**文件**：

```text
skills/
└── kerrigan-swarm/
    ├── SKILL.md           # 任务管理 skill
    └── scripts/           # 调用 kerrigan task/run
```

**验证方式**：手动验证 + 可选集成测试

---

## 依赖总览

| Crate | 用途 | Repo |
|-------|------|------|
| serde, serde_json | 序列化 | both |
| clap (derive) | CLI 解析 | both |
| uuid | task/node id 生成 | dag-engine |
| thiserror | 错误类型 | both |
| fs2 | 文件锁 | dag-engine |
| tokio | 异步运行时 | kerrigan-cli |
| minijinja | 模板渲染 | kerrigan-cli |
| assert_cmd, predicates | CLI 测试 | both (dev) |
| tempfile | 测试临时目录 | both (dev) |

## 执行顺序

```text
kerrigan-storage:
  Step 1 (Storage 库 + 存储管理) → Step 2 (CLI) → Step 3 (Storage Skills)

kerrigan-cli:
  Step 4 (Agent Adapter) → Step 5 (主循环) → Step 6 (Kerrigan Swarm Skills)
```

Step 1-3 在 kerrigan-storage repo 完成后，kerrigan-cli 通过 path 依赖引用。每一步完成后提交 + push。
