#!/usr/bin/env python3
"""Emit a shared Rust SDK and compile a plugin using only its .rmeta file."""

import argparse
import json
import re
import sys

from common import ROOT, Runner, new_log_dir, relative, sha256


def build_sdk(runner, target=None, relocation_model=None):
    compiler = runner.run("rustc-version", ["rustc", "--version", "--verbose"]).stdout
    host = next(line.split(": ", 1)[1] for line in compiler.splitlines() if line.startswith("host: "))
    target = target or host
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*", target):
        raise ValueError("--target must be a built-in target triple, not a JSON path")
    output = ROOT / "artifacts" / "sdk" / target
    output.mkdir(parents=True, exist_ok=True)

    flags = [
        "--edition=2024",
        "--target", target,
        "-C", "panic=abort",
        "-C", "codegen-units=1",
    ]
    if relocation_model:
        flags += ["-C", f"relocation-model={relocation_model}"]
    metadata = output / "libfw_api.rmeta"
    library = output / "libfw_api.rlib"
    plugin_object = output / "plugin.o"

    runner.run(
        "sdk-api",
        ["rustc", "crates/fw-api/src/lib.rs", "--crate-name=fw_api", "--crate-type=rlib"]
        + flags
        + [f"--emit=metadata={relative(metadata)},link={relative(library)}"],
    )
    runner.run(
        "plugin-from-rmeta",
        ["rustc", "application/src/lib.rs", "--crate-name=plugin", "--crate-type=rlib"]
        + flags
        + [
            "--extern", f"fw_api={relative(metadata)}",
            "-L", f"dependency={relative(output)}",
            f"--emit=obj={relative(plugin_object)}",
        ],
    )

    magic = plugin_object.read_bytes()[:4]
    if magic == b"\x7fELF":
        object_format = "ELF relocatable object (not a final firmware image)"
    elif magic in (b"\xcf\xfa\xed\xfe", b"\xfe\xed\xfa\xcf"):
        object_format = "Mach-O object (host demonstration, not ELF)"
    else:
        object_format = "target-native object; inspect before selecting a loader"

    # This manifest is a build record. A real loader does NOT consume it here.
    contract = {
        "schema": 1,
        "rustc_verbose": compiler.strip(),
        "target": target,
        "rustc_flags": flags,
        "api_source_sha256": sha256(ROOT / "crates/fw-api/src/lib.rs"),
        "rmeta_sha256": sha256(metadata),
        "rlib_sha256": sha256(library),
    }
    import hashlib
    contract_bytes = json.dumps(contract, sort_keys=True, separators=(",", ":")).encode()
    manifest = {
        **contract,
        "sdk_build_id": hashlib.sha256(contract_bytes).hexdigest(),
        "plugin_object_sha256": sha256(plugin_object),
        "plugin_object_format": object_format,
        "scope": "SDK generation and object compilation; no ELF loader or runtime SDK validation",
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"SDK: {relative(output)}", flush=True)
    print(f"Object: {object_format}", flush=True)
    return output, flags, manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", help="Installed built-in Rust target; defaults to host")
    parser.add_argument("--relocation-model", choices=["static", "pic"], help="Defaults to the target's default")
    args = parser.parse_args()
    runner = Runner(new_log_dir("sdk"))
    try:
        build_sdk(runner, args.target, args.relocation_model)
    except (OSError, ValueError, RuntimeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
