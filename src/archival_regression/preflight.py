"""Record infrastructure blockers without certifying archival or contract behavior."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import Request, ProxyHandler, HTTPRedirectHandler, build_opener

PASS = "PASS"
FAIL = "FAIL"
INCONCLUSIVE = "INCONCLUSIVE"
NOT_TESTED = "NOT TESTED"


class NoRedirect(HTTPRedirectHandler):
    """A local health endpoint must never forward this probe elsewhere."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise HTTPError(req.full_url, code, "RPC redirects are refused", headers, fp)


def run_probe(name, argv, *, cwd, env=None, timeout=15):
    """Only command execution status; even exit zero proves no campaign behavior."""
    record = {"id": name, "argv": argv, "timeout_seconds": timeout}
    try:
        result = subprocess.run(
            argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
            capture_output=True, text=True, timeout=timeout, check=False,
        )
        record.update(exit_code=result.returncode, stdout=result.stdout,
                      stderr=result.stderr)
        record["result"] = PASS if result.returncode == 0 else INCONCLUSIVE
    except (OSError, subprocess.TimeoutExpired) as error:
        record.update(exit_code=None, result=INCONCLUSIVE,
                      error=f"{type(error).__name__}: {error}")
    return record


def usable_backend(probe):
    """Check advertised availability, not launch/network/protocol correctness."""
    if probe.get("result") != PASS or probe.get("exit_code") != 0:
        return False
    try:
        if probe["id"] == "docker":
            version = json.loads(probe["stdout"])
            return isinstance(version, str) and bool(version.strip())
        if probe["id"] == "podman":
            info = json.loads(probe["stdout"])
            return isinstance(info, dict) and isinstance(info.get("host"), dict)
        if probe["id"] == "native-core":
            return bool(probe["stdout"].strip())
    except (KeyError, ValueError, TypeError, AttributeError):
        return False
    return False


def local_rpc_health(url, *, timeout=5):
    """Read-only localhost probe. Public networks and proxy forwarding are excluded."""
    parsed = urlsplit(url)
    if (parsed.scheme != "http" or parsed.hostname not in
            {"127.0.0.1", "::1", "localhost"} or parsed.username or
            parsed.password or parsed.query or parsed.fragment):
        raise ValueError("RPC health probe requires a plain local HTTP endpoint")
    record = {"id": "local-rpc", "url": url, "timeout_seconds": timeout}
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "getHealth"}).encode()
    request = Request(url, data=body, headers={"Content-Type": "application/json"})
    try:
        with build_opener(ProxyHandler({}), NoRedirect()).open(request, timeout=timeout) as response:
            raw = response.read(1024 * 1024 + 1)
            if len(raw) > 1024 * 1024:
                raise ValueError("health response exceeds limit")
            record.update(http_status=response.status, body=raw.decode("utf-8"))
            data = json.loads(record["body"])
            healthy = (isinstance(data, dict) and data.get("jsonrpc") == "2.0"
                       and data.get("id") == 1 and "error" not in data
                       and isinstance(data.get("result"), dict)
                       and data["result"].get("status") == "healthy")
            record["result"] = PASS if healthy else INCONCLUSIVE
    except (OSError, URLError, ValueError) as error:
        record.update(result=INCONCLUSIVE,
                      error=f"{type(error).__name__}: {error}")
        if isinstance(error, HTTPError):
            error.close()
    return record


def summarize(probes):
    """Preflight cannot produce campaign PASS or count a setup failure as a negative."""
    by_id = {probe["id"]: probe for probe in probes}
    backend = any(usable_backend(by_id.get(name, {}))
                  for name in ("docker", "podman", "native-core"))
    cli = by_id.get("stellar", {})
    cli_available = (cli.get("result") == PASS and cli.get("exit_code") == 0
                     and bool(cli.get("stdout", "").strip()))
    blockers = []
    if not backend:
        blockers.append("network/Core configuration failure: no available real Core backend")
    if not cli_available:
        blockers.append("provisioning unavailable: Stellar CLI missing or unusable")
    return {
        "phase": "infrastructure-preflight",
        "stage1_result": INCONCLUSIVE if blockers else NOT_TESTED,
        "backend_advertised_available": backend,
        "blockers": blockers,
        "campaign": {name: NOT_TESTED for name in (
            "provisioning", "control", "archival", "archive_observation",
            "restore", "application_invariant", "raw_verification",
            "negative", "retry", "repeatability")},
        "fresh_state_campaign_runs": 0,
        "independent_builds": 0,
        "note": "Availability/health probes do not execute the archival campaign.",
    }


def collect(project, stellar_cli, rpc_url):
    runtime = project / ".local/runtime"
    runtime.mkdir(parents=True, exist_ok=True, mode=0o700)
    runtime.chmod(0o700)
    config = project / ".local/config"
    config.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, "XDG_RUNTIME_DIR": str(runtime), "XDG_CONFIG_HOME": str(config)}
    commands = [
        ("docker", ["docker", "info", "--format", "{{json .ServerVersion}}"], None),
        ("podman", ["podman", "--root", str(project / ".local/container-root"),
                    "--runroot", str(project / ".local/container-runroot"),
                    "--storage-driver", "vfs", "info", "--format", "json"], env),
        ("native-core", ["stellar-core", "--version"], None),
        ("stellar", [stellar_cli, "--version"], None),
    ]
    probes = [run_probe(name, argv, cwd=project, env=override)
              for name, argv, override in commands]
    probes.append(local_rpc_health(rpc_url))
    return probes


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True,
                        help="New evidence directory (existing paths are refused)")
    parser.add_argument("--stellar-cli", default=shutil.which("stellar") or "stellar")
    parser.add_argument("--rpc-url", default="http://127.0.0.1:8000/soroban/rpc")
    args = parser.parse_args(argv)
    # Validate before starting commands or writing evidence.
    parsed = urlsplit(args.rpc_url)
    if (parsed.scheme != "http" or parsed.hostname not in {"127.0.0.1", "::1", "localhost"}
            or parsed.username or parsed.password or parsed.query or parsed.fragment):
        parser.error("--rpc-url must be a plain local HTTP endpoint")
    project = Path(__file__).resolve().parents[2]
    args.output.mkdir(parents=True, exist_ok=False)
    probes = collect(project, args.stellar_cli, args.rpc_url)
    report = summarize(probes)
    report["recorded_at"] = datetime.now(timezone.utc).isoformat()
    report["probes"] = probes
    report["source_hashes"] = {
        str(path.relative_to(project)): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted((project / "src").rglob("*.py"))
    }
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"stage1_result": report["stage1_result"],
                      "blockers": report["blockers"], "report": str(args.output / "report.json")},
                     indent=2))
    # No campaign was run, so never exit zero, even with an available backend.
    return 2


if __name__ == "__main__":
    sys.exit(main())
