# MIB Browser

A modern cross-platform MIB browser for macOS and Windows, built with Tauri 2, React, TypeScript, and Rust.

## Status

The repository currently contains the approved application scaffold and UI foundation. Rust SNMP/MIB integration is gated on the Phase 0 compatibility spike.

## Development prerequisites

- Node.js 24
- pnpm 10
- Current stable Rust toolchain
- Platform prerequisites from the [Tauri documentation](https://v2.tauri.app/start/prerequisites/)

## Commands

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
