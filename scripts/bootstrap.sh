#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tooling_root="$project_root/.tooling"
rustup_home="$tooling_root/rustup"
cargo_home="$tooling_root/cargo"
corepack_home="$tooling_root/corepack"
rustup_installer="$tooling_root/rustup-init.sh"

export RUSTUP_HOME="$rustup_home"
export CARGO_HOME="$cargo_home"
export COREPACK_HOME="$corepack_home"
export PATH="$cargo_home/bin:$PATH"

mkdir -p "$tooling_root" "$corepack_home"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Installing the stable Rust toolchain into $tooling_root"
  curl --fail --silent --show-error --location \
    https://sh.rustup.rs \
    --output "$rustup_installer"
  sh "$rustup_installer" \
    --default-toolchain stable \
    --profile minimal \
    --no-modify-path \
    --yes
fi

if ! command -v corepack >/dev/null 2>&1; then
  echo "Error: Node.js Corepack is required." >&2
  exit 1
fi

cd "$project_root"

echo "Activating the project pnpm version"
corepack pnpm --version

echo "Installing JavaScript dependencies"
corepack pnpm install --frozen-lockfile=false

echo "Running frontend validation"
corepack pnpm lint
corepack pnpm typecheck
corepack pnpm test
corepack pnpm build

echo "Running Rust validation"
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features

echo "Bootstrap and validation completed successfully."
