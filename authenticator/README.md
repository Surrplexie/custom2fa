# Rust Authenticator 🔐

A secure, cross-platform 2FA authenticator built in Rust.

## Features
- TOTP (RFC 6238 compliant)
- Offline code generation
- Modular architecture (CLI, desktop, mobile)
- Encrypted account database (AES-256-GCM + PBKDF2)
- OTP URI import (`otpauth://...`)
- QR image import (offline)
- Encrypted backup export/import with re-encryption

## Roadmap
- [x] Core TOTP engine
- [x] CLI interface
- [x] Encrypted storage (AES-256-GCM + PBKDF2)
- [ ] QR code parsing
- [ ] Desktop app (Tauri)
- [ ] Mobile apps (iOS + Android)

## Security Goals
- No plaintext secrets
- Memory-safe Rust implementation
- Hardware-backed key storage (future)