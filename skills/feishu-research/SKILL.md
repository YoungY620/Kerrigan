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

用 Kerrigan DAG 把"帮我调研一下最近几个群的聊天内容"这类请求,跑成一条**多群并行分析 → 跨群话题提取 → 按调研类型派生子任务簇 → 飞书父子文档网络**的可执行任务链。

> **前置依赖**:`kerrigan` skill 是本 skill 的前置依赖 —— 开始执行本 skill 前**必须先完整阅读** `kerrigan` skill 的所有指导文件。本 skill 假设你已经掌握 Kerrigan 的执行流程、节点粒度、依赖原则和通用调研方法论,不再重复这些内容,只讲"多群聊调研"这个场景如何落到 Kerrigan DAG 上。

---

## Scope

**一律使用本 skill** 的情形:用户要求对一个或多个飞书群的消息做调研、摘要、报告、复盘 —— 无论是**一次性请求**("帮我总结一下今天下午")还是**周期性任务**("每天早 8 点 / 每周一给我一份")。**没有"量小就跳过 Kerrigan"的例外**,即使只有 1 个群、50 条消息也走完整 DAG,因为:

1. 保证输出格式一致(飞书文档父子层级)
2. 保证调研深度一致(按类型走协议)
3. 增量追加(补群、延长时间、每日累积)天然支持

### 周期性执行是一等公民(非例外)

每日 / 每周定时总结是本 skill 的**主场景**之一,不是被排除的特殊情况。周期性任务的接入方式:

- 由 **Kitty 的 `SetReminder`**(cron 表达式)按时触发,每次触发都拉起一个新的 Kerrigan task 跑本 skill 的完整四阶段
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
│  输出: 该群摘要 + 过滤后原文 + 链接表 + 图片 OCR │
└─────────────────┬─────────────────────────────┘
                  ▼
┌──── Stage 2 · 跨群话题提取 (单节点) ─────────┐
│  cross-group-topic-extraction                 │
│  依赖: 所有 Stage 1 节点                      │
│  输出: 话题清单 (名称 / 涉及群 / 时间跨度 /    │
│         message_id 索引 / 初判调研类型)       │
└─────────────────┬─────────────────────────────┘
                  ▼
┌── Stage 3 · 每话题独立调研 (动态派生) ────────┐
│  topic-{X}-investigation   ← 编排节点         │
│    内部按类型派生"**一簇**"子节点 (见 Phase 6) │
│    完成后发布该话题的**飞书子文档**, 回传 URL  │
└─────────────────┬─────────────────────────────┘
                  ▼
┌── Stage 4a · 主报告撰写 (单节点) ─────────────┐
│  main-report-compose-md                       │
│  读所有 topic 的详细文档 URL,**编辑**合成主 md │
│  每话题: 摘要 + 核心结论 + 一张关键图 + 跳转链接│
└─────────────────┬─────────────────────────────┘
                  ▼
┌── Stage 4b · Markdown → 飞书主文档 ───────────┐
│  main-md-to-feishu                            │
│  建主文档 (标题=日期) → 上传图片 → 转 block   │
│   → 保留 inline link → create_nested_blocks   │
│  输出: 主文档 URL (给用户的唯一入口)          │
└───────────────────────────────────────────────┘
```

**Stage 3 的动态派生**是核心:Kerrigan 执行 Stage 2 **之前不知道话题是什么**,Stage 2 产出话题清单后,agent 才能根据清单给每个话题 `kerrigan add` 一个 `topic-X-investigation` 节点;这些节点内部再按调研类型继续派生子节点簇。这是 `guide.md` "Execute First, Decompose Later" 原则的典型应用。

---

## Phase 2 — Stage-by-Stage 节点设计

### Stage 1 · `group-{X}-analysis`

**summary**: `Analyze Feishu group {X} in time range [T0, T1]: filter chitchat, extract tech discussions, OCR images`

**description 必含**:
- **Objective**:对群 `{chat_id}` 在 `[T0, T1]` 区间内所有消息,产出(a) 保留 message_id/时间/sender 的**过滤后原文** `./raw/{X}.json`,(b) 该群一句话摘要,(c) 链接清单 `./links/{X}.json`,(d) 图片本地路径 + OCR 结果 `./ocr/{X}.json`
- **Context**:无前置节点
- **Scope**:
  - 用 `kitty_feishu` 的 `list_messages(chat_id, start_time, end_time, count=200)`,分页取完
  - 时区换算:先 `date` 确认本地时区,再换算成秒级 Unix 时间戳
  - 过滤规则见 Phase 4
  - 图片下载走 `/im/v1/images/{image_key}`,存 `./figs/{X}-{message_id}.{ext}`
- **Constraints**:
  - 不跨群读数据(独立性)
  - 不做话题识别(留给 Stage 2)
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
- **Objective**:通读所有 Stage 1 的 `./raw/*.json`,产出话题清单 `./topics.json`
- **Context**:依赖所有 `group-*-analysis`(读它们的 result_summary + raw/links/ocr 三类产出)
- **每个话题**必须含:
  - `name`:短且具体(非"AI"、"编程"这种宽泛词)
  - `涉及群`:至少 1 个,跨群讨论时列多个
  - `时间跨度`:每个涉及群的起止时间
  - `message_id 索引`:指向该话题最代表性的 3-10 条消息
  - `初判调研类型`:从 **tool / code / news / opinion / research** 中选 1 个或多个
  - `简要讨论摘要`:2-3 句
- **粒度**见 Phase 5
- **Output (result_summary)**:话题数量 / 每话题一行概要 / 清单文件路径
- 话题 ≥ 5 个时,本节点**发一份"话题提取清单"飞书子文档**作为主文档的子文档

**为什么单节点**:跨群聚类需要全局视角,无法并行。

### Stage 3 · `topic-{X}-investigation`(编排节点)

**summary**: `Investigate topic "{X}" with per-type subtask clusters, then publish Feishu sub-doc`

**description 必含**:
- **Objective**:对话题 `{X}` 产出(a) `./topics/{X}.md` 详细 markdown,(b) 飞书详细子文档(挂在主文档 parent_token 下)
- **Context**:依赖 `cross-group-topic-extraction` + 话题涉及的 `group-*-analysis` 节点
- **执行步骤**:
  1. 读 Stage 2 给出的初判类型(tool / code / news / opinion / research)
  2. **对每个类型,按 Phase 6 对应协议派生一簇子节点**(`kerrigan add`)
  3. 等所有子节点完成(失败降级策略见 7.6)
  4. 读所有子节点 result_summary,合成 `./topics/{X}.md`(结构见 7.2.a)
  5. 调用 7.3 的"md → 飞书"流水线建立**话题详细子文档**(parent_token = 主文档 token)
  6. 若原文/日志超长,按 7.0 触发条件继续派生孙子文档
- **Output (result_summary)**:话题详细文档 URL / 所有孙子文档 URL / 每个子节点的完成状态

**关键**:`topic-X-investigation` 本身**不做**具体调研工作,只编排子节点簇。真正的实验、采集、绘图都在子节点里。

### Stage 4a · `main-report-compose-md`

**summary**: `Compose the editorial main report markdown from all topic investigations`

**description 必含**:
- **Objective**:产出 `./main_report.md`(结构见 7.2.b)
- **Context**:依赖所有 `topic-*-investigation`
- **执行步骤**:
  1. 读每个 topic 节点的 result_summary,拿到话题详细子文档 URL
  2. **重新撰写**每个话题章节(见 7.1),**禁止直接拼接** result_summary
  3. 选 1-2 张最有代表性的图片(matplotlib 关键对比图 / 聊天信号最强的原图)放主报告
  4. 每话题章节末尾加 `→ [详见话题 X 详细文档](feishu_url)` 链接
- **Output (result_summary)**:`main_report.md` 路径 / 话题数量 / 引用的子文档 URL 列表

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
- **Stage 2 聚合**:话题是**涌现**的,不是消息级的。必须全局扫一遍才能发现"群 A 下午在聊 Redis 8,群 B 晚上也在聊"——这种跨群共性只有全量视角才看得见
- **Stage 3 深度**:聊天本身是线索,不是结论。话题识别后必须按**调研类型**走协议,让结论可追溯、可复现、可挑战
- **Stage 4 编辑**:调研过程的 result_summary 是工作底稿,不是读者想看的东西。必须有一个"编辑"角色通读所有话题再重写

### 3.2 依赖关系 = 信息需求

严格按 `guide.md` 的"依赖 = 信息需求"原则:
- ✅ `topic-X-investigation` 依赖涉及的 `group-*-analysis`(需要 raw message 引用)
- ✅ `news-{X}-crosscheck` 依赖所有 `news-{X}-source-*`(需要综合比对)
- ❌ `group-A-analysis` 不依赖 `group-B-analysis`(无信息流)
- ❌ `news-source-official` 不依赖 `news-source-community`(保独立,避免污染)

### 3.3 动态派生的触发条件

**第二轮派生(Stage 2 完成后)**:Stage 2 输出话题清单,agent 立刻为每个话题 `kerrigan add` 一个 `topic-X-investigation`,依赖列表 = `[cross-group-topic-extraction, 涉及的 group-*-analysis]`

**第三轮派生(Stage 3 编排节点启动后)**:agent 读话题元数据中的"初判调研类型",对每个类型调用 Phase 6 的对应模板派生一簇子节点。这一轮派生**完全在 topic-X-investigation 内部执行**,不影响其他话题。

**禁止提前派生**:不要在 Stage 1 还没跑完时就猜测话题、预建 Stage 3 节点。Kerrigan 能力是动态扩展,不是预测性规划。

### 3.4 小规模探测(先试水后放量)

对应 `guide.md` "Execute First, Decompose Later":

- 如果用户一次要求调研 10 个群,**第一轮只建 1-2 个 `group-*-analysis` 节点作为 pilot**,跑通整个 Stage 1 链路
- Pilot 产出验证:权限 OK、过滤质量可接受、图片下载成功、OCR 可用
- 通过后再批量建其余群的节点

---

## Phase 4 — Stage 1 的过滤与提取规则

### 4.1 闲聊识别(丢弃)

满足以下任一即判为闲聊,丢弃:
- 纯表情、贴图、"哈哈哈"、"+1"、"收到"、"@xxx 在吗"
- 餐饮/天气/通勤/吐槽(不涉及任何技术或决策)
- 对他人消息的单纯附和(无新增信息)
- 个人状态("请假"、"到了"、"下班")
- 重复的"早上好""晚安"

**保留边界**:带情绪但含信号的要留 —— "这个 API 又崩了,已经是这周第三次"是稳定性抱怨,有信号。

### 4.2 技术讨论识别(强制保留)

识别信号(满足任一即保留):
- 出现具体技术名词(框架、库、服务、协议、工具命令、版本号)
- 两人及以上就同一话题交替发言 ≥ 3 轮
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

### 4.4 图片 OCR

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

## Phase 5 — Stage 2 的话题提取规则

### 5.1 跨群聚类

- 用**语义相似度**聚类,不要只看字面关键词(Redis 和 KeyDB 的讨论要聚到一起)
- 同群同话题的连续消息聚为一条讨论段;跨群的相似讨论合并为一个话题,但各群时间跨度分别列出
- 单条"随口一提"不够成话题 —— **话题至少要有 3 条以上的消息支撑**,或一张有信息量的图片

### 5.2 调研类型初判

按以下规则初判 `调研类型`(可多选):

| 信号 | 归入类型 |
|---|---|
| 提及具体工具名 + repo/官网 + 使用场景 | tool |
| 提及代码片段、PR、commit、具体函数/模块实现 | code |
| 提及发布事件、版本更新、公司动态、价格/政策变化 | news |
| 参与者就选型 / 趋势 / 价值判断有不同立场 | opinion |
| 提及论文名、arxiv ID、学术概念、算法 | research |

**多类型叠加**常见:Redis 8 发布 = tool + news;某 paper 开源实现评测 = research + code + tool。在 Stage 3 要**同时**跑所有命中类型的协议。

### 5.3 话题粒度

- **过粗**:"AI"、"数据库"、"编程" —— 拒绝,继续细分
- **过细**:"@张三 14:32 说 Redis 内存占用高" —— 合并到父话题
- **合适**:"Redis 8 vs KeyDB 6 的吞吐对比"、"Claude 4.7 的 1M 上下文使用体验"、"PyTorch 3.0 release notes 讨论"

一个调研任务输出 **3-15 个**话题是合理区间。超 15 说明粒度过细,少于 3 说明原始信号稀薄。

---

## Phase 6 — 五种调研类型:每类**派生一簇子任务节点**

**关键原则**:归入某类型时,派生的**不是一个"xxx 类调研"单点**,而是**一簇细粒度子节点**。每个子节点职责单一、可并行/可重试、独立产出 result_summary。

`topic-X-investigation` 本身是**编排节点**,真正的工作都在它派生的子节点簇里。

### 6.1 工具类(tool)

**归入条件**:话题提及具体工具/框架/服务,有 repo / 官网 / 安装步骤。

**子节点簇**:

| 子节点 | 职责 | 依赖 |
|---|---|---|
| `tool-{X}-read-repo` (必) | 读 README + 主要文档,提取定位、主打指标、声明的竞品列表 | — |
| `tool-{X}-sandbox-setup` (必) | Docker 镜像拉取、容器启动、依赖安装、冒烟测试 | read-repo |
| `tool-{X}-benchmark-design` (必) | 设计压测方案:工作负载、并发参数、测量周期、采集方式;参数选择理由 | read-repo |
| `tool-{X}-benchmark-run-primary` (必) | 对主角工具压测,采集原始数据(JSON/CSV) | sandbox-setup + benchmark-design |
| `tool-{X}-benchmark-run-competitor-{Y}` (每个竞品 1 个) | 对每个竞品跑**同样方案**,独立节点便于并行 + 重试 | sandbox-setup + benchmark-design |
| `tool-{X}-analyze-plot` (必) | 汇总所有 benchmark 原始数据,matplotlib 绘图,写对比分析 | 所有 benchmark-run-* |

**必须产出的数据**:所用镜像/硬件/版本(环境)、关键命令(步骤)、原始数据文件路径、matplotlib PNG 路径(见 6.6)。

**退化**:
- 无 Docker/VM → `sandbox-setup` 直接 fail,后续级联 fail(**禁止**主机安装)
- 无 matplotlib 环境 → 沙盒内 `pip install`,仍禁污染主机
- benchmark 日志 ≥ 2000 字 → 按 7.0 规则拆孙子文档

### 6.2 代码类(code)

**归入条件**:话题讨论具体代码片段、PR、commit、实现技巧、具体函数/模块。

**子节点簇**:

| 子节点 | 职责 | 依赖 |
|---|---|---|
| `code-{X}-fetch-sources` (必) | `git clone` / 拿 PR diff / 定位 commit,列出要分析的文件路径 | — |
| `code-{X}-read-sources` (必) | 阅读关键代码,摘录核心逻辑和设计意图 | fetch |
| `code-{X}-test-requirements` | 分析要验证讨论中的技术声明,需要覆盖哪些 case / 输入 / 边界 | read |
| `code-{X}-develop-repro` | 沙盒内开发复现脚本 / benchmark / 测试用例 | test-requirements |
| `code-{X}-static-check` | 对原代码 + 复现代码跑 static analysis(clippy/ruff/mypy/tsc 等) | fetch + develop-repro |
| `code-{X}-run-repro` | 沙盒里跑复现/测试,收集结果(含失败用例) | develop-repro |
| `code-{X}-analyze` | 综合 static-check + run-repro,评估实现;有数值对比则 matplotlib 绘图 | 所有前置 |

**规则**:
- 话题只是"聊某段代码写得好"**没有技术声明要验证** → 退化为只跑 fetch + read + analyze
- 话题是"讨论某 PR 有 bug" → 必须跑到 `run-repro`,尝试触发
- static-check 即使全过也要留记录"运行了什么工具 / 版本 / 结果"

### 6.3 新闻类(news)

**归入条件**:时效性强的事件、版本发布、公司动态、价格/政策变化。

**子节点簇**(每个来源独立节点,**不互相读 result**):

| 子节点 | 职责 | 并行 |
|---|---|---|
| `news-{X}-source-official` (必) | 官方博客 / release notes / 官推 / 新闻发言人声明 | ✅ |
| `news-{X}-source-code` (必) | GitHub release / commit history / changelog | ✅ |
| `news-{X}-source-community` (必) | HN / Reddit / Twitter / Discord 社区反应 | ✅ |
| `news-{X}-source-media` | 二级媒体(TechCrunch / The Verge / 行业自媒体) | ✅ |
| `news-{X}-source-counter` | **反方专采**:质疑、否认、纠错的声音 | ✅ |
| `news-{X}-crosscheck` (必) | 读所有 source-* 的 result,识别一致/冲突/缺失,给置信度 | 依赖所有 source-* |

**硬性要求**:
- 至少 **3** 个独立 source 节点,否则 crosscheck 在 result 里标"单源,低置信"
- source 节点之间**不读彼此 result**(避免污染)—— 这一条违反 `guide.md` 的"依赖=信息需求"默认,但在新闻调研里是必要隔离
- crosscheck 必须给**原始时间戳 + 措辞差异**的表格,不能只给结论
- 原文引用 ≥ 2000 字 → 按 7.0 拆孙子文档

### 6.4 观点类(opinion)

**归入条件**:价值判断、技术选型争论、趋势预测。

**子节点簇**:

| 子节点 | 职责 | 并行 |
|---|---|---|
| `opinion-{X}-stakeholder-{人物 A}` | 采集立场 A 代表人物的公开表达,还原**核心论据**(不是 slogan) | ✅ |
| `opinion-{X}-stakeholder-{人物 B}` | 立场 B 代表人物 | ✅ |
| `opinion-{X}-stakeholder-{人物 C}` | 立场 C 代表人物 | ✅ |
| `opinion-{X}-context-survey` | 历史背景 / 前情 / 为何会有分歧 | ✅ |
| `opinion-{X}-synthesize` (必) | 读所有 stakeholder + context,做立场矩阵 + 共识/分歧识别 | 所有前置 |

**硬性要求**:
- 至少 **3** 个 stakeholder,代表**不同立场**(不是同立场的 3 人)
- 每个 stakeholder 节点**只采一人/一方**,不掺作者评价
- synthesize **不**强行得出"谁对",只做立场矩阵 + 论据点评

### 6.5 研究类(research)

**归入条件**:论文、实验、学术概念、算法。

**子节点簇**:

| 子节点 | 职责 | 依赖 |
|---|---|---|
| `research-{X}-paper-fetch` (必) | arxiv / 会议 / tech report 找原文,下载 PDF,摘录 abstract | — |
| `research-{X}-methodology-review` (必) | 核查方法:数据集 / 基线 / 指标 / 可复现声明 / 局限 | paper-fetch |
| `research-{X}-citation-trace` (必) | 后续引用追踪:是否有工作证实/推翻 | paper-fetch |
| `research-{X}-repro-design` | 若有官方代码,设计复现方案 | methodology-review |
| `research-{X}-repro-run` | 沙盒里跑复现,采集数据(环境/步骤同 6.6) | repro-design |
| `research-{X}-synthesize` (必) | 综合所有前置,给结论 | 所有前置 |

**规则**:
- 无代码可复现 → 跳过 repro-design / repro-run
- 复现涉及指标对比(paper claim vs 复现值 vs baseline) → matplotlib 绘图是硬要求
- 引用追踪 < 5 篇 → result 里标"引用不足,证据有限"

### 6.6 实验操作的记录规范(工具/代码/研究类通用)

凡是通过 Kerrigan 节点**实际下载 / 安装 / 运行 / 测试**的场景,result_summary 和最终报告**必须**包含两块:

#### ① 实验环境

- **沙盒类型**:Docker 镜像 + tag(如 `ubuntu:24.04`)/ VM 型号
- **硬件**:CPU(如 `Apple M2 Pro`)/ RAM / GPU(如有)/ 网络(带宽敏感测试)
- **关键软件版本**:主工具 + 依赖 + 数据集版本(如 `redis 8.0.0` / `wrk 4.2.0`)
- **资源限制**:实际 `docker run --cpus 4 --memory 8g` 等 flag

#### ② 实验步骤简述(5-10 步,人读的那种)

```
1. 拉镜像: docker pull xxx
2. 启动容器: docker run ... (关键 flag 写出)
3. 安装工具: apt-get install ... / cargo build ...
4. 准备数据: 下载 dataset / 生成 fixture
5. 预热: 先跑 N 秒让缓存稳定
6. 压测: wrk -t4 -c100 -d30s http://...
7. 采集: 从 log / prometheus / stdout 解析
8. 竞品: 重复步骤 3-7
9. 绘图: matplotlib 读 JSON → bar chart
10. 清理: docker rm / docker rmi
```

**原则**:
- **不贴完整脚本**(太长),只写关键命令 + 参数选择理由
- 复杂 fixture / bench 脚本另存 `./scripts/`,步骤里指向它
- **参数选择必说明**:为什么 `-c 100` 不 `-c 1000`?为什么测 30s 不 60s?
- 在话题详细文档里用独立 `## 实验环境` / `## 实验步骤` 子标题呈现

---

## Phase 7 — 报告合成与飞书落地

### 7.0 飞书文档的父子层级结构

**要求**:同一轮调研的所有产物,**必须挂成主文档的子文档**(Feishu 原生 parent-child 层级),不能是散落的独立文档。

最终用户在飞书 UI 看到的树:

```
📄 2026-04-18 群聊调研总览           ← 主文档
   │
   ├─ 📄 话题 1 详细调研               ← 子
   │     ├─ 📄 话题 1 · 原始聊天记录   ← 孙(原文 ≥ 3000 字时才建)
   │     └─ 📄 话题 1 · 实验日志全文   ← 孙(日志 ≥ 2000 字时才建)
   │
   ├─ 📄 话题 2 详细调研               ← 子
   │
   ├─ 📄 群 A 聊天底稿                 ← 子(保留消息 ≥ 200 条时建)
   ├─ 📄 群 B 聊天底稿                 ← 子
   │
   └─ 📄 话题提取清单                  ← 子(话题 ≥ 5 时建)
```

**实现**:
- 用飞书 Wiki 的 node 层级(`create_wiki_node(parent_node_token=...)`)**或**飞书 Drive 的 folder 嵌套
- 具体 API 调用参考 `kitty-feishu` 的 `rules/docx.md` / `openplatform-docs.md`,本 skill 只约束**层级关系**
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

- 通读所有话题详细子文档 URL + result_summary
- **重写**每个话题章节:流畅、紧凑、读者导向
- 去除过程噪音(踩坑、失败尝试、节点调试信息)
- 保留关键结论 / 数据 / 来源标注
- 跨话题去重(同一链接/图片只保留最相关位置)

**禁止**:
- 直接拼各 topic 节点 result_summary
- 原样贴节点之间的 reminder / debug
- 输出 > 5000 字的"大杂烩"

### 7.2 两种 markdown 结构

本 skill 产出两份 markdown:一份话题详细文档(每个 topic 一份)+ 一份主报告。**章节组成、字段、长度约束**在本节以**规则列表**形式全部给出;具体 markdown 模板示例见本 skill 目录下的 `markdown-templates.md`(仅当你需要动手写 md 前想参考骨架时阅读,其中不含新规则)。

#### 7.2.a 话题详细文档 (`./topics/{X}.md`) 的必含章节

每个 `topic-X-investigation` 节点产出,**完整、详尽**。按话题类型选用章节,非该类型章节省略。

| 章节(H2) | 何时必有 | 内容约束 |
|---|---|---|
| H1 标题行 = `{话题名称}` | 始终 | 与 Stage 2 话题清单里的名称一致 |
| 元信息(**来源**/**类型**) | 始终 | 来源列涉及群 + 该群的起止时间;类型列所有命中的调研类型 |
| `## 讨论还原` | 始终 | 2-4 段,提炼讨论脉络,**不是** message 级堆砌 |
| `## 聊天原图` | 该话题在 Stage 1 OCR 里有保留图片 | 用 `![alt](./figs/...)` 本地相对路径嵌入所有保留的图 |
| `## 实验环境` | 触发了 Phase 6.6(工具/代码/研究类跑了沙盒实验) | 必含字段:沙盒类型 / 硬件 / 软件版本 / 资源限制 |
| `## 实验步骤` | 同上 | 5-10 步简述;关键命令 + 参数选择理由;**禁贴完整脚本** |
| `## 性能实测` | 工具类或有性能数据的代码/研究类 | matplotlib PNG 本地引用 + 对比表格 |
| `## 多源交叉验证` | 新闻类 | 各源原始时间戳 + 措辞差异表;冲突识别 |
| `## 立场矩阵` | 观点类 | stakeholder × 核心论据;共识 / 分歧 |
| `## 方法学与引用` | 研究类 | 论文方法核查 + 后续引用追踪 |
| `## 原文索引` | 始终 | `message_id` / 群 / 时间戳 / @发言人 的索引列表 |

**超长内容处理**:任意章节实际内容超过 7.0 阈值(聊天原文 ≥ 3000 字 / 实验日志 ≥ 2000 字 / 多源引用 ≥ 2000 字)时,拆出孙子文档,本章节只留摘要 + `[完整 X →](feishu_url)` 链接。

#### 7.2.b 主报告 (`./main_report.md`) 的必含章节

Stage 4a 产出,**简洁、指路型**。

| 章节 | 内容约束 |
|---|---|
| H1 标题 = `群聊调研总览 — YYYY-MM-DD` | 当日日期(本地时区) |
| `## 概览` | 覆盖群列表 / 时间窗 / 话题数 / 调研类型分布;有前序期时加 `上一期: [日期](url)` 行(游标里的 `last_digest_doc_url`) |
| 每个话题一个 H1 `# {话题名}` | **≤ 10 行**(不含 1 张可选关键图);必含:来源、类型、核心结论(1 句)、跳转链接 `→ **[详见话题 X 详细文档](feishu_url)**` |
| 末尾 H1 `# 聊天参考(未深度调研)` | 列降级话题(数据不足 / 无沙盒 / 无法归类),每条一行写明降级原因 |

#### 7.2.c md 硬约束(两类文档都适用)

- 所有 `![alt](path)` 必须是工作目录下**相对路径**,PNG/JPG 放 `./figs/`,转换时统一上传
- **禁止**用网络 URL 引用图片
- 所有 `[text](https://xxx.feishu.cn/docx/...)` 必须保留为可点击 inline link(7.3 流水线会渲染成蓝色)
- 表格用标准 markdown `|...|`,复杂表格降级为 code block

> **需要完整 md 模板示例**?见 `markdown-templates.md`(同目录子文件)。该文件只是复现本节规则对应的骨架,不新增约束;改章节/字段/约束请改本节,不是那里。

### 7.3 Markdown → 飞书 的共享流水线

话题详细文档 (Stage 3 内部) 和主文档 (Stage 4b) **共用**此流水线:

```
输入: md 文件路径 + title + (可选) parent_token

1. 读 md 文件
2. document_id = kitty_feishu.create_document(title, parent_token=parent_token)
3. 扫 md 中所有 ![alt](path):
     对每个本地 PNG/JPG:
         image_token = kitty_feishu.upload_image(document_id, path)
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
| 话题的聊天原文 | 话题文档的**孙** | 原文 ≥ 3000 字 |
| 话题的实验日志 | 话题文档的**孙** | 日志 ≥ 2000 字 |
| 话题的多源原文引用 | 话题文档的**孙** | 引用 ≥ 2000 字 |
| 群 A / B / C 聊天底稿 | 主文档的**子** | 单群保留消息 ≥ 200 条 |
| 话题提取清单 | 主文档的**子** | 话题 ≥ 5 个 |
| `main_report.md` | 不单独发 | 4b 转换后即主文档 |

最终用户只需一个**主文档 URL** 即可下钻到全部细节。

### 7.5 为什么 4a 和 4b 拆成两个节点

- **关注点分离**:4a 是内容编辑(LLM),4b 是格式转换(工程)
- **可单独重试**:md 写坏了单独重跑 4a;图片/链接传失败单独重跑 4b
- **可审计**:`main_report.md` 落盘,用户可先审 md 再决定发飞书

### 7.6 话题降级

- **禁止**把机器人没验证过的聊天结论直接作为 H1 话题
- 没走完调研协议的话题(数据不足 / 无沙盒 / 类型无法判定)归到最末 H1 **`# 聊天参考(未深度调研)`**,列话题名 + 来源 + 降级原因
- 降级话题**不发独立详细子文档**

---

## Phase 8 — 常见陷阱

1. **时区**:Feishu `create_time` 是秒级 Unix 时间戳,用户说"今天下午"必须先 `date` 确认本地时区再换算。写进 Stage 1 节点 description,别让 agent 猜。

2. **权限**:bot 必须在群里才能 `list_messages`。Stage 1 节点最常见失败原因是 bot 没被拉进群 —— 这种 failure_reason 要写明"请先把 bot 拉入 {chat_id}",别让下游空等。

3. **thread 消息**:群开了话题功能(`thread_id` 非空),同话题消息在 Stage 1 要**聚拢**,不按时间线打散。

4. **图片 token 过期**:老消息(> 30 天)的 `image_key` 可能失效。Stage 1 下载时识别并标记,不要派 OCR 节点白跑。

5. **过滤过激**:Stage 1 result_summary 里留 **3 条被过滤消息样本**,供用户对 Phase 4 规则微调。保留 `discarded/{X}.json` 便于回溯。

6. **话题粒度失控**:Stage 2 产出 > 20 个话题 → 规则没 work,agent 应该合并细粒度话题。产出 < 2 个 → 原始信号稀薄,在 result 里标"可能无需本轮调研"。

7. **Docker 不可用**:Phase 6 的工具/代码/研究类节点硬依赖沙盒。检测到无 Docker 时**立即 fail 该类子节点**,**不允许**降级到主机运行 —— 这是保护用户环境的红线。

8. **原文引用丢失**:Stage 3 话题详细文档要求标"群 + 时间 + message_id",Stage 1 必须在 raw JSON 里**完整保留** `message_id / sender / create_time / thread_id`;漏任何一项,下游无法溯源。

9. **父子层级断裂**:7.0 顺序 —— 主文档必须**最先**建,拿到 parent_token 再建子孙;反过来会让子文档悬挂,UI 树破。

10. **inline link 被渲染成纯文本**:Stage 4b 转换主文档时,`[text](feishu_url)` 必须保留蓝色可点击 —— 如果看到文档里链接是黑字,立即重跑 4b 并在 result 标告警。

11. **中间 md 被删**:`main_report.md` 和 `./topics/*.md` 是可审计的中间产物,**别在交付后清理掉**。

12. **增量追加**:用户看完想"再加一个群"/"延长一天" → 直接 `kerrigan add` 新的 `group-*-analysis`,它完成后会级联触发 Stage 2 重建(Stage 2 节点可重置为 pending 再 ready),Stage 3/4 按话题变化动态增减。

---

## Appendix — 完整 DAG 依赖图(示例)

**场景**:用户要总结群 A / 群 B 今天下午的聊天,Stage 2 提取出话题 "Redis 8 vs KeyDB 6"(工具+新闻)和"某 arxiv paper 复现"(研究+代码)。

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
   Stage 3 (动态派生, 每话题 1 编排节点)                   │
   topic-redis8-investigation   topic-arxivpaper-inv.     │
         │                            │                    │
         │ (派生 tool + news 簇)     │ (派生 research + code 簇)
         ▼                            ▼
   ┌──────────────────────┐   ┌────────────────────────────┐
   │ tool-redis8-read-repo│   │ research-paper-fetch       │
   │ tool-redis8-sandbox  │   │ research-methodology-review│
   │ tool-redis8-bench-   │   │ research-citation-trace    │
   │   design             │   │ research-repro-design      │
   │ tool-redis8-bench-   │   │ research-repro-run         │
   │   run-primary        │   │ research-synthesize        │
   │ tool-redis8-bench-   │   │                            │
   │   run-competitor-    │   │ code-paper-fetch-sources   │
   │   keydb              │   │ code-paper-read-sources    │
   │ tool-redis8-analyze- │   │ code-paper-develop-repro   │
   │   plot               │   │ code-paper-static-check    │
   │                      │   │ code-paper-run-repro       │
   │ news-redis8-source-  │   │ code-paper-analyze         │
   │   official           │   └────────────┬───────────────┘
   │ news-redis8-source-  │                │
   │   code               │                │
   │ news-redis8-source-  │                │
   │   community          │                │
   │ news-redis8-source-  │                │
   │   counter            │                │
   │ news-redis8-         │                │
   │   crosscheck         │                │
   └──────────┬───────────┘                │
              │ (编排节点汇总,建话题详细文档) │
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
- Stage 2 跑完 → 建每个话题的 `topic-X-investigation`(编排节点)
- 每个 `topic-X-investigation` 开跑 → 在节点内按类型派生子节点簇
- 所有 `topic-*-investigation` 完成 → 建 `main-report-compose-md` + `main-md-to-feishu`

这是典型的"execute first, decompose later"—— 每一轮派生都基于前一轮的实际产出,**绝不提前规划超过两层依赖**。
