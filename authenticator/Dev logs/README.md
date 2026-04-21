# Development logs

7 April 2026 - 0.0.1 - Update and Patches
- Project creation, temporary and file code creation
- Setup on folder and proper files for continuing

8 April 2026 - 0.0.2 - Update and Patches
- Updating dependencies
- Developing encryption 
- Adding accounts

20 April 2026 - 0.0.3 - Update and Patches
- Implemented encrypted account storage (AES-256-GCM, PBKDF2 key derivation) so secrets are not written to disk in plaintext.
- Expanded CLI: add, list, code, `otpauth://` URI import, QR image import, and encrypted backup export/import with re-encryption into the local database passphrase.
- Added `otp_uri` parsing module and offline QR decoding path (`image` + `rqrr`) for standard TOTP provisioning URIs.
- Hardened sensitive paths with `zeroize` for derived keys, passphrase byte copies, and backup plaintext handling where applicable.
- Added unit tests for encrypt/decrypt roundtrip, wrong-passphrase failure, and basic OTP URI parsing.
