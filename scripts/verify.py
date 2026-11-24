#!/usr/bin/env python3
"""Run the host example, metadata-only compilation, and negative type checks."""

import json
import sys

from build_sdk import build_sdk
from common import ROOT, Runner, new_log_dir, relative

CASES = {
    "lifetime_escape": (None, "lifetime may not live long enough"),
    "use_after_move": ("E0382", "borrow of moved value"),
    "move_while_borrowed": ("E0505", "cannot move out of"),
    "provider_lifetime": ("E0308", "mismatched types"),
}


def verify():
    runner = Runner(new_log_dir("verify"))
    report_path = ROOT / "artifacts" / "verification.json"
    # Invalidate the previous summary before running, so a failure is not hidden.
    report_path.write_text(json.dumps({"status": "running", "logs": relative(runner.log_dir)}) + "\n")
    try:
        runner.run("format", ["cargo", "fmt", "--all", "--", "--check"])
        runner.run("workspace-build", ["cargo", "build", "--workspace", "--locked", "--offline"])
        runner.run("host-run", ["cargo", "run", "--quiet", "--locked", "--offline", "-p", "firmware-sim"])
        output, flags, manifest = build_sdk(runner)

        for name, (expected_code, expected_message) in CASES.items():
            result = runner.run(
                name,
                [
                    "rustc", f"tests/compile-fail/{name}.rs",
                    f"--crate-name={name}", "--crate-type=rlib", "--error-format=json",
                ]
                + flags
                + [
                    "--extern", f"fw_api={relative(output / 'libfw_api.rmeta')}",
                    "-L", f"dependency={relative(output)}",
                    f"--emit=obj={relative(runner.log_dir / (name + '.o'))}",
                ],
                expected_exit=1,
            )
            diagnostics = []
            for line in result.stderr.splitlines():
                try:
                    item = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if item.get("$message_type") == "diagnostic" and item.get("level") == "error":
                    diagnostics.append(item)
            matched = any(
                expected_message in item.get("message", "")
                and (expected_code is None or (item.get("code") or {}).get("code") == expected_code)
                for item in diagnostics
            )
            if not matched:
                raise RuntimeError(f"{name}: failed for an unexpected reason; see saved diagnostic")
            print(f"CONFIRMED {name}: expected Rust type error", flush=True)

        report = {
            "status": "passed",
            "compiler": manifest["rustc_verbose"],
            "target": manifest["target"],
            "positive_checks": ["format", "workspace-build", "host-run", "sdk-api", "plugin-from-rmeta"],
            "negative_checks": list(CASES),
            "scope": "Separate Rust crates and metadata-only object compilation. Host execution uses static linkage. No runtime ELF loading.",
            "commands": runner.records,
        }
        report_path.write_text(json.dumps(report, indent=2) + "\n")
        print(f"PASS all checks; {relative(report_path)}", flush=True)
        return 0
    except (OSError, ValueError, RuntimeError) as error:
        report_path.write_text(json.dumps({
            "status": "failed", "error": str(error), "commands": runner.records,
        }, indent=2) + "\n")
        print(f"FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(verify())
