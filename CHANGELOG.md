# Changelog

## [1.1.0] - 2026-04-27

### Added
- NodeContract system with mandatory JSON Schema contracts, syntax validation, subsumption checks, and runtime predecessor/output validation (@patrick)
- Strict state machine enforcement: only `exec` can transition nodes to Running (@patrick)
- `dag override` command for recovering from contract creation errors without deleting history (@patrick)
- SQLite database migration system using PRAGMA user_version (@patrick)
- `--successor` flag for DAG edges with cycle detection and `cascade_pending` propagation (@patrick)
- Next.js visualization dashboard (`viz/`) for DAG inspection (@patrick)
- One-liner install script with public GitHub install script (@patrick)
- `feishu-research` skill for multi-group chat digests (@patrick)

### Changed
- AGENTS.md updated with core design decisions: append-only storage, result-as-communication thesis, and current project structure (@patrick)
- `feishu-research` skill restructured into 6 Types with orthogonal hands-on tag and terminology cleanup (@patrick)
- README rewritten for single-binary workflow and skill install flow (@patrick)
- Default agent configuration bundle added with deployment guide (@patrick)
- Parallel development isolation strategy documented as git worktrees (@patrick)

### Fixed
- Detach mode child process crash on encountering Running node state (@patrick)
- Agent binary install path corrected to `kerrigan-agent` (@Kerrigan)
- CI `check:version` script YAML parsing fixed with block literal (@patrick)

### Removed
- Output reminder envelope from CLI responses (superseded by NodeContract schema) (@patrick)

## [1.0.1] - 2026-04-15

### Added
- Reminder envelope to all CLI responses with improved error messages and docs (@patrick)
- Agent benchmark research notes (@patrick)

## [1.0.0] - 2026-04-14

### Changed
- Rewrote as single-crate async CLI (@patrick)

## [0.1.1] - 2026-04-14

### Added
- Instructions for agents to keep background monitor on board file (@patrick)

### Fixed
- Added --verbose to claude adapter, fixed loop stall and board messaging issues (@patrick)

## [0.1.0] - 2026-04-13

### Added
- Unified CLI with run/resume/status/tell as single entry point (@patrick)
- DAG engine with node/edge management and persistence (@patrick)
- SQLite WAL mode storage backend replacing JSON files (@patrick)
- Agent adapter layer supporting Claude Code and Kimi CLI (@patrick)
- Streaming agent output capture with log files (@patrick)
- Stream-json output format from Claude and Kimi adapters (@patrick)
- Tracing infrastructure with -v verbosity flag (@patrick)
- Guide file system deployed to shared directory with prompt injection (@patrick)
- kerrigan-swarm skill for external agent task management (@patrick)
- Enhanced DAG validator with cycle detection and reference checks (@patrick)
- Comprehensive test suite with integration tests (@patrick)
- GitHub Actions workflows and CI pipeline (@patrick)
- Release script with auto-detected version bump from commit types (@patrick)
- AGENTS.md with conventional commits rules and project structure (@patrick)

### Changed
- Merged submodules into monorepo structure (@patrick)
- Translated all guide files, prompts, and code strings to English (@patrick)
- Simplified init template with auto-created origin node (@patrick)
- Updated READMEs for monorepo structure (@patrick)

### Fixed
- Tracing filter mismatch for kerrigan_cli crate (@patrick)
- Init template depth limit description (@patrick)
- Binary names and user flow in kerrigan-swarm SKILL.md (@patrick)
- Changelog contributor extraction preferring author name (@patrick)
