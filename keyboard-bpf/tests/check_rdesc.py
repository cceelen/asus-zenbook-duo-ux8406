#!/usr/bin/env python3
"""Check the report descriptors the HID-BPF program hands to the kernel.

Takes the constants and the byte arrays out of the program's source and holds
them against the descriptors captured from the keyboard (below: USB interface 4
and Bluetooth):
  - each capture has the size the program's probe asks for, and the signature
    of the hotkey collection at the offset the program uses;
  - the vendor collection the fixup re-states over Bluetooth is the captured one.
Then applies the arrays at those offsets, as the program's fixup does, and
walks the result item by item:
  - every item is complete and the collections are balanced;
  - the hotkey field has one usage for each value of its logical range;
  - report 0x5a is 5 input bytes and 15 feature bytes;
  - every other report keeps its size.
The statements of the fixup are not run here, only its constants and arrays.
Prints the fields of report 0x5a. Exit status 1 if anything is off.

usage: check_rdesc.py PROGRAM.bpf.c
"""

import pathlib
import re
import sys

USB_IF4 = bytes.fromhex(
    "0600ff0901a101854209061500 26ff0075089503b102854309061500 26ff0075089503b102"
    "0600ff854109051500 26ff007508960001b102c0"
    "0631ff0976a101855a09761500 26ff0075089505810285 5a09767508950fb102c0".replace(
        " ", ""
    )
)
BLUETOOTH = bytes.fromhex(
    "05010906a1018502050719e029e7150025017501950881029501750881039505"
    "7501050819012905910295017503910395067508150026ff00050719002aff00"
    "8100c00600ff0906a101850d0906150027ffff000075209501b10285dd09dd15"
    "0026ff007508953f810285de09de9102c0050c0901a101850319002aff021500"
    "26ff02751895018100c006e0ff0901a10185210900150026ff007508953f8102"
    "852009009102c00631ff0976a101855a0976150026ff00750895058102855a09"
    "767508950fb102c00600ff0901a10185420906150026ff0075089503b1028543"
    "0906150026ff0075089503b1020600ff85410905150026ff007508960001b102"
    "c0"
)


def c_array(source, name, defines):
    body = re.search(rf"{name}\[\]\s*=\s*\{{(.*?)\}};", source, re.DOTALL).group(1)
    body = re.sub(r"/\*.*?\*/", "", body, flags=re.DOTALL)
    out = []
    for token in body.replace("\n", " ").split(","):
        token = token.strip()
        if token:
            out.append(int(defines.get(token, token), 0))
    return bytes(out)


def items(rdesc):
    """Yield (tag byte without size, data value, raw size) for each short item."""
    pos = 0
    while pos < len(rdesc):
        prefix = rdesc[pos]
        size = (0, 1, 2, 4)[prefix & 3]
        if prefix == 0xFE or pos + 1 + size > len(rdesc):
            raise ValueError(f"bad item at {pos}")
        yield (
            prefix & 0xFC,
            int.from_bytes(rdesc[pos + 1 : pos + 1 + size], "little"),
            size,
        )
        pos += 1 + size


def report_sizes(rdesc):
    """{(kind, report id): bits}, and the fields of each report."""
    sizes, fields = {}, {}
    state = {"id": 0, "size": 0, "count": 0, "page": 0, "lmin": 0, "lmax": 0}
    usages, depth = [], 0
    for tag, value, size in items(rdesc):
        if tag == 0x04:
            state["page"] = value
        elif tag == 0x14:
            state["lmin"] = value
        elif tag == 0x24:
            state["lmax"] = value
        elif tag == 0x74:
            state["size"] = value
        elif tag == 0x94:
            state["count"] = value
        elif tag == 0x84:
            state["id"] = value
        elif tag in (0x08, 0x18, 0x28):
            usages.append(value if size == 4 else (state["page"] << 16) | value)
        elif tag == 0xA0:
            depth += 1
            usages = []
        elif tag == 0xC0:
            depth -= 1
            usages = []
        elif tag in (0x80, 0x90, 0xB0):
            kind = {0x80: "input", 0x90: "output", 0xB0: "feature"}[tag]
            key = (kind, state["id"])
            sizes[key] = sizes.get(key, 0) + state["size"] * state["count"]
            fields.setdefault(key, []).append(
                (
                    state["size"],
                    state["count"],
                    value,
                    state["lmin"],
                    state["lmax"],
                    usages,
                )
            )
            usages = []
        if depth < 0:
            raise ValueError("collection closed twice")
    if depth != 0:
        raise ValueError("collection left open")
    return sizes, fields


def enum_constants(source):
    """Return {name: value as string} for every enum constant in the source.

    A constant without "=" follows the one before it; a value is a number or
    the name of an earlier constant.
    """
    source = re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.DOTALL)
    constants = {}
    for body in re.findall(r"\benum\b[^{;]*\{(.*?)\}", source, re.DOTALL):
        value = -1
        for entry in body.split(","):
            entry = entry.strip()
            if not entry:
                continue
            name, _, expr = (part.strip() for part in entry.partition("="))
            value = int(constants.get(expr, expr), 0) if expr else value + 1
            constants[name] = str(value)
    return constants


def main(source_path):
    source = pathlib.Path(source_path).read_text()
    defines = enum_constants(source)
    signature = c_array(source, "kHotkeySignature", defines)
    fixed = c_array(source, "kFixedHotkeys", defines)
    restated = c_array(source, "kBtVendorCollection", defines)
    collection = int(defines["kHotkeyCollectionSize"], 0)
    report = int(defines["kHotkeyReportId"], 0)
    failed = False

    for name, prefix, original, tail in (
        ("usb", "Usb", USB_IF4, b""),
        ("bluetooth", "Bt", BLUETOOTH, restated),
    ):
        size = int(defines["k" + prefix + "RdescSize"], 0)
        offset = int(defines["k" + prefix + "HotkeyOffset"], 0)
        if len(original) != size:
            print(
                f"{name}: the captured descriptor has {len(original)} bytes, "
                f"the program expects {size}"
            )
            failed = True
        if original[offset : offset + len(signature)] != signature:
            print(f"{name}: the hotkey collection does not start at offset {offset}")
            failed = True
        if original[offset + collection :] != tail:
            print(f"{name}: what follows the hotkeys is not what the program re-states")
            failed = True
        new = original[:offset] + fixed + tail
        try:
            before, _ = report_sizes(original)
            after, fields = report_sizes(new)
        except ValueError as error:
            print(f"{name}: {error}")
            failed = True
            continue
        print(f"{name}: {len(original)} -> {len(new)} bytes, hotkeys at {offset}")
        if before != after:
            print(
                "  report sizes differ:",
                sorted(set(before.items()) ^ set(after.items())),
            )
            failed = True
        for kind in ("input", "feature"):
            for bits, count, flags, lmin, lmax, usages in fields.get(
                (kind, report), []
            ):
                names = " ".join(f"{u:08x}" for u in usages)
                print(
                    f"  0x{report:02x} {kind:<7} {count:2d} x {bits} bit, "
                    f"flags 0x{flags:02x}, logical {lmin}..{lmax}, usages {names}"
                )
                # Data, Array: the value picks one of the usages.
                if not flags & 3 and lmax - lmin + 1 != len(usages):
                    print(f"  {len(usages)} usages for {lmax - lmin + 1} values")
                    failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
