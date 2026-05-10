# Release build — steps and checklist

Use this when preparing a **public or tagged release** of the `authenticator` workspace (CLI + desktop + core).

## Current release line

| Crate | Version | Notes |
|-------|---------|--------|
| `custom2fa_core` | 0.2.0 | Full `otpauth` TOTP parameters; RFC 6238 algorithms |
| `custom2fa_cli` | 0.2.0 | Matches core |
| `custom2fa_desktop` | 0.2.0 | Matches core |

Bump all three `version = "..."` fields together when you cut the next SemVer release.

## 1) Preconditions

- Rust toolchain installed (`rustup` / stable channel).
- Working directory: `authenticator/` (the folder that contains the workspace `Cargo.toml`).
- For a public GitHub push: re-read [PUBLIC_REPO_CHECKLIST.md](PUBLIC_REPO_CHECKLIST.md) and confirm no local vault/backup/secret files are tracked.

## 2) Clean verification

```powershell
cd path\to\custom2fa\authenticator
cargo test --workspace
cargo check --workspace
```

Resolve any failures before tagging or publishing binaries.

## 3) Release binaries (Windows)

```powershell
cargo build --workspace --release
```

**Outputs:**

| App | Path |
|-----|------|
| Desktop hub | `authenticator\target\release\custom2fa_desktop.exe` |
| CLI | `authenticator\target\release\custom2fa_cli.exe` |

(Paths relative to the repository; adjust drive prefix as needed.)

## 4) Smoke test (before publishing)

**Desktop**

- Launch `custom2fa_desktop.exe`: open DB, **Load Accounts**.
- Add or select an account that uses **non-default** TOTP settings (e.g. SHA256, 60s period, or 8 digits) if you have a test vault; **Generate Current Code** and **Copy Code**.
- Confirm manual add: change algorithm / period / digits, save, regenerate code.

**CLI**

- `custom2fa_cli --db <path> list` — rows should show `[ALGORITHM · Ns · M digits]`.
- `custom2fa_cli --db <path> code --label <label>` — output width matches digit count.

**Backups (optional but recommended)**

- Export backup with a throwaway passphrase; import into a **copy** of the DB path and confirm accounts load.

## 5) Versioning, changelog, and git tag

1. Set matching `version = "x.y.z"` in:
   - `authenticator/core/Cargo.toml`
   - `authenticator/cli/Cargo.toml`
   - `authenticator/desktop/Cargo.toml`
2. Append an entry to `authenticator/Dev logs/README.md` with date and highlights.
3. Adjust [USER_GUIDE.md](USER_GUIDE.md) if behavior or CLI flags changed.
4. Commit with a clear message, for example: `Release v0.2.0: full otpauth TOTP parameters`.

**Optional annotated tag** (from repository root):

```text
git tag -a v0.2.0 -m "Release v0.2.0: otpauth algorithm/period/digits + RFC 6238 TOTP"
git push origin v0.2.0
```

Use the next SemVer in both `Cargo.toml` files and the tag name.

## 6) Artifacts to attach (e.g. GitHub Release)

- `custom2fa_desktop.exe`
- `custom2fa_cli.exe`
- Link to [USER_GUIDE.md](USER_GUIDE.md) and [BUILD_AND_LAYOUT.md](BUILD_AND_LAYOUT.md)

Do **not** attach real `*.c2fa` or personal backup files.

## 7) Quick copy-paste (maintainer)

```powershell
cd path\to\custom2fa\authenticator
cargo test --workspace
cargo build --workspace --release
# Smoke-test the two .exe files, then tag if desired.
```
