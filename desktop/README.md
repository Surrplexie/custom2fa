# Custom2FA Desktop Hub

Simple GUI for the existing Rust core so you can manage 2FA without CLI commands.

## Run

From `authenticator/`:

`cargo run -p custom2fa_desktop`

## What it supports

- Load encrypted account DB (path remembered across launches)
- Add account from manual Base32 secret (masked; spaces/hyphens ignored)
- Import from `otpauth://` URI
- Import from QR image file (Browse…)
- Import from camera QR snapshot (single-frame scan)
- Live TOTP cards with countdown; click to copy (clipboard clears after 30s)
- Search, categories, edit, and delete
- Hide codes until clicked; idle auto-lock; Ctrl+F / Ctrl+N / Ctrl+L
- Change vault passphrase without re-importing
- Save/load DB passphrase to OS keychain; optional auto-unlock
- Export/import encrypted backups (merge or replace)

Full usage, paths, and troubleshooting: [`../docs/USER_GUIDE.md`](../docs/USER_GUIDE.md).