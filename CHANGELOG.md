# Changelog

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
