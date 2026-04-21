# Rust Authenticator

Cross-platform **offline TOTP** tooling: shared Rust **core**, a **CLI**, and a **desktop hub** (egui). See the full guides in [`docs/`](docs/README.md).

## Documentation

- [User guide](docs/USER_GUIDE.md)
- [Build and layout](docs/BUILD_AND_LAYOUT.md)
- [Public repo checklist](docs/PUBLIC_REPO_CHECKLIST.md)

## Features

- TOTP generation (common defaults: SHA-1, 30s step, 6 digits)
- Offline operation (no network dependency for normal use)
- Encrypted account database (`*.c2fa`, AES-256-GCM + PBKDF2)
- `otpauth://` URI import
- QR import from image files; optional single-frame webcam capture (desktop)
- Encrypted backup export/import with passphrase re-encryption
- CLI hidden passphrase prompts; optional OS keychain integration in GUI

## Roadmap

- [x] Core TOTP engine
- [x] CLI interface
- [x] Encrypted storage (AES-256-GCM + PBKDF2)
- [x] OTP URI + QR decoding path
- [x] Desktop hub (egui) — first-party GUI in this workspace
- [ ] Optional Tauri shell / packaging polish (if desired later)
- [ ] Mobile apps (iOS + Android)

## Security goals

- No plaintext secrets on disk inside the vault file
- Memory-safe Rust implementation
- Stronger platform integration (keychain, hardening) incrementally
