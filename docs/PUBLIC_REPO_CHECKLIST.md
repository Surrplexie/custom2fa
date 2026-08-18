# Public GitHub checklist (security + hygiene)

This repository is intended to be **public**. Treat anything that can decrypt or recover secrets as **sensitive**.

## Never commit

- Real vault files: `*.c2fa`
- Real backup exports (encrypted JSON you generated for yourself)
- Screenshots that still encode a valid provisioning QR
- `.env` files with secrets
- Private keys: `*.pem`, `*.key`, etc.

The repo-root `.gitignore` helps, but it **does not** retroactively remove already-tracked files from git history.

## Recommended developer workflow

1. Create `local/` at repo root (ignored by git).
2. Store your personal vault as `local/accounts.c2fa` (or similar).
3. Point the app’s **Database file** field to that path.
4. Keep **proof screenshots** only for non-sensitive UI states, or redact QR payloads.

## If you accidentally pushed a secret

Rotate the affected 2FA enrollment on the service (disable old authenticator, re-enable), and consider the leaked material compromised.
