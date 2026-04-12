---
name: changelog
description: >
  Analyze git commits since last release tag and write a structured
  CHANGELOG entry to the project's CHANGELOG.md file. You must use
  file editing tools to write the file directly.
---

# Changelog Generator

You will receive git commit data since the last release. Your job is to analyze the commits, generate a structured changelog entry, and **write it directly to the CHANGELOG.md file** using your file editing tools.

## File Location

The changelog file is: `CHANGELOG.md` in the project root directory (the working directory you are given).

## File Format (Keep a Changelog)

The file starts with `# Changelog` on the first line, followed by version entries in reverse chronological order (newest first). Each entry:

```markdown
## [VERSION] - DATE

### Added
- description (@contributor)

### Changed
- description (@contributor)

### Fixed
- description (@contributor)

### Removed
- description (@contributor)
```

## What You Must Do

1. Read the commit data provided in your prompt
2. Analyze and categorize each commit
3. Generate the new version entry
4. **Read the existing `CHANGELOG.md`** file (if it exists)
5. **Write the updated `CHANGELOG.md`** with the new entry inserted after the `# Changelog` title, before any existing entries
6. **Do NOT modify or delete any existing version entries** — only prepend the new one

If `CHANGELOG.md` does not exist, create it with the `# Changelog` title and the new entry.

## Categorization Rules

- **Categorize by actual change**, not just commit prefix. Read the commit message to understand what really happened.
  - `Added`: new features, new files, new capabilities
  - `Changed`: modifications to existing behavior, refactors, updates
  - `Fixed`: bug fixes, error corrections
  - `Removed`: deleted features, removed files
- **Merge related commits** into a single entry if they are part of the same logical change (e.g., "add feature X" + "fix typo in feature X" = one "Added" entry).
- **Extract contributor**: prefer commit author name (e.g., `(@username)`). If the name is not available or ambiguous, use the full email address.
- **Be concise** — each entry should be one line, describing the user-visible change.
- **Skip trivial commits** like merge commits or submodule pointer updates that carry no meaningful change.
- Only include sections that have entries (don't output empty sections).

## Important

- You MUST write the file yourself. Do not just output markdown text.
- You MUST NOT modify any existing released entries in the file.
- After writing, confirm by outputting: `CHANGELOG.md updated with version [VERSION]`
