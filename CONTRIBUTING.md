# Contributing to review_helper

Thank you for your interest in contributing! This document gives an overview of the project architecture, how to build and test the project, and the code conventions used.

## Table of Contents

- [Building from Source](#building-from-source)
- [Running Tests](#running-tests)
- [Architecture Overview](#architecture-overview)
  - [UI Layer](#ui-layer)
  - [Controller Layer](#controller-layer)
  - [Worker](#worker)
  - [Model](#model)
  - [Storage](#storage)
  - [Repositories Cache](#repositories-cache)
- [Code Conventions](#code-conventions)
- [Submitting Changes](#submitting-changes)

---

## Building from Source

**Requirements:**
- Rust toolchain (stable)
- Git installed and available on `PATH`

```sh
cargo build --release
```

For improved font rendering set the following environment variable before running:

```sh
SLINT_BACKEND=winit-skia ./target/release/review-helper
```

---

## Running Tests

```sh
cargo test
```

Some tests use `serial_test` to avoid file system conflicts when running in parallel. The `mockcmd` crate is used to mock external git commands in tests.

To run Clippy:

```sh
cargo clippy
```

---

## Architecture Overview

The application follows a clear separation between UI, controller, worker and storage layers.

```
┌─────────────────────────────────┐
│           UI (Slint)            │
└────────────────┬────────────────┘
                 │ callbacks
┌────────────────▼────────────────┐
│           Controller            │
└────────────────┬────────────────┘
                 │ WorkerMessage (channel)
┌────────────────▼────────────────┐
│      Worker (background thread) │
│  ┌──────────┐  ┌─────────────┐  │
│  │WorkerImpl│  │  UiUpdater  │  │
│  └──────────┘  └─────────────┘  │
└───────┬────────────────┬────────┘
        │                │
┌───────▼──────┐  ┌──────▼───────┐
│  Repositories│  │   Storage    │
│    (cache)   │  │ (file system)│
└──────────────┘  └──────────────┘
```

### UI Layer

Located in `ui/` (Slint files) and `src/ui.rs` (generated bindings).

The UI is built with [Slint](https://slint.dev/). All UI state is managed through Slint globals and models. The UI never directly accesses storage or git — it only communicates through callbacks that are wired up by the controllers.

### Controller Layer

Located in `src/controller/`.

Each controller module wires up the Slint callbacks for a specific part of the UI. When the user interacts with the UI, the controller sends a `WorkerMessage` over an unbounded channel to the worker thread. Controllers run on the main/UI thread.

| Module | Responsibility |
|---|---|
| `review_helper_controller` | Adding and deleting repositories |
| `repository_controller` | Loading repositories, changing base branch, managing reviews |
| `review_controller` | Loading reviews, managing notes and file diffs |
| `review_helper_settings_controller` | Saving and refreshing application settings |
| `commit_picker_controller` | Commit picker interactions |
| `file_diffs_controller` | Opening files in the configured editor |
| `file_picker_controller` | File picker dialog |
| `utils_controller` | Shared utility callbacks |

### Worker

Located in `src/worker/`.

The worker runs in a **dedicated background thread** and processes `WorkerMessage` values received from the controllers via a channel. It is the only place where git commands, storage access and heavy computation happen.

- **`WorkerImpl`** — contains all the business logic (loading, saving, git operations)
- **`UiUpdater`** — sends UI updates back to the main thread via `slint::Weak::upgrade_in_event_loop`
- **`ReviewHelperSettings`** — manages loading and saving the application configuration

Errors that are recoverable (e.g. git command failed) are reported to the UI via `UiUpdater::report_error`. Errors that indicate a programming bug (e.g. an ID not found in the cache) cause a `panic!` with a `[BUG]` prefix, which is caught by the global panic hook and shown to the user as a native dialog before the application exits.

### Model

Located in `src/model/`.

Contains Slint-compatible models and proxy models used to display data in the UI.

- **`IdModel`** — a custom Slint model that tracks items by a stable `usize` ID
- **`FileDiffProxyModels`** — wraps the file diff model with filter and sort support
- **`NotesProxyModels`** — wraps the notes model with filter and sort support
- **`CommitProxyModels`** — wraps the commit model with filter and sort support
- **`RepositoriesProxyModels`** — manages proxy models per repository and review
- **`model_utils`** — helper functions to retrieve models from the Slint UI tree

### Storage

Located in `src/storage/`.

Handles persistence to the file system. The `ReviewHelperStorage` trait defines the interface. The current implementation `ReviewHelperFileStorage` stores data as TOML files under the OS-specific application data directory.

```
<data_dir>/review-helper/
├── repositories.toml
└── <repository-name>/
    ├── <review-name>.toml
    └── <review-name>_notes.md
```

### Repositories Cache

Located in `src/repositories.rs`.

An in-memory cache that holds the state of all loaded repositories, reviews, notes and file diffs during the application's lifetime. All items are identified by stable numeric IDs (`RepositoryId`, `ReviewId`, `NoteId`, `FileDiffId`) which are also used as keys in the Slint `IdModel`.

---

## Code Conventions

- **Clippy** is enforced via `#![deny(clippy::all)]` — all code must pass without warnings
- **Formatting** is enforced via `rustfmt` — run `cargo fmt` before committing
- **Error handling:**
  - Recoverable errors (git failures, storage errors) → report via `UiUpdater::report_error`
  - Programming bugs (ID not found in cache) → `unwrap_or_else(|| panic!("[BUG] ..."))`
  - Avoid `expect(&format!(...))` — use `unwrap_or_else(|| panic!(...))` instead so the message is only evaluated on failure
- **Worker messages** — new features that require background work should be added as a new variant to `WorkerMessage` and handled in `WorkerImpl::worker_loop`
- **Log file** — use the `log` crate macros (`log::info!`, `log::error!`, etc.) for diagnostics

---

## Submitting Changes

1. Fork the repository and create a feature branch
2. Make your changes and ensure `cargo clippy` and `cargo test` pass
3. Update `CHANGELOG.md` under the `Unreleased` section
4. Open a pull request with a clear description of the changes
