"""Find the smallest thread stack the stack-depth regression test passes on.

Builds the `runtime_tests` test binary (debug unless --release) and bisects
`LURQ_STACK_TEST_BYTES` for
`runtime::stack_depth::editor_sized_rebuild_from_an_input_event_fits_a_one_mebibyte_stack`.
A stack overflow aborts the binary, so every probe runs in its own process.

    python scripts/stack-depth-probe.py [--features a,b] [--step 16384]
"""

import argparse
import json
import os
import subprocess
import sys

TEST = "runtime::stack_depth::editor_sized_rebuild_from_an_input_event_fits_a_one_mebibyte_stack"


def build(features, release):
    command = ["cargo", "test", "-p", "lurq", "--test", "runtime_tests", "--no-run", "--message-format=json"]
    if features:
        command += ["--features", features]
    if release:
        command.append("--release")
    output = subprocess.run(command, check=True, capture_output=True, text=True).stdout
    for line in output.splitlines():
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact" and message.get("executable") and message["target"]["name"] == "runtime_tests":
            return message["executable"]
    sys.exit("runtime_tests executable not found")


def passes(executable, stack):
    env = dict(os.environ, LURQ_STACK_TEST_BYTES=str(stack))
    result = subprocess.run([executable, "--exact", TEST, "--test-threads=1"], env=env, capture_output=True, text=True)
    return result.returncode == 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--features", default="")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--step", type=int, default=16 * 1024)
    parser.add_argument("--max", type=int, default=64 * 1024 * 1024)
    args = parser.parse_args()

    executable = build(args.features, args.release)
    low, high = 0, args.max
    if not passes(executable, high):
        sys.exit(f"fails even with {high} bytes")
    while high - low > args.step:
        middle = (low + high) // 2 // args.step * args.step
        if middle <= low:
            break
        if passes(executable, middle):
            high = middle
        else:
            low = middle
    print(f"smallest passing stack: {high} bytes ({high / 1024:.0f} KiB); fails at {low} bytes")


if __name__ == "__main__":
    main()
