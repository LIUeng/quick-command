# Development Progress

Last updated: 2026-08-14

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
- [~] Implement no-result directory creation confirmation — creation action exists; explicit path confirmation UI remains.
- [x] Implement settings for shortcut and workspace roots.
- [x] Register and update the global shortcut with validation, rollback, and keycap-style display.

## Milestone 4 — Release readiness

- [ ] Add integration coverage for main command flows.
- [ ] Validate macOS GUI `PATH` behavior.
- [ ] Validate packaged app permissions and signing configuration.
- [ ] Document local development, packaging, and troubleshooting.

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
