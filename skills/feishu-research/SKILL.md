---
name: feishu-research
description: >
  Use Kerrigan's DAG framework to investigate Feishu group-chat content end-to-end —
  from parallel per-group analysis, through cross-group topic extraction, into
  per-topic deep research (tool verification in sandboxes, code reproduction,
  news cross-check, opinion survey, academic review), culminating in a Feishu
  document tree with a main summary and linked sub-documents.
---

# Feishu Research

用 Kerrigan DAG 把"帮我调研一下最近几个群的聊天内容"这类请求,跑成一条**多群并行分析 → 跨群主题识别 → 按Types派生子任务簇 → 飞书父子文档网络**的可执行任务链。

> **前置依赖**:`kerrigan` skill 是本 skill 的前置依赖 —— 开始执行本 skill 前**必须先完整阅读** `kerrigan` skill 的所有指导文件。本 skill 假设你已经掌握 Kerrigan 的执行流程、节点粒度、依赖原则和通用调研方法论,不再重复这些内容,只讲"多群聊调研"这个场景如何落到 Kerrigan DAG 上。

> **术语定义**:本 skill 的核心研究单元是 **主题 (subject)** —— 一条消息只要指向具体对象(repo / 产品 / 论文 / 服务 / 事件 / 故障 / **时间锚点的通知或决定**)就足以立为一个主题。主题**不要求被多人讨论**;单条消息也立项。不要把中文"话题"的"被讨论"含义带进来误用。

---

## 产品硬承诺(不可降级)

核心卖点**不是信息聚合,是主动动手**。以下三条不可跳过,不得以"信号弱 / 主题多 / 单条提名"为借口降级:

1. **tool-or-code 主题默认派生 `verify-*`**(§6.7),哪怕只被一条消息提名。`read-only` 需在主题元信息明写理由(闭源 / 付费 / 无试用 / 地域受限)。
2. **event 主题默认 ≥3 独立源 + `crosscheck` 不得省略**(§6.2)。不足 3 源时 `crosscheck` result_summary 明写"单源低置信"+ 已尝试 source 列表。
3. **`verify-*-design` 是"深度"自适应,不是"做不做"自适应**:有基线 → benchmark;无基线 → 最小试用(docker pull + quickstart + 截图 + 300 字纪要);环境根本不具备 → 降 read-only 并在文档开头写原因。**无基线 ≠ 不跑**。

`quality-check` 按 design 目标校准严格度,但"**真动手的证据**"(日志 / 截图 / 命令输出)不打折,没证据一律 fail。资源不够 → 削主题数(合并 social / team-ops 呈现),**不砍动手**。

---

## Scope

**一律使用本 skill** 的情形:用户要求对一个或多个飞书群的消息做调研、摘要、报告、复盘 —— 无论是**一次性请求**("帮我总结一下今天下午")还是**周期性任务**("每天早 8 点 / 每周一给我一份")。**没有"量小就跳过 Kerrigan"的例外**,即使只有 1 个群、50 条消息也走完整 DAG,因为:

1. 保证输出格式一致(飞书文档父子层级)
2. 保证调研深度一致(按类型走协议)
3. 增量追加(补群、延长时间、每日累积)天然支持

### 周期性执行是一等公民(非例外)

每日 / 每周定时总结是本 skill 的**主场景**之一,不是被排除的特殊情况。周期性任务的接入方式:

- 由 **agent runtime 的 `SetReminder`**(cron 表达式)按时触发,每次触发都拉起一个新的 Kerrigan task 跑本 skill 的完整四阶段
- 或由用户手动在飞书里 `@bot 日报` / `@bot 周报` 触发
- 两种触发方式下,skill 内部执行逻辑完全相同

### 增量游标(必读)

周期性运行的关键是**记录上一轮处理到哪**,避免下一轮重复摘要已看过的消息。每一轮结束时,必须持久化一份游标状态,下一轮 Stage 1 开始时读取并从游标之后继续。

**游标落盘位置**:持久化到 `kerrigan` 的工作目录(具体路径和读写方式遵循 `kerrigan` skill 定义的 task 工作空间约定;本 skill 不规定具体路径)。

**游标内容**(每个监控的群一条记录):

```json
{
  "chat_id": "oc_xxx",
  "last_message_id": "om_xxx",
  "last_create_time": 1713436800,
  "last_digest_at": "2026-04-18T08:00:00+08:00",
  "last_digest_doc_url": "https://xxx.feishu.cn/docx/..."
}
```

**Stage 1 使用游标的规则**:
- 读取该群的游标记录
- `list_messages(chat_id, start_time=last_create_time, end_time=now)`,**排除** `message_id == last_message_id` 的那条(避免重复处理边界消息)
- 如果拿到 0 条新消息,该群的 `group-*-analysis` 节点直接标 `succeeded` 并在 result_summary 写"本轮无新消息",不参与后续 Stage 2 聚类
- 跑完后更新游标到本轮最后一条消息的 id / time

**游标缺失的兜底**:

- **首次监控某群**(新加入监控列表、该群游标文件不存在):默认**拉取该群全部历史消息**作为初始窗口
  - 执行前先用 `list_messages` 不带 `start_time` 粗探一次,看看总量
  - 若消息量过大(阈值参考:单群 > 1000 条 或 时间跨度 > 1 周) → **硬截断到最近 1 周**(`start_time = now - 7 days`)作为初始窗口,并在本轮主文档概览处写明"本群为首次监控,已截断至最近 7 天,更早历史未纳入"
  - 若消息量可控(≤ 1000 条 且 时间跨度 ≤ 1 周) → 全量纳入
- **游标文件损坏 / 缺字段**:按用户指定的时间窗口启动;未指定则按"首次监控"规则(默认全量,超量截断到 1 周)
- **用户要求"重新跑一次"**:完全按用户指定的时间窗口启动,**忽略**现有游标

以上任一情形跑完后,都正常写入新游标(`last_message_id` = 本轮处理的最后一条消息 id)。下一轮起就走正常的增量游标路径。

**跨轮次的主文档关系**:每轮生成的是**本轮独立的主文档**(标题 = 本轮日期/周次),不追加到上一轮的文档里。上一轮主文档的 URL 保留在游标的 `last_digest_doc_url`,可在本轮主文档"概览"部分加一句 `上一期: [yyyy-mm-dd 调研](last_digest_doc_url)`,形成历史链。

### 不适用的场景

- 实时秒级/分钟级流式监控(那需要常驻进程逐条消息处理,架构上 Kerrigan 不擅长)
- 对已生成的调研文档做小修改(直接编辑文档即可,不必重跑整个 DAG)

---

## Phase 1 — 四阶段 DAG 总览

```
┌─────── Stage 1 · 并行 per-group 分析 ────────┐
│  group-A-analysis   group-B-analysis   ...    │   (并行, 无相互依赖)
│  输出: 该群摘要 + 过滤后原文 + 工具提名 +      │
│        链接表 + 图片 OCR                        │
└─────────────────┬─────────────────────────────┘
                  ▼
┌──── Stage 2 · 跨群主题提取 (单节点) ─────────┐
│  cross-group-topic-extraction                 │
│  依赖: 所有 Stage 1 节点                      │
│  输出: 主题清单 (名称 / 涉及群 / 时间跨度 /    │
│         message_id 索引 / Types + hands-on)   │
└─────────────────┬─────────────────────────────┘
                  ▼
┌── Stage 3 · 每主题独立调研 (动态派生) ────────┐
│  topic-{X}-investigation   ← 编排节点         │
│    内部按类型派生"**一簇**"子节点 (见 Phase 6) │
│    完成后发布该主题的**飞书子文档**, 回传 URL  │
└─────────────────┬─────────────────────────────┘
                  ▼
┌── Stage 4a · 主报告撰写 (单节点) ─────────────┐
│  main-report-compose-md                       │
│  读所有 topic 的详细文档 URL,**编辑**合成主 md │
│  每主题: 摘要 + 核心结论 + 一张关键图 + 跳转链接│
└─────────────────┬─────────────────────────────┘
                  ▼
┌── Stage 4b · Markdown → 飞书主文档 ───────────┐
│  main-md-to-feishu                            │
│  建主文档 (标题=日期) → 上传图片 → 转 block   │
│   → 保留 inline link → create_nested_blocks   │
│  输出: 主文档 URL (给用户的唯一入口)          │
└───────────────────────────────────────────────┘
```

**Stage 3 的动态派生**是核心:Kerrigan 执行 Stage 2 **之前不知道主题是什么**,Stage 2 产出主题清单后,agent 才能根据清单给每个主题 `kerrigan add` 一个 `topic-X-investigation` 节点;这些节点内部再按Types继续派生子节点簇。这是 `guide.md` "Execute First, Decompose Later" 原则的典型应用。

---

## Phase 2 — Stage-by-Stage 节点设计

### Stage 1 · `group-{X}-analysis`

**summary**: `Analyze Feishu group {X} in time range [T0, T1]: filter chitchat, extract tech discussions, OCR images`

**description 必含**:
- **Objective**:对群 `{chat_id}` 在 `[T0, T1]` 区间内所有消息,产出(a) 保留 message_id/时间/sender 的**过滤后原文** `./raw/{X}.json`,(b) 该群一句话摘要,(c) **工具提名** `./tool-mentions/{X}.json`(§4.4),(d) 链接清单 `./links/{X}.json`,(e) 图片本地路径 + OCR 结果 `./ocr/{X}.json`
- **Context**:无前置节点
- **Scope**:
  - 用 `agent_feishu` 的 `list_messages(chat_id, start_time, end_time, count=200)`,分页取完
  - 时区换算:先 `date` 确认本地时区,再换算成秒级 Unix 时间戳
  - 过滤规则见 Phase 4
  - 图片下载走 `/im/v1/images/{image_key}`,存 `./figs/{X}-{message_id}.{ext}`
- **Constraints**:
  - 不跨群读数据(独立性)
  - 不做主题识别(留给 Stage 2)
  - 不写任何飞书文档(留给 Stage 3 和 Stage 4)
- **Output (result_summary)**:
  - 总消息数 / 保留数 / 过滤率
  - 类型分布(text / post / image / file / sticker / ...)
  - 实际时间跨度(首末 create_time)
  - 4 份产出文件路径
  - 若原文保留 ≥ 200 条,此节点**发一份"群 {X} 聊天底稿"飞书子文档**,URL 记入 result_summary
  - 失败(权限/超量/空群)→ failure_reason

**并行性**:N 个群 = N 个独立节点,全部并行。

### Stage 2 · `cross-group-topic-extraction`

**summary**: `Extract cross-group topics from all group analyses`

**description 必含**:
- **Objective**:通读所有 Stage 1 的 `./raw/*.json` + `./tool-mentions/*.json` + `./links/*.json` + `./ocr/*.json`,产出主题清单 `./subjects.json`(§5.1)
- **Context**:依赖所有 `group-*-analysis`
- **每个主题**必须含:
  - `name`:短且具体(非"AI"、"编程"这种宽泛词)
  - `涉及群`:至少 1 个,跨群讨论时列多个
  - `时间跨度`:每个涉及群的起止时间
  - `message_id 索引`:指向该主题最代表性的消息(**单条立项的主题就是 1 条**)
  - `初判 Types`:从 **tool-or-code / event / opinion / incident / team-ops / social** 中命中的所有类型(可多选)
  - `hands-on 标签`:`hands-on-available` 或 `read-only`(默认 hands-on-available,除非明确闭源/付费/无试用)
  - `简要摘要`:1-3 句
- **粒度**见 Phase 5
- **Output (result_summary)**:主题数量 / 每主题一行概要 / 清单文件路径
- 主题 ≥ 5 个时,本节点**发一份"主题提取清单"飞书子文档**作为主文档的子文档

**为什么单节点**:跨群聚类需要全局视角,无法并行。

### Stage 3 · `topic-{X}-investigation`(编排节点)

**summary**: `Investigate topic "{X}" with per-type subtask clusters, then publish Feishu sub-doc`

**description 必含**:
- **Objective**:对主题 `{X}` 产出(a) `./topics/{X}.md` 详细 markdown,(b) 飞书详细子文档(挂在主文档 parent_token 下)
- **Context**:依赖 `cross-group-topic-extraction` + 主题涉及的 `group-*-analysis` 节点
- **执行步骤**:
  1. 读 Stage 2 给出的 Types 列表和 hands-on 标签
  2. **每个命中的 Type 派生对应子节点簇(§6.1–6.6);hands-on-available 时额外派生 `verify-*` 簇,包括强制的 `verify-quality-check` 终节点(§6.7)**(`kerrigan add`)
  3. 等所有子节点完成(失败处理见 §6.7 "暴露 > 降级" 原则)。**`verify-quality-check` 未通过时**:首选修复后重跑;次选**保留实验章节并显式标注失败**(不降级藏问题);仅在环境根本不具备时才真正降级为 read-only 并删除实验章节
  4. 读所有子节点 result_summary,合成 `./topics/{X}.md`(结构见 7.2.a)
  5. 调用 7.3 的"md → 飞书"流水线建立**主题详细子文档**(parent_token = 主文档 token)
  6. 若原文/日志超长,按 7.0 触发条件继续派生孙子文档
- **Output (result_summary)**:主题详细文档 URL / 所有孙子文档 URL / 每个子节点的完成状态

**关键**:`topic-X-investigation` 本身**不做**具体调研工作,只编排子节点簇。真正的实验、采集、绘图都在子节点里。

### Stage 4a · `main-report-compose-md`

**summary**: `Compose the editorial main report markdown from all topic investigations`

**description 必含**:
- **Objective**:产出 `./main_report.md`(结构见 7.2.b)
- **Context**:依赖所有 `topic-*-investigation`
- **执行步骤**:
  1. 读每个 topic 节点的 result_summary,拿到主题详细子文档 URL
  2. **重新撰写**每个主题章节(见 7.1),**禁止直接拼接** result_summary
  3. 选 1-2 张最有代表性的图片(matplotlib 关键对比图 / 聊天信号最强的原图)放主报告
  4. 每主题章节末尾加 `→ [详见主题 X 详细文档](feishu_url)` 链接
- **Output (result_summary)**:`main_report.md` 路径 / 主题数量 / 引用的子文档 URL 列表

### Stage 4b · `main-md-to-feishu`

**summary**: `Convert main_report.md to the Feishu master document with preserved links and images`

**description 必含**:
- **Objective**:建主飞书文档,返回 URL
- **Context**:依赖 `main-report-compose-md`
- **执行步骤**:见 7.3 流水线,**inline link 必须保留为可点击**
- **Output (result_summary)**:主文档 URL / 上传图片数 / inline link 数 / 转换告警

---

## Phase 3 — 规划思路(Planning Thinking)

### 3.1 为什么是"独立采集 → 综合提炼 → 深度调研 → 编辑交付"四阶段

- **Stage 1 独立**:各群分析零耦合,保证 agent 不会被"先看了 A 群"而带偏"B 群的判断"。对应 `guide-research.md` 的多视角独立采集
- **Stage 2 聚合**:主题是**涌现**的,不是消息级的。必须全局扫一遍才能发现"群 A 下午在聊 Redis 8,群 B 晚上也在聊"——这种跨群共性只有全量视角才看得见
- **Stage 3 深度**:聊天本身是线索,不是结论。主题识别后必须按**命中的 Types + hands-on 标签**派生对应子节点簇,让结论可追溯、可复现、可挑战
- **Stage 4 编辑**:调研过程的 result_summary 是工作底稿,不是读者想看的东西。必须有一个"编辑"角色通读所有主题再重写

### 3.2 依赖关系 = 信息需求

严格按 `guide.md` 的"依赖 = 信息需求"原则:
- ✅ `topic-X-investigation` 依赖涉及的 `group-*-analysis`(需要 raw message 引用)
- ✅ `news-{X}-crosscheck` 依赖所有 `news-{X}-source-*`(需要综合比对)
- ❌ `group-A-analysis` 不依赖 `group-B-analysis`(无信息流)
- ❌ `news-source-official` 不依赖 `news-source-community`(保独立,避免污染)

### 3.3 动态派生的触发条件

**第二轮派生(Stage 2 完成后)**:Stage 2 输出主题清单,agent 立刻为每个主题 `kerrigan add` 一个 `topic-X-investigation`,依赖列表 = `[cross-group-topic-extraction, 涉及的 group-*-analysis]`

**第三轮派生(Stage 3 编排节点启动后)**:agent 读主题元数据中的 Types 和 hands-on 标签,对每个命中的 Type 派生 §6.1–6.6 对应子节点簇;hands-on-available 时再派生 `verify-*` 簇(§6.7)。这一轮派生**完全在 topic-X-investigation 内部执行**,不影响其他主题。

**禁止提前派生**:不要在 Stage 1 还没跑完时就猜测主题、预建 Stage 3 节点。Kerrigan 能力是动态扩展,不是预测性规划。

### 3.4 小规模探测(先试水后放量)

对应 `guide.md` "Execute First, Decompose Later":

- 如果用户一次要求调研 10 个群,**第一轮只建 1-2 个 `group-*-analysis` 节点作为 pilot**,跑通整个 Stage 1 链路
- Pilot 产出验证:权限 OK、过滤质量可接受、图片下载成功、OCR 可用
- 通过后再批量建其余群的节点

---

## Phase 4 — Stage 1 的过滤与提取规则

### 4.1 闲聊识别(丢弃)

**丢弃**:纯表情 / "+1 / 收到 / 哈哈" / 餐饮天气通勤吐槽 / 单纯附和 / 纯个人状态("到了 / 下班了 / 在的") / 重复问候。

**强制保留**(满足任一,单条也保):

1. **工具/技术指向**:含 repo URL(`github.com/*` / `huggingface.co/*` / `arxiv.org/*` 等) 或 具体框架/库/服务/产品名 + 任何评价词("好用 / 挺火 / 实测过 / 刷屏 / 看傻了")
2. **含时间锚点的事务通知**:时间(`明天 / 下周 / 从 X 起`) + 决策动词(`改为 / 发版 / 停服 / beta / release / 整顿 / 开坑`) 或事务名词(`例会 / 团建 / 休假 / 请假 / OKR / review 流程`)。典型:"我下周一到五请假" / "明天发版" / "下周公司内 beta"
3. **故障信号**:报错 / 栈 / "炸了" / "又挂了",无论有无回复
4. social 达 §5.3 阈值 → 升格 social 主题
5. 团队内务(会议/团建/安排)零散一条也保,归 team-ops

**优先级**:含时间锚点 + 事务影响 → 一律保留,压倒"个人状态"判定。

### 4.2 技术讨论识别(强制保留)

识别信号(满足任一即保留):
- 出现具体技术名词(框架、库、服务、协议、工具命令、版本号)
- 两人及以上就同一主题交替发言 ≥ 3 轮
- 附代码片段、报错、链接、截图
- 出现对比/选型("x 比 y 好"、"应该用 x 还是 y")
- 出现结论词("最后决定"、"结论是"、"所以")

保留时的附加字段:
- `参与者`(飞书 user_id / @name 列表)
- `是否有结论`(true/false)
- `是否有悬而未决的问题`(true/false)

### 4.3 链接抽取

对所有 `msg_type in {text, post}` 消息:
- 正则 `https?://[\w.-]+(?:/[^\s\u4e00-\u9fff]*)?` 提取所有 URL
- 分类:`github.com/*`、`x.com/*` / `twitter.com/*`、`arxiv.org/*`、飞书文档(`*.feishu.cn/*`)、博客、其他
- 对每条 URL 做 ≤ 15s 轻量元数据抓取(`<title>` / og:description),失败留空
- 输出 `./links/{X}.json`:`[{url, type, title, excerpt, msg_id, sender, time}]`

### 4.4 工具提名抽取

单条"repo 链接 + 一两句体感"的消息即是 `tool-or-code` 主题候选,Stage 1 独立归档,不走 §4.1 闲聊过滤、不设消息数阈值。

**触发**(任一):
- 工具域 URL:`github.com/*` / `huggingface.co/*` / `npmjs.com/*` / `arxiv.org/*` / `pypi.org/*` / `crates.io/*` / 产品官网
- 提名语义:"朋友说 / 很好用 / 推荐 / 实测 / 刷屏 / 挺火 / 有人试过么 / 很厉害"
- 无名提名也记(如"shuyi 老师 blog 有个神奇工具"),`tool_name_guess` 留空 + `needs_disambiguation: true`

**产出 `./tool-mentions/{X}.json`**,每条字段:`msg_id / sender / time / url / tool_name_guess / type_guess / excerpt / compat_notes / follow_ups_count / needs_disambiguation`。

### 4.5 图片 OCR

对 `msg_type == image` 的消息:
- 从 `body.content` 解析 `image_key`
- 下载 `/im/v1/images/{image_key}` 到 `./figs/{X}-{msg_id}.{ext}`
- 喂多模态模型,产出 100-300 字中文描述;若是代码/表格/截图,**保留关键文字原文**(便于后续被 Stage 3 引用)
- 判价值保留/丢弃:
  - **保留**:含文字(代码/命令/报错)、数据图表、架构图、UI 截图
  - **丢弃**:表情包、宠物/风景、纯装饰
- 输出 `./ocr/{X}.json`:`[{msg_id, image_key, local_path, category, description, has_text}]`
- 失败不中断,记入 `errors` 数组

---

## Phase 5 — Stage 2 的主题识别规则

### 5.1 主题识别

**输入**:Stage 1 的 `./raw/*.json` + `./tool-mentions/*.json` + `./links/*.json` + `./ocr/*.json`。**输出**:`./subjects.json`。

**两条路径并列**:
- **讨论聚类**(多轮讨论):语义相似度聚类,不看字面关键词;Redis 和 KeyDB 的讨论聚一起。跨群合并,各群时间跨度分列
- **单条锚点立项**(推荐 / 通知 / 事件点):`./tool-mentions/*.json` 每条提名、含时间锚点的事务通知、含版本号/事件点/故障时间点的单条 —— 跨群去重后**每条即一个主题**

**去重的原则 —— 保留 > 合并**:

目标不是构造完美的 dedup key,**是不让任何一条提名 / 通知 / 事件信号丢掉**。合并只是为了呈现不啰嗦,不是过滤。

- 把看起来指向同一对象的提名**归到一个主题下**即可(Hermes 的 3 条不管通过 URL / name 还是语义判断是同一对象,都合并)
- **拿不准是不是同一对象就分开立两个主题**,不要偷偷丢掉任何一条;Stage 3 调研时自然会发现重合并合并报告
- 合并后的主题保留**所有**涉及群、所有 msg_id、所有 sender —— 任何被合并的原条目都能在主题详细文档的 `## 原文索引` 里找到

### 5.2 立项条件

**任一即立项**(不要求被多人讨论):
- 具体 repo / 产品 / 论文 / 服务 → `tool-or-code`
- 版本号 / 发布时间 / 厂商动作 → `event`
- 含时间锚点的计划 / 通知 / 决定("明天发版" / "下周公司内 beta" / "OKR 改为双月") → `team-ops`(通知型,§6.5)
- 故障时间点 + 报错 / 影响范围 → `incident`
- 立场分明的选型 → `opinion`
- `social` 例外,仍需 §5.3 阈值

**不立项**:无具体指向的纯陈述 / 已被其他主题完全覆盖 / 个人隐私人事评价(§6.5 红线)。

### 5.3 主题标注:Types(多选)+ hands-on 标签(独立)

每个主题打两组标,两组**正交**。

#### Types(可多选,≥1 个)

| 信号 | Type |
|---|---|
| 讨论具体工具/框架/库/repo/PR/commit/源码(含论文 + 开源实现) | **tool-or-code** |
| 外部世界的新闻/版本/厂商/行业动态/价格政策变化 | **event** |
| 参与者有**明显不同立场**的选型/趋势/价值讨论 | **opinion** |
| bug/报错/prod 异常/服务故障/性能退化("坏了,要查") | **incident** |
| 团队内务:团建/会议/纪要/工作规划/人事/假期/流程 | **team-ops** |
| 非工作**集体热议**(八卦/影视/游戏/兴趣等,**必须满足阈值**) | **social** |

**每类独立判断是否命中,可同时命中多个**。同时命中即**分别**派生对应子节点簇(§6)。

**social 阈值**(未达阈值按 §4.1 丢弃):消息 ≥20 条 + 持续 ≥30 分钟或占时段比例 ≥30% + 参与人数 ≥3。

#### hands-on 标签(独立于 Types)

**默认倾向是动手**——能写代码验证就写,不要停在"读了就算"。

- `hands-on-available` —— **默认**标签,只要能通过写代码/脚本/API 调用做验证就打这个。路径包括:沙盒跑代码 / 公开可试用工具 / 飞书 API 交叉验证 / 本地 Python 脚本测试 / Docker benchmark / 论文开源实现复现
- `read-only` —— **退化标签**,只在确实无法写代码验证时用:闭源且无试用 / 付费且无试用 / 数据不可得 / 沙盒环境缺失 / 无公开实现

hands-on-available 会额外派生 `verify-*` 子簇(§6.7),与命中的 Types 独立叠加。**判断原则**:有一丝"能写脚本验证"的可能就标 hands-on-available,让 agent 在 `verify-design` 节点里评估可行性;别提前自我设限为 read-only。

#### 示例

| 主题 | Types | hands-on |
|---|---|---|
| Redis 8 vs KeyDB 压测 | tool-or-code + event | ✅ |
| LLM 公司限流新闻 | event | ❌ |
| 新产品发布想实测 | event | ✅ |
| 群里聊某实用工具(社交热议) | social + tool-or-code | ✅ |
| 某综艺 80 条集体吐槽 | social | ❌ |
| Rust vs Go 争论 | opinion | ❌ |
| prod OOM 复盘 | incident(+ opinion 若方案分歧) | ✅ |
| 团建/OKR | team-ops | ❌ |
| arxiv paper(有开源) | tool-or-code(+ event 若追溯引用) | ✅ |
| 单条提名 "superpowers 很好用" | tool-or-code | ✅ |
| 单条通知 "下周公司内 beta" | team-ops(通知型) | ❌ |

### 5.4 主题粒度

- **过粗**:"AI"、"数据库"、"编程" —— 拒绝,继续细分
- **过细**:"@张三 14:32 说 Redis 内存占用高" —— 合并到父主题
- **合适**:"Superpowers 试用"、"Redis 8 vs KeyDB 6 压测"、"下周公司内 beta 通知"

**上限由资源吃住:能跑完 verify / crosscheck 才立项**。主题数 > 20 → 合并 social / team-ops 呈现或削主题,不砍动手验证。

---

## Phase 6 — 子节点簇:按 Types 和 hands-on 标签派生

**原则**:`topic-X-investigation` 是编排节点,真正的工作在它派生的子节点簇里。命中的每个 Type 派生自己的簇(§6.1–6.6);标了 hands-on-available 额外派生 `verify-*` 簇(§6.7)。所有子节点职责单一、可并行/可重试、独立产出 result_summary。

### 6.1 `tc-*` —— tool-or-code(阅读分析,不含沙盒)

| 子节点 | 职责 |
|---|---|
| `tc-{X}-fetch` (必) | `git clone` / 拿 PR diff / 定位 commit / 下载论文 / 下载文档 |
| `tc-{X}-read` (必) | 读 README / 关键源码 / 论文方法,摘录定位、指标、竞品 |
| `tc-{X}-static-check` | clippy/ruff/mypy/tsc 等,留"工具/版本/结果"记录,全过也要写 |
| `tc-{X}-analyze` (必) | 综合 read + static-check,评估实现 |

沙盒跑代码/benchmark 的工作由 `verify-*` 簇(§6.7)承担,不在本簇内。

### 6.2 `event-*` —— 外部事件多源核实

| 子节点 | 职责 | 并行 |
|---|---|---|
| `event-{X}-source-official` (必) | 官方博客/release notes/官推/发言人声明 | ✅ |
| `event-{X}-source-code` (必) | GitHub release/commit/changelog | ✅ |
| `event-{X}-source-community` (必) | HN/Reddit/X/Discord 社区反应 | ✅ |
| `event-{X}-source-media` | 二级媒体 | ✅ |
| `event-{X}-source-counter` | 反方:质疑/否认/纠错 | ✅ |
| `event-{X}-crosscheck` (必) | 原始时间戳 + 措辞差异表,识别一致/冲突/缺失,给置信度 | 依赖所有 source |

**硬约束**(呼应产品硬承诺 2):至少 3 个独立 source;**不足 3 源时 `crosscheck` 节点不得省略**,result_summary 明写"单源低置信"+ 已尝试 source 列表。source 节点间**不读彼此 result**(避免污染);原文引用 ≥2000 字拆孙子文档(§7.0)。

### 6.3 `opinion-*` —— 多立场采集

| 子节点 | 职责 | 并行 |
|---|---|---|
| `opinion-{X}-stakeholder-{A/B/C}` | 每人一节点,采集**核心论据**(不是 slogan),不掺作者评价 | ✅ |
| `opinion-{X}-context-survey` | 历史背景/为何分歧 | ✅ |
| `opinion-{X}-synthesize` (必) | 立场矩阵 + 共识/分歧;**不**强判谁对 | 依赖前置 |

**硬约束**:至少 3 个 stakeholder 且代表**不同**立场。

### 6.4 `incident-*` —— 故障诊断

| 子节点 | 职责 |
|---|---|
| `incident-{X}-symptom-collect` (必) | 时间线 / 报错 message / 影响范围 / 触发条件 |
| `incident-{X}-hypothesis-generate` (必) | 根因假设按可能性排序 |
| `incident-{X}-root-cause-analysis` (必) | 指向具体代码/配置/外部因素 |
| `incident-{X}-fix-review` | 评估已讨论/已实施的修复方案 + 回归测试建议 |

沙盒复现由 `verify-*` 簇承担。无 hands-on 时 `root-cause-analysis` 必须标"未复现,基于推理"。

### 6.5 `team-ops-*` —— 团队内务记录(分两档,**不做**调研)

**判别**(看主题的消息数):
- ≤ 2 条 + 含时间锚点(如"下周公司内 beta"、"周五 3 点例会") → **通知型**
- ≥ 3 条多轮讨论(会议纪要 / 方案讨论) → **纪要型**

**通知型**(轻量):

| 子节点 | 职责 |
|---|---|
| `team-ops-{X}-log` (必) | 一句话产出 `{时间 / 事件 / 责任人或群体 / 影响范围}`;无待办直接写"仅通知,无行动项" |

**纪要型**(完整):

| 子节点 | 职责 |
|---|---|
| `team-ops-{X}-extract-facts` (必) | 时间/参与者/地点/主题/背景 |
| `team-ops-{X}-extract-decisions` (必) | who/what/when;无决策如实写"仅通知" |
| `team-ops-{X}-extract-action-items` (必) | todo,每条必含**责任人 + 期限**;无则写"无待办" |
| `team-ops-{X}-synthesize` (必) | 合成备忘 |

**红线**(两档通用):人事评价 / 薪酬 / 个人八卦一律**不进主报告**,result_summary 写"含敏感内容,已剔除"。**不做**外部核实、不做沙盒、不跨群聚合(除非明确跨组会议)。

### 6.6 `social-*` —— 集体热议情绪记录

**不做调研,只做主题 + 热度 + 情绪记录**。必须满足 §5.3 的 social 阈值才成主题。

| 子节点 | 职责 |
|---|---|
| `social-{X}-extract-theme` (必) | 具体主题(哪部剧/哪个明星公开事件/哪个游戏) |
| `social-{X}-gauge-heat` (必) | 消息数/人数/时长 + 代表性原文 2-3 条 |
| `social-{X}-synthesize` (必) | 一段话情绪/倾向总结;**不**多源核实、**不**价值判断 |

**红线**:只记公开娱乐内容;涉及具体同事/同行/第三方个人的八卦一律**不展开**,主报告仅一句"群内有 personal 讨论,已因隐私未展开"。在最终报告里**不单独 H1**,统一放末尾 `# 群内非工作热议` 段落下每主题一段。

### 6.7 `verify-*` —— 动手验证(跨 Type 通用,hands-on-available 触发)

和 Types 正交。无论命中哪类 Type,只要 hands-on-available,都可以按需派生此簇;由 Types 决定验证目标。

| 子节点 | 职责 |
|---|---|
| `verify-{X}-env-setup` (必) | Docker 镜像拉取 / VM / venv / 本地 script 准备;**禁主机安装** |
| `verify-{X}-design` (必) | 验证方案。**深度自适应**(呼应产品硬承诺 3):有基线/竞品 → 完整 benchmark;无基线/单条提名 → 最小试用(docker pull + quickstart + 截图 + 300-500 字纪要);环境不具备 → 降 read-only 并在主题文档开头写明原因。**无基线 ≠ 不跑** |
| `verify-{X}-run-primary` (必) | 主实验,采集原始数据 |
| `verify-{X}-run-competitor-{Y}` | 每竞品一节点跑同方案(适用于 benchmark 对比场景) |
| `verify-{X}-analyze-plot` (必) | 汇总分析;涉及性能/数值对比必须 matplotlib 绘图 |
| `verify-{X}-quality-check` (必,**独立 agent**) | **终节点**,独立审核上述 `verify-*` 产出是否真的做了事 |

**`quality-check` 是硬性要求,不可省略**。由一个**独立 agent**(fresh session,不读其他 verify-* 节点的 result_summary 原文,只按落盘产物审核)执行。

**检查清单**(每项产出 pass/fail + 证据,任一 fail 即整体 fail):

1. **代码真的写了**:`./scripts/` 下存在实验脚本,非空、不是占位 `TODO` / `pass`;diff 上看是可运行代码
2. **代码真的跑了**:工作目录下存在**执行日志**(`logs/*.log` 等),日志里有 timestamp、exit code、实际输出;Docker 场景下有 `docker ps/inspect` 足迹或容器 id
3. **拿到了执行结果数据**:`data/` 目录下存在 benchmark 原始数据文件(JSON/CSV),大小 > 0 且字段结构合理(非空数组、非占位 sample)
4. **图表真的绘制了**:`figs/*.png` 存在,打开能看到数据(非空白、非 placeholder);agent 应 `file figs/*.png` 确认是 PNG 且大小 > 5KB,并核对关键数字能和 `data/` 对上
5. **报告内容真实反映实验**:`./topics/{X}.md` 里 `## 性能实测` / `## 实验步骤` 的数字必须在 `data/` 原始文件里找得到 —— **禁止**编造 / 估算 / 凑数

#### 失败处理的核心原则:**暴露 > 降级**

**"没有暴露问题"比"错误降级"更不可接受**。quality-check 失败时,**默认行为是保留残缺的实验记录并显式标注问题**,而不是偷偷降级成 read-only 把实验这一段藏起来。

失败时的处理(按优先级):

1. **首选 — 修复后重跑**:quality-check fail 后,agent 先尝试补做(补代码 / 补执行 / 补数据 / 补图表),重跑 quality-check。这是最干净的路径
2. **次选 — 暴露失败,照常交付**:修复不了或成本太高 → **不降级为 read-only**,保留 `## 性能实测` / `## 实验步骤` 章节,但章节开头**必须**插入一段 `> ⚠️ 本节实验未通过质量验证`,列出具体失败的检查项(1-5 号哪项没过)+ 证据(缺失的 data 文件 / 空白 PNG / 数字对不上原始数据等)+ agent 做到了哪一步没做到的是什么。主报告的"事件速览表"对应行也要标 `⚠️ 实验不完整`
3. **仅在环境根本不具备时** — env-setup 因无 Docker/VM 彻底 fail、根本没法动手 → 此时才允许降级为 read-only,且必须在主题文档开头写一行"本主题初判 hands-on-available,但环境不具备(缺 Docker),已降级为纯阅读",**禁止**在 read-only 产物里混入 `## 性能实测` / `## 实验步骤` 章节

**严令禁止**:
- 删除 `## 性能实测` / `## 实验步骤` 章节,装作本就是 read-only 主题
- quality-check fail 后,只把失败细节写进 Kerrigan 节点 result_summary(那是工作底稿),而没同步到最终飞书文档给用户看
- 把失败的实验"包装"成看起来完整的数据交付

**飞书 API 交叉验证**同样适用:`env-setup` 准备 Python 环境 + 凭证,`design`/`run` 调 feishu client 拉数据核对;`quality-check` 审核是否真的调了 API、拿到真实响应、没用伪造数据。失败时同样走上面三档处理。

### 6.8 实验操作记录规范(`verify-*` 通用)

凡派生了 `verify-*` 节点,result_summary 和最终报告**必须**含两块:

**① 实验环境**:沙盒类型(Docker 镜像+tag / VM)/ 硬件(CPU/RAM/GPU/网络)/ 关键版本(主工具+依赖+数据集)/ 资源限制(`--cpus --memory` 等实际 flag)

**② 实验步骤**(5-10 步人读版):

```
1. docker pull xxx
2. docker run ... (关键 flag)
3. 装工具 / 准备数据 / 预热
4. 压测命令 (wrk -t4 -c100 -d30s http://...)
5. 采集指标 (log/prom/stdout 解析)
6. 竞品: 重复 2-5
7. matplotlib 读 JSON → bar chart
8. docker rm
```

**原则**:不贴完整脚本(放 `./scripts/`);参数选择必说明理由(为什么 -c 100 不 -c 1000);在主题详细文档用独立 `## 实验环境` / `## 实验步骤` 子标题呈现。

---

## Phase 7 — 报告合成与飞书落地

### 7.0 飞书文档的父子层级结构

**要求**:同一轮调研的所有产物,**必须挂成主文档的子文档**(Feishu 原生 parent-child 层级),不能是散落的独立文档。

最终用户在飞书 UI 看到的树:

```
📄 2026-04-18 群聊调研总览           ← 主文档
   │
   ├─ 📄 主题 1 详细调研               ← 子
   │     ├─ 📄 主题 1 · 原始聊天记录   ← 孙(**必建**,所有话题都要)
   │     └─ 📄 主题 1 · 实验日志全文   ← 孙(日志 ≥ 2000 字时才建)
   │
   ├─ 📄 主题 2 详细调研               ← 子
   │     └─ 📄 主题 2 · 原始聊天记录   ← 孙(**必建**)
   │
   ├─ 📄 群 A 聊天底稿                 ← 子(保留消息 ≥ 200 条时建)
   ├─ 📄 群 B 聊天底稿                 ← 子
   │
   └─ 📄 主题提取清单                  ← 子(主题 ≥ 5 时建)
```

**实现**:
- 用飞书 Wiki 的 node 层级(`create_wiki_node(parent_node_token=...)`)**或**飞书 Drive 的 folder 嵌套
- 具体 API 调用参考 `agent-feishu` 的 `rules/docx.md` / `openplatform-docs.md`,本 skill 只约束**层级关系**
- 所有子/孙文档在主文档正文里通过 inline link 呈现 —— **既是 UI 树父子关系,也是正文跳转**

**超长中间产物拆孙子文档**的触发条件(任一满足即拆):
- 过滤后的群聊原文 ≥ 3000 字
- 实验日志 / 完整命令输出 ≥ 2000 字
- 多源交叉验证逐源引用 ≥ 2000 字

**顺序(重要)**:
1. 先建**主文档空壳**(拿到 parent_token)
2. 再建所有子/孙文档(挂到 parent_token 下)
3. 回填每个子/孙文档 URL 到父层的 markdown
4. 最后转换父层 markdown 到飞书 blocks 写入

### 7.1 核心原则:编辑,不是堆砌

Stage 4a 的 `main-report-compose-md` 是"**编辑**"角色,不是"拼接"角色:

- 通读所有主题详细子文档 URL + result_summary
- **重写**每个主题章节:流畅、紧凑、读者导向
- 去除过程噪音(踩坑、失败尝试、节点调试信息)
- 保留关键结论 / 数据 / 来源标注
- 跨主题去重(同一链接/图片只保留最相关位置)

**禁止**:
- 直接拼各 topic 节点 result_summary
- 原样贴节点之间的 reminder / debug
- 输出 > 5000 字的"大杂烩"

### 7.2 两种 markdown 结构

本 skill 产出两份 markdown:一份主题详细文档(每个 topic 一份)+ 一份主报告。**章节组成、字段、长度约束**在本节以**规则列表**形式全部给出;具体 markdown 模板示例见本 skill 目录下的 `markdown-templates.md`(仅当你需要动手写 md 前想参考骨架时阅读,其中不含新规则)。

#### 7.2.a 主题详细文档 (`./topics/{X}.md`) 的必含章节

每个 `topic-X-investigation` 节点产出,**完整、详尽**。按主题类型选用章节,非该类型章节省略。

| 章节(H2) | 何时必有 | 内容约束 |
|---|---|---|
| H1 标题行 = `{主题名称}` | 始终 | 与 Stage 2 主题清单里的名称一致 |
| 元信息(**来源**/**Types**/**hands-on**) | 始终 | 来源列涉及群 + 起止时间;Types 列所有命中类型;hands-on 标签 |
| `## 讨论还原` | 始终 | 2-4 段,提炼讨论脉络,**不是** message 级堆砌 |
| `## 聊天原图` | Stage 1 OCR 有保留图片 | `![alt](./figs/...)` 本地相对路径 |
| `## 实验环境` | 派生了 `verify-*` 簇 | 沙盒类型 / 硬件 / 软件版本 / 资源限制(§6.8) |
| `## 实验步骤` | 同上 | 5-10 步简述;关键命令 + 参数选择理由;**禁贴完整脚本** |
| `## 性能实测` | `verify-*` 产出数值对比 | matplotlib PNG + 对比表格 |
| `## 多源交叉验证` | 命中 `event` | 各源原始时间戳 + 措辞差异表;冲突识别 |
| `## 立场矩阵` | 命中 `opinion` | stakeholder × 核心论据;共识 / 分歧 |
| `## 现象与根因` | 命中 `incident` | 时间线 / 报错 / 假设 → 根因(未复现时明确标注) |
| `## 会议纪要与行动项` | 命中 `team-ops` | 决策点 / 行动项(责任人+期限)|
| `## 情绪速记` | 命中 `social` | 主题 / 热度 / 代表性原文 2-3 条 |
| `## 原文索引` | **始终(必建)** | 列出所有相关消息:`message_id` / 群 / 时间戳 / @发言人 / **消息全文摘录**;末尾必须附 `→ [完整原文上下文](feishu_url)` 链接到本主题的「原始聊天记录」孙子文档 |

**原文上下文子文档(必建,无论消息多少)**:`topic-X-investigation` 必须为该主题产出一份**「原始聊天记录」孙子文档**,挂在本主题详细文档下。文档内容:该主题所有相关消息的完整原文(sender / time / message_id / 消息全文,按时间顺序排列)。字数 ≥ 3000 时内容不截断,继续全量展示;字数极短时同样建文档,哪怕只有 2-3 条消息。文档建好后 URL 写入 `## 原文索引` 章节末尾,且必须同步到 Stage 4a 的主报告对应主题条目中。

**超长内容处理**:任意章节实际内容超过阈值(实验日志 ≥ 2000 字 / 多源引用 ≥ 2000 字)时,拆出孙子文档,本章节只留摘要 + `[完整 X →](feishu_url)` 链接。(原文上下文子文档已强制建立,无需单独判断。)

#### 7.2.b 主报告 (`./main_report.md`) 的必含章节

Stage 4a 产出,**简洁、指路型**。

| 章节 | 内容约束 |
|---|---|
| H1 标题 = `群聊调研总览 — YYYY-MM-DD` | 当日日期(本地时区) |
| `## 概览` | 覆盖群列表 / 时间窗 / 主题数 / Types 分布(6 类各多少个)/ hands-on 比例;有前序期时加 `上一期: [日期](url)` 行(游标里的 `last_digest_doc_url`) |
| `## 本期通知与安排` | 专为 team-ops 通知型主题(§6.5)。每条一行:`日期 · 事件 · 来源群/发言人 · 影响范围`。无通知型则省略,**不进末尾"聊天参考"兜底**(通知是数据充分,不是数据不足)。 |
| 每个主题一个 H1 `# {主题名}` | **≤ 10 行**(不含 1 张可选关键图);必含:来源、类型、核心结论(1 句)、两条跳转链接:`→ **[详见主题 X 详细文档](feishu_url)**` 和 `→ **[原文上下文](feishu_url_raw)**`(指向该主题的「原始聊天记录」孙子文档) |
| 末尾 H1 `# 聊天参考(未深度调研)` | 列降级主题(数据不足 / 无沙盒 / 无法归类),每条一行写明降级原因 |

#### 7.2.c md 硬约束(两类文档都适用)

- 所有 `![alt](path)` 必须是工作目录下**相对路径**,PNG/JPG 放 `./figs/`,转换时统一上传
- **禁止**用网络 URL 引用图片
- 所有 `[text](https://xxx.feishu.cn/docx/...)` 必须保留为可点击 inline link(7.3 流水线会渲染成蓝色)
- 表格用标准 markdown `|...|`,复杂表格降级为 code block

> **需要完整 md 模板示例**?见 `markdown-templates.md`(同目录子文件)。该文件只是复现本节规则对应的骨架,不新增约束;改章节/字段/约束请改本节,不是那里。

### 7.3 Markdown → 飞书 的共享流水线

主题详细文档 (Stage 3 内部) 和主文档 (Stage 4b) **共用**此流水线:

```
输入: md 文件路径 + title + (可选) parent_token

1. 读 md 文件
2. document_id = agent_feishu.create_document(title, parent_token=parent_token)
3. 扫 md 中所有 ![alt](path):
     对每个本地 PNG/JPG:
         image_token = agent_feishu.upload_image(document_id, path)
         path → image_token 映射入 dict
4. 解析 md AST,逐节点转飞书 block:
     - # / ## / ### → heading1/2/3 block
     - 段落 → text block (含 inline 样式)
     - 表格 → table block;若 API 限制,降级为 code block
     - ![alt](path) → image block(image_token 来自步骤 3 的 dict)
     - [text](http://...) / [text](https://...) → text block 内的 inline link
        * feishu.cn/docx/ 链接也走这条路径
        * 渲染为可点击蓝色样式
     - `code` / ```fenced``` → 对应 code block
     - - / 1. 列表 → bullet/ordered list block
5. create_nested_blocks(document_id, blocks)
6. 返回文档 URL

输出: 飞书文档 URL
```

**注意**:
- Inline link 必须保留,不能降级为纯 URL 文本
- 不识别的 block(如深度嵌套 admonition)→ 降级为纯文本 block,**不要 fail**
- result_summary 含:文档 URL / 上传图片数 / inline link 数 / 转换告警

### 7.4 中间产物飞书化总表

| 产物 | 挂在哪 | 触发条件 |
|---|---|---|
| 每个 topic 详细调研 | 主文档的**子** | **必发** |
| 主题的原始聊天记录 | 主题文档的**孙** | **必建**(无消息数下限) |
| 主题的实验日志 | 主题文档的**孙** | 日志 ≥ 2000 字 |
| 主题的多源原文引用 | 主题文档的**孙** | 引用 ≥ 2000 字 |
| 群 A / B / C 聊天底稿 | 主文档的**子** | 单群保留消息 ≥ 200 条 |
| 主题提取清单 | 主文档的**子** | 主题 ≥ 5 个 |
| `main_report.md` | 不单独发 | 4b 转换后即主文档 |

最终用户只需一个**主文档 URL** 即可下钻到全部细节。

### 7.5 为什么 4a 和 4b 拆成两个节点

- **关注点分离**:4a 是内容编辑(LLM),4b 是格式转换(工程)
- **可单独重试**:md 写坏了单独重跑 4a;图片/链接传失败单独重跑 4b
- **可审计**:`main_report.md` 落盘,用户可先审 md 再决定发飞书

### 7.6 主题降级

- **禁止**把机器人没验证过的聊天结论直接作为 H1 主题
- 没走完调研协议的主题(数据不足 / 无沙盒 / 类型无法判定)归到最末 H1 **`# 聊天参考(未深度调研)`**,列主题名 + 来源 + 降级原因
- 降级主题**不发独立详细子文档**

---

## Phase 8 — 常见陷阱

1. **时区**:Feishu `create_time` 是秒级 Unix 时间戳,用户说"今天下午"必须先 `date` 确认本地时区再换算。写进 Stage 1 节点 description,别让 agent 猜。

2. **权限**:bot 必须在群里才能 `list_messages`。Stage 1 节点最常见失败原因是 bot 没被拉进群 —— 这种 failure_reason 要写明"请先把 bot 拉入 {chat_id}",别让下游空等。

3. **thread 消息**:群开了主题功能(`thread_id` 非空),同主题消息在 Stage 1 要**聚拢**,不按时间线打散。

4. **图片 token 过期**:老消息(> 30 天)的 `image_key` 可能失效。Stage 1 下载时识别并标记,不要派 OCR 节点白跑。

5. **过滤过激**:Stage 1 result_summary 里留 **3 条被过滤消息样本**,供用户对 Phase 4 规则微调。保留 `discarded/{X}.json` 便于回溯。

6. **主题粒度失控**:Stage 2 产出 > 20 个主题 → 规则没 work,agent 应该合并细粒度主题。产出 < 2 个 → 原始信号稀薄,在 result 里标"可能无需本轮调研"。

7. **数消息条数不看指向性**(最易犯):单条提名的工具(repo URL + 体感)、单条时间锚点通知("下周公司内 beta")都应**立项为主题**(§5.2)。看条数不看指向性 → 典型误杀,直接砸产品核心卖点。

8. **个人状态误杀事务通知**:含时间范围的"请假 / 休假 / 下周不在"一律保留(§4.1);只有纯"到了 / 下班了"才丢。时间锚点 + 事务影响 压倒"个人状态"判定。

9. **信息聚合退化(产品红线)**:主题数多或单条信号弱时偷懒跳过 `verify-*` 或 event 3 源 crosscheck 只做摘要汇总 —— 直接违反产品硬承诺。主题可以多、可以浅,但 tool-or-code 必须动手、event 必须多源。资源不够 → 削主题数,不砍动手。

7. **Docker 不可用**:`verify-*` 簇硬依赖沙盒。检测到无 Docker 时**整个主题自动降级为 read-only**(已产出节点保留,主报告标"动手尝试失败"),**不允许**绕过沙盒到主机运行 —— 这是保护用户环境的红线。
8. **team-ops 敏感信息**:人事评价、薪酬、个人八卦**禁止**进主报告或子文档,只在 result_summary 写"含敏感内容已剔除"。依赖飞书文档的权限控制也不算充分,源头就不写。
9. **social 阈值滥用**:agent 容易把少量娱乐对话强行当 social 主题。只有同时满足 §5.3 三条(≥20 条 + 时长/占比 + ≥3 人)才成主题,否则按 §4.1 丢弃。
10. **"写了代码就当跑了"**:常见失败模式是 agent 派生了 `verify-run-primary` 但只写了脚本没真正执行,或只贴了一张占位 PNG。**必须**有独立 `verify-quality-check` 按 §6.7 的 5 项清单审核。
11. **暴露 > 降级 —— 不准藏问题**:quality-check fail 后,**默认保留实验章节 + 显式 `⚠️` 标注失败环节**,让用户看见 agent 动手了但没做完。**严令禁止**偷偷删掉实验章节、装成本就是 read-only —— 这种"看起来完整"的假交付比老实认错更严重。只有环境根本不具备(无 Docker/VM)时才允许真正降级,且必须在文档开头写明降级原因。

8. **原文引用丢失**:Stage 3 主题详细文档要求标"群 + 时间 + message_id",Stage 1 必须在 raw JSON 里**完整保留** `message_id / sender / create_time / thread_id`;漏任何一项,下游无法溯源。

9. **父子层级断裂**:7.0 顺序 —— 主文档必须**最先**建,拿到 parent_token 再建子孙;反过来会让子文档悬挂,UI 树破。

10. **inline link 被渲染成纯文本**:Stage 4b 转换主文档时,`[text](feishu_url)` 必须保留蓝色可点击 —— 如果看到文档里链接是黑字,立即重跑 4b 并在 result 标告警。

11. **中间 md 被删**:`main_report.md` 和 `./topics/*.md` 是可审计的中间产物,**别在交付后清理掉**。

12. **增量追加**:用户看完想"再加一个群"/"延长一天" → 直接 `kerrigan add` 新的 `group-*-analysis`,它完成后会级联触发 Stage 2 重建(Stage 2 节点可重置为 pending 再 ready),Stage 3/4 按主题变化动态增减。

---

## Appendix — 完整 DAG 依赖图(示例)

**场景**:用户要总结群 A / 群 B 今天下午的聊天,Stage 2 提取出主题 "Redis 8 vs KeyDB 6"(工具+新闻)和"某 arxiv paper 复现"(研究+代码)。

```
                              Stage 1 (并行)
         ┌─────────────────────┬──────────────────────────┐
         ▼                     ▼                          │
  group-A-analysis       group-B-analysis                 │
         │                     │                          │
         └──────────┬──────────┘                          │
                    ▼                                     │
              Stage 2 (单节点)                            │
       cross-group-topic-extraction                       │
                    │                                     │
         ┌──────────┴──────────┐                          │
         ▼                     ▼                          │
   Stage 3 (动态派生, 每主题 1 编排节点)                   │
   topic-redis8-investigation   topic-arxivpaper-inv.     │
   Types: tool-or-code+event    Types: tool-or-code       │
   hands-on: available          hands-on: available       │
         │                            │                    │
         ▼                            ▼                    │
   ┌──────────────────────┐   ┌────────────────────────────┐
   │ tc-redis8-fetch      │   │ tc-paper-fetch             │
   │ tc-redis8-read       │   │ tc-paper-read              │
   │ tc-redis8-static-ck  │   │ tc-paper-static-check      │
   │ tc-redis8-analyze    │   │ tc-paper-analyze           │
   │                      │   │                            │
   │ event-redis8-source- │   │ verify-paper-env-setup     │
   │   official           │   │ verify-paper-design        │
   │ event-redis8-source- │   │ verify-paper-run-primary   │
   │   code               │   │ verify-paper-analyze-plot  │
   │ event-redis8-source- │   └────────────┬───────────────┘
   │   community          │                │
   │ event-redis8-        │                │
   │   crosscheck         │                │
   │                      │                │
   │ verify-redis8-       │                │
   │   env-setup          │                │
   │ verify-redis8-design │                │
   │ verify-redis8-run-   │                │
   │   primary            │                │
   │ verify-redis8-run-   │                │
   │   competitor-keydb   │                │
   │ verify-redis8-       │                │
   │   analyze-plot       │                │
   └──────────┬───────────┘                │
              │ (编排节点汇总,建主题详细文档) │
              ▼                             ▼
         topic-redis8 详细子文档       topic-arxivpaper 详细子文档
              │                             │
              └──────────┬──────────────────┘
                         ▼
                    Stage 4a (单节点)
              main-report-compose-md
                         │
                         ▼
                    Stage 4b (单节点)
              main-md-to-feishu
                         │
                         ▼
               主飞书文档 URL (含 inline link 到所有子文档)
```

**派生时机**:
- 建 task 时只建 `group-A-analysis` 和 `group-B-analysis` 两个初始节点
- Stage 1 跑完 → 建 `cross-group-topic-extraction`
- Stage 2 跑完 → 建每个主题的 `topic-X-investigation`(编排节点)
- 每个 `topic-X-investigation` 开跑 → 按 Types 派生对应簇(§6.1–6.6);hands-on-available 时额外派生 `verify-*` 簇(§6.7)
- 所有 `topic-*-investigation` 完成 → 建 `main-report-compose-md` + `main-md-to-feishu`

这是典型的"execute first, decompose later"—— 每一轮派生都基于前一轮的实际产出,**绝不提前规划超过两层依赖**。
