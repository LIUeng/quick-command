# Quick Command Requirements

## Product statement

Quick Command reduces the steps required to inspect, navigate, open, and operate on local workspaces. A user invokes a compact window with a global shortcut, enters a command such as `code example`, resolves any required file or directory context, and executes an explicit action.

## MVP users and platform

- Primary user: developer with multiple local projects.
- Primary platform: macOS.
- Other desktop platforms are deferred until the macOS workflow is stable.

## Functional requirements

### Launcher

- Open or hide the launcher with a configurable global shortcut.
- Focus the command input when the launcher appears.
- Support keyboard-only selection and execution.
- Hide after a successful launch; remain visible after an error.

### Command input

- Parse input into an executable and argument array.
- Apply directory lookup only to configured command rules such as `code` and `cursor`.
- Never evaluate pipes, redirects, command substitution, or chained shell expressions.
- Clearly report an empty command, unknown executable, invalid path, or launch failure.
- Classify known commands by presentation, launcher, operation, or navigation behavior.
- Resolve required workspace, directory, or file context before execution.
- Show explicit candidate actions when a command argument can represent either a file or a directory.

### Command behavior

- Presentation commands such as `ls`, `ll`, and `cat` capture output for display inside the application.
- Presentation commands return typed directory or text-file data; the client must not render raw terminal stdout as the primary interface.
- Directory results show file types and summary counts, with size and modification metadata in detailed mode.
- Text-file results use a bounded, line-numbered reader and reject oversized, binary, or non-UTF-8 content with a recovery hint.
- Launcher commands such as `code` and `open` start an external application and hide the launcher after success.
- Operation commands such as `mkdir` use validated Rust filesystem operations and confirm the destination when required.
- Navigation commands such as `cd` update Quick Command's active context instead of spawning an ineffective child-process shell builtin.
- Shell aliases such as `ll` are implemented as explicit internal mappings, never through shell evaluation.
- A command definition declares its execution mode, context requirement, path intent, risk level, and success behavior.

### Project discovery

- Let users configure one or more workspace roots.
- Index directories below enabled roots to a bounded depth.
- Match by directory name and path.
- Rank exact matches, prefix matches, fuzzy matches, favorites, frequency, and recency.
- Show the absolute path when names are ambiguous.
- Treat configured workspaces as user-approved search and operation roots, not as a mandatory permanent default.
- Ask the user to select a workspace when an action requires one and no active context resolves it.
- Order workspace choices by recent use and allow the user to change the selected context.

### File and directory intent

- `code` and `open` accept either files or directories.
- Existing absolute or relative paths are resolved by filesystem type.
- A trailing path separator expresses directory intent.
- A plain argument such as `code example` searches indexed project directories and the active context, then presents explicit actions when the result is ambiguous.
- A missing `code` target must not silently become a directory. The user chooses between opening a file path, creating a project directory, or executing the original path intent.
- File lookup is performed on demand and is not included in the global directory index by default.

### Directory creation

- When no project matches, offer directory creation as one explicit action below a user-selected workspace root.
- Show the final absolute path before creation.
- Reject traversal outside the workspace root.
- Launch only after creation succeeds.

### History and settings

- Display the latest 30 successful commands when the query is empty.
- Persist workspace roots, command rules, global shortcut, history, and directory weights.
- Allow editing the shortcut and managing workspace roots through a native folder picker.
- Allow adding and removing workspace roots without deleting local files.
- Allow deleting individual visible history entries without resetting project usage weight.

## Non-functional requirements

- Typical local search should feel immediate, targeting under 200 ms.
- Filesystem and process access must stay in Rust.
- Ranking must be deterministic and unit tested.
- The UI must be operable without a mouse.
- User-facing failures must include a concrete recovery hint.

## Explicitly out of scope for MVP

- General shell emulation or interactive terminal output.
- Pipes, redirects, `&&`, `;`, aliases, and shell functions.
- Cloud synchronization or plugin marketplace.
- Whole-disk indexing.
- Automatic execution of arbitrary scripts.

## Acceptance scenarios

1. `code example` lists matching projects and opens the selected absolute path.
2. Two directories named `example` are distinguishable by path.
3. A successful launch improves that directory's future rank.
4. A failed launch does not update successful history or usage weight.
5. Missing projects can be created only inside an enabled workspace.
6. Empty input displays at most 30 recent successful commands.
7. Shell operators are rejected instead of being evaluated.
8. `code example` never silently creates a directory when file intent is possible.
9. `Alt+1` and other macOS Option combinations retain their physical shortcut key.
10. Deleting a workspace removes its index entries but does not delete local files or history.
11. Deleting a history entry persists across restart and does not change directory frecency.
12. `ls` and `ll` render a structured directory browser, and `cat` renders a bounded text reader without exposing raw terminal output.
13. `cd` resolves indexed, relative, parent, and absolute directories inside enabled workspaces and updates application context without spawning a process.
