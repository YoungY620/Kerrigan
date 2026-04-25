# Node I/O Contract 设计

**范围**：DAG 节点输入输出的结构化约束  
**基准版本**：v1.0.1  
**日期**：2026-04-25

---

## 问题

节点间通过 `result_summary` 自由文本传递数据，没有任何结构约束。类型不匹配、字段缺失、格式错误只在 LLM 消耗完 token 之后才暴露。

## 设计

**新创建的节点必须声明 contract**，包含两部分：

**`predecessor`**：以前驱节点 ID 为 key 的 map，`--depends-on` 中列出的每一个前驱都必须在这里有对应的 schema 条目。每个前驱产出的结构不同，分别声明、分别验证。无前驱的根节点可省略此字段（或传空 map）。

**`output`**：对自身输出的承诺，`schema` 始终必填。执行成功时 result_summary 必须满足此 schema。

**向后兼容**：DB 中已有的、无 contract 的节点，执行时（`dag update`）只报 warning，和data并列的字段，不阻断，继续正常流程。

两个检查时机：

| 时机 | 检查内容 | 失败行为 |
|------|---------|---------|
| `dag add` | contract 是否完整（必须覆盖所有前驱）；若前驱已声明 `output.schema`，做静态兼容性检查 | contract 不完整→阻断；静态不兼容→warn |
| `dag update --status succeeded` | 逐个验证前驱实际输出；验证自身 result_summary | 阻断，返回带选项的错误提示 |

## `dag update` 被阻断时的错误处理

当 `output.schema` 验证失败时，有两种情况需要区分：

**A. 执行确实失败了**：用 `status=failed` 正常标记即可。

**B. 执行成功了，但创建节点时设置的 contract 有误**：将本节点标记为 `failed`，在 `failure_reason` 中以 `ContractCreationError:` 前缀说明哪些字段不符，同时将实际产出存入 `result_summary` 保留证据，再创建一个新节点承接实际成功的输出继续 DAG。

错误提示中给出两条明确操作指令（见附录 E）。`failure_reason` 的 `ContractCreationError:` 前缀便于后续 reflect 分析时识别根因，无需新增字段。

---

## 附录 A：contract 格式（JSON Schema）

```jsonc
{
  // predecessor: { "<node-id>": <JSON Schema> }
  // key 是 --depends-on 中的节点 ID，每个前驱各自声明，互不干扰
  "predecessor": {
    "abc-111": {                          // 前驱节点 A：负责分析 REST 接口
      "type": "object",
      "required": ["endpoints"],
      "properties": {
        "endpoints": {
          "type": "array",
          "minItems": 1,
          "items": { "type": "string" }
        },
        "base_url": { "type": "string" }
      }
    },
    "abc-222": {                          // 前驱节点 B：负责获取鉴权 token
      "type": "object",
      "required": ["auth_token"],
      "properties": {
        "auth_token": {
          "type": "string",
          "pattern": "^Bearer .+"         // 正则匹配
        },
        "expires_in": {
          "type": "integer",
          "minimum": 60                   // 数值范围：至少 60 秒有效期
        }
      }
    }
  },
  "output": {
    "schema": {                           // 可选，标准 JSON Schema Draft 2020-12
      "type": "object",
      "required": ["status", "deployed_url"],
      "properties": {
        "status": {
          "type": "string",
          "enum": ["deployed", "partial"] // 枚举取值
        },
        "deployed_url": {
          "type": "string",
          "pattern": "^https://"
        },
        "error_count": {
          "type": "integer",
          "maximum": 0
        }
      }
    }
  }
}
```

`predecessor` map 中未列出的前驱节点不做 schema 验证（unconstrained）。某个前驱的 result_summary 无法解析为 JSON 时，视为 `{}`，在验证日志中记录警告。`dag add` 时若 map 中的某个 key 不在 `--depends-on` 列表里，返回错误（声明了一个不存在的前驱的 schema）。

---

## 附录 B：数据类型变更

`src/model.rs` 新增：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeContract {
    // key = predecessor node ID，value = 该前驱 result_summary 须满足的 JSON Schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predecessor: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputRequirement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}
```

`Node` 和 `NewNode` 各新增 `contract: Option<NodeContract>` 字段。`NodeMetadata` 不变。

---

## 附录 C：存储变更

`src/db.rs` 的 `open_task()` 末尾追加幂等迁移（仅一列）：

```rust
conn.execute_batch(
    "ALTER TABLE nodes ADD COLUMN contract_json TEXT;"
).ok();
```

新增 `schema_validation_log` 表：

```sql
CREATE TABLE IF NOT EXISTS schema_validation_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id     TEXT NOT NULL,
    phase       TEXT NOT NULL CHECK(phase IN ('static_schema', 'runtime_predecessor', 'runtime_output')),
    result      TEXT NOT NULL CHECK(result IN ('ok', 'warn', 'fail')),
    details     TEXT NOT NULL DEFAULT '',   -- jsonschema 返回的错误列表（JSON）
    created_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_svl_node ON schema_validation_log(node_id);
```

---

## 附录 D：engine.rs 改动

**`add_node`**（前驱存在性验证之后、INSERT 之前）：

```
// 1. contract 完整性检查（强制）
if contract 缺失:
    return Err("--contract is required. Declare predecessor schemas for all \
                depends-on nodes and an output schema.")

if contract.output.schema 缺失:
    return Err("contract.output.schema is required")

if depends_on 非空 且 contract.predecessor 未覆盖所有 depends_on 节点:
    缺失的 = depends_on - contract.predecessor.keys()
    return Err("contract.predecessor missing schemas for: {缺失的}")

if contract.predecessor 中有 key 不在 depends_on 里:
    return Err("contract.predecessor key '{node_id}' not in --depends-on list")

// 2. 静态兼容性检查（仅当前驱已声明 output.schema 时，warn 不 fail）
for (node_id, expected_schema) in contract.predecessor:
    读取前驱的 contract_json → output.schema
    若前驱已声明 output.schema:
        jsonschema::validate(pred_output_schema_instance, &expected_schema)
        失败 → 写 schema_validation_log(static_schema, warn) + 在 data 中附提示
```

**`update_nodes`**（status=succeeded 时）：

```
// 无 contract 的旧节点：warn 并跳过所有验证
if contract 缺失:
    写 schema_validation_log(runtime_output, warn, "no contract declared")
    继续正常流程

// 1. 逐个验证前驱实际输出
for (node_id, expected_schema) in contract.predecessor:
    读取该前驱的 result_summary，解析为 JSON（失败则视为 {}，写 warn）
    jsonschema::validate(&pred_json, &expected_schema)
    失败 → 写 schema_validation_log(runtime_predecessor, fail) + 阻断 + 返回错误

// 2. 验证自身输出
result_summary 必须是合法 JSON，否则阻断
jsonschema::validate(&result_json, &contract.output.schema)
失败 → 写 schema_validation_log(runtime_output, fail) + 阻断 + 返回两选项错误提示
```

`get_node` 查询新增 `contract_json` 列的读取。

---

## 附录 E：被阻断时的错误提示格式

```json
{
  "error": "output schema validation failed for node 'abc-123':\n  - field 'deployed_url': required but missing\n  - field 'status': must be one of [\"deployed\",\"partial\"] but got \"done\"\n\nIf execution actually failed, use status=failed normally.\n\nIf execution succeeded but the contract was wrong at creation time:\n  Step 1 — mark this node as failed with reason prefix 'ContractCreationError:' (preserve actual output in result_summary):\n    kerrigan dag update --task <T> --input '[{\"id\":\"<N>\",\"status\":\"failed\",\"metadata\":{\"failure_reason\":\"ContractCreationError: output_schema mismatch - deployed_url missing\",\"result_summary\":\"<actual output here>\"}}]'\n  Step 2 — create a corrected node to carry the successful result forward:\n    kerrigan dag add --task <T> --summary '...' --depends-on <predecessors> [--contract '{corrected contract}']"
}
```

`failure_reason` 的 `ContractCreationError:` 前缀是唯一的约定，无需新增 `error_type` 字段或数据库列。

---

## 附录 F：新增依赖

`Cargo.toml` 新增：

```toml
jsonschema = { version = "0.29", default-features = false }
```

`default-features = false` 关闭 HTTP $ref 解析，无需网络，编译更快。

---

## 附录 G：error.rs 新增变体

```rust
#[error("--contract is required for '{node_summary}'. \
    Declare predecessor schemas for all --depends-on nodes and an output schema. \
    Nodes already in the database without a contract will only warn at execution time.")]
ContractMissing { node_summary: String },

#[error("contract incomplete for '{node_summary}': {details}")]
ContractViolation { node_summary: String, details: String },

#[error("output schema validation failed for node '{node_id}':\n{validation_errors}\n\n{recovery_hint}")]
OutputSchemaFailed { node_id: String, validation_errors: String, recovery_hint: String },
```

---

## 附录 H：新增/修改文件清单

| 文件 | 变化 |
|------|------|
| `src/model.rs` | 新增 `NodeContract`（predecessor 为 `HashMap<NodeId, JsonValue>`）、`OutputRequirement`；`Node`/`NewNode` 各加 `contract` 字段 |
| `src/db.rs` | 幂等 ALTER TABLE 一列 + `schema_validation_log` 表 |
| `src/engine.rs` | `add_node` 静态检查 + `update_nodes` 两个运行时检查 + `get_node` 读 `contract_json` |
| `src/cli.rs` | `dag add --contract` 参数 |
| `src/main.rs` | 解析 contract JSON，传入 `NewNode` |
| `src/error.rs` | `ContractViolation`、`OutputSchemaFailed` 变体 |
| `src/lib.rs` | 导出 `NodeContract` |
| `Cargo.toml` | 新增 `jsonschema` 依赖 |
