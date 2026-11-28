# asus-ux8406-second-screen

**A helper for the lower screen of the ASUS Zenbook Duo UX8406. It sets the
screen to off while the keyboard lies on it, and keeps it at the brightness of
the upper screen. udev runs it; there is no service and nothing to set.**

For the installation, refer to the [README](../README.md#installation).

## What it does

`asus-ux8406-second-screen dock`:

- It looks for the keyboard on the dock port: USB device `0b05:1bf2` (UX8406CA)
  or `0b05:1b2c` (UX8406MA) on port 6 of the controller `0000:00:14.0`.
- It writes `off` or `detect` to the status attribute of the DRM connector
  `cardN-eDP-2`. The desktop sees a monitor that is unplugged or plugged in and
  applies its saved layout.
- With the keyboard on the cable or on Bluetooth, the lower screen stays on.

`asus-ux8406-second-screen brightness`:

- It copies the brightness of the upper screen to the lower screen.
- Each desktop also keeps a brightness of its own for each screen, and puts it
  back when a screen returns. Thus the two screens can differ after the keyboard
  leaves the dock or after a resume. The
  [KDE keys program](../kde-keys/README.md) and the
  [GNOME extension](../gnome-keys/README.md) keep the lower screen equal to the
  upper screen through the desktop. The helper still copies the value in sysfs,
  for the screens outside a desktop session, such as the login screen. With the
  equal ranges of the two panels, the helper and the desktop set the same value.

`asus-ux8406-second-screen release` gives the lower screen back to the kernel's
own detection. The packages run it when they are removed.

The udev rules in `udev/` run `dock` when the keyboard arrives on or leaves the
dock port and when the connector appears, and `brightness` when the backlight of
the upper screen changes.

On a machine whose DMI product name does not contain `UX8406` or `UX8407`, the
helper does nothing. What is tested on the hardware:
[Test reports](../README.md#test-reports).

## Test

```sh
meson setup build -Dcargo_tests=enabled
meson test -C build --suite asus-ux8406-second-screen
```

The tests run the program on fake sysfs trees in `tests/fixtures/`. The option
`-r DIR` points the program at such a tree.

## Upstream

- Brightness: a kernel series makes brightness a property of each display
  connector. Refer to [Upstream fixes](../README.md#upstream-fixes). With that
  kernel and a compositor that uses the property, the kernel refuses writes to
  the backlight attribute, and `brightness` does nothing.
- Dock state: no upstream work. The kernel does not report the dock state.
