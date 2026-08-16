Dev log. Major named updates are REM-1, REM-2, … (REM-1.2 style suffixes are commit-only and are not listed here.)

## REM-1

* Project creation, temporary and file code creation
* Setup on folder and proper files for continuing

## REM-2

* Updating dependencies
* Developing encryption
* Adding accounts

## REM-3

- Implemented encrypted account storage (AES-256-GCM, PBKDF2 key derivation) so secrets are not written to disk in plaintext.
- Expanded CLI: add, list, code, `otpauth://` URI import, QR image import, and encrypted backup export/import with re-encryption into the local database passphrase.
- Added `otp_uri` parsing module and offline QR decoding path (`image` + `rqrr`) for standard TOTP provisioning URIs.
- Hardened sensitive paths with `zeroize` for derived keys, passphrase byte copies, and backup plaintext handling where applicable.
- Added unit tests for encrypt/decrypt roundtrip, wrong-passphrase failure, and basic OTP URI parsing.

## REM-4

- Added a desktop GUI hub (`custom2fa_desktop`) for offline account management without requiring CLI commands.
- Implemented hidden passphrase prompt flow in CLI with optional argument fallback for automation.
- Added account management UX in desktop app: search, select, edit, and delete stored accounts.
- Integrated OS keychain support to save/load/clear the database passphrase securely from the GUI.
- Added camera-based QR scan import (single-frame capture) alongside existing QR image file import.
- Updated workspace/build configuration and docs so core, CLI, and desktop apps compile and run together.

## REM-5

- Verified end-to-end real-world setup flow: account import, encrypted storage load, and live 6-digit TOTP generation in GUI.
- Confirmed successful activation/use of authenticator-based 2FA in production-style account setup flow.
- Improved QR import handling by normalizing pasted file paths (including accidental surrounding quotes) and adding clearer error feedback.
- Validated desktop build outputs and launch process for `custom2fa_desktop.exe` with current workspace configuration.
- Documented usage guidance for passphrase handling, code generation workflow, and offline recovery/backup expectations.
- Added a visible **Copy Code** action in the desktop hub; wrote the release runbook in `authenticator/docs/RELEASE.md`.

## REM-6

- Extended `Account` and vault JSON with TOTP algorithm (`SHA1` / `SHA256` / `SHA512`), period, and digits; older vaults still load with defaults SHA1 · 30s · 6 digits.
- Implemented RFC 6238 `otpauth://` parsing for `algorithm`, `period`, and `digits`; unit tests aligned with RFC 6238 Appendix B.
- Atomic vault and backup writes: temp file in the same directory, `fsync`, then rename/replace.
- Desktop hub scrubs sensitive memory on Lock Vault, app exit, and Drop.
- Workspace crates at **0.2.0**.

## REM-7

- Desktop QoL: remembered vault path and window size, native file pickers, masked secret fields, 30s clipboard clear, idle auto-lock, hide-codes toggle, keyboard shortcuts (Ctrl+F / Ctrl+N / Ctrl+L).
- Default vault path is the OS local-data directory instead of `!2fa` / CWD; optional keychain auto-unlock on launch.
- `decode_secret` strips spaces, hyphens, and Base32 padding (matching the user guide).
- Change vault passphrase in Settings and via CLI `change-passphrase`.
- Backup import **merges** by default (skip duplicate labels); GUI prompts Merge vs Replace; CLI `--replace` restores overwrite.
- Duplicate-secret warning on add/import (GUI confirm / CLI `--force`).
- CLI: `edit`, `delete --yes`, `--category`, `list --json`, `code --watch`.
- Windows vault replace uses `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` (no delete-then-rename window).
- GitHub Actions CI (`cargo fmt`, `clippy -D warnings`, `cargo test`) on Ubuntu and Windows.
- Workspace crates bumped to **0.3.0**. User guide rewritten for the live-card GUI.
