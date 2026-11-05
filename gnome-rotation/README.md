# gnome-shell-extension-builtin-screen-rotation

**A GNOME Shell extension that turns the built-in screens when the laptop stands
on its side or upside down, also with a keyboard, a touchpad or a mouse
connected. It is made for the two screens of the ASUS Zenbook Duo UX8406, but it
operates on a laptop of any model. Do not use it on a machine whose screen is
mounted turned. Each screen can also be turned by hand, on its own, from the
quick settings.**

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

It needs GNOME Shell 45 to 51 (only 50 is tested). For the sensor it needs
`iio-sensor-proxy`, which gives the orientation; the turn by hand does not need
it. On a machine without a built-in screen, the extension does nothing.

## What it does

- Laptop on its left or right side: the built-in screens that are on are turned.
  Two screens are put side by side, in the order in which they lie.
- Laptop upright again: the layout from before comes back.
- Laptop upside down: the built-in screens that are on are turned by 180
  degrees. Two screens go one below the other, the bottom one on top.

### Turn a screen by hand

The menu of "Auto-rotate" in the quick settings has a section for each built-in
screen: "Upper screen" and "Lower screen" for two screens, "Built-in screen" for
one. Each section has four buttons: upright, left side up, right side up and
upside down. The button of the present orientation is marked. Each screen can
have any orientation, also a different one than the other screen. With the upper
screen upright and the lower one upside down, two people who sit opposite each
other read the screen near to them the right way up.

- A turn by hand switches "Auto-rotate" off, so that the sensor does not undo
  it. The switch shows "Turned by hand".
- Switch "Auto-rotate" on again: all screens go back under the sensor and turn
  to the orientation of the laptop at once.
- The turns are for the session only. The saved setting of "Auto-rotate" does
  not change because of a turn by hand: after a new login the screens follow the
  laptop again. A screen lock does not end a turn by hand.
- The screens stay as they lie: two screens one below the other stay so, in the
  same order, and two screens side by side stay so. They are placed again from
  the top left corner of the two, with their new sizes. An external monitor
  beside them moves as for the sensor, and slides along the new edge of the two
  screens if one screen is narrower than the other. If that gives no layout that
  GNOME takes, all monitors go in one row.
- When the monitors change (the keyboard goes on or off the lower screen, a
  monitor is plugged in) and GNOME sets the layout again, each screen that is
  shown goes back to its orientation. A screen that is off keeps its orientation
  until it is back. If GNOME does not take the orientation of a screen, that
  screen is not held any more, and the error is in the journal.
- Switch "Auto-rotate" on again: screens that are turned differently get their
  place from the order of the built-in screens in GNOME: the upper screen is the
  first one.
- Not for a screen that is off, a screen that is mirrored, or a monitor without
  a mode: the buttons of such a screen do nothing.

An external monitor that is beside the built-in screens, right of them or below
them, moves to make room. Other monitors stay. The change is for the session
only; the saved `monitors.xml` does not change. With the built-in screens off,
the extension does nothing.

## Use

### Check the sensor first

The extension does not choose a sensor. It takes the orientation that
`iio-sensor-proxy` gives, the same one that GNOME's own rotation uses. Which
accelerometer that is, and which way is up, is set for the whole system by udev,
not in the extension. Check it before you install:

```sh
monitor-sensor
```

Stand the laptop on its left side, on its right side, and upright again. Each
turn prints a line `Accelerometer orientation changed:` with `left-up`,
`right-up` or `normal`.

- **The orientation matches the laptop:** nothing to set. Install the extension.
- **No accelerometer:** `monitor-sensor` prints `No accelerometer`. The
  extension then shows two buttons that turn the screens by hand.
- **The orientation is wrong** (for example `right-up` when the left side is
  up): the accelerometer is mounted turned, and its mount matrix is wrong or
  missing. Correct it with a local hwdb entry, below.
- **Two accelerometers** (one in the screen, one in the base):
  `iio-sensor-proxy` takes the one with `ACCEL_LOCATION=display`. If it takes
  the one in the base, set `ACCEL_LOCATION=base` for that one in the same entry.

To see the sensors and what udev sets for them:

```sh
for d in /sys/bus/iio/devices/iio:device*; do
  echo "$d: $(cat "$d/name") $(cat "$d/../modalias" 2>/dev/null)"
  udevadm info -q property "$d" | grep -E '^ACCEL_'
done
cat /sys/class/dmi/id/modalias
```

A local entry goes in `/etc/udev/hwdb.d/61-sensor-local.hwdb`. The first line
matches the sensor by the modalias of its parent and the machine by its DMI
string, both from the commands above; the format and the meaning of the matrix
are explained at the top of systemd's
[60-sensor.hwdb](https://github.com/systemd/systemd/blob/main/hwdb.d/60-sensor.hwdb).
The form of an entry, for a sensor turned by 90 degrees:

```text
sensor:modalias:PARENT-MODALIAS*:dmi:*:svnVENDOR:pnPRODUCT:*
 ACCEL_MOUNT_MATRIX=0, -1, 0; 1, 0, 0; 0, 0, 1
```

Put in the modalias of the parent and the vendor and product of your machine
(the `svn` and `pn` parts of its DMI string), and try the matrix with
`monitor-sensor` until the three orientations are right. Then load it (as root):

```sh
systemd-hwdb update
udevadm trigger -v -p DEVNAME=/dev/iio:deviceX
systemctl restart iio-sensor-proxy
```

Where the entry is right for every machine of the model, it belongs in systemd:
the comments of `60-sensor.hwdb` say how to send it.

### Install

Install the package `gnome-shell-extension-builtin-screen-rotation`: the
[README](../README.md#installation) tells how to add the repository for your
distribution. You do not need the other packages. Log in again, then enable the
extension:

```sh
gnome-extensions enable builtin-screen-rotation@cceelen.github.io
```

The switch "Auto-rotate" in the quick settings turns the rotation on and off. It
is GNOME's setting `orientation-lock`. While it is off, the extension does not
use the sensor. Its menu turns each screen by hand
([Turn a screen by hand](#turn-a-screen-by-hand)).

The switch shows only when `iio-sensor-proxy` gives the orientation and the
machine has a built-in screen. When GNOME turns the screen itself (no pointer
device connected), GNOME shows its own switch for the same setting, and this one
is hidden.

When no sensor gives the orientation, the item is no switch: it opens the menu
for each screen, and two buttons are next to it. Each button turns all built-in
screens by 90 degrees, counterclockwise or clockwise: left side up, upright,
right side up. To try the switch on a machine with a sensor, or the buttons,
stop the service:

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
