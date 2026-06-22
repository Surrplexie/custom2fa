Dev logs rules; REM-1, REM-2 any of these terms are the new 'updates' like "Update 1", all major logs entered here. Note; REM-1.2, REM-1.4 etc. are only for GitHub commits and pushing, never named here.

## REM-1

* Project creation, temporary and file code creation
* Setup on folder and proper files for continuing

## REM-2

* Updating dependencies
* Developing encryption
* Adding accounts

## REM-3
- Implemented encrypted account storage (AES-256-GCM, PBKDF2 key derivation) so secrets are not written to disk in plaintext.
- Expanded CLI: add, list, code, `otpauth://` URI import, QR image import, and encrypted backup export/import with re-encryption into the local database passphrase.
- Added `otp_uri` parsing module and offline QR decoding path (`image` + `rqrr`) for standard TOTP provisioning URIs.
- Hardened sensitive paths with `zeroize` for derived keys, passphrase byte copies, and backup plaintext handling where applicable.
- Added unit tests for encrypt/decrypt roundtrip, wrong-passphrase failure, and basic OTP URI parsing.
## REM-4
- Added a desktop GUI hub (`custom2fa_desktop`) for offline account management without requiring CLI commands.
- Implemented hidden passphrase prompt flow in CLI with optional argument fallback for automation.
- Added account management UX in desktop app: search, select, edit, and delete stored accounts.
- Integrated OS keychain support to save/load/clear the database passphrase securely from the GUI.
- Added camera-based QR scan import (single-frame capture) alongside existing QR image file import.
- Updated workspace/build configuration and docs so core, CLI, and desktop apps compile and run together.
## REM-2
- Verified end-to-end real-world setup flow: account import, encrypted storage load, and live 6-digit TOTP generation in GUI.
- Confirmed successful activation/use of authenticator-based 2FA in production-style account setup flow.
- Improved QR import handling by normalizing pasted file paths (including accidental surrounding quotes) and adding clearer error feedback.
- Validated desktop build outputs and launch process for `custom2fa_desktop.exe` with current workspace configuration.
- Documented usage guidance for passphrase handling, code generation workflow, and offline recovery/backup expectations.
## REM-3
- Added a visible **Copy Code** action in the desktop hub that copies the current TOTP to the OS clipboard; documented empty-state behavior in the app status line.
- Wrote a formal release runbook: `authenticator/docs/RELEASE.md` (test, `cargo build --release`, artifact paths, smoke test, optional git tag, GitHub release asset guidance).
- Linked the release doc from the documentation index in `authenticator/docs/README.md`.
- Confirmed `cargo check -p custom2fa_desktop` after copy-button layout fix so the shipping GUI build includes the control without requiring a prior code generation.
## REM-4
- Extended `Account` and vault JSON with **TOTP algorithm** (`SHA1` / `SHA256` / `SHA512`), **period** (seconds), and **digits**; older vaults still load with defaults **SHA1 · 30s · 6 digits** via serde defaults.
- Implemented RFC 6238 `otpauth://` parsing for `algorithm`, `period`, and `digits`; code generation uses **HMAC-SHA1/256/512** with dynamic truncation and unit tests aligned with RFC 6238 Appendix B vectors.
- CLI: `add` accepts optional `--algorithm`, `--period`, `--digits`; `list` shows parameters per row; `code` formats width to match stored digit count.
- Desktop hub: manual add and account editor include algorithm selection plus period and digits fields; **Generate Current Code** follows each account’s stored parameters.
- Workspace crates bumped to **0.2.0** (`custom2fa_core`, `custom2fa_cli`, `custom2fa_desktop`). Release runbook and user guide updated for this cut.
## REM-6
- Atomic vault and backup writes: temp file in the same directory, `fsync`, then rename/replace so a crash mid-write cannot corrupt an existing `.c2fa` vault or backup JSON.
- Added `Account::zeroize_secrets` and `zeroize_accounts` in core for scrubbing decoded TOTP secret bytes from memory.
- Desktop hub scrubs sensitive memory on **Lock Vault**, **app exit** (`on_exit`), and **Drop**: account secrets, vault/backup passphrases, manual-add/edit secret fields, and cached live TOTP strings.
- Added storage unit tests for atomic write roundtrip, replace-in-place, and no leftover `.tmp` files after a successful save.
## REM-5

## REM-3

## REM-4

## REM-5

## REM-6

## REM-4

## REM-5

## REM-6

## REM-7

## REM-5

## REM-6

## REM-7

## REM-8

## REM-6

## REM-7

## REM-8

## REM-9

## REM-7

## REM-8

## REM-9

## REM-10

## REM-8

## REM-9

## REM-10

## REM-11

## REM-9

## REM-10

## REM-11

## REM-12

## REM-10

## REM-11

## REM-12

## REM-13

## REM-11

## REM-12

## REM-13

## REM-14

## REM-12

## REM-13

## REM-14

## REM-15

## REM-13

## REM-14

## REM-15

## REM-16

## REM-14

## REM-15

## REM-16

## REM-17

## REM-15

## REM-16

## REM-17

## REM-18

## REM-16

## REM-17

## REM-18

## REM-19

## REM-17

## REM-18

## REM-19

## REM-20

## REM-18

## REM-19

## REM-20

## REM-21

## REM-19

## REM-20

## REM-21

## REM-22

## REM-20

## REM-21

## REM-22

## REM-23

## REM-21

## REM-22

## REM-23

## REM-24

## REM-22

## REM-23

## REM-24

## REM-25

## REM-23

## REM-24

## REM-25

## REM-26

## REM-24

## REM-25

## REM-26

## REM-27

## REM-25

## REM-26

## REM-27

## REM-28

## REM-26

## REM-27

## REM-28

## REM-29

## REM-27

## REM-28

## REM-29

## REM-30

## REM-28

## REM-29

## REM-30

## REM-31

## REM-29

## REM-30

## REM-31

## REM-32

## REM-30

## REM-31

## REM-32

## REM-33

## REM-31

## REM-32

## REM-33

## REM-34

## REM-32

## REM-33

## REM-34

## REM-35

## REM-33

## REM-34

## REM-35

## REM-36

## REM-34

## REM-35

## REM-36

## REM-37

## REM-35

## REM-36

## REM-37

## REM-38

## REM-36

## REM-37

## REM-38

## REM-39

## REM-37

## REM-38

## REM-39

## REM-40

## REM-38

## REM-39

## REM-40

## REM-41

## REM-39

## REM-40

## REM-41

## REM-42

## REM-40

## REM-41

## REM-42

## REM-43

## REM-41

## REM-42

## REM-43

## REM-44

## REM-42

## REM-43

## REM-44

## REM-45

## REM-43

## REM-44

## REM-45

## REM-46

## REM-44

## REM-45

## REM-46

## REM-47

## REM-45

## REM-46

## REM-47

## REM-48

## REM-46

## REM-47

## REM-48

## REM-49

## REM-47

## REM-48

## REM-49

## REM-50

## REM-48

## REM-49

## REM-50

## REM-51

## REM-49

## REM-50

## REM-51

## REM-52

## REM-50

## REM-51

## REM-52

## REM-53

## REM-51

## REM-52

## REM-53

## REM-54

## REM-52

## REM-53

## REM-54

## REM-55

## REM-53

## REM-54

## REM-55

## REM-56

## REM-54

## REM-55

## REM-56

## REM-57

## REM-55

## REM-56

## REM-57

## REM-58

## REM-56

## REM-57

## REM-58

## REM-59

## REM-57

## REM-58

## REM-59

## REM-60

## REM-58

## REM-59

## REM-60

## REM-61

## REM-59

## REM-60

## REM-61

## REM-62

## REM-60

## REM-61

## REM-62

## REM-63

## REM-61

## REM-62

## REM-63

## REM-64

## REM-62

## REM-63

## REM-64

## REM-65

## REM-63

## REM-64

## REM-65

## REM-66

## REM-64

## REM-65

## REM-66

## REM-67

## REM-65

## REM-66

## REM-67

## REM-68

## REM-66

## REM-67

## REM-68

## REM-69

## REM-67

## REM-68

## REM-69

## REM-70

## REM-68

## REM-69

## REM-70

## REM-71

## REM-69

## REM-70

## REM-71

## REM-72

## REM-70

## REM-71

## REM-72

## REM-73

## REM-71

## REM-72

## REM-73

## REM-74

## REM-72

## REM-73

## REM-74

## REM-75

## REM-73

## REM-74

## REM-75

## REM-76

## REM-74

## REM-75

## REM-76

## REM-77

## REM-75

## REM-76

## REM-77

## REM-78

## REM-76

## REM-77

## REM-78

## REM-79

## REM-77

## REM-78

## REM-79

## REM-80

## REM-78

## REM-79

## REM-80

## REM-81

## REM-79

## REM-80

## REM-81

## REM-82

## REM-80

## REM-81

## REM-82

## REM-83

## REM-81

## REM-82

## REM-83

## REM-84

## REM-82

## REM-83

## REM-84

## REM-85

## REM-83

## REM-84

## REM-85

## REM-86

## REM-84

## REM-85

## REM-86

## REM-87

## REM-85

## REM-86

## REM-87

## REM-88

## REM-86

## REM-87

## REM-88

## REM-89

## REM-87

## REM-88

## REM-89

## REM-90

## REM-88

## REM-89

## REM-90

## REM-91

## REM-89

## REM-90

## REM-91

## REM-92

## REM-90

## REM-91

## REM-92

## REM-93

## REM-91

## REM-92

## REM-93

## REM-94

## REM-92

## REM-93

## REM-94

## REM-95

## REM-93

## REM-94

## REM-95

## REM-96

## REM-94

## REM-95

## REM-96

## REM-97

## REM-95

## REM-96

## REM-97

## REM-98

## REM-96

## REM-97

## REM-98

## REM-99

## REM-97

## REM-98

## REM-99

## REM-100

## REM-98

## REM-99

## REM-100

## REM-101

## REM-99

## REM-100

## REM-101

## REM-102

## REM-100

## REM-101

## REM-102

## REM-103

## REM-101

## REM-102

## REM-103

## REM-104

## REM-102

## REM-103

## REM-104

## REM-105

## REM-103

## REM-104

## REM-105

## REM-106

## REM-104

## REM-105

## REM-106

## REM-107

## REM-105

## REM-106

## REM-107

## REM-108

## REM-106

## REM-107

## REM-108

## REM-109

## REM-107

## REM-108

## REM-109

## REM-110

## REM-108

## REM-109

## REM-110

## REM-111

## REM-109

## REM-110

## REM-111

## REM-112

## REM-110

## REM-111

## REM-112

## REM-113

## REM-111

## REM-112

## REM-113

## REM-114

## REM-112

## REM-113

## REM-114

## REM-115

## REM-113

## REM-114

## REM-115

## REM-116

## REM-114

## REM-115

## REM-116

## REM-117

## REM-115

## REM-116

## REM-117

## REM-118

## REM-116

## REM-117

## REM-118

## REM-119

## REM-117

## REM-118

## REM-119

## REM-120

## REM-118

## REM-119

## REM-120

## REM-121

## REM-119

## REM-120

## REM-121

## REM-122

## REM-120

## REM-121

## REM-122

## REM-123

## REM-121

## REM-122

## REM-123

## REM-124

## REM-122

## REM-123

## REM-124

## REM-125

## REM-123

## REM-124

## REM-125

## REM-126

## REM-124

## REM-125

## REM-126

## REM-127

## REM-125

## REM-126

## REM-127

## REM-128

## REM-126

