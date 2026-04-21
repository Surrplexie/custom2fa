# User guide — Custom2FA (Rust)

This project is an **offline-first TOTP authenticator**: it generates time-based one-time codes from secrets you import. It does **not** phone home; network access is not required for normal use.

## What you run

| App | Crate | Typical use |
|-----|-------|-------------|
| **Desktop hub** | `custom2fa_desktop` | Buttons, forms, QR image path, optional camera scan |
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

- The app stores accounts in a file you choose. The GUI default is `accounts.c2fa` in the **current working directory** unless you change **Database file**.
- The file is **encrypted at rest** with a **database passphrase** you invent. It is **not** provided by Discord, Google, etc.
- You must use the **same passphrase** each time for the **same database file**.
- If you forget the passphrase, the vault file cannot be decrypted; you must create a new DB and re-import secrets from the services’ 2FA reset flows.

### Recommended layout for a public repo clone

Create a folder that stays **out of git** (see repo-root `.gitignore`):

- `local/` at repository root — put your real `accounts.c2fa` and exports here.
- In the GUI/CLI, set **Database file** to something like `..\local\accounts.c2fa` (adjust relative path from where you launch the app).

## Desktop hub — typical workflow

1. **Database file** — path to your `.c2fa` vault (use `local\...` for personal data).
2. **Database passphrase** — your master password for that vault.
3. Optional: **Save / Load passphrase to OS keychain** (Windows Credential Manager). This is **per user profile on this PC**; it does not sync to other machines.
4. **Load Accounts** — decrypts and lists accounts in memory for the session UI.
5. Add or import:
   - **Manual secret**: issuer, label, Base32 secret (spaces allowed in the key; the app normalizes).
   - **OTP URI**: paste full `otpauth://totp/...`.
   - **QR image path**: full path to a PNG/JPG screenshot. Surrounding quotes are trimmed automatically.
   - **Camera index**: integer such as `0` (first webcam). Use **Scan QR From Camera** only for live QR, not for file import.
6. **Generate code** — pick label, click **Generate Current Code**. Codes are **6 digits**, **30-second** step, **SHA-1** TOTP (common default).
7. **Backup** — export an encrypted backup JSON with a **separate backup passphrase**; store offline. Import decrypts with backup passphrase and **re-encrypts** into your current DB passphrase.

## CLI — command overview

Global flags:

- `--db <path>` — database file (default `accounts.c2fa`).
- `--passphrase <text>` — optional; if omitted, CLI prompts securely.

Commands (subcommands):

- `add --issuer ... --label ... --secret <BASE32>`
- `list`
- `code --label <label>`
- `import-uri --uri "otpauth://..."`
- `import-qr --image <path>`
- `export-backup --backup <path> --backup-passphrase <text>`
- `import-backup --backup <path> --backup-passphrase <text>`

## Troubleshooting

| Symptom | Likely cause | What to do |
|---------|----------------|------------|
| “Passphrase cannot be empty” | Entered nothing at prompt | Type passphrase even though it is hidden; press Enter |
| Import QR does nothing / file not found | Bad path, quotes, or truncated filename | Use full path; try without quotes; confirm file exists in Explorer |
| Invalid code on website | Wrong secret, wrong clock, expired code | Re-import correct secret; enable OS automatic time; generate fresh code |
| “Label already exists” | Duplicate account label | Delete/rename in GUI, or pick new label in CLI |
| Camera scan fails | Wrong index / permissions / no QR in frame | Set index `0` or `1`; grant camera permission; center QR |

## Limitations (current)

- No built-in cloud sync (copy DB or backup file yourself).
- QR via **file path** or **single camera frame** — not a continuous live scanner UI.
- TOTP parameters are aligned with the most common defaults; exotic URI parameters may not be fully honored end-to-end.
