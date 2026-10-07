# asus-ux8406-keyboard-state

A helper for the detachable keyboard of the ASUS Zenbook Duo UX8406. It keeps
the keyboard light and the mode of the function row (F1–F12 or hotkeys) when the
keyboard changes between the dock, the cable and Bluetooth. It is part of the
package [asus-zenbook-duo-ux8406-keyboard-bpf](../keyboard-bpf/README.md).

## Why

The keyboard is a different HID device on each connection. It starts each
connection with its light off. The HID-BPF program of the keyboard is loaded for
each device and holds the light and the row mode only for that device. When the
device goes, udev-hid-bpf removes the program and its state.

## How it operates

udev runs the helper. There is no service.

| Event                              | Command       | Result                                              |
| ---------------------------------- | ------------- | --------------------------------------------------- |
| A HID device of the keyboard goes  | `save DEVICE` | writes the state of its program to a file in `/run` |
| A HID device of the keyboard comes | `load DEVICE` | gives the state to udev as properties of the device |

udev-hid-bpf gives the properties to the program when it loads it, and the
program sets the keyboard to them. The
[README of the program](../keyboard-bpf/README.md#what-it-does) lists the
properties.

When the keyboard goes onto the dock, the USB device comes at the same time as
the Bluetooth device goes (seen on the UX8406CA: less than 0.1 s between the
two). Thus `load` first reads the state of a connection that is still there, and
uses the file only when there is none.

The rule file `80-asus-ux8406-keyboard-state.rules` must sort before
`81-hid-bpf.rules` of udev-hid-bpf: the properties must be there before the
program is loaded, and the state must be read before the program is removed.

The state is in `/run/asus-ux8406-keyboard-state`. It is kept until the laptop
starts again.

## Safety

- The helper accepts only the names of the keyboard's HID devices.
- It writes only a state with values that the keyboard knows, and replaces the
  file in one step.
- It prints only the two properties. A file with other text is refused.
- It needs root to read the state of the program. udev runs it as root.

## Test

```sh
meson test -C build --suite asus-ux8406-keyboard-state
```

The tests run without the keyboard and without root. A directory stands in for
`/sys/fs/bpf`. They also compare the names of the properties, the name of the
state and the ids of the keyboard with the program and the udev rule.

The read of a state from the kernel has no test: it needs root and a loaded
program. It was tried on the UX8406CA.
