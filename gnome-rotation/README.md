# gnome-shell-extension-builtin-screen-rotation

**A GNOME Shell extension that turns the built-in screens when the laptop stands
on its side, also with a keyboard, a touchpad or a mouse connected. It is made
for the two screens of the ASUS Zenbook Duo UX8406, but it operates on a laptop
of any model. Do not use it on a machine whose screen is mounted turned.**

GNOME turns a built-in screen only when no pointer device is connected, and it
turns one screen. This extension does it for all built-in screens.

## Which machines

Nothing in the extension is specific to the UX8406: it uses the screens that the
compositor marks as built-in, and takes their order from the layout. Only the
UX8406CA is tested; a report from another laptop is useful (refer to
[CONTRIBUTING.md](../CONTRIBUTING.md#report-your-results)).

The extension takes each built-in screen as mounted upright in the machine. Some
small tablets and convertibles have a screen that is mounted turned (the kernel
then reports a panel orientation). On such a machine the extension turns the
picture wrongly. The compositor (mutter) knows the panel orientation, but up to
GNOME 51 it gives it neither to extensions nor over D-Bus (`GetCurrentState`).

It needs GNOME Shell 45 to 51 (only 50 is tested) and `iio-sensor-proxy`, which
gives the orientation. On a machine without a built-in screen, the extension
does nothing.

## What it does

- Laptop on its left or right side: the built-in screens that are on are turned.
  Two screens are put side by side, in the order in which they lie.
- Laptop upright again: the layout from before comes back.
- Laptop upside down: no change.

An external monitor that is beside the built-in screens, right of them or below
them, moves to make room. Other monitors stay. The change is for the session
only; the saved `monitors.xml` does not change. With the built-in screens off,
the extension does nothing.

## Use

Install the package `gnome-shell-extension-builtin-screen-rotation`: the
[README](../README.md#installation) tells how to add the repository for your
distribution. You do not need the other packages. Log in again, then enable the
extension:

```sh
gnome-extensions enable builtin-screen-rotation@cceelen.github.io
```

The switch "Auto-rotate" in the quick settings turns the rotation on and off. It
is GNOME's setting `orientation-lock`. While it is off, the extension does not
use the sensor.

The switch shows only when `iio-sensor-proxy` gives the orientation and the
machine has a built-in screen. When GNOME turns the screen itself (no pointer
device connected), GNOME shows its own switch for the same setting, and this one
is hidden.

When no sensor gives the orientation, two buttons are in the place of the
switch. Each turns the built-in screens by 90 degrees, counterclockwise or
clockwise: left side up, upright, right side up. To try the buttons on a machine
with a sensor, stop the service:

```sh
sudo systemctl stop iio-sensor-proxy
```

## Test

```sh
meson test -C build --suite gnome-rotation
```

`rotation.js` has the logic and no GNOME imports; `sensor.js` calls
`net.hadess.SensorProxy`. For the files that the two extensions share, and for
an installation from the tree, refer to
[CONTRIBUTING.md](../CONTRIBUTING.md#an-extension-without-a-package).

`tests/rotate.js` follows the orientation with the code of the extension, from
outside GNOME Shell, and applies the layouts. Use it to try the rotation without
a new login; disable the extension first.

```sh
gjs -m tests/rotate.js 120
```
