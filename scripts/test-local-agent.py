"""Run Rust UDP integration tests against a private Net-SNMP process on macOS."""
import os
from pathlib import Path
import secrets
import shutil
import socket
import subprocess
import tempfile
import time
import sys
import sqlite3
import uuid

root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="mib-agent-") as directory:
    tmp = Path(directory)
    env = dict(os.environ, SNMP_PERSISTENT_DIR=directory, SNMPCONFPATH=directory,
               MIBS="", MIB_TEST_MIB="/usr/share/snmp/mibs/SNMPv2-MIB.txt")
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    community, phrase = secrets.token_hex(16), secrets.token_hex(16)
    config = tmp / "fixture.conf"
    config.write_text(f"""agentaddress udp:127.0.0.1:{port}
view fixture included .1.3.6.1.2.1.1
rwcommunity {community} 127.0.0.1 -V fixture
createUser fixtureNone
createUser fixtureAuth SHA-256 "{phrase}"
createUser fixturePriv SHA-256 "{phrase}" AES "{phrase}"
rouser fixtureNone noauth -V fixture
rouser fixtureAuth auth -V fixture
rouser fixturePriv priv -V fixture
""")
    config.chmod(0o600)
    env.update(MIB_TEST_PORT=str(port), MIB_TEST_COMMUNITY=community, MIB_TEST_PASSPHRASE=phrase,
               CARGO_HOME=str(root / ".tooling/cargo"), RUSTUP_HOME=str(root / ".tooling/rustup"))
    local_agent = root / ".tooling/net-snmp-5.9.4/agent/snmpd"
    binary = os.environ.get("MIB_TEST_AGENT") or (str(local_agent) if local_agent.is_file() else shutil.which("snmpd"))
    if not binary:
        raise RuntimeError("Set MIB_TEST_AGENT to a Net-SNMP daemon with SHA-256 support")
    log_path = tmp / "startup.log"
    with log_path.open("w") as log, subprocess.Popen([binary, "-r", "-f", "-Lo", "-C", "-c", str(config), "-p", str(tmp / "agent.pid")],
                          cwd=directory, env=env, stdout=log, stderr=subprocess.STDOUT) as agent:
        try:
            for _ in range(50):
                if agent.poll() is not None:
                    raise RuntimeError("Local Agent could not start: " + log_path.read_text().replace(community, "[redacted]").replace(phrase, "[redacted]")[-1200:])
                if (tmp / "agent.pid").exists():
                    break
                time.sleep(0.1)
            subprocess.run([str(root / ".tooling/cargo/bin/cargo"), "test", "--manifest-path",
                            "src-tauri/Cargo.toml", "--all-features", "isolated_agent_", "--",
                            "--ignored", "--test-threads=1"], check=True, cwd=root, env=env)
            if "--desktop" in sys.argv:
                run_id = str(uuid.uuid4())
                db = Path.home() / "Library/Application Support" / f"com.mibbrowser.qualification.{run_id}" / "workspace.sqlite"
                env["MIB_SMOKE_ID"] = run_id
                try:
                    for phase in ("import", "restart"):
                        result = subprocess.run([str(root / "src-tauri/target/release/examples/desktop_smoke")],
                                                env=dict(env, MIB_SMOKE_PHASE=phase), cwd=root,
                                                capture_output=True, text=True, timeout=130)
                        output = result.stdout + result.stderr
                        if community in output or phrase in output:
                            raise RuntimeError("Test credential appeared in desktop output")
                        print(output, flush=True)
                        result.check_returncode()
                    if not db.is_file() or community.encode() in db.read_bytes() or phrase.encode() in db.read_bytes():
                        raise RuntimeError("Workspace database missing or contains test credential")
                    print("Desktop output and SQLite credential checks passed")
                finally:
                    if db.is_file():
                        with sqlite3.connect(db) as database:
                            row = database.execute("SELECT id FROM profiles WHERE name='loopback-fixture'").fetchone()
                        if row:
                            subprocess.run(["security", "delete-generic-password", "-s", "com.mibbrowser.desktop.profiles",
                                            "-a", str(uuid.UUID(row[0]))], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        finally:
            if agent.poll() is None:
                agent.terminate()
