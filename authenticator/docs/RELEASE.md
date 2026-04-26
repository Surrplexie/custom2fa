# Release build — steps and checklist

Use this when preparing a **public or tagged release** of the `authenticator` workspace (CLI + desktop + core).

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

- Run `custom2fa_desktop.exe`: load DB, pick account, **Generate Current Code**, **Copy Code** (clipboard).
- Run `custom2fa_cli`: `list` / `code` with your test vault.
- If you use backups: one export and one import on a throwaway copy of the DB.

## 5) Versioning and tags (optional)

- Bump version fields in `authenticator/**/Cargo.toml` if you are doing a SemVer cut.
- Commit changelog/dev-log updates.
- Example tag (from repo root):

```text
git tag -a v0.1.0 -m "Release: desktop hub + CLI + encrypted vault"
git push origin v0.1.0
```

## 6) Artifacts to attach (e.g. GitHub Release)

- `custom2fa_desktop.exe`
- `custom2fa_cli.exe`
- Link to [USER_GUIDE.md](USER_GUIDE.md) and [BUILD_AND_LAYOUT.md](BUILD_AND_LAYOUT.md)

Do **not** attach real `*.c2fa` or personal backup files.
