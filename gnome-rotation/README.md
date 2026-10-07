# gnome-shell-extension-builtin-screen-rotation

A GNOME Shell extension that turns the built-in screens when the laptop stands
on its side. It is made for the two screens of the ASUS Zenbook Duo UX8406.

GNOME turns a built-in screen only when no pointer device is connected, and it
turns one screen. This extension does it for all built-in screens, also with a
keyboard, a touchpad or a mouse connected.

Nothing in it is specific to the UX8406: it uses the screens that the compositor
marks as built-in, and takes their order from the layout. Only the UX8406CA is
tested.

The extension takes each built-in screen as mounted upright in the machine. Some
small tablets and convertibles have a screen that is mounted turned (the kernel
then reports a panel orientation). On such a machine the extension turns the
picture wrongly: do not use it there. The compositor (mutter) knows the panel
orientation, but up to GNOME 51 it gives it neither to extensions nor over D-Bus
(`GetCurrentState`).

## What it does

- Laptop on its left or right side: the built-in screens that are on are turned.
  Two screens are put side by side, in the order in which they lie.
- Laptop upright again: the layout from before comes back.
- Laptop upside down: no change.

An external monitor that is beside the built-in screens, right of them or below
them, moves to make room. Other monitors stay. The change is for the session
only; the saved `monitors.xml` does not change. With the built-in screens off,
the extension does nothing.

The orientation comes from `iio-sensor-proxy`. On a machine without a built-in
screen, the extension does nothing.

GNOME Shell 45 to 51 is declared. Only GNOME Shell 50 is tested.

## Use

After the installation, log in again. GNOME Shell on Wayland finds a new
extension only when it starts.

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

The tests use node and need no GNOME session. `rotation.js` has the logic and no
GNOME imports; `sensor.js` calls `net.hadess.SensorProxy`. `state.js` and
`display.js` are links to `../gnome-common`: the two extensions of this project
use the same files, and the build installs a copy in each. The tests of the two
extensions also share their fixtures: `../gnome-common/tests`.

`tests/rotate.js` follows the orientation with the code of the extension, from
outside GNOME Shell, and applies the layouts. Use it to try the rotation without
a new login; disable the extension first.

```sh
gjs -m tests/rotate.js 120
```

## Install for one user without a package

```sh
meson setup build --prefix ~/.local -Dscreen=false -Dguard=false -Dkeyboard=false -Dgnome=false
meson install -C build
```

Remove the packaged extension first: it has the same UUID.
