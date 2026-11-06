#!/usr/bin/env python3
"""Build and run the host harness of the HID-BPF program.

The program's source is compiled UNCHANGED as host C, against fakes of the BPF
headers and of the kernel's functions (tests/host/shim/, first on the include
path). The tests (tests/host/test_*.c) call the program's entry points.

modes:
  plain     compile with the host C compiler and run
  sanitize  clang with AddressSanitizer and UndefinedBehaviorSanitizer
  coverage  clang source-based coverage; fails unless the program's file has
            100 % of lines, functions and branches (llvm-cov export)

usage: run_host.py MODE PROGRAM.bpf.c [WORKDIR]
"""

import json
import os
import pathlib
import shutil
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent / "host"
SHIM = HERE / "shim"
TESTS = sorted(HERE.glob("test_*.c")) + [HERE / "fake_kernel.c"]
# C23 by its earlier name: gcc 13 (openSUSE Leap 16.0) does not know -std=c23.
COMMON = ["-std=c2x", "-g", "-O0", "-Wall", "-Wextra", "-Wno-unused-parameter"]


def run(cmd, **kwargs):
    print("+", " ".join(str(c) for c in cmd), flush=True)
    return subprocess.run(cmd, check=True, **kwargs)


def tool(name):
    path = shutil.which(name)
    if path is None:
        sys.exit(f"run_host.py: {name} not found")
    return path


def compile_one(compiler, flags, source, obj):
    run([compiler, *flags, "-c", str(source), "-o", str(obj)])
    return str(obj)


def harness_flags(extra):
    # The quoted includes of the program resolve against its own directory
    # first, where there are no BPF headers; the shim comes first after that.
    return COMMON + ["-I", str(SHIM), "-I", str(HERE)] + list(extra)


def check_symbol_sizes(obj):
    """The loader fills the udev property variables of the program and reads
    the map's declaration: a size other than the declared one is a different
    interface."""
    out = subprocess.run(
        ["nm", "--print-size", str(obj)], capture_output=True, text=True, check=True
    ).stdout
    sizes = {
        ln.split()[3]: int(ln.split()[1], 16)
        for ln in out.splitlines()
        if len(ln.split()) == 4
    }
    # The map: the fakes lay its declaration out as byte arrays, the type
    # (array, 2), the entries (1), the key (4 bytes) and the value (2).
    wanted = {
        "UDEV_PROP_ASUS_UX8406_KBD_BACKLIGHT": 4,
        "UDEV_PROP_ASUS_UX8406_KBD_FN_LOCK": 4,
        "zenbook_duo_kbd_state": 2 + 1 + 4 + 2,
    }
    for name, size in wanted.items():
        if sizes.get(name) != size:
            sys.exit(f"run_host.py: {name} is {sizes.get(name)} bytes, not {size}")


def compile_harness(compiler, program, out, extra, program_extra=(), sizes=True):
    """Program and tests are separate translation units: only the program is
    instrumented for coverage."""
    flags = harness_flags(extra)
    objects = [
        compile_one(compiler, flags + list(program_extra), program, out / "program.o")
    ]
    if sizes:  # AddressSanitizer pads the variables
        check_symbol_sizes(objects[0])
    for source in TESTS:
        objects.append(compile_one(compiler, flags, source, out / (source.stem + ".o")))
    binary = out / "harness"
    run([compiler, *flags, *program_extra, *objects, "-o", str(binary)])
    return binary


def check_shadowing(compiler, program):
    """No header of the program may come from the vendored BPF headers."""
    result = subprocess.run(
        [compiler, *COMMON, "-I", str(SHIM), "-H", "-fsyntax-only", str(program)],
        capture_output=True,
        text=True,
        check=True,
    )
    included = [
        ln.lstrip(". ") for ln in result.stderr.splitlines() if ln.startswith(".")
    ]
    vendored = pathlib.Path(program).resolve().parent.parent / "include"
    stray = [h for h in included if pathlib.Path(h).resolve().is_relative_to(vendored)]
    if stray:
        sys.exit(f"run_host.py: vendored headers were used: {stray}")


def coverage(program, out):
    clang = tool("clang")
    profdata_tool, cov_tool = tool("llvm-profdata"), tool("llvm-cov")
    binary = compile_harness(
        clang,
        program,
        out,
        ["-Werror"],
        ["-fprofile-instr-generate", "-fcoverage-mapping"],
    )
    raw = out / "harness.profraw"
    run([binary], env={**os.environ, "LLVM_PROFILE_FILE": str(raw)})
    data = out / "harness.profdata"
    run([profdata_tool, "merge", "-sparse", str(raw), "-o", str(data)])
    report = run(
        [
            cov_tool,
            "export",
            "--summary-only",
            f"--instr-profile={data}",
            str(binary),
            str(program),
        ],
        stdout=subprocess.PIPE,
        text=True,
    ).stdout
    totals = json.loads(report)["data"][0]["totals"]
    bad = False
    for kind in ("lines", "functions", "branches"):
        entry = totals[kind]
        print(f"{kind}: {entry['covered']}/{entry['count']} ({entry['percent']:.2f} %)")
        if entry["count"] == 0 or entry["covered"] != entry["count"]:
            bad = True
    run([cov_tool, "report", f"--instr-profile={data}", str(binary), str(program)])
    if bad:
        sys.exit("run_host.py: coverage of the program is below 100 %")


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    mode, program = sys.argv[1], pathlib.Path(sys.argv[2]).resolve()
    out = (
        pathlib.Path(
            sys.argv[3]
            if len(sys.argv) > 3
            else os.environ.get("TMPDIR", "/tmp") + "/ux8406-bpf-host"
        )
        / mode
    )
    shutil.rmtree(out, ignore_errors=True)
    out.mkdir(parents=True)
    if mode == "plain":
        cc = os.environ.get("CC") or shutil.which("cc") or tool("clang")
        check_shadowing(cc, program)
        run([compile_harness(cc, program, out, [])])
    elif mode == "sanitize":
        clang = tool("clang")
        san = [
            "-fsanitize=address,undefined",
            "-fno-sanitize-recover=all",
            "-fno-omit-frame-pointer",
            "-Werror",
        ]
        check_shadowing(clang, program)
        run(
            [compile_harness(clang, program, out, san, sizes=False)],
            env={**os.environ, "ASAN_OPTIONS": "detect_leaks=1"},
        )
    elif mode == "coverage":
        coverage(program, out)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        sys.exit(
            f"run_host.py: step failed with status {error.returncode}\n"
            f"{(error.stderr or '').rstrip()}"
        )
