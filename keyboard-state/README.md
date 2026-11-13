# asus-ux8406-keyboard-state

**A helper for the detachable keyboard of the ASUS Zenbook Duo UX8406. It keeps
the keyboard light and the mode of the function row (F1–F12 or hotkeys) when the
keyboard changes between the dock, the cable and Bluetooth, and switches the
light off while the laptop sleeps. udev and systemd run it; there is no service
and nothing to set.**

It is part of the package
[asus-zenbook-duo-ux8406-keyboard-bpf](../keyboard-bpf/README.md). The HID-BPF
program of that package holds the light and the row mode for one HID device
only, and the keyboard is a different HID device on each connection: when the
device goes, udev-hid-bpf removes the program and its state.

## How it operates

| Event                              | Command          | Result                                              |
| ---------------------------------- | ---------------- | --------------------------------------------------- |
| A HID device of the keyboard goes  | `save DEVICE`    | writes the state of its program to a file in `/run` |
| A HID device of the keyboard comes | `load DEVICE`    | gives the state to udev as properties of the device |
| The laptop is about to sleep       | `sleep pre ...`  | switches the light off                              |
| The laptop has woken up            | `sleep post ...` | sets the light to the level that the program holds  |

udev-hid-bpf gives the properties to the program when it loads it, and the
program sets the keyboard to them. The
[README of the program](../keyboard-bpf/README.md#state-at-a-connection) lists
the properties.

When the keyboard goes onto the dock, the USB device comes at the same time as
the Bluetooth device goes (seen on the UX8406CA: less than 0.1 s between the
two). Thus `load` first reads the state of a connection that is still there, and
uses the file only when there is none.

The rule file `80-asus-ux8406-keyboard-state.rules` must sort before
`81-hid-bpf.rules` of udev-hid-bpf: the properties must be there before the
program is loaded, and the state must be read before the program is removed.

## Sleep

The keyboard keeps its light on while the laptop sleeps. A hook in
`system-sleep` of systemd (`asus-ux8406-keyboard-state`) runs `sleep` with the
words that systemd gives it: `pre` or `post`, and the kind of sleep, which is
not used. Before the sleep, the helper sends the report that sets the light to 0
to the hidraw node of each HID device of the keyboard that has the program.
After the sleep, it sends the level that the state map of the program holds. The
map is not changed: if the keyboard goes while the laptop sleeps, `save` writes
the true level, and the next connection starts with it.

A failure for one device does not stop the others. The helper then reports the
first failure and exits with an error. systemd logs it and goes on with the
sleep.

The state is in `/run/asus-ux8406-keyboard-state`. It is kept until the laptop
starts again.

## Safety

- The helper accepts only the names of the keyboard's HID devices.
- It writes only a state with values that the keyboard knows, and replaces the
  file in one step.
- It prints only the two properties. A file with other text is refused.
- It needs root to read the state of the program and to open a hidraw node. udev
  and systemd run it as root.
- `sleep` sends one report with one level to the devices that have the program.
  It takes the level only from the map, after a check of the value.

## Test

```sh
meson setup build -Dcargo_tests=enabled
meson test -C build --suite asus-ux8406-keyboard-state
```

The tests run without the keyboard and without root. A directory stands in for
`/sys/fs/bpf`, `/sys` and `/dev`, and a fake takes the reports. They also
compare the names of the properties, the name of the state and the ids of the
keyboard with the program and the udev rule.

The read of a state from the kernel has no test: it needs root and a loaded
program. It was tried on the UX8406CA. Run as root, a test makes a virtual HID
device with `/dev/uhid`, sends the report to its hidraw node with the real
sender, and checks the bytes that the device gets.
