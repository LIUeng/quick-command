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

Apple signing and notarization are currently optional for local distribution. Users may need to approve an unsigned application manually after installation or an update.

## In-app updater setup

The Tauri updater signature is required even while Apple signing and notarization are deferred. It protects the update channel from modified or substituted artifacts; it does not identify the developer to macOS Gatekeeper.

Generate the updater key pair once on a trusted development machine:

```bash
pnpm tauri signer generate -w ~/.tauri/quick-command.key
```

Choose a strong password when prompted. Keep both the private key and its password outside this repository and include them in a separate backup. Losing the private key prevents existing installations from trusting future updates.

Configure the GitHub repository under **Settings → Secrets and variables → Actions**:

- Secret `TAURI_SIGNING_PRIVATE_KEY`: complete contents of the generated private-key file.
- Secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: password chosen during key generation.
- Variable `TAURI_SIGNING_PUBLIC_KEY`: complete contents of the generated public-key file.

The public key is not secret, but the release workflow injects it at compile time through `QUICK_COMMAND_UPDATER_PUBKEY`. Local builds without this value show the current version and explain that application updates are disabled.

Updater artifact generation is enabled only by `src-tauri/tauri.release.conf.json`. The normal local `pnpm tauri build` command therefore does not require updater signing credentials.

Never replace the updater key pair after releasing an updater-enabled version unless a deliberate key-rotation migration has first shipped with the required trust material.

## Publishing an update

Keep the version identical in these files:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

Validate synchronization with:

```bash
pnpm version:check
```

Then run the normal checks, commit the version and release notes, and push a matching semantic-version tag:

```bash
git tag v0.2.0
git push origin main
git push origin v0.2.0
```

The `Release` GitHub Actions workflow will:

1. Verify that the tag and all three project versions match.
2. Build a universal macOS application for Apple Silicon and Intel.
3. Sign the updater archive with the Tauri updater private key.
4. Upload the application artifacts, signatures, and `latest.json` to a draft GitHub Release.

Review the draft, replace the generated body with meaningful release notes, and publish it. Only a published release becomes available through the application's **设置 → 软件更新 → 检查更新** action.

Do not delete an already published artifact while clients may be downloading it. If a release must be withdrawn, remove it from the update channel and publish a higher patch version containing the corrected or previous implementation; the updater does not downgrade by default.

## Packaged update validation

Application updates cannot be fully validated from `pnpm tauri dev`. Test with two packaged versions:

1. Install and open the older packaged version.
2. Publish a newer version whose updater metadata targets the same architecture.
3. Check for updates from Settings and verify the displayed version and release notes.
4. Download the update and verify progress is visible.
5. Confirm the application relaunches and reports the new version.
6. Confirm settings, active context, history, and indexes remain intact.
7. Repeat the global-shortcut, folder-picker, editor-launch, and focus-loss checks after updating.

Until Developer ID signing and notarization are enabled, also confirm the manual macOS approval experience for the updated application.

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
