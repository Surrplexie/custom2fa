# Custom2FA Desktop Hub

Simple GUI for the existing Rust core so you can manage 2FA without CLI commands.

## Run

From `authenticator/`:

`cargo run -p custom2fa_desktop`

## What it supports

- Load encrypted account DB (`accounts.c2fa` by default)
- Add account from manual Base32 secret
- Import from `otpauth://` URI
- Import from QR image file
- Import from camera QR snapshot (single-frame scan)
- Generate current 6-digit TOTP code
- Search, edit, and delete accounts
- Save/load DB passphrase to OS keychain
- Export/import encrypted backups