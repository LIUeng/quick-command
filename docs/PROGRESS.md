# Development Progress

Last updated: 2026-08-18

## Milestone 0 — Definition

- [x] Extract MVP scope from the original README.
- [x] Define security boundaries and acceptance scenarios.
- [x] Add repository instructions for future agents.
- [x] Define architecture and development workflow.

## Milestone 1 — Runnable foundation

- [x] Scaffold Tauri 2, React, TypeScript, Vite, and Tailwind CSS.
- [x] Add launcher window configuration and application metadata.
- [x] Add typed frontend-to-Rust command boundary.
- [x] Verify frontend typecheck and production build.
- [x] Verify Rust tests and Tauri compilation.

## Milestone 2 — Core launcher

- [x] Implement command parsing without shell evaluation.
- [x] Implement configured workspace indexing.
- [x] Implement deterministic fuzzy/frecency ranking.
- [x] Implement safe process execution.
- [x] Persist settings, index metadata, and recent successful history.

## Milestone 3 — User experience

- [x] Implement Spotlight-style launcher interface.
- [x] Implement keyboard navigation and execution states.
- [x] Hide the launcher on focus loss, successful execution, and Escape.
- [x] Apply a transparent, rounded launcher window surface.
- [x] Restore the hidden launcher from the macOS Dock and support window dragging.
- [x] Constrain the settings overlay to the rounded launcher surface.
- [x] Implement no-result directory creation confirmation with final-path and creation-list preview.
- [x] Implement settings for shortcut and workspace roots.
- [x] Register and update the global shortcut with validation, rollback, and keycap-style display.

## Milestone 4 — Release readiness

- [ ] Add integration coverage for main command flows.
- [~] Validate macOS GUI `PATH` behavior — deterministic fallback resolution is implemented; packaged-app manual validation remains.
- [ ] Validate packaged app permissions and signing configuration.
- [ ] Document local development, packaging, and troubleshooting.

## Milestone 5 — Context-aware command system

- [x] Define command categories, execution modes, context requirements, path intents, risk levels, and success behaviors.
- [x] Add the initial Rust command catalog for `ls`, `ll`, `cat`, `code`, `open`, `mkdir`, and `cd`.
- [x] Replace the workspace textarea with native folder selection and add/remove controls.
- [x] Add reusable runtime workspace/context selection with a persisted active context.
- [x] Replace `canCreate` with explicit candidate actions.
- [x] Resolve plain-name `code` arguments as file-or-directory without silent directory creation.
- [x] Capture and render structured presentation-command output for `ls`, `ll`, and `cat`.
- [x] Implement `cd` as an internal active-context action.
- [x] Implement `ll` as an explicit structured `ls -al` mapping.
- [x] Implement `mkdir` with workspace-bound preview, confirmation, and rollback.
- [x] Add individual history deletion without resetting frecency.
- [x] Persist resolved action metadata in history.
- [x] Support confirmed multi-level project directory creation for `code`.

## Milestone 6 — Security and release preparation

- [x] Reject commands that are not in the trusted command catalog.
- [x] Resolve launcher executables without relying on the macOS GUI process `PATH` alone.
- [x] Add state corruption recovery and versioned migrations.
- [ ] Add integration coverage for critical launcher flows.
- [ ] Complete production bundle, signing, notarization, and CSP validation.
- [ ] Document packaged-app installation and troubleshooting.

## Decision log

- 2026-08-13: Limit MVP to structured executable + arguments; arbitrary shell syntax is out of scope.
- 2026-08-13: Target macOS first.
- 2026-08-13: Use versioned JSON persistence for the first executable slice; migrate to SQLite when query/migration needs justify it.
- 2026-08-13: Frontend `pnpm check` and `pnpm build` pass.
- 2026-08-13: Fixed the first manual Rust build findings: added the required Tauri icon source, removed an unused import, and corrected the `MutexGuard` snapshot borrow.
- 2026-08-13: `cargo check` and all 5 Rust unit tests pass; the runnable foundation is verified for the first commit.
- 2026-08-13: Completed launcher focus behavior, transparent rounded presentation, and runtime shortcut reassignment with symbolic keycaps.
- 2026-08-13: Disabled the macOS native shadow for the transparent window and constrained the launcher surface to eliminate irregular translucent window-edge artifacts.
- 2026-08-13: Added Dock reopen handling, draggable launcher regions, layered CSS elevation, and rounded settings overlay clipping.
- 2026-08-13: Added the explicit Tauri window-drag permission and API-driven dragging; reduced shadows to stay within the transparent window padding and avoid rectangular clipping.
- 2026-08-13: Added a repository-wide Git commit convention requiring Conventional Commit subjects plus functional summaries and verification details for agent memory continuity.
- 2026-08-13: Fixed shortcut recording by entering an explicit capture mode and listening at the settings-window level; the captured keycaps update the form immediately and are registered when the user clicks Save.
- 2026-08-14: Added a centralized Rust error-presentation boundary: debug builds retain technical detail, while release builds expose only stable, actionable client messages.
- 2026-08-14: Fixed shortcut capture to preserve macOS Command and Control as distinct modifiers instead of collapsing both into `CommandOrControl`.
- 2026-08-14: Normalized shortcut keys from physical `KeyboardEvent.code` values so macOS Option combinations such as `Alt+1` save as `Alt+1` instead of localized characters such as `¡`.
- 2026-08-14: Added the explicit Tauri window-hide capability and isolated best-effort UI hiding from command execution errors, preventing a successful command such as `ls -a` from flashing a `window.hide` failure.
- 2026-08-17: Replaced the mandatory-default-workspace direction with runtime context selection from user-approved workspace roots.
- 2026-08-17: Defined `code` as file-or-directory and prohibited silently converting a missing target into a directory.
- 2026-08-17: Started the context-aware command system with a typed Rust command catalog shared by parsing decisions.
- 2026-08-17: Added native multi-folder workspace selection, pending add/remove controls, Rust path canonicalization/deduplication, and auto-hide suspension while the system picker is open.
- 2026-08-17: Replaced implicit project creation with explicit `code` actions for opening a file or creating a directory, followed by runtime workspace selection and validated execution.
- 2026-08-17: Added a reusable workspace picker and persisted active context, cleared invalid context when workspace settings change, and fixed keyboard workspace selection precedence.
- 2026-08-17: Implemented structured directory and text-file presentation for `ls`, `ll`, and `cat`, including bounded reads, friendly metadata views, interactive navigation, and automatic context selection/retry.
- 2026-08-17: Implemented `cd` as an internal context update with indexed and explicit path resolution, workspace-bound parent traversal, typed success feedback, and rejection of unfinished internal command fallthrough.
- 2026-08-17: Implemented `mkdir` as a two-phase internal operation with canonical target preview, keyboard confirmation, workspace-bound validation, index/history updates, and rollback when persistence fails.
- 2026-08-17: Added persistent single-entry history deletion with hover/focus controls, idempotent backend handling, and tests proving directory frecency remains unchanged.
- 2026-08-17: Added backward-compatible typed history action metadata across launcher, presentation, navigation, and filesystem operation flows, with concise labels for new and legacy records.
- 2026-08-17: Added confirmed `code x-pro/test01` project creation with safe relative-path validation, multi-directory preview, symlink-boundary checks, structured VS Code launch, and empty-directory rollback on launch failure.
- 2026-08-17: Closed raw-command fallthrough and added shell-free macOS executable resolution across inherited PATH, common binary directories, and supported application bundles; packaged-app PATH validation remains pending.
- 2026-08-18: Added data version 2 migration, previous-state backups, timestamped corrupt-file preservation, automatic backup recovery, future-version protection, and a dismissible startup recovery notice.
