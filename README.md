# MIB Browser

A modern cross-platform MIB browser for macOS and Windows, built with Tauri 2, React, TypeScript, and Rust.

## Status

The v1.0.0 development branch includes an offline MIB workspace, SNMP queries and confirmed SET, connection profiles, and history. Release qualification is in progress. See the [user guide and candidate notes](src-tauri/resources/USER-GUIDE.md).

## Development prerequisites

- Node.js 24
- pnpm 10
- Current stable Rust toolchain
- Platform prerequisites from the [Tauri documentation](https://v2.tauri.app/start/prerequisites/)

## Commands

To install the project-local toolchain and dependencies, then run validation:

```sh
sh scripts/bootstrap.sh
```

The bootstrap keeps Rust and Corepack state under the ignored `.tooling/` directory and does not modify the global shell profile.

Individual development commands:

```sh
pnpm install
pnpm tauri dev
pnpm lint
pnpm typecheck
pnpm test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
pnpm tauri build
```

### Windows x64 development

Install Node.js 24 with Corepack, Visual Studio C++ Build Tools with the Windows SDK, and WebView2. Open PowerShell in the repository and run:

```powershell
.\scripts\bootstrap.ps1
corepack.cmd pnpm tauri dev
```

The script installs a project-local Rust toolchain and runs frontend and Rust checks. To build an NSIS installer from the same shell, run `corepack.cmd pnpm tauri build --bundles nsis`.

### Local integration checks

Run `python3 scripts/test-local-agent.py` on macOS with Python 3, the project Rust toolchain, the system SNMPv2-MIB, and a Net-SNMP daemon compiled with SHA-256 and AES. Set `MIB_TEST_AGENT` to its executable. The script uses a temporary loopback port, ephemeral USM credentials and private state. The Apple-supplied 5.6 daemon lacks SHA-256.

To refresh the bundled license inventory, run `python3 scripts/license-inventory.py` with the bootstrap Cargo, Rustup and Corepack environment after fetching locked dependencies for the three desktop targets.

For macOS desktop qualification, run `sh scripts/test-desktop.sh` in a logged-in session with the same local Agent available. The harness launches the actual WebView twice in a unique workspace, exercises search and module controls, and loads a temporary Keychain profile for loopback queries. The runner checks the test database and desktop output for the fixture credentials, then removes its own Keychain entry.

For a Universal macOS candidate, install the x86_64-apple-darwin and aarch64-apple-darwin Rust targets, then run `CI=true corepack pnpm tauri build --target universal-apple-darwin --bundles dmg` in the bootstrap environment. The Windows bundle configuration includes the offline WebView2 installer. Developer ID signing, notarization, Authenticode signing and clean-system installation require the corresponding release environments.
