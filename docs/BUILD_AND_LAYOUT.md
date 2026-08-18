# Build, layout, and artifact locations

## Repository layout (high level)

```text
custom2fa/                      ← Git repository root
  README.md                     ← Project landing page + screenshots
  .gitignore                    ← Public-repo hygiene (secrets, target/, local/)
  docs/
    images/                     ← Proof / marketing screenshots (tracked on purpose)
  authenticator/                ← Rust workspace (core + CLI + desktop)
    Cargo.toml                  ← workspace members
    Cargo.lock                  ← Pinned dependencies
    core/                       ← Library: crypto, storage, TOTP, OTP URI, QR decode
    cli/                        ← `custom2fa_cli`
    desktop/                    ← `custom2fa_desktop` (egui window)
    docs/                       ← Detailed guides (this folder)
```

## Build commands

Always run Cargo from `authenticator/` unless you pass `--manifest-path`.

Check compile:

```powershell
cd authenticator
cargo check --workspace
```

Release build (recommended for sharing a local `.exe`):

```powershell
cd authenticator
cargo build --workspace --release
```

Run without packaging:

```powershell
cd authenticator
cargo run -p custom2fa_desktop
cargo run -p custom2fa_cli -- list
```

## Where Windows `.exe` files appear

After `cargo build --release` from `authenticator/`:

| Binary | Release path |
|--------|----------------|
| Desktop hub | `authenticator/target/release/custom2fa_desktop.exe` |
| CLI | `authenticator/target/release/custom2fa_cli.exe` |

Debug builds (faster iteration, larger/slower binaries):

| Binary | Debug path |
|--------|------------|
| Desktop hub | `authenticator/target/debug/custom2fa_desktop.exe` |
| CLI | `authenticator/target/debug/custom2fa_cli.exe` |

## Workspace crates

| Path | Package name | Role |
|------|--------------|------|
| `authenticator/core` | `custom2fa_core` | Shared library |
| `authenticator/cli` | `custom2fa_cli` | Terminal interface |
| `authenticator/desktop` | `custom2fa_desktop` | GUI hub |

## Tests

From `authenticator/`:

```powershell
cargo test --workspace
```
