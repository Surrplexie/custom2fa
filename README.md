# custom2fa

An offline-first, cross-platform **TOTP authenticator** built in Rust. Generates time-based one-time codes entirely on your device — no cloud, no telemetry, no network dependency for normal use.

> **License:** MIT — This is a personal project. The author provides no warranty and accepts no responsibility for any loss, account lockout, or security incident arising from use. See [LICENSE](LICENSE) for full terms.

---

## Table of Contents

- [Features](#features)
- [Downloads (Pre-built Binaries)](#downloads-pre-built-binaries)
- [Supported Platforms](#supported-platforms)
- [Building from Source](#building-from-source)
- [Desktop Hub — Usage](#desktop-hub--usage)
- [CLI — Usage](#cli--usage)
- [Encrypted Database](#encrypted-database)
- [Importing Accounts](#importing-accounts)
- [Backup and Restore](#backup-and-restore)
- [Troubleshooting](#troubleshooting)
- [Project Structure](#project-structure)
- [Documentation](#documentation)
- [Security Notes](#security-notes)
- [License](#license)

---

## Features

- **TOTP generation** — RFC 6238 compliant; supports SHA-1, SHA-256, and SHA-512; configurable period and digit count (defaults: SHA-1, 30 s, 6 digits)
- **Fully offline** — no internet connection required for code generation
- **Encrypted vault** — accounts stored in an `*.c2fa` file encrypted with AES-256-GCM + PBKDF2
- **Modern desktop GUI** — dark theme, sidebar navigation, live-refresh account cards with countdown timers, category grouping, search filter, one-click copy
- **CLI** — scriptable terminal interface for automation and headless environments
- **OTP URI import** — paste any `otpauth://totp/…` URI
- **QR import** — decode from an image file (PNG/JPG) or a single webcam frame (desktop)
- **Encrypted backup** — export/import with a separate backup passphrase; re-encrypts into your current vault passphrase on import
- **OS keychain integration** — optional passphrase storage via Windows Credential Manager (desktop)
- **Account categories** — organise accounts into named groups; filterable in the sidebar
- **Memory-safe** — implemented entirely in Rust; no plaintext secrets written to disk inside the vault

---

## Downloads (Pre-built Binaries)

Pre-built releases for Windows and Linux are available on the **[Releases page](https://github.com/Surrplexie/custom2fa/releases)**.

| Platform | Desktop GUI | CLI |
|----------|-------------|-----|
| Windows (x86-64) | `custom2fa_desktop.exe` | `custom2fa_cli.exe` |
| Linux (x86-64) | `custom2fa_desktop` | `custom2fa_cli` |

**Download steps:**

1. Go to [https://github.com/Surrplexie/custom2fa/releases](https://github.com/Surrplexie/custom2fa/releases).
2. Expand the latest release's **Assets** section.
3. Download the archive for your platform (e.g., `custom2fa-windows-x86_64.zip` or `custom2fa-linux-x86_64.tar.gz`).
4. Extract the archive to a folder of your choice.
5. Run the executable directly — no installer required.

> **Windows note:** Windows Defender SmartScreen may warn about an unrecognized binary. Click **More info → Run anyway** if you trust the source. The executables are not code-signed.

> **Linux note:** After extracting, mark the binaries executable if needed:
> ```bash
> chmod +x custom2fa_desktop custom2fa_cli
> ```

---

## Supported Platforms

| Platform | Desktop GUI | CLI |
|----------|:-----------:|:---:|
| Windows 10/11 (x86-64) | ✓ | ✓ |
| Linux (x86-64, glibc) | ✓ | ✓ |
| macOS | Not tested | Not tested |

> The codebase is cross-platform Rust; macOS may compile and run, but it is not officially tested or supported.

---

## Building from Source

### Prerequisites

- **Rust toolchain** — install via [https://rustup.rs](https://rustup.rs) (stable channel recommended)
- **Linux only** — the egui desktop crate requires system packages for display/graphics:
  ```bash
  # Debian / Ubuntu
  sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
                   libxkbcommon-dev libssl-dev pkg-config
  ```

### Build (release)

All Cargo commands must be run from the `authenticator/` subdirectory.

**Windows (PowerShell):**
```powershell
cd authenticator
cargo build --workspace --release
```

**Linux / macOS (bash):**
```bash
cd authenticator
cargo build --workspace --release
```

### Output locations

| Binary | Windows | Linux |
|--------|---------|-------|
| Desktop GUI | `authenticator/target/release/custom2fa_desktop.exe` | `authenticator/target/release/custom2fa_desktop` |
| CLI | `authenticator/target/release/custom2fa_cli.exe` | `authenticator/target/release/custom2fa_cli` |

### Run without installing

```powershell
# Desktop
cargo run -p custom2fa_desktop

# CLI (pass -- to separate cargo flags from app flags)
cargo run -p custom2fa_cli -- --help
```

### Run tests

```bash
cargo test --workspace
```

---

## Desktop Hub — Usage

### Interface overview

The window is divided into a **left sidebar** and a **main content area**.

**Left sidebar:**

- App title and vault status (open / locked)
- **Search box** — filters the account list in real time across issuer, label, and category
- **Category list** — click any category (or "All") to filter the main view; categories come from each account's assigned group
- **Navigation buttons** — Accounts · Add/Import · Backup/Restore · Settings
- **Lock Vault** button at the bottom — clears all decrypted data from memory

**Main area — Accounts view:**

Each account is displayed as a card showing:
- **Issuer** (bold, coloured) and **label**
- **Category** if set
- **Algorithm · digits · period** metadata
- **Live TOTP code** — click the code directly to copy it to the clipboard
- **Countdown progress bar** — turns yellow below 10 seconds, red below 5 seconds
- **Edit** and **Del** buttons — Edit opens a modal dialog; Del shows a confirmation prompt

The codes auto-refresh without any manual action (~4 updates per second for a smooth timer).

### First launch workflow

1. **Launch** `custom2fa_desktop` (or `custom2fa_desktop.exe` on Windows).
2. Open **Settings** (sidebar).
3. **Database file** — enter the path to your `.c2fa` vault file. The default is `accounts.c2fa` in the current working directory. Use a path outside the repository to keep personal data away from source control.
4. **Database passphrase** — enter the master passphrase for that vault. This is invented by you and never sent anywhere.
5. Optionally click **Save to keychain** (Windows Credential Manager) to avoid re-entering the passphrase on next launch.
6. Click **Load Accounts** — the vault is decrypted and you are taken to the Accounts view.

### Adding accounts

Open **+ Add / Import** (sidebar) and choose a tab:

| Tab | How to use |
|-----|-----------|
| **Manual Secret** | Enter issuer, label, Base32 secret, optional category, then choose algorithm (SHA-1/SHA-256/SHA-512), period (15/30/60/90 s), and digits (6/7/8) from the dropdowns |
| **OTP URI** | Paste the full `otpauth://totp/…` string |
| **QR Image** | Paste the full path to a PNG or JPG file (surrounding quotes are stripped automatically) |
| **Camera** | Enter the camera index (e.g., `0`), then click **Scan QR From Camera** for a single-frame capture |

### Editing and deleting accounts

Click **Edit** on any account card. A modal dialog opens with all fields pre-populated. Leave the secret field blank to keep the existing secret. Click **Save Changes** to commit, or **Cancel** to discard.

Click **Del** to open a confirmation dialog before permanent deletion.

### Backup

Open **Backup / Restore** (sidebar), enter a backup file path and a **separate** backup passphrase, then click **Export Backup**. To restore, provide the same backup file and passphrase and click **Import Backup**.

---

## CLI — Usage

All commands accept the following global flags:

| Flag | Description |
|------|-------------|
| `--db <path>` | Path to the vault file (default: `accounts.c2fa` in the current directory) |
| `--passphrase <text>` | Vault passphrase; if omitted the CLI prompts securely (input is hidden) |

### Commands

```
custom2fa_cli [--db <path>] [--passphrase <text>] <COMMAND>
```

| Command | Description |
|---------|-------------|
| `add --issuer <name> --label <label> --secret <BASE32>` | Add an account manually. Optional: `--algorithm SHA1\|SHA256\|SHA512`, `--period <seconds>`, `--digits <n>` |
| `list` | List all account labels in the vault |
| `code --label <label>` | Generate the current TOTP code for an account |
| `import-uri --uri "otpauth://…"` | Import an account from an OTP URI |
| `import-qr --image <path>` | Import an account by decoding a QR image file |
| `export-backup --backup <path> --backup-passphrase <text>` | Export an encrypted backup |
| `import-backup --backup <path> --backup-passphrase <text>` | Import from an encrypted backup |

**Examples:**

```bash
# Add an account
custom2fa_cli --db ~/2fa/accounts.c2fa add --issuer GitHub --label me@example.com --secret JBSWY3DPEHPK3PXP

# List accounts
custom2fa_cli --db ~/2fa/accounts.c2fa list

# Generate a code (passphrase will be prompted)
custom2fa_cli --db ~/2fa/accounts.c2fa code --label me@example.com

# Import from a QR image
custom2fa_cli --db ~/2fa/accounts.c2fa import-qr --image ~/Downloads/qr.png
```

---

## Encrypted Database

- Vault files use the `.c2fa` extension and are encrypted with **AES-256-GCM + PBKDF2**.
- The passphrase is **never** stored in plaintext; it is stretched via PBKDF2 to derive the encryption key.
- If you lose the passphrase, the vault **cannot** be recovered. Re-enroll 2FA on each service using their account recovery flow.
- Keep backups of both the vault file and your backup passphrase in a secure offline location.

### Recommended layout when cloning this repository

To avoid accidentally committing personal vault data, store your live database outside the repository or inside the `local/` folder, which is excluded by `.gitignore`:

```
custom2fa/
  local/              ← ignored by git; safe to put accounts.c2fa here
    accounts.c2fa
    backup-2fa.json
```

In the GUI or CLI, point `--db` at the path outside the repo, e.g.:
- Windows: `C:\Users\you\2fa\accounts.c2fa`
- Linux: `~/2fa/accounts.c2fa`

---

## Importing Accounts

| Method | How |
|--------|-----|
| Manual secret | Enter the Base32 secret from the service's 2FA setup page |
| OTP URI | Use `otpauth://totp/Label?secret=…&issuer=…` (some services provide this) |
| QR image file | Screenshot the QR code, save as PNG/JPG, then import via path |
| Camera (desktop) | Use **Scan QR From Camera** with your webcam showing the QR code |
| Encrypted backup | Import a previously exported `.json` backup file |

> **HOTP** (`otpauth://hotp/…`) and other non-TOTP URI types are **not** supported.

---

## Backup and Restore

**Export a backup:**

```bash
custom2fa_cli --db ~/2fa/accounts.c2fa \
  export-backup --backup ~/2fa/backup-2fa.json \
  --backup-passphrase "your-backup-passphrase"
```

The backup file is independently encrypted with the backup passphrase — it is **not** the same as your vault passphrase.

**Import a backup:**

```bash
custom2fa_cli --db ~/2fa/accounts.c2fa \
  import-backup --backup ~/2fa/backup-2fa.json \
  --backup-passphrase "your-backup-passphrase"
```

On import the accounts are decrypted from the backup and re-encrypted into your current vault passphrase.

Store backup files in at least one location that is physically separate from your primary device (e.g., encrypted USB drive, password manager attachment, printed QR of the Base32 secrets).

---

## Troubleshooting

| Symptom | Likely cause | Resolution |
|---------|-------------|------------|
| "Passphrase cannot be empty" | Nothing typed at the hidden prompt | Type the passphrase — input is invisible by design; press Enter |
| QR import does nothing / file not found | Incorrect path, extra quotes, or truncated filename | Use the full absolute path; try removing surrounding quotes; verify the file exists |
| Generated code rejected by the website | Wrong secret, clock drift, or stale code | Re-import the correct secret; ensure OS time is set to automatic/NTP; generate a fresh code |
| "Label already exists" | Duplicate account label | Delete or rename the existing entry in the GUI, or choose a different label in the CLI |
| Camera scan fails | Wrong camera index, missing permissions, or QR not visible | Try index `0` or `1`; grant camera access in OS settings; center the QR code in the camera frame |
| Windows SmartScreen warning on downloaded `.exe` | Binary is not code-signed | Click **More info → Run anyway** if you trust the release source |
| Linux: missing shared libraries at startup | Required system packages not installed | Install the packages listed in the [Building from Source](#building-from-source) section |

---

## Project Structure

```
custom2fa/
├── README.md                        ← You are here
├── LICENSE                          ← MIT
├── .gitignore
├── authenticator/                   ← Rust workspace
│   ├── Cargo.toml                   ← Workspace manifest (core, cli, desktop)
│   ├── Cargo.lock
│   ├── core/                        ← custom2fa_core — crypto, storage, TOTP engine, OTP URI/QR
│   ├── cli/                         ← custom2fa_cli  — terminal interface
│   ├── desktop/                     ← custom2fa_desktop — egui GUI
│   ├── scripts/                     ← Helper / release scripts
│   └── docs/                        ← Detailed developer and user guides
│       ├── USER_GUIDE.md
│       ├── BUILD_AND_LAYOUT.md
│       ├── RELEASE.md
│       ├── PUBLIC_REPO_CHECKLIST.md
│       └── images/                  ← Screenshots
└── local/                           ← (git-ignored) personal vault data — create this yourself
```

---

## Documentation

Detailed guides live in `authenticator/docs/`:

| Guide | Contents |
|-------|----------|
| [User Guide](authenticator/docs/USER_GUIDE.md) | GUI and CLI usage, database/passphrase behaviour, backups, troubleshooting |
| [Build and Layout](authenticator/docs/BUILD_AND_LAYOUT.md) | Folder map, Cargo commands, binary output locations |
| [Release Steps](authenticator/docs/RELEASE.md) | Test, release build, smoke test, tagging, shipping binaries |
| [Public Repo Checklist](authenticator/docs/PUBLIC_REPO_CHECKLIST.md) | What must never be committed to a public repository |
| [Docs Index](authenticator/docs/README.md) | Table of contents for `authenticator/docs/` |

---

## Screenshots

> Screenshots below show an earlier build. The current GUI features a dark sidebar layout, live countdown cards, and category filtering.

![Screenshot 1](authenticator/docs/images/img%20(1).png)

![Screenshot 2](authenticator/docs/images/img%20(2).png)

![Screenshot 3](authenticator/docs/images/img%20(3).png)

![Screenshot 4](authenticator/docs/images/img%20(4).png)

![Screenshot 5](authenticator/docs/images/img%20(5).png)

![Screenshot 6](authenticator/docs/images/img%20(6).png)

---

## Security Notes

- No plaintext secrets are written inside the vault file.
- The vault uses AES-256-GCM (authenticated encryption) with a PBKDF2-derived key — data integrity is verified on every load.
- There is currently no built-in cloud sync; synchronising the vault file across devices is the user's responsibility.
- This project has **not** undergone a formal third-party security audit. Use accordingly and do not treat it as a production-grade security appliance.
- Memory-safety is provided by Rust's ownership model; sensitive byte buffers are zeroed where practical.

---

## License

This project is released under the **MIT License**.

```
MIT License

Copyright (c) 2026 Surrplexie

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

This is a **personal project** maintained by the author for research and personal use. It is shared publicly as-is, with no guarantees of correctness, security, fitness for purpose, or ongoing maintenance. **The author is not responsible for any loss, account lockout, data loss, or security breach** that results from use of this software.
