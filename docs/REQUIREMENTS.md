# Quick Command Requirements

## Product statement

Quick Command reduces the steps required to open a local project. A user invokes a compact window with a global shortcut, enters a command such as `code example`, selects a matching project, and launches it.

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

### Project discovery

- Let users configure one or more workspace roots.
- Index directories below enabled roots to a bounded depth.
- Match by directory name and path.
- Rank exact matches, prefix matches, fuzzy matches, favorites, frequency, and recency.
- Show the absolute path when names are ambiguous.

### Directory creation

- When no project matches, offer to create it below a selected or default workspace root.
- Show the final absolute path before creation.
- Reject traversal outside the workspace root.
- Launch only after creation succeeds.

### History and settings

- Display the latest 30 successful commands when the query is empty.
- Persist workspace roots, command rules, global shortcut, history, and directory weights.
- Allow editing the shortcut and default workspace.

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

