# Quick Command Agent Guide

## Product scope

Quick Command is a macOS-first desktop launcher for opening local projects with installed applications. The MVP is not a general-purpose terminal.

## Required stack

- Rust + Tauri 2
- React + TypeScript
- Tailwind CSS
- pnpm only for frontend dependency management

## Safety constraints

- Never pass user input to `sh -c`, `bash -c`, or equivalent shell evaluation.
- Spawn an executable with a structured argument array.
- Directory creation must resolve to a path under a configured workspace root.
- Only successful launches update usage weight and successful history.
- Do not log environment variables, secrets, or complete sensitive arguments.

## Architecture boundaries

- React owns presentation and transient UI state.
- Rust owns filesystem access, process spawning, persistence, indexing, and global shortcuts.
- Tauri command payloads and responses must use explicit serializable types.
- Keep search ranking deterministic and independently testable.
- Persist user data in the Tauri application data directory, never in the repository.

## Development workflow

1. Read `docs/REQUIREMENTS.md`, `docs/ARCHITECTURE.md`, and `docs/PROGRESS.md` before changing code.
2. Update the relevant task in `docs/PROGRESS.md` when starting and finishing work.
3. Add or update tests for Rust domain logic and frontend behavior where practical.
4. Run `pnpm check` and `cargo test --manifest-path src-tauri/Cargo.toml` before marking a milestone done.
5. Do not silently expand the MVP into arbitrary shell execution.

## Git commit message convention

Use Conventional Commits for the subject line:

```text
<type>(<optional-scope>): <concise change summary>
```

Allowed types:

- `feat`: user-facing functionality
- `fix`: bug fix
- `refactor`: internal change without intended behavior change
- `docs`: documentation only
- `test`: tests only
- `build`: dependencies, packaging, or build configuration
- `chore`: repository maintenance

Keep the subject imperative, specific, and under 72 characters. Do not use vague subjects such as `update code`, `fix issues`, or `changes`.

Every functional commit must include a body that acts as a memory aid for future agents. Use this format:

```text
feat(shortcut): support runtime shortcut reassignment

Summary:
- Capture shortcuts as key combinations and display symbolic keycaps.
- Unregister the previous shortcut before registering the new value.
- Restore the previous shortcut when registration or persistence fails.

Verification:
- pnpm check
- cargo test --manifest-path src-tauri/Cargo.toml
```

Commit body requirements:

- `Summary` lists the important user-visible behavior and architectural decisions.
- Mention notable limitations, follow-up work, or compatibility concerns when applicable.
- `Verification` lists the exact checks that passed; write `Not run` with a reason if validation was skipped.
- Keep one logical change per commit whenever practical.
- Update `docs/PROGRESS.md` in the same commit when milestone status or implementation decisions change.
- Before committing, inspect `git diff --cached` and confirm generated files, caches, secrets, and local state are not staged.

## Status notation

- `[ ]` TODO
- `[~]` IN PROGRESS
- `[x]` DONE
- `[!]` BLOCKED, with a short reason
