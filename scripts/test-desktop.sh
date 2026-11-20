#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export CARGO_HOME="$PWD/.tooling/cargo" RUSTUP_HOME="$PWD/.tooling/rustup" COREPACK_HOME="$PWD/.tooling/corepack"
export PATH="$CARGO_HOME/bin:$PATH"
corepack pnpm build
cargo build --release --manifest-path src-tauri/Cargo.toml --features tauri/custom-protocol --example desktop_smoke
python3 scripts/test-local-agent.py --desktop
