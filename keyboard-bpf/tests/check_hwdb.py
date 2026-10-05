#!/usr/bin/env python3
"""Check that the hwdb entries load the object the build produces.

udev-hid-bpf loads whatever file name a HID_BPF_* property of the matching
hwdb entry gives; a name that does not exist in firmware/hid/bpf makes the
program silently not load. So every such property in the hwdb file has to
name the object file the build produces, and every entry has to enable
HID-BPF.

usage: check_hwdb.py FILE.hwdb OBJECT-NAME
Exit status 1 if anything is off.
"""

import sys


def entries(text):
    """Yield (match line, [property lines]) for each hwdb entry."""
    match, props = None, []
    for line in text.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            if match is not None and not line.strip():
                yield match, props
                match, props = None, []
            continue
        if line[0] not in " \t":
            if match is not None:
                yield match, props
            match, props = line.strip(), []
        elif match is not None:
            props.append(line.strip())
    if match is not None:
        yield match, props


def main(path, obj):
    with open(path, encoding="utf-8") as handle:
        found = list(entries(handle.read()))
    failed = not found
    if not found:
        print(f"{path}: no entry")
    for match, props in found:
        names = [p.split("=", 1)[1] for p in props if p.startswith("HID_BPF_")]
        if not names:
            print(f"{match}: no HID_BPF_ property")
            failed = True
        for name in names:
            if name != obj:
                print(f"{match}: names {name}, the build produces {obj}")
                failed = True
        if ".HID_BPF=1" not in props:
            print(f"{match}: .HID_BPF=1 is missing")
            failed = True
    if not failed:
        print(f"{path}: {len(found)} entries, all name {obj}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2]))
