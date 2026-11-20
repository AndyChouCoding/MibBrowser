"""Generate installer notices from project-local, locked Rust and JS packages."""
import hashlib
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
packages = {}
notices = {}


def register(name, version, license_id, source, directory):
    ids = []
    for path in sorted(directory.iterdir()):
        if path.is_file() and path.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT")):
            content = "\n".join(line.rstrip() for line in path.read_text(encoding="utf-8", errors="replace").splitlines()).strip()
            digest = hashlib.sha256(content.encode()).hexdigest()[:16]
            if content:
                notices[digest] = content
                ids.append(digest)
    packages[name, version, source] = (license_id, ids)


for target in ("aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-pc-windows-msvc"):
    data = json.loads(subprocess.check_output([
        "cargo", "metadata", "--locked", "--offline", "--manifest-path", "src-tauri/Cargo.toml",
        "--format-version", "1", "--filter-platform", target], cwd=root))
    for pkg in data["packages"]:
        if pkg["source"]:
            register(pkg["name"], pkg["version"], pkg["license"] or pkg["license_file"],
                     "Rust", Path(pkg["manifest_path"]).parent)

js = json.loads(subprocess.check_output(["corepack", "pnpm", "licenses", "list", "--prod", "--json"], cwd=root))
for group in js.values():
    for pkg in group:
        for directory in map(Path, pkg["paths"]):
            manifest = json.loads((directory / "package.json").read_text())
            register(pkg["name"], manifest["version"], pkg["license"], "npm", directory)

output = ["Third-party notices", "", "Locked dependencies for macOS and Windows, including build dependencies.",
          "The reference after each package identifies the corresponding upstream notice below.", ""]
for (name, version, source), (license_id, ids) in sorted(packages.items()):
    upstream = f"https://crates.io/crates/{name}/{version}" if source == "Rust" else f"https://www.npmjs.com/package/{name}/v/{version}"
    output.append(f"{source} {name} {version} | {license_id} | {', '.join(ids) or upstream}")
for digest, content in sorted(notices.items()):
    output.extend(["", f"----- {digest} -----", content])
(root / "src-tauri/resources/THIRD-PARTY.txt").write_text("\n".join(output) + "\n")
