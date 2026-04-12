# Information Gathering and Research Guide

The core of systematic research: **Multi-source independent collection → Cross-validation → Empirical verification → Structured report**.

---

## Research Process

### Multi-perspective Information Collection (Independent and Parallel)

Brainstorm thoroughly on the research topic, approaching from different angles (not limited to the following examples):
- Academic perspective: Papers, surveys, benchmarks
- Industry perspective: Products, open-source projects, technical blogs
- Community perspective: Forum discussions, user feedback, GitHub issues/stars
- Business perspective: Market size, business models, profitability, competitive landscape, funding status
- Macro environment: Impact of relevant economic trends, policies and regulations, and geopolitical factors on technology choices or markets
- Historical perspective: What was the technical context, industry context, and market environment at the time? Why was this choice made then? What changed afterwards?

Each perspective is executed by an **independent node**, without referencing each other — ensuring the collected information is independent, rather than one node influencing another node's judgment.

### Cross-validation

After results from multiple collection nodes converge, perform cross-validation:
- Is the information from different sources consistent?
- Are there any contradictory findings?
- Do contradictions require further investigation?

### Hands-on Experience and Experimentation

**Anything that can be operated or interacted with (downloadable tools, available source code, online demos) must be actually tried and tested.** Do not rely solely on documentation and others' evaluations.

Experiment report requirements:
- **Falsifiability**: Clearly state under what conditions the experimental conclusions would be overturned
- **Reproducibility**: Record the environment, versions, and parameters so others can reproduce the results
- **Quantitative comparison**: Use key metrics from the relevant domain for quantitative comparison

Examples of key metrics for different domains:
- Databases: Performance (QPS/TPS), response time (P50/P99), throughput, resource usage
- Social products: User count, DAU/MAU, community activity, reputation and reviews
- API services: Supported interface types, data accuracy, latency, timeliness, SDK ecosystem, authentication methods
- Development tools: Installation complexity, learning curve, documentation quality, community support

Use tables for horizontal comparison across multiple products. When performance data is involved, create comparison charts.

For scenario comparisons and quantitative experiments, experimental design and reports must follow the AB testing guidelines — see `{{ shared_dir }}/guide-decision-ab-testing.md`.

### Experiments Involving Code Development

If experiments require writing code (benchmark scripts, comparison tests), **the software development management guide must be followed** (`{{ shared_dir }}/guide-software-development.md`). Experimental code also requires quality assurance.

---

## Research Report Standards

A qualified research report must include:

- **Research background**: Why the investigation was conducted, objectives and scope
- **Methodology**: What methods were used to collect and verify information
- **Key findings**: Organized by topic, with information sources noted for each finding
- **Empirical data**: Quantitative data takes priority over qualitative descriptions. Attach experiment reports where applicable
- **Cross-validation results**: Consistency analysis across different information sources
- **Limitations**: Areas not covered, potential biases
- **Conclusions and recommendations**: Actionable conclusions, avoiding ambiguity

**Unqualified research reports**:
- Cited only a single source
- Did not distinguish between facts and opinions
- Data lacks timeliness annotations (when was the data from?)
- Conclusions are not falsifiable
