# Rust Authenticator 🔐

A secure, cross-platform 2FA authenticator built in Rust.

## Features
- TOTP (RFC 6238 compliant)
- Offline code generation
- Modular architecture (CLI, desktop, mobile)

## Roadmap
- [x] Core TOTP engine
- [x] CLI interface
- [ ] Encrypted storage (AES-256-GCM)
- [ ] QR code parsing
- [ ] Desktop app (Tauri)
- [ ] Mobile apps (iOS + Android)

## Security Goals
- No plaintext secrets
- Memory-safe Rust implementation
- Hardware-backed key storage (future)