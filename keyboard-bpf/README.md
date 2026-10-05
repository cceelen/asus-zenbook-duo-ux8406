# asus-zenbook-duo-ux8406-keyboard-bpf

A HID-BPF program for the detachable keyboard of the ASUS Zenbook Duo UX8406,
and the hwdb entry with which [udev-hid-bpf] loads it when the keyboard
connects.

| Keyboard | Bus       | Id          | State      |
| -------- | --------- | ----------- | ---------- |
| UX8406CA | USB       | `0b05:1bf2` | tested     |
| UX8406CA | Bluetooth | `0b05:1bf3` | tested     |
| UX8406MA | USB       | `0b05:1b2c` | not tested |

The program attaches only if the report descriptor of the keyboard is that of
the UX8406CA keyboard.

## What it does

| Key                             | Result                                          |
| ------------------------------- | ----------------------------------------------- |
| F4                              | keyboard light on and off (the program does it) |
| F5, F6                          | display brightness down, up                     |
| F9                              | microphone mute                                 |
| F11 (emoji)                     | `KEY_PROG1`                                     |
| F12 (MyASUS)                    | `KEY_F13`                                       |
| F8 (swap windows)               | `KEY_F19`                                       |
| Key right of F12 (lower screen) | `KEY_F18`                                       |
| Fn+Esc                          | switches the row between F1–F12 and the hotkeys |

The volume keys and the display switch (F7) operate without this program.

Each connection starts in F1–F12 mode. The dock and Bluetooth are two HID
devices and do not share the mode.

The comment at the top of the source gives the protocol of the keyboard.

## Requirements

- Linux with HID-BPF (`CONFIG_HID_BPF`) and BTF, [udev-hid-bpf] and udev.
- Linux 6.11 or later by design. Kernels before 7.0 have the kfunc
  `bpf_wq_set_callback` as `bpf_wq_set_callback_impl`; `include/wq_compat.h`
  uses the one that is there. Only Linux 7.2 is tested.
- To build: clang with the BPF target, the libbpf headers and the kernel headers
  of Linux 6.10 or later.

## Test

```sh
meson test -C build --suite keyboard-bpf
```

- `rdesc` compares the descriptors in the source with those captured from the
  keyboard.
- `hwdb` makes sure that each hwdb entry names the object that the build makes.
  With a wrong name the program does not load, and there is no message.
- `inspect` lets udev-hid-bpf read the object. It loads nothing. It runs only if
  udev-hid-bpf is installed.

## Licence and headers

GPL-2.0-only. The kernel loads a HID-BPF program only with a GPL-compatible
licence.

- `include/hid_bpf.h`, `hid_bpf_helpers.h`, `hid_report_descriptor_helpers.h`,
  `hid_bpf_async.h`: from `drivers/hid/bpf/progs` of Linux 7.2, not changed,
  copyright Benjamin Tissoires.
- `include/vmlinux.h`, `include/wq_compat.h`: written for this program.

## Findings for upstream

For `drivers/hid/hid-asus.c` (refer to
[Upstream fixes](../README.md#upstream-fixes)):

- After the host sends a command to the keyboard, the keyboard does not switch
  the function row by itself. A driver that sets the keyboard light must also
  handle Fn+Esc.
- On Bluetooth the keyboard refuses the light report if it does not have the
  declared 16 bytes.

For udev-hid-bpf: it pins each map below the directory of the device, and a map
that is pinned by name makes the load fail. Thus a program cannot pass state
from the USB device of the keyboard to its Bluetooth device.

[udev-hid-bpf]: https://gitlab.freedesktop.org/libevdev/udev-hid-bpf
