# Quick Command Architecture

## Component flow

```text
React UI
  -> typed Tauri commands
Rust application service
  -> parser / matcher / ranker
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
- `parser`: safe tokenization and unsupported-shell detection.
- `search`: deterministic fuzzy and frecency ranking.
- `store`: versioned persistence in the application data directory.
- `commands`: Tauri command boundary and application orchestration.

## Persistence model

The MVP state contains:

- settings and enabled workspace roots;
- indexed directory records;
- successful launch history;
- per-directory use count and last-used timestamp;
- command rules describing which argument is a directory query.

History is pruned to a reasonable internal cap while the UI returns the latest 30 entries.

## Execution contract

Input is parsed without a shell. The executable is resolved from the application environment and launched with `std::process::Command::args`. Shell control operators are rejected. For a directory-aware rule, only the designated argument is replaced by the selected absolute path.

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
