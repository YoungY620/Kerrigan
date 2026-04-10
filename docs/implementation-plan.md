# Kerrigan 实现计划

## Context

将 thinking-space/kerrigan 中与 kitty 耦合的多智能体 swarm 机制，用 Rust 重新实现。按设计文档重新设计，不是直接移植。

**Repo 结构**：
- `kerrigan-dag-engine`（独立 repo/submodule）：DAG 引擎核心库 + CLI，纯粹的任务图状态机
- `kerrigan-cli`（独立 repo/submodule）：编排层，依赖 kerrigan-dag-engine，包含 agent adapter、主循环、skills、~/.kerrigan/ 管理

核心原则：每一步高内聚、充分测试、对外接口承诺绝对有效。

## 步骤

### Step 1: DAG 引擎核心库（kerrigan-dag-engine）

**目标**：实现纯库层的 DAG 引擎，5 个接口全部可用，带完整单元测试。

**文件**：
```
kerrigan-dag-engine/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── engine.rs      # DagEngine struct + 5 个接口实现
│   ├── model.rs       # Task, TaskSummary, TaskUpdate, TaskStatus, TaskMetadata
│   ├── persist.rs     # JSON 序列化/反序列化 + 原子写入
│   └── error.rs       # DagError 枚举
├── tests/
│   └── engine_test.rs # 集成测试
```

**实现内容**：
- `TaskStatus` 枚举：Pending / Ready / Running / Succeeded / Failed
- `Task` 结构体：id, summary, description, predecessors, successors, status, metadata
- `TaskMetadata`：result_summary, failure_reason, spawned_tasks, session_ids
- `DagEngine` 5 个接口：
  - `add_task(NewTask, depends_on: Vec<TaskId>) -> Result<TaskId>`
  - `list_all() -> Vec<TaskSummary>`
  - `list_ready() -> Vec<TaskSummary>`
  - `update_tasks(Vec<TaskUpdate>) -> Result<()>`（含级联 Ready 逻辑）
  - `get_task(TaskId) -> Option<Task>`
- JSON 持久化：每次变更后原子写入（write tmp + rename）
- 文件锁：`fs2` crate 的 flock，保证并发安全

**依赖**：serde, serde_json, uuid, thiserror, fs2

**测试要点**：
- 基础 CRUD：add → get → list_all 一致性
- 依赖解析：add A, add B(depends_on A) → B 是 Pending；complete A → B 变 Ready
- 多前驱：C depends_on [A, B]，A 完成后 C 仍 Pending，B 也完成后 C 变 Ready
- 级联深度：A→B→C→D 链式级联
- 失败不传播：A Failed → B 永远 Pending
- update_tasks 批量：一次更新多个任务
- 环检测：add_task 不能引入环
- 持久化往返：engine → save → load → engine，状态完全一致
- 并发文件锁：两个进程同时操作同一个 dag.json

**验证方式**：`cargo test`，100% 通过

---

### Step 2: DAG CLI（kerrigan-dag-engine）

**目标**：在同一 repo 中为 DAG 引擎提供 CLI 入口，JSON 输入输出，可被 agent skills 直接调用。

**文件**：
```
kerrigan-dag-engine/src/
├── main.rs
├── cli.rs        # kerrigan-dag add/list/ready/update/get 子命令
```

**CLI 接口**：
```bash
kerrigan-dag add --file <dag.json> --summary "..." --description "..." --depends-on id1,id2
kerrigan-dag list --file <dag.json>                    # list_all, JSON output
kerrigan-dag ready --file <dag.json>                   # list_ready, JSON output
kerrigan-dag update --file <dag.json> --input <json>   # update_tasks, JSON input
kerrigan-dag get --file <dag.json> --id <task_id>      # get_task, JSON output
```

- 所有输出为 JSON（方便 agent 解析）
- `--file` 指定 dag.json 路径
- `update` 从 stdin 或 `--input` 读取 JSON

**依赖**：clap (derive), assert_cmd, predicates (dev)

**测试要点**：
- CLI 端到端：通过 `assert_cmd` 测试每个子命令的 stdin/stdout/exit code
- 错误场景：不存在的 dag-file、不存在的 task id、非法 JSON 输入
- 与 Step 1 的库层一致性：CLI 输出 == 直接调用引擎的结果

**验证方式**：`cargo test` + 手动 CLI 调用验证

---

### Step 3: `~/.kerrigan/` 目录管理 + 任务注册表（kerrigan-cli）

**目标**：在 kerrigan-cli 中实现跨任务的全局管理层。

**文件**：
```
kerrigan-cli/
├── Cargo.toml              # 依赖 kerrigan-dag-engine（git submodule 或 path）
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── registry/
│   │   ├── mod.rs
│   │   ├── store.rs       # Registry CRUD (registry.json)
│   │   └── model.rs       # RegistryEntry
│   └── cli/
│       ├── mod.rs
│       ├── task.rs        # kerrigan task create/list/status/inspect
│       └── init.rs        # kerrigan init
```

**目录结构**：
```
~/.kerrigan/
├── registry.json
├── lock
└── tasks/<task-id>/
    ├── dag.json
    ├── config.json
    ├── sessions.json
    ├── board.md
    └── shared/
```

**CLI 接口**：
```bash
kerrigan init                                      # 初始化 ~/.kerrigan/
kerrigan task create --summary "..." --workdir .   # 创建任务，生成目录
kerrigan task list                                 # 列出所有注册的任务
kerrigan task status <task-id>                     # 查看某任务的 DAG 状态摘要
kerrigan task inspect <task-id>                    # 查看详细信息
```

**测试要点**：
- init 幂等：多次调用不破坏已有数据
- create → list 一致性
- 并发 create：两个进程同时创建不冲突
- 目录结构完整性：create 后所有文件都存在

**验证方式**：`cargo test` + 手动验证

---

### Step 4: Agent Adapter（kerrigan-cli）

**目标**：实现 agent CLI 的封装层，支持 Claude Code 作为首个 adapter。

**文件**：
```
kerrigan-cli/src/
├── adapter/
│   ├── mod.rs
│   ├── traits.rs      # AgentAdapter trait
│   ├── claude.rs      # Claude Code adapter
│   └── session.rs     # session id 生成与管理
```

**AgentAdapter trait**：
```rust
pub trait AgentAdapter {
    async fn run(&self, prompt: &str, session_id: &str, skills_dir: Option<&Path>) -> Result<AgentOutput>;
    async fn resume(&self, session_id: &str, prompt: &str) -> Result<AgentOutput>;
}
```

**Claude Code adapter**：
- 调用 `claude --print --session-id <id> --prompt <prompt>` 非交互式模式
- 支持 `--allowedTools` 和 skills 注入
- 解析 stdout 为 AgentOutput

**Session 管理**：`krg_{timestamp}_{random}`，记录到 `sessions.json`

**测试要点**：
- trait 可 mock：用 MockAdapter 测试上层逻辑
- session id 唯一性
- Claude adapter 命令行构造正确性（不真实调用）
- 可选集成测试

**验证方式**：`cargo test`

---

### Step 5: 提示词模板 + 主循环（kerrigan-cli）

**目标**：实现 Kerrigan 的核心编排循环。

**文件**：
```
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
```
loop {
    ready = dag.list_ready()
    if ready.is_empty() { break }
    for task in ready {
        // 构造 prompt（注入前置任务的 result_summary）
        // 通过 adapter 启动 subagent
        // 解析结果，调用 dag.update_tasks()
    }
}
```

**CLI 接口**：
```bash
kerrigan run <task-id> --adapter claude
kerrigan run <task-id> --dry-run
```

**测试要点**：
- 模板渲染正确性
- 循环终止：mock adapter，DAG 全部完成后退出
- 失败不传播，循环继续
- 衍生任务正确插入
- dry-run 不调用 adapter
- 留言板注入

**验证方式**：`cargo test` + mock adapter 端到端跑完 3 节点 DAG

---

### Step 6: Skills 封装

**目标**：将 DAG 引擎和 Kerrigan CLI 封装为 skills。

**文件**（放在 monorepo docs/ 或 kerrigan-cli/skills/）：
```
skills/
├── kerrigan-dag/
│   ├── SKILL.md           # DAG 操作 skill
│   └── scripts/           # 调用 kerrigan-dag 子命令
└── kerrigan-swarm/
    ├── SKILL.md           # 任务管理 skill
    └── scripts/           # 调用 kerrigan task/run
```

两层 skills：
1. **kerrigan-dag skills**：供 loop 内 agent 操作 DAG
2. **kerrigan-swarm skills**：供外部 agent 创建/管理后台任务

**验证方式**：手动验证 + 可选集成测试

---

## 依赖总览

| Crate | 用途 | Repo |
|-------|------|------|
| serde, serde_json | 序列化 | both |
| clap (derive) | CLI 解析 | both |
| uuid | task id 生成 | dag-cli |
| thiserror | 错误类型 | both |
| fs2 | 文件锁 | dag-cli |
| tokio | 异步运行时 | kerrigan-cli |
| minijinja | 模板渲染 | kerrigan-cli |
| assert_cmd, predicates | CLI 测试 | both (dev) |
| tempfile | 测试临时目录 | both (dev) |

## 执行顺序

```
kerrigan-dag-engine:
  Step 1 (DAG 引擎库) → Step 2 (DAG CLI)

kerrigan-cli:
  Step 3 (~/.kerrigan/ + 注册表) → Step 4 (Agent Adapter) → Step 5 (主循环) → Step 6 (Skills)
```

Step 1-2 在 kerrigan-dag-engine repo 完成后，kerrigan-cli 通过 path 依赖或 git 依赖引用。每一步完成后提交 + push。
