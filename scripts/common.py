"""Small standard-library helpers for reproducible commands and saved logs."""

import datetime
import hashlib
import json
import shlex
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def relative(path):
    return str(Path(path).relative_to(ROOT))


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def new_log_dir(kind):
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
    path = ROOT / "artifacts" / "logs" / f"{kind}-{stamp}"
    path.mkdir(parents=True)
    return path


class Runner:
    def __init__(self, log_dir):
        self.log_dir = log_dir
        self.records = []

    def run(self, name, command, expected_exit=0):
        command = [str(arg) for arg in command]
        # Preserve the original exit code; do not pipe or truncate the process.
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
        log = self.log_dir / f"{name}.log"
        log.write_text(
            f"$ {shlex.join(command)}\nexit_code={result.returncode}\n\n"
            f"--- stdout ---\n{result.stdout}\n--- stderr ---\n{result.stderr}"
        )
        self.records.append(
            {
                "name": name,
                "command": command,
                "exit_code": result.returncode,
                "expected_exit": expected_exit,
                "log": relative(log),
            }
        )
        self.save()
        if result.returncode != expected_exit:
            raise RuntimeError(
                f"{name}: expected exit {expected_exit}, got {result.returncode}; "
                f"see {relative(log)}"
            )
        print(f"PASS {name}: exit={result.returncode}; {relative(log)}", flush=True)
        return result

    def save(self):
        (self.log_dir / "commands.json").write_text(
            json.dumps(self.records, indent=2) + "\n"
        )
