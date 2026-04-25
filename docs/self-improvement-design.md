# Kerrigan 自改进系统设计

**基准版本**：v1.0.1（2512 LOC）  
**目标版本**：v1.1.0（+~1800 LOC）  
**日期**：2026-04-25

---

## 一、问题

Kerrigan 当前是无状态的——每次任务执行后，经验不被保留，相同的错误可以反复出现。节点间通过 `result_summary` 自由文本通信，没有任何结构约束，类型不匹配只在 LLM 消耗完 token 之后才暴露。

目标：在不触碰 Rust 内核的前提下，让 Kerrigan 从每次执行中学习并持续改进。

---

## 二、不可变边界

系统的安全基础是严格区分什么能改、什么不能改：

```
不可变（永远不修改）          可变（本系统修改的目标）
─────────────────────         ──────────────────────────────────────
src/*.rs Rust 二进制           ~/.kerrigan/skills/kerrigan/*.md
DAG 执行语义                   ~/.kerrigan/gene.db
engine.rs / exec.rs            ~/.kerrigan/proposals/<id>.yaml
                               task.db → schema_validation_log 表
```

Rust 二进制是 Meta-Agent，永不修改自己。`SKILL.md`、`guide.md`、Gene 记忆是 Target，由系统根据执行结果迭代改进。这是所有安全保证的前提。

---

## 三、三个方向的连接关系

三个方向不是独立功能，而是一条数据管线的三段：

```
① dag add / dag update
      ↓ 结构化校验
   schema_validation_log（失败信号的来源）

② kerrigan reflect --task <id>
      ↓ DAG 拓扑反向遍历
   BlameReport → Diagnosis（5类，规则优先）
      ↓
   ModificationProposal（YAML，带 target 命名空间）
      ↓
   Safety Gate（置信度/预算/循环检测）

③ kerrigan proposal apply <id>
      ├── Gene 记忆更新（Hot/Warm/Cold 分层）
      └── guide / SKILL.md 修改（archive 版本控制）

   下次任务执行时：
   prompt.rs 读取 Hot Tier Gene 注入到 Worker prompt
```

每一段的输出是下一段的输入。方向①产生的结构化失败信号，让方向②的诊断精度从模糊猜测变为有证据的归因。方向②产生的具体提案，是方向③的操作指令。

---

## 四、方向①：为"结果即通信"加契约

**现状**：`result_summary` 是自由文本，节点 A 产出代码分析结果，节点 B 期望数据库 schema，`--depends-on` 不做任何类型检查。

**方案**：为每个节点添加可选的 `NodeContract`，声明其 input/output 的 JSON Schema。契约在两个时机校验：

- **`dag add` 时（静态）**：检查新节点的 input_schema 与前驱 output_schema 是否兼容，按 policy（warn/fail）决定是否阻断
- **`dag update` 成功时（运行时）**：验证实际 result_summary 是否符合声明的 output_schema

不声明 contract 的节点行为完全不变（`None = ⊤`，接受一切）。这保证向后兼容，使 contract 成为增量增强而非破坏性改造。

**关键决策**：选用 JSON Schema 而非 Rust 类型系统——节点由 LLM 在运行时动态创建，无法在编译时确定类型；JSON Schema 是唯一同时满足"LLM 能理解并生成"和"有成熟 Rust 验证库"的方案。初期用手写 L1+L2 检查（required 字段 + 类型兼容），不引入外部 crate，覆盖 ~80% 的实际错误场景。

校验结果写入 `schema_validation_log` 表，这是方向②的原始输入。

---

## 五、方向②：结构因果分析 → 反思 → 提案

**现状**：失败了就重试，没有根因分析，也不知道该改什么。

**方案**：`kerrigan reflect --task <id>` 命令，对已执行完（失败）的任务做三步分析：

**Blame（归因）**：纯 DAG 拓扑推理，0 LLM 调用。反向遍历 DAG，无失败前驱的失败节点 = RootCause，连锁失败 = Propagated。把 schema_validation_log 中的失败也附加到对应节点。

**Diagnosis（诊断）**：三层级联，从确定性最高的类别开始：
1. 规则引擎（0 LLM）：resource_failure（timeout/429/OOM，正则匹配）、schema_mismatch（直接来自 validation log）
2. 轻量 LLM（`--llm` 标志，默认关闭）：decomposition_failure、quality_failure
3. 兜底：prompt_failure

约 60% 的失败场景（Resource + Schema）可以零 LLM 成本完成诊断。

**置信度的来源**：置信度不是 LLM 的自我评估（LLM 自报置信度校准极差），而是从两套不同机制推算：

- 规则路径：公式计算，与 LLM 无关。SchemaMismatch 有 fail 记录 → 0.99，只有 warn → 0.80；ResourceFailure 按命中 pattern 数推算，单一命中 ~0.80，多 pattern 同时命中最高 0.95。
- LLM 路径：给 LLM 一份针对该类别的结构化证据核查清单，每项只回答 YES/NO，`confidence = YES_count / total_items × category_weight`。LLM 回答的是"这条证据存在吗"，而不是"你有多大把握"。PromptFailure 作为兜底，置信度从 0.5 起，每被排除一条其他类别的证据就上调——**越是排除了所有其他可能，才归入 PromptFailure，置信度反而越高**（见附录 B.3）。

**Proposal 生成**：每类失败对应固定的修改策略（见附录 B.3），产出 YAML 提案文件，写入 `~/.kerrigan/proposals/`。提案带 `target` 命名空间（`gene.*` 或 `code.*`）和推算出的 `confidence` 分数。

**Safety Gate**：置信度 ≥ 0.8 自动批准，0.5-0.8 需人工确认，<0.5 直接拒绝。循环检测：同一 target 7 天内 apply + rollback ≥ 2 次则阻断。

---

## 六、方向③：可变层——Gene 记忆与代码自修改

### Gene（可变记忆）

结构：三层 SQLite 存储（`gene.db`）：

- **Hot Tier**（≤ 2000 字符，始终注入 prompt）：最高分的核心经验，由 `composite_score` 动态排名组装
- **Warm Tier**（全量，按需检索）：所有有效 Gene 条目
- **Cold Tier**（归档，不参与检索）：长期低分条目

`composite_score = 0.35 × importance + 0.25 × recency + 0.40 × vote_count_norm`

进化操作（全部有 changelog 记录）：ADD / UPVOTE / DOWNVOTE / EDIT / MERGE。关键原则：**DOWNVOTE 而非 DELETE**——被当前证据反驳的经验可能在未来被新证据重新支持。

Gene 通过 `prompt.rs` 的 `## Accumulated Experience` section 注入每个 Worker 的 system prompt。Hot 为空时该 section 不出现，优雅降级。

### 可变代码（Harness 自修改）

`kerrigan proposal apply <id>` 对两类 target 有不同处理路径：

- **`gene.*`**：调用 `GeneOp::Add` 写入 gene.db
- **`code.*`**：修改 `~/.kerrigan/skills/kerrigan/` 下的指导文件

代码修改前必须先创建 archive 快照（`archive/SKILL.v002.md`）。Rollback = 从 archive 恢复文件 + Downvote 对应 Gene 条目。没有 archive 就没有 apply，顺序强制保证。

---

## 七、实施路线图

四个 Phase，严格顺序（每个 Phase 都是下一个的前提）：

| Phase | 内容 | 新增 LOC | 交付能力 |
|-------|------|---------|---------|
| **P0** 基础设施 | gene.db + schema_validation_log 表 + NodeContract 类型 | ~200 | 存储层就绪 |
| **P1** I/O Schema | dag add --contract + 静态/运行时验证 | ~700 | 类型错误在 dag add 时拦截 |
| **P2** 反思引擎 | kerrigan reflect + blame/diagnosis/proposal | ~500 | 失败任务产出可操作提案 |
| **P3** 可变层 | kerrigan gene/proposal + apply/rollback + prompt 注入 | ~400 | 经验自动积累，指导文件自动演进 |

P1 可以独立部署验证。P2 在 P1 之前也能工作（ResourceFailure 路径不依赖 schema log）。P3 的 Gene 注入在 P2 之前可以手动 `gene add` 预热，不必等 reflect 流程跑通。

---

## 八、新增命令一览

```
dag add --contract <JSON>           # P1：声明节点 I/O 契约
kerrigan reflect --task <id>        # P2：分析并产出修改提案
  [--positive]                      #   提取成功任务的正向经验
  [--llm]                           #   启用 LLM 诊断（默认关闭）
kerrigan proposal list/show/apply/reject/rollback  # P2/P3
kerrigan gene list/add/core         # P3：直接操作 Gene 记忆
```

所有新命令遵循现有 JSON 信封格式。

---

## 附录 A：存储变更

### A.1 `~/.kerrigan/` 新增结构

```
~/.kerrigan/
├── gene.db                          ← 新：全局 Gene 存储（P0）
├── proposals/<id>.yaml              ← 新：reflect 产出的提案（P2）
└── skills/kerrigan/
    └── archive/                     ← 新：指导文件版本历史（P3）
        ├── SKILL.v001.md
        ├── SKILL.v002.md
        └── guide.v001.md
```

### A.2 `task.db` 新增两张表（P0）

```sql
CREATE TABLE IF NOT EXISTS schema_registry (
    name        TEXT PRIMARY KEY,
    schema_json TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS schema_validation_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id     TEXT NOT NULL,
    phase       TEXT NOT NULL CHECK(phase IN ('static','runtime')),
    result      TEXT NOT NULL CHECK(result IN ('ok','warn','fail')),
    details     TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL
);
```

`nodes` 表新增列（幂等迁移，`.ok()` 忽略"列已存在"）：

```sql
ALTER TABLE nodes ADD COLUMN contract_json TEXT;
```

### A.3 `gene.db` schema（P0）

```sql
CREATE TABLE IF NOT EXISTS gene_entries (
    id              TEXT PRIMARY KEY,
    content         TEXT NOT NULL,
    category        TEXT NOT NULL DEFAULT '',
    tier            TEXT NOT NULL DEFAULT 'warm'
                        CHECK(tier IN ('hot','warm','cold')),
    importance      REAL NOT NULL DEFAULT 5.0,
    vote_count      INTEGER NOT NULL DEFAULT 0,
    access_count    INTEGER NOT NULL DEFAULT 0,
    last_accessed   TEXT,
    source_task_id  TEXT,
    source_proposal_id TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS gene_changelog (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    gene_id     TEXT NOT NULL,
    operation   TEXT NOT NULL CHECK(operation IN
                    ('add','upvote','downvote','edit','merge','archive','resurrect')),
    old_content TEXT,
    new_content TEXT,
    proposal_id TEXT,
    changed_at  TEXT NOT NULL
);
```

---

## 附录 B：核心数据类型

### B.1 NodeContract（`src/model.rs` 新增）

```rust
pub enum SchemaMismatchPolicy { Warn, Fail, Coerce }   // 默认 Warn

pub struct NodeContract {
    pub input_schema:       Option<JsonValue>,  // None = ⊤
    pub output_schema:      Option<JsonValue>,  // None = ⊤
    pub max_duration_secs:  Option<u64>,
    pub max_tokens:         Option<u64>,
    pub on_schema_mismatch: SchemaMismatchPolicy,
}
```

### B.2 Reflect 类型（`src/reflect.rs` 新文件）

```rust
// Blame
pub enum BlameClass { RootCause, Propagated, Bystander }

pub struct BlameEntry {
    pub node_id: String, pub blame_class: BlameClass,
    pub blame_score: f64,            // RootCause=1.0，级联衰减
    pub failure_reason: Option<String>,
    pub schema_violations: Vec<String>,
}

// Diagnosis
pub enum FailureCategory {
    ResourceFailure,      // 规则引擎，0 LLM
    SchemaMismatch,       // 直接来自 validation log，0 LLM
    DecompositionFailure, // LLM 可选
    QualityFailure,       // LLM 可选
    PromptFailure,        // 兜底
}

// Proposal
pub struct ModificationProposal {
    pub id: String,                        // "prop-YYYY-MM-DD-NNN"
    pub schema_version: String,            // "1"
    pub task_id: String,
    pub blame_node_id: String,             // 根因节点
    pub failure_category: FailureCategory,
    pub evidence: serde_json::Value,       // 推理链（见 B.3 YAML 示例）
    pub proposal_type: ProposalType,       // MemoryUpdate | CodeUpdate
    pub target: String,                    // 命名空间（见 B.4）
    pub action: ProposalAction,            // Append | Set | Delete
    pub content: String,                   // 字面值，直接写入目标
    pub confidence: f64,
    pub scope: ProposalScope,              // Local | Global
    pub requires_human_approval: bool,
    pub status: ProposalStatus,            // Pending | Approved | Applied | RolledBack
    pub created_at: String,
    // 仅 Applied 后填充：
    pub applied_gene_id: Option<String>,
    pub applied_archive_version: Option<String>,
}
```

### B.3 Proposal YAML 示例

每个提案以 YAML 文件存储于 `~/.kerrigan/proposals/<id>.yaml`，`evidence` 字段保留完整推理链供人工审核。

**ResourceFailure（规则路径，0 LLM）**：
```yaml
schema_version: "1"
id: prop-2026-04-25-001
created_at: "1745558400"
status: pending
task_id: abc-123
blame_node_id: "9f3a-..."
failure_category: resource_failure
evidence:
  matched_patterns: [timeout, rate_limit]
  failure_reason: "HTTP 429 Too Many Requests after 12 retries"
  node_summary: "Parallel web search across 8 sources"
confidence: 0.85          # 0.75 + 2/total_patterns × 0.20
requires_human_approval: false
proposal_type: code_update
target: code.contract.parallel_web_search
action: append
scope: local
content: |
  max_duration_secs: 120
  注意：并行搜索节点触发 429，建议将并行度限制在 5 以内或添加 rate-limiter 前驱节点。
```

**DecompositionFailure（LLM 证据清单路径）**：
```yaml
schema_version: "1"
id: prop-2026-04-25-002
created_at: "1745558412"
status: pending
task_id: abc-123
blame_node_id: "2c7b-..."
failure_category: decomposition_failure
evidence:
  checklist:
    needs_more_predecessor_info: true    # Q1 YES
    sibling_nodes_same_pattern: true     # Q2 YES
    too_many_actions_in_one_node: false  # Q3 NO
    all_predecessors_succeeded: true     # Q4 YES
  yes_count: 3
  failure_reason: "Cannot complete review without knowing test coverage results"
  node_summary: "Review entire codebase for security issues"
confidence: 0.675          # 3/4 × 0.90
requires_human_approval: true
proposal_type: memory_update
target: gene.decomposition_patterns
action: append
scope: local
content: |
  代码审查节点依赖测试覆盖率数据时，必须将测试分析设为前驱节点，不能并行执行。
  错误模式：把"代码审查"和"测试验证"合并进一个节点。
```

**PromptFailure（排除法，已排除 3 类）**：
```yaml
schema_version: "1"
id: prop-2026-04-25-003
created_at: "1745558430"
status: pending
task_id: abc-123
blame_node_id: "7e1d-..."
failure_category: prompt_failure
evidence:
  excluded_categories:
    - resource_failure      # 无 timeout/429/OOM 信号
    - schema_mismatch       # 无 schema validation fail 记录
    - decomposition_failure # checklist YES_count = 1，低于阈值
  failure_reason: "不确定应该输出什么格式，产出了空结果"
  node_description_length: 12   # 描述过短（< 50 字）
confidence: 0.86           # 0.50 + 3 × 0.12
requires_human_approval: false
proposal_type: code_update
target: code.skill.kerrigan
action: append
scope: global
content: |
  当节点 description 少于 50 字且无明确输出格式要求时，Agent 应在执行前
  先写下预期输出格式，再开始执行。

### B.3 Proposal 生成策略（失败类别 → 修改目标 → 置信度推算）

| 失败类别 | proposal_type | target | 置信度来源 |
|---------|--------------|--------|-----------|
| ResourceFailure | CodeUpdate | `code.contract.<node>` | 规则：`0.75 + matched_patterns/total × 0.20`，上限 0.95 |
| SchemaMismatch | CodeUpdate | `code.contract.<node>` | 规则：有 fail 记录 → 0.99，仅 warn → 0.80 |
| DecompositionFailure | MemoryUpdate | `gene.decomposition_patterns` | 证据清单 YES/NO（见下） |
| QualityFailure | MemoryUpdate | `gene.quality_criteria` | 证据清单 YES/NO（见下） |
| PromptFailure | CodeUpdate | `code.skill.kerrigan` | 排除法：`0.50 + excluded_categories × 0.12` |

**LLM 路径的证据清单**（LLM 只回答 YES/NO，不自评置信度）：

DecompositionFailure（`category_weight = 0.90`）：
```
1. failure_reason 提到"需要更多信息"或"前驱结果不足"？
2. 有多个兄弟节点以相似模式失败？
3. 节点 description 要求在单次执行中完成超过 3 个不相关动作？
4. 所有前驱节点均 succeeded（排除传播失败）？
confidence = YES_count / 4 × 0.90
```

QualityFailure（`category_weight = 0.85`）：
```
1. 所有前驱节点均 succeeded？
2. failure_reason 提到"不完整"/"不准确"/"质量不足"？
3. 无 resource/schema 失败信号？
4. 节点曾成功执行过同类任务（session_ids > 0）？
confidence = YES_count / 4 × 0.85
```

PromptFailure（兜底，排除法）：
```
confidence = 0.50 + (其他类别被排除的数量) × 0.12
# 排除了 ResourceFailure + SchemaMismatch + DecompositionFailure → 0.50 + 3×0.12 = 0.86
# 所有其他类别均无证据时，PromptFailure 的置信度最高
```

### B.4 Target 命名空间

| 前缀 | 指向 |
|------|------|
| `gene.<category>` | gene.db，写入指定分类 |
| `code.skill.kerrigan` | `skills/kerrigan/SKILL.md` |
| `code.guide.main` | `skills/kerrigan/guide.md` |
| `code.guide.research` | `skills/kerrigan/guide-research.md` |
| `code.guide.software` | `skills/kerrigan/guide-software-development.md` |
| `code.contract.<summary>` | 特定节点的 NodeContract 参数 |

### B.5 Gene 操作（`src/gene.rs`）

```rust
pub enum GeneOp {
    Add     { content: String, category: String, importance: f64, source_task_id: Option<String> },
    Upvote  { id: String },
    Downvote{ id: String },
    Edit    { id: String, new_content: String },
    Merge   { keep_id: String, drop_id: String },
}

// composite_score = 0.35 × importance_norm + 0.25 × recency_decay + 0.40 × vote_norm
// Hot Tier 升降规则：
//   Warm → Hot : composite_score > 0.8 且 Hot 总长 ≤ 2000 字符
//   Hot → Warm : composite_score < 0.8 或 7 天未访问
//   Warm → Cold: composite_score < 0.2 且 30 天未访问 且 vote_count < 2
```

### B.6 Safety Gate 参数

```
confidence ≥ 0.80  → AutoApprove
confidence  0.50-0.80 → requires_human_approval = true
confidence < 0.50  → Reject
同一 target 7 天内 Applied + RolledBack ≥ 2 次 → 阻断（循环检测）
每日自动批准上限：15 个
单次 reflect 产出上限：5 个提案
```

---

## 附录 C：Schema 验证实现说明

初期不引入 `jsonschema` crate（避免增加依赖），手写 L1+L2 检查：

- **L1**：required 字段存在性
- **L2**：同名 property 的 JSON 类型一致（string/number/boolean/array/object）

覆盖约 80% 的实际错误场景。`schema_check_compatibility(provider, consumer)` 返回 `Compatible / PartiallyCompatible / Incompatible`。后续可无缝替换为完整 `jsonschema` 实现，接口不变。

新增依赖建议：`serde_yaml = "0.9"`（Proposal 序列化），`regex = "1"`（ResourceFailure 检测）。

---

## 附录 D：新增源文件清单

| 文件 | Phase | 职责 |
|------|-------|------|
| `src/gene.rs` | P0 | GeneDb CRUD + composite_score + build_core_gene |
| `src/schema.rs` | P1 | schema_check_compatibility + schema_validate_data |
| `src/reflect.rs` | P2 | blame_analysis + diagnose + generate_proposals + SafetyGate |
| `src/apply.rs` | P3 | apply_proposal + rollback_proposal + archive_current_version |

现有文件改动：`model.rs`（+NodeContract）、`db.rs`（+2张表+1列）、`engine.rs`（+静态/运行时验证调用）、`cli.rs`（+新命令）、`main.rs`（+新命令路由）、`prompt.rs`（+Gene注入）、`error.rs`（+SchemaMismatch变体）、`lib.rs`（+4个模块导出）。
