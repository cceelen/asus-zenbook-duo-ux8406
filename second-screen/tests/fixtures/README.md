# Test fixtures

Each directory is a small stand-in for `/sys`. A test copies one into a
temporary directory (see `tests/common/mod.rs`) and runs asus-ux8406-second-screen against the
copy, so the trees here are never written to.

The kernel's symbolic links in `class/backlight` and `bus/usb/devices` are not
stored: `assert_fs` follows links when it copies, which would turn them into
copies of the devices. The test helper creates them after the copy, one for
every backlight device below `devices` and one for every USB device below the
dock port's root hub, pointing at the device the way the kernel's do.

| Tree | What it stands in for |
| --- | --- |
| `ux8406ca-docked` | A UX8406CA with both panels' connectors and backlights, the firmware's screenpad backlight, and the keyboard on the dock port (`3-6`). The upper panel is at 253 of 400, the lower one at 170 of 400, the screenpad at 40 of 255. |
| `ux8406ca-panels-only` | A UX8406CA with both connectors (and the `card1-eDP-2-backlight` entry that is not a connector) but no backlights and no USB devices: keyboard not docked. |
| `ux8406ca-screenpad-only` | A UX8406CA whose only backlight is the firmware's screenpad one. |
| `ux8406ca-upper-panel-only` | A UX8406CA with the upper panel's connector only, and the keyboard on the dock port. |
| `ux8406ca-keyboard-on-cable` | A UX8406CA with both connectors and the keyboard's ids on port `3-2`, where the cable puts it. |
| `usb-other-product-on-dock-port` | Only a USB device on the dock port with the keyboard's vendor but another product id. |
| `usb-other-vendor-on-dock-port` | Only a USB device on the dock port with the keyboard's product id but another vendor. |
