# User guide — Custom2FA (Rust)

This project is an **offline-first TOTP authenticator**: it generates time-based one-time codes from secrets you import. It does **not** phone home; network access is not required for normal use.

## What you run

| App | Crate | Typical use |
|-----|-------|-------------|
| **Desktop hub** | `custom2fa_desktop` | Live account cards, search, categories, QR import, camera scan |
| **CLI** | `custom2fa_cli` | Scripting, terminals, automation |

Rust workspace root: `authenticator/`.

## Prerequisites

- [Rust / `cargo`](https://rustup.rs) installed and on `PATH`.

Windows note: if `cargo` is not found in PowerShell, call it explicitly:

`& "$env:USERPROFILE\.cargo\bin\cargo.exe" --version`

## Build and run (quick)

From `authenticator/`:

```powershell
cargo build --workspace --release
```

Run GUI (dev profile, faster compile):

```powershell
cargo run -p custom2fa_desktop
```

Run CLI:

```powershell
cargo run -p custom2fa_cli -- --help
```

Exact output locations for binaries are documented in [BUILD_AND_LAYOUT.md](BUILD_AND_LAYOUT.md).

## Encrypted database (`*.c2fa`)

- The desktop app stores the last-used vault path in a per-user config file (`config.json` under the OS config directory, e.g. `%APPDATA%\custom2fa` on Windows). The default vault location is the OS local-data directory (`%LOCALAPPDATA%\custom2fa\accounts.c2fa` on Windows).
- The file is **encrypted at rest** with a **database passphrase** you invent. It is **not** provided by Discord, Google, etc.
- You must use the **same passphrase** each time for the **same database file**.
- If you forget the passphrase, the vault file cannot be decrypted; you must create a new DB and re-import secrets from the services’ 2FA reset flows.
- You can rotate the passphrase from **Settings → Change vault passphrase** (desktop) or `change-passphrase` (CLI) without re-importing accounts.

### Recommended layout for a public repo clone

Create a folder that stays **out of git** (see repo-root `.gitignore`):

- `local/` at repository root — put your real `accounts.c2fa` and exports here.
- In the GUI, use **Browse…** or set **Database file** to something like `..\local\accounts.c2fa`.

## Desktop hub — typical workflow

1. Launch the app. If you previously saved the passphrase to the OS keychain and **Unlock from keychain on launch** is enabled, the vault opens automatically.
2. Otherwise open **Settings**: confirm the vault path (Browse…), enter the passphrase, optionally save it to the keychain, then **Load Accounts**.
3. The **Accounts** view shows live TOTP cards with a countdown. Click the code (or **Copy**) to copy it. Copied codes are cleared from the clipboard after 30 seconds if the clipboard still holds that code.
4. Add or import from **+ Add / Import** (or Ctrl+N):
   - **Manual secret**: issuer, label, Base32 secret (spaces, hyphens, and padding are stripped), optional category, algorithm / period / digits.
   - **OTP URI**: paste full `otpauth://totp/...`.
   - **QR image**: Browse… or paste a path to a PNG/JPG. Surrounding quotes are trimmed.
   - **Camera index**: integer such as `0`. **Scan QR From Camera** captures a single frame.
5. Search with the sidebar box or Ctrl+F. Filter by category. Expand a card for Edit / Delete.
6. **Backup / Restore**: choose a backup file (Browse…), enter a **separate** backup passphrase. Export writes an encrypted JSON. Import asks whether to **Merge** (keep existing accounts, skip duplicate labels) or **Replace** the whole vault.
7. **Lock Vault** (or Ctrl+L) scrubs decrypted secrets from memory. Idle auto-lock (default 5 minutes) does the same.

### Privacy and shortcuts

| Setting / key | Effect |
|---------------|--------|
| Hide codes until clicked | Cards show dots until you click to reveal and copy |
| Idle auto-lock | Off / 1 / 5 / 15 minutes |
| Ctrl+F | Focus search |
| Ctrl+N | Add / Import |
| Ctrl+L | Lock vault |
| Esc | Close the current dialog |

## CLI — command overview

Global flags:

- `--db <path>` — database file (default `accounts.c2fa` in the current directory).
- `--passphrase <text>` — optional; if omitted, CLI prompts securely.

Commands:

- `add --issuer ... --label ... --secret <BASE32>` — optional `--algorithm`, `--period`, `--digits`, `--category`. `--force` allows a duplicate secret.
- `edit --label <current>` — optional `--new-label`, `--issuer`, `--secret`, `--algorithm`, `--period`, `--digits`, `--category`.
- `delete --label <label> --yes`
- `list` — optional `--json`
- `code --label <label>` — optional `--watch`
- `import-uri --uri "otpauth://..."` — optional `--force`
- `import-qr --image <path>` — optional `--force`
- `export-backup --backup <path> [--backup-passphrase <text>]`
- `import-backup --backup <path> [--backup-passphrase <text>]` — merges by default; `--replace` overwrites the vault
- `change-passphrase [--new-passphrase <text>]`

## Troubleshooting

| Symptom | Likely cause | What to do |
|---------|----------------|------------|
| “Passphrase cannot be empty” | Entered nothing at prompt | Type passphrase even though it is hidden; press Enter |
| Import QR does nothing / file not found | Bad path, quotes, or truncated filename | Use **Browse…**, or a full path without quotes |
| Invalid code on website | Wrong secret, wrong clock, expired code | Re-import correct secret; enable OS automatic time; generate fresh code |
| “Label already exists” | Duplicate account label | Delete/rename in GUI, or pick a new label in the CLI |
| “This secret is already stored” | Same Base32 secret on another account | Confirm **Add anyway** in the GUI, or pass `--force` in the CLI |
| Camera scan fails | Wrong index / permissions / no QR in frame | Set index `0` or `1`; grant camera permission; center QR |
| Vault locked after a few minutes | Idle auto-lock | Unlock again, or set idle auto-lock to Off in Settings |

## Limitations (current)

- No built-in cloud sync (copy the vault or an encrypted backup yourself).
- QR via **file path** or **single camera frame** — not a continuous live scanner UI.
- **HOTP** (`otpauth://hotp/...`) and other non-TOTP URI types are not supported.
