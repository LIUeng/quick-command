# Quick Command Architecture

## Component flow

```text
React UI
  -> typed Tauri commands
Rust application service
  -> parser / command catalog / resolver
  -> matcher / ranker / candidate actions
  -> filesystem indexer
  -> safe process executor
  -> JSON persistence (MVP)
Application data directory
```

SQLite remains the preferred storage target once migrations and richer queries are required. The first executable slice uses a versioned JSON state file to avoid introducing database migration work before behavior is validated.

## Frontend modules

- `App.tsx`: launcher orchestration and keyboard interaction.
- `lib/api.ts`: typed wrapper around Tauri commands.
- `lib/types.ts`: shared frontend domain shapes.
- `components/`: presentation components as the UI grows.

The frontend does not read directories or launch processes directly.

## Rust modules

- `models`: serializable API and persistence types.
- `command_catalog`: known command definitions and their execution contracts.
- `parser`: safe tokenization and unsupported-shell detection.
- `executable_resolver`: trusted executable lookup across the process `PATH`, common macOS binary directories, and known application bundles.
- `resolver`: context and file/directory intent resolution (planned).
- `presentation`: bounded filesystem inspection and typed view models for `ls`, `ll`, and `cat`.
- `search`: deterministic fuzzy and frecency ranking.
- `store`: versioned persistence in the application data directory.
- `commands`: Tauri command boundary and application orchestration.

## Persistence model

The MVP state contains:

- settings and enabled workspace roots;
- the current active directory context, when one has been selected;
- indexed directory records;
- successful launch history;
- per-directory use count and last-used timestamp;
- command rules describing which argument is a directory query.

History is pruned to a reasonable internal cap while the UI returns the latest 30 entries.

## History deletion contract

- Each visible successful-history entry has a stable identifier and can be deleted independently.
- Deletion removes only the matching history item; directory use counts, last-used timestamps, indexes, settings, and local files remain unchanged.
- The backend persists a cloned next state before replacing in-memory data, so a save failure leaves the current state intact.
- Repeating deletion for an already removed identifier is treated as an idempotent success.

## History action metadata contract

- Each newly recorded success stores a typed action kind in addition to its display command, effective arguments, and resolved target.
- Initial action kinds distinguish generic launch, project open, file open, create-and-open, directory listing, text reading, context navigation, and directory creation.
- The action field is optional during deserialization so existing version-1 JSON state remains readable without an eager migration.
- The UI derives short localized labels from the stable action kind and labels entries without metadata as legacy records.
- Action metadata records classification only; it does not duplicate file contents or add complete environment information.

## Execution contract

Input is parsed without a shell. The executable is resolved from the application environment and launched with `std::process::Command::args`. Shell control operators are rejected. For a directory-aware rule, only the designated argument is replaced by the selected absolute path.

The parser rejects executables that are absent from the trusted command catalog. A catalog miss is a terminal validation failure and must not become a raw process launch. Registered external commands are resolved to an executable file before spawning. Resolution checks the inherited GUI `PATH`, user-local binary directories, Homebrew/system directories, and known CLI locations inside supported application bundles. It never evaluates shell startup files or invokes `which`, `sh -c`, or an equivalent shell command.

Executable resolution intentionally does not inspect or log environment contents. A missing executable produces a stable recovery message directing the user to install or enable the application's command-line tool. User-configurable executable paths remain future work.

Before execution, a known command is resolved through this pipeline:

```text
input
  -> safe tokenization
  -> command definition lookup
  -> argument and path-intent resolution
  -> workspace / directory / file context selection
  -> candidate action selection
  -> optional confirmation
  -> structured execution
  -> history with resolved action metadata
```

Command categories are user-facing labels. Runtime behavior is driven by explicit properties:

- execution mode: spawn, capture, or internal;
- context requirement: none, workspace, directory, or file;
- path intent: none, directory, file, or file-or-directory;
- risk level: safe, confirm, or restricted;
- success behavior: hide, show output, show a message, or update active context.

`code example` is intentionally treated as file-or-directory. If no exact existing path resolves it, the resolver returns multiple candidate actions rather than silently creating a directory.

For a safe multi-level project path such as `code x-pro/test01`, project creation uses a two-phase plan:

1. Canonicalize the selected enabled workspace.
2. Walk each existing path component, requiring directories and rejecting symbolic-link resolution outside that workspace.
3. Record each missing directory without writing and return the final path plus creation list for confirmation.
4. Reparse the command and recompute the plan at confirmation time; the recomputed workspace and target must match the preview.
5. Create missing directories one at a time and launch `code` with the final absolute path through a structured argument array.
6. If launch fails, remove only directories created by this operation, in reverse order and only while they remain empty.

An explicit `./` argument retains command-line path semantics and does not enter the workspace project-creation flow. A trailing separator expresses directory intent and suppresses the open-as-file candidate.

## Presentation output contract

- `ls` and `ll` are implemented through Rust filesystem metadata rather than spawning a shell command and parsing stdout.
- `ll` explicitly enables hidden and detailed directory fields, equivalent to the intended `ls -al` behavior.
- Directory responses contain typed entries, counts, file sizes, modification times, and truncation state; the UI renders them as navigable rows.
- `cat` resolves one file inside an enabled workspace and returns a bounded UTF-8 text document with filename, language hint, size, and line count.
- Text files larger than 256 KB or 5000 lines, binary content, and non-UTF-8 data are rejected with an actionable message.
- A missing relative context returns a typed context request so the frontend can reuse the workspace selector and retry the command.

## Workspace context contract

- Configured workspaces are approved roots for search and filesystem operations.
- A permanent default workspace is not required.
- The launcher may maintain an active context and per-command recent workspace choices.
- The active context is persisted as an optional canonical directory under an enabled workspace; removing or disabling its workspace clears it.
- If an action requires a workspace and none can be inferred safely, the UI presents the configured workspace list.
- Removing a workspace rebuilds directory indexes but never deletes files or history.
- Workspace paths returned from the native picker are canonicalized and deduplicated by Rust before persistence.

## Navigation contract

- `cd` is an internal application action and never spawns a shell builtin or child process.
- A selected indexed result supplies an already resolved directory; explicit absolute and relative paths are canonicalized before use.
- Relative paths resolve from the active context. If none exists, the command returns a typed context request and resumes after workspace selection.
- Parent traversal is allowed only while the canonical result remains inside an enabled workspace.
- Successful navigation persists the new active context, records successful history, and returns a typed UI notice while keeping the launcher visible.
- Internal commands without a completed safe workflow are rejected instead of falling through to external process spawning.

## Filesystem operation contract

- `mkdir` is a two-phase internal action: preview first, explicit confirmation second.
- Preview resolves a canonical existing parent and returns the final target and owning workspace without writing to disk.
- Confirmation reparses the original command and recomputes the target; client-provided paths are accepted only when they exactly match the recomputed plan.
- The first safe flow creates one directory only. Options and recursive parent creation are rejected.
- The single-directory restriction applies to `mkdir`; confirmed `code` project creation has a separate workspace-rooted multi-level plan.
- Successful creation updates the in-memory index and history. If state persistence fails, the newly created empty directory is removed as a rollback.
- The active context remains unchanged after creation, matching normal `mkdir` behavior.

## Error presentation contract

- Rust owns the boundary between internal failures and user-facing messages.
- Debug builds may append the underlying system or library error for local diagnosis.
- Release builds return only stable, actionable user messages and must not expose error types, raw OS error codes, or library diagnostics in the client UI.
- User-facing messages should explain the recovery action without leaking environment variables or sensitive arguments.

## Indexing contract

- Scan only configured workspace roots.
- Skip hidden directories and common heavy directories such as `node_modules`, `target`, and `.git`.
- Use a bounded recursion depth.
- Return useful errors without discarding the last valid index.
