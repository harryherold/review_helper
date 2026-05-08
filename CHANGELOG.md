# Changelog

All notable changes to this project will be documented in this file.

## [v0.4]

### Added
- Logger — application now writes a log file (`review_helper.log`) for diagnostics
- Error popup — errors are now displayed in a dedicated popup dialog instead of being silently ignored
- Panic handler — unexpected internal errors now show a native dialog with instructions to check the log file before the application exits
- Rename reviews
- Notes view — improved with filter and sort support, notes can be added by pressing Enter, confirmation timer before deletion
- Commit picker — improved with filtering, sorting and merge base detection
- File diff setup view — new view for configuring the diff range (start/end commit)
- Overall statistics — review overview now shows progress statistics for files and notes
- Difference statistics — shows added/removed lines and change type breakdown per review
- Repository settings tab — dedicated settings per repository (e.g. base branch configuration)
- Review selection tab — with confirmation timer before deletion
- Note references — notes can be linked to specific changed files
- FileDiff proxy model — file diff list supports filtering by file name and review state, and sorting
- Notes proxy model — notes list supports filtering by text and context, and sorting
- Updated to Slint 1.15.1

### Changed
- Error handling significantly improved throughout the application
- Worker architecture refactored — UI updates extracted into a dedicated `UiUpdater` struct
- Repository cache refactored — repositories, reviews, notes and file diffs are now tracked by stable IDs
- Base branch validation — the application now checks whether the configured base branch exists before applying changes
- Commits are now reloaded automatically when a new repository is added
- Application layout simplified and improved
- Icons updated

---

## [v0.3]

- Added determining merge-branch
- Improved tests
- Added determining automatically diff-tools
- Implemented executing heavy git commands asynchronously with tokio
- Added pattern matching in file filtering
- Updated license
- Added application icons for Windows and Linux

## [v0.2]

- Persist notes into markdown files
- Track unsaved changes and show indicator
- Added dark and light theming
- Configure and store diff-tool, editor command, color theme into application config
- Added commit picker
- Notes view
- Show inline notes of changed files

## [v0.1]

- Diff two commits of a Git repository
