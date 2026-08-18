# Quick Command Release and Troubleshooting

## Local validation

Run the automated checks before creating a bundle:

```bash
pnpm check
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

Create a local macOS bundle with:

```bash
pnpm tauri build
```

Build output is written below `src-tauri/target/release/bundle/`. Do not commit generated bundles, signing credentials, provisioning data, or notarization secrets.

## Packaged macOS checklist

- Confirm the new icon appears correctly in Finder, the Dock, the application switcher, and the DMG/application bundle.
- Launch the application from Finder or the Dock rather than a terminal.
- Confirm the global shortcut opens, focuses, and hides the launcher.
- Open the native workspace folder picker and verify focus-loss auto-hide remains suspended while it is visible.
- Run `code` or another installed trusted editor command to verify packaged GUI executable resolution.
- Exercise `ls`, `cat`, `cd`, and confirmed `mkdir` inside an enabled workspace.
- Check the WebView console for CSP violations; production must not request Vite development origins.
- Restart the application and verify settings, active context, history, and indexes reload.
- When upgrading a development build that used `com.quickcommand.app`, verify the first `com.quickcommand.launcher` launch imports the previous state and shows the migration notice.

## Signing and notarization

Signing and notarization require a valid Apple Developer identity and must be completed outside source control. Never commit certificates, passwords, API keys, or exported profiles.

After producing the release bundle, validate the actual artifact with the macOS signing, Gatekeeper, and notarization tools available in the release environment. Record the application version, architecture, signing identity name, and validation results without recording secrets.

## Troubleshooting

### Editor command not found

Finder and Dock launches do not inherit the full interactive-shell `PATH`. Quick Command checks the inherited path, common Homebrew/system directories, user-local binary directories, and supported application-bundle CLI paths. Confirm the editor is installed and that its command-line tool is enabled.

### Global shortcut does not register

Choose another shortcut in Settings. Another application may already own the combination. Saving a replacement shortcut restores the previous shortcut if registration fails.

### Launcher does not hide or drag

Verify the packaged capability file includes window hide and start-dragging permissions. Test both focus-loss hiding and the draggable header area.

### State recovery notice appears

The primary state file was missing or malformed. Quick Command preserves malformed data as `state.corrupt-*.json` and attempts to restore `state.backup.json`. These files live in the Tauri application-data directory, not in the repository.

### CSP violation appears

Do not broadly disable CSP. Identify the exact blocked local resource or IPC origin and add the narrowest required directive separately to development or production policy.
