---
name: changelog
description: >
  Analyze git commits since last release tag and generate a structured
  CHANGELOG entry following Keep a Changelog format. Categorizes commits
  by type, extracts contributors, and produces markdown output.
---

# Changelog Generator

You will receive a git log of commits since the last release. Analyze each commit and generate a structured CHANGELOG entry.

## Input

You will be given:
1. The new version number
2. Today's date
3. The raw git log (commit hash, author, date, message) since the last tag

## Output

Output **only** the markdown block to prepend to CHANGELOG.md. No explanation, no preamble. Format:

```markdown
## [VERSION] - DATE

### Added
- description (@author)

### Changed
- description (@author)

### Fixed
- description (@author)

### Removed
- description (@author)
```

## Rules

- **Categorize by actual change**, not just commit prefix. Read the commit message and description to understand what really happened.
  - `Added`: new features, new files, new capabilities
  - `Changed`: modifications to existing behavior, refactors, updates
  - `Fixed`: bug fixes, error corrections
  - `Removed`: deleted features, removed files
- **Merge related commits** into a single entry if they are part of the same logical change (e.g., "add feature X" + "fix typo in feature X" = one "Added" entry).
- **Extract contributor**: 优先使用 commit author name（如 `(@username)`），如果 name 不可用或不明确，使用完整邮箱。
- **Be concise** — each entry should be one line, describing the user-visible change.
- **Skip trivial commits** like "update submodule pointer" or merge commits that carry no meaningful change.
- Only include sections that have entries (don't output empty sections).
